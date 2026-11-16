use crate::chrome::default_chrome_path;
use crate::config::Config;
use crate::db::{Db, ImageMapRow, PostStatsRow};
use crate::error::{Error, Result};
use crate::offload::OffloadEngine;
use axum::{extract::{Path, Query, State}, response::IntoResponse, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub engine: Option<Arc<OffloadEngine>>,
    pub db_ok: bool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    db: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MetricsResponse {
    status: &'static str,
    pid: u32,
    db: bool,
    uptime_seconds: u64,
    memory_mb: f64,
    cpu_percent: f64,
    total_images_offloaded: u64,
    total_bytes_saved: u64,
    active_tasks: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsResponse {
    status: &'static str,
    total_images: u64,
    offloaded_images: u64,
    failed_images: u64,
    pending_images: u64,
    replaced_images: u64,
    total_bytes_original: u64,
    total_bytes_offloaded: u64,
    total_bytes_saved: u64,
    total_posts_processed: u64,
    last_processed_post_id: Option<i64>,
    last_processed_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImageResponse {
    id: i64,
    origin_url: String,
    blogger_url: Option<String>,
    blogger_image_id: Option<String>,
    wp_post_id: i64,
    wp_attachment_id: Option<i64>,
    mime_type: Option<String>,
    file_size: Option<i64>,
    original_size_bytes: Option<i64>,
    offload_size_bytes: Option<i64>,
    width: Option<i32>,
    height: Option<i32>,
    status: String,
    error_message: Option<String>,
    redirect_active: bool,
    replaced_at: Option<DateTime<Utc>>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PostResponse {
    id: i64,
    title: Option<String>,
    status: Option<String>,
    total_images: i64,
    offloaded_images: i64,
    replaced_images: i64,
    blogger_post_id: Option<String>,
    blogger_post_url: Option<String>,
    blogger_blog_id: Option<String>,
    linked_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct LogEntry {
    timestamp: String,
    level: String,
    subsystem: String,
    message: String,
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<u32>,
    offset: Option<u32>,
    status: Option<String>,
}

pub fn create_router(state: AppState, dist_dir: PathBuf) -> Router {
    let index = dist_dir.join("index.html");
    let static_files = ServeDir::new(dist_dir.clone()).not_found_service(ServeFile::new(index));
    let api_routes = Router::new()
        .route("/health", get(health))
        .route("/api/health", get(health))
        .route("/api/metrics", get(metrics))
        .route("/api/status", get(metrics))
        .route("/api/stats", get(stats))
        .route("/api/images", get(list_images))
        .route("/api/posts", get(list_posts))
        .route("/api/logs", get(list_logs))
        .route("/api/audits", get(list_audits))
        .route("/api/config", get(get_config))
        .route("/api/posts/:id/process", post(process_post))
        .with_state(state.clone());

    // Register ALL routes directly - API routes first, then static files as fallback
    Router::new()
        .merge(api_routes)
        .fallback_service(static_files)
        .layer(CorsLayer::permissive())
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(HealthResponse { status: "ok", service: "wp-offload-daemon", db: state.db_ok })
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.as_ref();
    let db = if let Some(e) = engine { e.db() } else { return Json(MetricsResponse { status: "error", pid: std::process::id(), db: false, uptime_seconds: 0, memory_mb: 0.0, cpu_percent: 0.0, total_images_offloaded: 0, total_bytes_saved: 0, active_tasks: 0 }); };

    // Get basic stats from DB
    let stats = db.get_stats().await.unwrap_or_default();

    Json(MetricsResponse {
        status: "ok",
        pid: std::process::id(),
        db: state.db_ok,
        uptime_seconds: 0, // Would need process start time tracking
        memory_mb: 0.0, // Would need system info
        cpu_percent: 0.0,
        total_images_offloaded: stats.get("replaced").and_then(|v| v.as_u64()).unwrap_or(0),
        total_bytes_saved: 0, // Would need to calculate
        active_tasks: 0,
    })
}

async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    let engine = state.engine.as_ref();
    let db = if let Some(e) = engine { e.db() } else { return Json(StatsResponse { status: "error", total_images: 0, offloaded_images: 0, failed_images: 0, pending_images: 0, replaced_images: 0, total_bytes_original: 0, total_bytes_offloaded: 0, total_bytes_saved: 0, total_posts_processed: 0, last_processed_post_id: None, last_processed_at: None }); };

    let stats = db.get_stats().await.unwrap_or_default();

    Json(StatsResponse {
        status: "ok",
        total_images: stats.get("total").and_then(|v| v.as_u64()).unwrap_or(0),
        offloaded_images: stats.get("uploaded").and_then(|v| v.as_u64()).unwrap_or(0) + stats.get("replaced").and_then(|v| v.as_u64()).unwrap_or(0),
        failed_images: stats.get("failed").and_then(|v| v.as_u64()).unwrap_or(0),
        pending_images: stats.get("pending").and_then(|v| v.as_u64()).unwrap_or(0) + stats.get("uploading").and_then(|v| v.as_u64()).unwrap_or(0) + stats.get("replacing").and_then(|v| v.as_u64()).unwrap_or(0),
        replaced_images: stats.get("replaced").and_then(|v| v.as_u64()).unwrap_or(0),
        total_bytes_original: 0,
        total_bytes_offloaded: 0,
        total_bytes_saved: 0,
        total_posts_processed: 0,
        last_processed_post_id: None,
        last_processed_at: None,
    })
}

async fn list_images(State(state): State<AppState>, Query(q): Query<ListQuery>) -> impl IntoResponse {
    let engine = state.engine.as_ref();
    let db = if let Some(e) = engine { e.db() } else { return Json(vec![]); };

    let limit = q.limit.unwrap_or(50) as usize;
    let offset = q.offset.unwrap_or(0) as usize;
    let status_filter = q.status;

    match db.list_images(limit, offset, status_filter).await {
        Ok(images) => Json(images.into_iter().map(|im| ImageResponse {
            id: im.id,
            origin_url: im.origin_url,
            blogger_url: im.blogger_url,
            blogger_image_id: im.blogger_image_id,
            wp_post_id: im.post_id.unwrap_or(0),
            wp_attachment_id: im.attachment_id,
            mime_type: im.mime_type,
            file_size: im.original_size_bytes.or(im.offload_size_bytes),
            original_size_bytes: im.original_size_bytes,
            offload_size_bytes: im.offload_size_bytes,
            width: im.width,
            height: im.height,
            status: im.status,
            error_message: im.error_message,
            redirect_active: im.redirect_active != 0,
            replaced_at: im.replaced_at.and_then(|s: String| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
            created_at: im.created_at.unwrap_or_default(),
            updated_at: im.updated_at.unwrap_or_default(),
        }).collect()),
Err(e) => {
            tracing::error!("list_images failed: {}", e);
            Json(Vec::<ImageResponse>::new())
        }
    }
}

async fn list_posts(State(state): State<AppState>, Query(q): Query<ListQuery>) -> impl IntoResponse {
    let engine = state.engine.as_ref();
    let db = if let Some(e) = engine { e.db() } else { return Json(vec![]); };

    let limit = q.limit.unwrap_or(50) as usize;
    let offset = q.offset.unwrap_or(0) as usize;

    match db.list_posts_with_stats(limit, offset).await {
        Ok(posts) => Json(posts.into_iter().map(|p| PostResponse {
            id: p.id,
            title: Some(p.title),
            status: Some(p.status),
            total_images: p.total_images,
            offloaded_images: p.offloaded_images,
            replaced_images: p.replaced_images,
            blogger_post_id: p.blogger_post_id,
            blogger_post_url: p.blogger_post_url,
            blogger_blog_id: p.blogger_blog_id,
            linked_at: p.linked_at.and_then(|s: String| DateTime::parse_from_rfc3339(&s).ok()).map(|dt| dt.with_timezone(&Utc)),
        }).collect()),
        Err(e) => {
            tracing::error!("list_posts failed: {}", e);
            Json(Vec::<PostResponse>::new())
        }
    }
}

async fn list_logs(State(_state): State<AppState>) -> impl IntoResponse {
    // Return recent structured logs - for now return empty
    // In production, this would query a log store or in-memory buffer
    Json(Vec::<LogEntry>::new())
}

async fn list_audits(State(_state): State<AppState>) -> impl IntoResponse {
    Json(Vec::<DailyAuditRecord>::new())
}

#[derive(Serialize)]
struct DailyAuditRecord {
    id: String,
    timestamp: String,
    date_str: String,
    target_site_url: String,
    device: String,
    lighthouse_score: u32,
    lcp: f64,
    lcp_before: f64,
    cls: f64,
    fcp: f64,
    ttfb_blogger_ms: u32,
    ttfb_origin_ms: u32,
    bandwidth_saved_gb: f64,
    images_audited: u32,
    status: String,
    notes: String,
    locations: Vec<AuditLocation>,
}

#[derive(Serialize)]
struct AuditLocation {
    region: String,
    city: String,
    ttfb_ms: u32,
}

async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = &state.config;
    Json(serde_json::json!({
        "daemon": {
            "port": cfg.agent.port,
            "poll_interval_sec": cfg.agent.poll_interval_secs,
            "replace_in_db": cfg.agent.replace_in_db,
            "backup_before_replace": cfg.agent.backup_before_replace,
            "enable_old_url_redirect_301": cfg.agent.enable_old_url_redirect_301,
        },
        "blogger": {
            "blog_id": cfg.blogger.blog_id,
            "upload_via": cfg.blogger.upload_via,
            "one_wp_post_per_blogger_post": cfg.blogger.one_wp_post_per_blogger_post,
        },
        "chrome": {
            "executable": cfg.chrome.executable,
            "user_data_dir": cfg.chrome.user_data_dir,
            "profile_directory": cfg.chrome.profile_directory,
            "headless": cfg.chrome.headless,
            "remote_debugging_port": cfg.chrome.remote_debugging_port,
        },
        "site": {
            "url": cfg.site.url,
            "uploads_dir": cfg.site.uploads_dir,
        }
    }))
}

async fn process_post(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let Some(engine) = state.engine.as_ref() else {
        return (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok":false,"error":"engine/db not ready"})));
    };
    match engine.process_post(id).await {
        Ok(v) => (axum::http::StatusCode::OK, Json(serde_json::json!({"ok":true,"data":v}))),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok":false,"error":e.to_string()}))),
    }
}

pub async fn run_server(config: Config, host_override: Option<String>, port_override: Option<u16>, dist_dir: PathBuf) -> Result<()> {
    let host = host_override.unwrap_or_else(|| config.agent.host.clone());
    let port = port_override.unwrap_or(config.agent.port);
    let cfg = Arc::new(config);

    let (engine, db_ok) = match OffloadEngine::new(cfg.clone()).await {
        Ok(e) => {
            let e = Arc::new(e);
            let bg = e.clone();
            tokio::spawn(async move { bg.run_poll_loop().await; });
            (Some(e), true)
        }
        Err(e) => {
            tracing::error!("DB/engine init failed (UI still served): {}", e);
            (None, false)
        }
    };

    if !dist_dir.exists() {
        tracing::warn!("web-ui dist not found: {} (build agents/web-ui)", dist_dir.display());
    }

    let state = AppState { config: cfg, engine, db_ok };
    let app = create_router(state, dist_dir);
    let addr: SocketAddr = format!("{host}:{port}").parse().map_err(|e| Error::ConfigError(format!("invalid bind addr: {e}")))?;
    tracing::info!("wp-offload-daemon listening on http://{addr} (web-ui + API), chrome={}", default_chrome_path().display());
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| Error::NetworkError(format!("bind: {e}")))?;
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await.map_err(|e| Error::NetworkError(format!("server: {e}")))?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown");
}

pub fn resolve_dist_dir() -> PathBuf {
    if let Ok(p) = std::env::var("WEB_UI_DIST") { return PathBuf::from(p); }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let a = cwd.join("web-ui").join("dist");
    if a.exists() { return a; }
    cwd.join("..").join("web-ui").join("dist")
}