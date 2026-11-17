use crate::chrome::default_chrome_path;
use crate::config::Config;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::offload::{CurrentTask, OffloadEngine};
use crate::webhook::{run_blogger_queue_worker, MediaUploadPayload, WebhookProcessor};
use axum::{extract::{Path, Query, State}, http::StatusCode, response::{sse::{Event, KeepAlive, Sse}, IntoResponse}, routing::{get, post}, Json, Router};
use futures_util::stream::{self, Stream};
use std::convert::Infallible;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub engine: Option<Arc<OffloadEngine>>,
    pub db: Option<Db>,
    pub db_ok: bool,
    pub current_task: Arc<RwLock<Option<CurrentTask>>>,
    pub webhook: Option<Arc<WebhookProcessor>>,
    pub http: reqwest::Client,
    pub started_at: Instant,
    pub cfg_path: Option<String>,
    /// Live config (updated on PUT /api/config without restart)
    pub config_live: Arc<RwLock<crate::config::Config>>,
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
    tokio_workers: u64,
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
    post_type: Option<String>,
    total_images: i64,
    offloaded_images: i64,
    replaced_images: i64,
    blogger_post_id: Option<String>,
    blogger_post_url: Option<String>,
    blogger_blog_id: Option<String>,
    linked_at: Option<DateTime<Utc>>,
}

// LogEntry served from crate::logs

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<u32>,
    offset: Option<u32>,
    status: Option<String>,
}

async fn req_log(req: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let started = Instant::now();
    let res = next.run(req).await;
    let status = res.status().as_u16();
    let ms = started.elapsed().as_millis();
    if method == axum::http::Method::POST || status >= 400 || ms > 2000 {
        tracing::info!(target: "api_server", stage = "http", method = %method, target = %path, status = status, elapsed_ms = ms as u64, "http request");
    } else {
        tracing::debug!(target: "api_server", stage = "http", method = %method, target = %path, status = status, elapsed_ms = ms as u64, "http request");
    }
    res
}

pub fn create_router(state: AppState, dist_dir: PathBuf) -> Router {
    let index = dist_dir.join("index.html");
    let static_files = ServeDir::new(dist_dir.clone()).fallback(ServeFile::new(index));
    let api_routes = Router::new()
        .route("/health", get(health))
        .route("/api/health", get(health))
        .route("/api/metrics", get(metrics))
        .route("/api/status", get(metrics))
        .route("/api/stats", get(stats))
        .route("/api/images", get(list_images))
        .route("/api/posts", get(list_posts))
        .route("/api/attachments", get(list_attachments))
        .route("/api/logs", get(list_logs))
        .route("/api/logs/stream", get(list_logs_stream))
        .route("/api/audits", get(list_audits))
        .route("/api/audits/run", post(run_audit))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/tasks/current", get(get_current_task))
        .route("/api/tasks/stream", get(get_current_task_stream))
        .route("/api/posts/:id/process", post(process_post))
        .route("/api/attachments/:id/process", post(process_attachment))
        .route("/api/scan/run", post(run_scan))
        .route("/api/scan/status", get(scan_status))
        .route("/api/chrome/preflight", get(chrome_preflight))
        .route("/api/webhooks/media-upload", post(media_upload_webhook))
        .route("/webhook/media-upload", post(media_upload_webhook))
        .route("/cdn/*path", get(cdn_proxy))
        .route("/wp-content/uploads/*path", get(cdn_proxy_uploads))
        .with_state(state.clone());

    // Register ALL routes directly - API routes first, then static files as fallback
    Router::new()
        .merge(api_routes)
        .fallback_service(static_files)
        .layer(axum::middleware::from_fn(req_log))
        .layer(CorsLayer::permissive())
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(HealthResponse { status: "ok", service: "wp-offload-daemon", db: state.db_ok })
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let stats = if let Some(db) = state.db.as_ref() {
        db.get_stats().await.unwrap_or_default()
    } else if let Some(e) = state.engine.as_ref() {
        e.db().get_stats().await.unwrap_or_default()
    } else {
        serde_json::json!({})
    };
    let active_tasks = if state.current_task.read().await.as_ref().map(|t| t.active).unwrap_or(false) { 1 } else { 0 };
    let offloaded = stats.get("replaced").and_then(|v| v.as_u64()).unwrap_or(0)
        + stats.get("uploaded").and_then(|v| v.as_u64()).unwrap_or(0);
    Json(MetricsResponse {
        status: "running",
        pid: std::process::id(),
        db: state.db_ok,
        uptime_seconds: state.started_at.elapsed().as_secs(),
        memory_mb: crate::sysinfo_lite::rss_mb().unwrap_or(0.0),
        cpu_percent: crate::sysinfo_lite::cpu_percent().unwrap_or(0.0),
        tokio_workers: crate::sysinfo_lite::worker_threads(),
        total_images_offloaded: offloaded,
        total_bytes_saved: 0,
        active_tasks,
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
    let db = if let Some(db) = state.db.as_ref() { db } else if let Some(e) = state.engine.as_ref() { e.db() } else { return Json(vec![]); };

    let limit = q.limit.unwrap_or(50) as usize;
    let offset = q.offset.unwrap_or(0) as usize;

    match db.list_posts_with_stats(limit, offset).await {
        Ok(posts) => Json(posts.into_iter().map(|p| PostResponse {
            id: p.id,
            title: Some(p.title),
            status: Some(p.status),
            post_type: Some(p.post_type),
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

async fn list_attachments(State(state): State<AppState>, Query(q): Query<ListQuery>) -> impl IntoResponse {
    let Some(db) = state.db.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok":false,"error":"db not ready"}))).into_response();
    };
    let limit = (q.limit.unwrap_or(100) as usize).clamp(1, 1000);
    let offset = q.offset.unwrap_or(0) as usize;
    let total = db.count_attachments().await.unwrap_or(0);
    match db.list_attachments_with_offload(limit, offset).await {
        Ok(items) => {
            let mut res = Json(items).into_response();
            if let Ok(v) = axum::http::HeaderValue::from_str(&total.to_string()) {
                res.headers_mut().insert("x-total-count", v);
            }
            res
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok":false,"error":e.to_string()}))).into_response(),
    }
}

async fn process_attachment(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let Some(db) = state.db.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok":false,"error":"db not ready"})));
    };
    let Some(webhook) = state.webhook.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok":false,"error":"webhook processor not ready"})));
    };
    tracing::info!(target: "api_server", stage = "scan", attachment_id = id, "manual attachment process requested");
    let res = match db.get_attachment_payload(id).await {
        Ok(Some(payload)) => match webhook.process_media_upload(payload).await {
            Ok(v) => (StatusCode::OK, Json(v)),
            Err(e) => {
                tracing::error!(target: "webhook", stage = "scan", attachment_id = id, "attachment process failed: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok":false,"error":e.to_string()})))
            }
        },
        Ok(None) => {
            tracing::warn!(target: "api_server", stage = "scan", attachment_id = id, "attachment not found");
            (StatusCode::NOT_FOUND, Json(serde_json::json!({"ok":false,"error":"attachment not found"})))
        }
        Err(e) => {
            tracing::error!(target: "api_server", stage = "scan", attachment_id = id, "attachment payload load failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok":false,"error":e.to_string()})))
        }
    };
    res
}

#[derive(Deserialize)]
struct ScanBody {
    batch: Option<usize>,
}

async fn run_scan(State(state): State<AppState>, body: Option<Json<ScanBody>>) -> impl IntoResponse {
    let Some(webhook) = state.webhook.clone() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok": false, "error": "webhook processor not ready"}))).into_response();
    };
    let processor = webhook.clone();
    let cfg = state.config.clone();
    let task = state.current_task.clone();
    let batch = body.and_then(|b| b.batch);
    tokio::spawn(async move {
        match crate::scan::scan_once(processor, cfg, task, batch).await {
            Ok(n) => tracing::info!(target: "api_server", stage = "scan", count = n, "manual scan finished"),
            Err(e) => tracing::warn!(target: "api_server", stage = "scan", "manual scan failed: {}", e),
        }
    });
    (StatusCode::ACCEPTED, Json(serde_json::json!({"ok": true, "started": true}))).into_response()
}

async fn scan_status(State(state): State<AppState>) -> impl IntoResponse {
    let Some(db) = state.db.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok": false, "error": "db not ready"}))).into_response();
    };
    let pending = db.count_pending_attachments().await.unwrap_or(0);
    let total = db.count_attachments().await.unwrap_or(0);
    let (queued, uploaded, finished, failed) = db.blogger_queue_counts().await.unwrap_or((0, 0, 0, 0));
    let cfg = state.config_live.read().await;
    let imgbb_ready = cfg.imgbb.enabled && !cfg.imgbb.api_key.trim().is_empty();
    Json(serde_json::json!({
        "ok": true,
        "pending": pending,
        "total": total,
        "processed": (total - pending).max(0),
        "scan_enabled": cfg.agent.scan_enabled,
        "scan_interval_secs": cfg.agent.scan_interval_secs,
        "scan_batch_size": cfg.agent.scan_batch_size,
        "finish_workers": cfg.agent.finish_workers,
        "delete_local_after_offload": cfg.agent.delete_local_after_offload,
        "imgbb_ready": imgbb_ready,
        "imgbb_upload_format": cfg.imgbb.upload_format,
        "blogger_ready": !cfg.blogger.blog_id.trim().is_empty() && !cfg.blogger.blog_id.contains("YOUR_"),
        "queue": {
            "queued": queued,
            "uploaded": uploaded,
            "finished": finished,
            "failed": failed
        }
    })).into_response()
}

async fn chrome_preflight(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.config_live.read().await;
    let chrome = cfg.chrome.executable.clone()
        .unwrap_or_else(|| crate::chrome::default_chrome_path().to_string_lossy().to_string());
    let profile_dir = cfg.chrome.user_data_dir.clone()
        .unwrap_or_else(|| crate::chrome::default_user_data_dir().to_string_lossy().to_string());
    let port = cfg.chrome.remote_debugging_port;
    let port_open = tokio::time::timeout(
        std::time::Duration::from_millis(600),
        tokio::net::TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false);
    let chrome_exists = std::path::Path::new(&chrome).exists();
    let profile_exists = std::path::Path::new(&profile_dir).exists();

    let mut advice: Vec<String> = Vec::new();
    if !chrome_exists {
        advice.push(format!("Không tìm thấy Chrome tại {}", chrome));
    }
    if !profile_exists {
        advice.push(format!("Chưa có profile Chrome tại {} (sẽ được tạo khi chạy)", profile_dir));
    }
    if !port_open {
        advice.push(format!(
            "Port {} chưa mở. Nếu Chrome đang chạy sẵn, hãy tắt hẳn (Cmd+Q) rồi để daemon tự mở, hoặc mở Chrome với --remote-debugging-port={}",
            port, port
        ));
    }
    if !cfg.blogger.blog_id.trim().is_empty() && !cfg.blogger.blog_id.contains("YOUR_") {
        // ok
    } else {
        advice.push("Blogger Blog ID chưa hợp lệ (phải là ID thật, không phải placeholder)".into());
    }
    if advice.is_empty() {
        advice.push("Chrome/profile/port/blog id đều sẵn sàng".into());
    }

    Json(serde_json::json!({
        "ok": true,
        "chrome_path": chrome,
        "chrome_exists": chrome_exists,
        "profile_dir": profile_dir,
        "profile_exists": profile_exists,
        "debug_port": port,
        "port_open": port_open,
        "headless": cfg.chrome.headless,
        "blogger_id": cfg.blogger.blog_id,
        "advice": advice
    }))
}

#[derive(Deserialize)]
struct LogsQuery {
    limit: Option<usize>,
    offset: Option<usize>,
    level: Option<String>,
    subsystem: Option<String>,
    q: Option<String>,
    attachment_id: Option<i64>,
    post_id: Option<i64>,
    task_id: Option<String>,
}

async fn list_logs(State(state): State<AppState>, Query(q): Query<LogsQuery>) -> impl IntoResponse {
    let filter = crate::logs::LogFilter {
        limit: Some(q.limit.unwrap_or(200).clamp(1, 2000)),
        offset: Some(q.offset.unwrap_or(0)),
        level: q.level,
        subsystem: q.subsystem,
        q: q.q,
        attachment_id: q.attachment_id,
        post_id: q.post_id,
        task_id: q.task_id,
    };
    if let Some(db) = state.db.as_ref() {
        match crate::logs::query(db, &filter).await {
            Ok((entries, total)) => {
                let mut res = Json(serde_json::json!({"ok": true, "logs": entries, "total": total})).into_response();
                if let Ok(v) = axum::http::HeaderValue::from_str(&total.to_string()) {
                    res.headers_mut().insert("x-total-count", v);
                }
                res
            }
            Err(e) => {
                tracing::warn!("query_logs failed, falling back to memory buffer: {}", e);
                let entries = crate::logs::recent(filter.limit.unwrap_or(200));
                Json(serde_json::json!({"ok": true, "logs": entries, "total": entries.len(), "degraded": true})).into_response()
            }
        }
    } else {
        let entries = crate::logs::recent(filter.limit.unwrap_or(200));
        Json(serde_json::json!({"ok": true, "logs": entries, "total": entries.len()})).into_response()
    }
}

async fn list_logs_stream(State(_state): State<AppState>) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let st = stream::unfold(0usize, |last_len| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let all = crate::logs::recent(500);
        let start = if all.len() >= last_len { last_len } else { 0 };
        let new_items = if start < all.len() { all[start..].to_vec() } else { Vec::new() };
        let payload = serde_json::json!({"ok": true, "logs": new_items}).to_string();
        Some((Ok(Event::default().event("logs").data(payload)), all.len()))
    });
    Sse::new(st).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keepalive"))
}

async fn list_audits(State(state): State<AppState>, Query(q): Query<ListQuery>) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(50) as usize;
    let Some(db) = state.db.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok":false,"error":"db not ready"}))).into_response();
    };
    match db.list_audits(limit).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok":false,"error":e.to_string()}))).into_response(),
    }
}

#[derive(Deserialize)]
struct RunAuditBody {
    urls: Option<Vec<String>>,
}

async fn run_audit(State(state): State<AppState>, body: Option<Json<RunAuditBody>>) -> impl IntoResponse {
    let Some(db) = state.db.as_ref().cloned() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok":false,"error":"db not ready"}))).into_response();
    };
    let urls = body.and_then(|b| b.0.urls).unwrap_or_default();
    let runner = std::sync::Arc::new(crate::audit::AuditRunner::new(db, state.config.clone(), state.current_task.clone()));
    tokio::spawn(async move {
        if let Err(e) = runner.run(urls).await {
            tracing::error!(target: "api_server", stage = "audit", "audit run failed: {}", e);
        }
    });
    tracing::info!(target: "api_server", stage = "audit", "audit run started");
    (StatusCode::ACCEPTED, Json(serde_json::json!({"ok": true, "started": true}))).into_response()
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

async fn put_config(State(state): State<AppState>, Json(body): Json<serde_json::Value>) -> impl IntoResponse {
    let Some(path) = state.cfg_path.clone() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "error": "config path unknown"}))).into_response();
    };
    let current: serde_json::Value = match std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(v) => v,
        None => serde_json::json!({}),
    };
    let mut body = body;
    // keep server-side-only paths when the UI sends an empty value
    if let Some(site) = body.get_mut("site").and_then(|v| v.as_object_mut()) {
        let wp_path = site.get("wp_path").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
        if wp_path.is_empty() {
            let existing = current
                .get("site")
                .and_then(|i| i.get("wp_path"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            site.insert("wp_path".into(), serde_json::json!(existing));
        }
    }
    if let Some(v) = body.get_mut("imgbb").and_then(|v| v.as_object_mut()) {
        let incoming = v.get("api_key").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
        if v.get("clear_api_key").and_then(|x| x.as_bool()).unwrap_or(false) {
            v.insert("api_key".into(), serde_json::json!(""));
            v.remove("clear_api_key");
        } else if incoming.is_empty() {
            let existing = current
                .get("imgbb")
                .and_then(|i| i.get("api_key"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            v.insert("api_key".into(), serde_json::json!(existing));
        }
        v.remove("clear_api_key");
    }
    let merged = merge_json(current, body);
    // validate against Config schema
    match serde_json::from_value::<crate::config::Config>(merged.clone()) {
        Ok(cfg) => {
            if let Err(e) = cfg.validate() {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "error": e.to_string()}))).into_response();
            }
            *state.config_live.write().await = cfg.clone();
            let pretty = serde_json::to_string_pretty(&merged).unwrap_or_else(|_| "{}".into());
            let tmp = format!("{}.tmp", path);
            if let Err(e) = std::fs::write(&tmp, pretty) {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "error": format!("write failed: {e}")}))).into_response();
            }
            if let Err(e) = std::fs::rename(&tmp, &path) {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "error": format!("replace failed: {e}")}))).into_response();
            }
            tracing::info!(target: "api_server", stage = "config", "config saved to {}", path);
            Json(serde_json::json!({
                "ok": true,
                "path": path,
                "requires_restart": ["agent.host", "agent.port", "chrome.*", "database.*"],
                "note": "Đã ghi file config. Khởi động lại daemon để áp dụng host/port/chrome/database."
            })).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "error": format!("invalid config: {e}")}))).into_response(),
    }
}

fn merge_json(base: serde_json::Value, patch: serde_json::Value) -> serde_json::Value {
    match (base, patch) {
        (serde_json::Value::Object(mut b), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                let entry = b.remove(&k).unwrap_or(serde_json::Value::Null);
                b.insert(k, merge_json(entry, v));
            }
            serde_json::Value::Object(b)
        }
        (_, p) => p,
    }
}

async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.config_live.read().await.clone();
    let mut root = serde_json::to_value(&cfg).unwrap_or_else(|_| serde_json::json!({}));

    if let Some(obj) = root.as_object_mut() {
        // never expose DB password to the UI
        if let Some(db) = obj.get_mut("database").and_then(|v| v.as_object_mut()) {
            db.insert("password".into(), serde_json::json!(""));
            db.insert("password_set".into(), serde_json::json!(!cfg.database.password.is_empty()));
        }
        if let Some(imgbb) = obj.get_mut("imgbb").and_then(|v| v.as_object_mut()) {
            imgbb.insert("api_key_set".into(), serde_json::json!(!cfg.imgbb.api_key.trim().is_empty()));
            imgbb.insert("api_key".into(), serde_json::json!(""));
        }
        // legacy alias kept for older UI components
        let agent = obj.get("agent").cloned().unwrap_or_else(|| serde_json::json!({}));
        obj.insert("_meta".into(), serde_json::json!({
            "path": state.cfg_path.clone().unwrap_or_else(|| "(không xác định)".into()),
            "source": "DAEMON_CONFIG",
            "pid": std::process::id(),
        }));
        obj.insert("daemon".into(), serde_json::json!({
            "host": agent.get("host").cloned().unwrap_or(serde_json::json!("0.0.0.0")),
            "port": agent.get("port").cloned().unwrap_or(serde_json::json!(2095)),
            "poll_interval_sec": agent.get("poll_interval_secs").cloned().unwrap_or(serde_json::json!(60)),
            "replace_in_db": agent.get("replace_in_db").cloned().unwrap_or(serde_json::json!(true)),
            "backup_before_replace": agent.get("backup_before_replace").cloned().unwrap_or(serde_json::json!(true)),
            "enable_old_url_redirect_301": agent.get("enable_old_url_redirect_301").cloned().unwrap_or(serde_json::json!(true)),
            "scan_enabled": agent.get("scan_enabled").cloned().unwrap_or(serde_json::json!(true)),
            "scan_interval_secs": agent.get("scan_interval_secs").cloned().unwrap_or(serde_json::json!(900)),
            "scan_batch_size": agent.get("scan_batch_size").cloned().unwrap_or(serde_json::json!(20)),
            "delete_local_after_offload": agent.get("delete_local_after_offload").cloned().unwrap_or(serde_json::json!(true)),
        }));
    }
    Json(root)
}

async fn get_current_task(State(state): State<AppState>) -> impl IntoResponse {
    let task = state.current_task.read().await.clone();
    Json(serde_json::json!({"ok": true, "task": task}))
}

async fn get_current_task_stream(State(state): State<AppState>) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let st = stream::unfold((state, true), |(state, first)| async move {
        if !first {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        let task = state.current_task.read().await.clone();
        let payload = serde_json::json!({"ok": true, "task": task}).to_string();
        Some((Ok(Event::default().event("task").data(payload)), (state, false)))
    });
    Sse::new(st).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keepalive"))
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

async fn cdn_proxy(State(state): State<AppState>, Path(path): Path<String>) -> impl IntoResponse {
    proxy_by_path(state, path).await
}

async fn cdn_proxy_uploads(State(state): State<AppState>, Path(path): Path<String>) -> impl IntoResponse {
    let full = format!("wp-content/uploads/{}", path.trim_start_matches('/'));
    proxy_by_path(state, full).await
}

async fn proxy_by_path(state: AppState, path: String) -> impl IntoResponse {
    let Some(db) = state.db.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "db not ready").into_response();
    };
    match db.get_cdn_target_by_path(&path).await {
        Ok(Some(target)) => match state.http.get(&target).send().await {
            Ok(resp) => {
                let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
                let ct = resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("application/octet-stream").to_string();
                match resp.bytes().await {
                    Ok(bytes) => (status, [(axum::http::header::CONTENT_TYPE, ct), (axum::http::header::CACHE_CONTROL, "public, max-age=31536000, immutable".to_string())], bytes).into_response(),
                    Err(e) => (StatusCode::BAD_GATEWAY, format!("read upstream: {e}")).into_response(),
                }
            }
            Err(e) => (StatusCode::BAD_GATEWAY, format!("fetch upstream: {e}")).into_response(),
        },
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("cdn lookup error: {e}")).into_response(),
    }
}

async fn media_upload_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<MediaUploadPayload>,
) -> impl IntoResponse {
    // auth: Bearer token same as daemon_api_token
    let expected = state.config.agent.daemon_api_token.clone();
    if !expected.is_empty() {
        let auth = headers.get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok()).unwrap_or("");
        let ok = auth.strip_prefix("Bearer ").map(|t| t == expected).unwrap_or(false);
        if !ok {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"ok":false,"error":"unauthorized"})));
        }
    }
    let Some(webhook) = state.webhook.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ok":false,"error":"webhook processor not ready"})));
    };
    match webhook.process_media_upload(payload).await {
        Ok(v) => (StatusCode::OK, Json(v)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok":false,"error":e.to_string()}))),
    }
}

pub async fn run_server(config: Config, host_override: Option<String>, port_override: Option<u16>, dist_dir: PathBuf) -> Result<()> {
    let host = host_override.unwrap_or_else(|| config.agent.host.clone());
    let port = port_override.unwrap_or(config.agent.port);
    let cfg = Arc::new(config);
    let current_task: Arc<RwLock<Option<CurrentTask>>> = Arc::new(RwLock::new(None));

    let (engine, db_ok) = match OffloadEngine::new(cfg.clone(), current_task.clone()).await {
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

    let db_shared = if db_ok {
        match Db::new((*cfg).clone()) {
            Ok(db) => {
                if let Err(e) = db.ensure_pipeline_tables().await { tracing::warn!("ensure pipeline tables failed: {}", e); }
                if let Err(e) = db.ensure_log_table().await { tracing::warn!("ensure log table failed: {}", e); }
                if let Err(e) = db.ensure_audit_table().await { tracing::warn!("ensure audit table failed: {}", e); }
                crate::logs::spawn_writer(db.clone()).await;
                Some(db)
            }
            Err(e) => { tracing::warn!("shared db init failed: {}", e); None }
        }
    } else { None };

    let webhook = if let Some(db) = db_shared.clone() {
        let wp = Arc::new(WebhookProcessor::new(db, cfg.clone()));
        let bg_wp = wp.clone();
        let bg_cfg = cfg.clone();
        let bg_engine = engine.clone();
        tokio::spawn(async move { run_blogger_queue_worker(bg_wp, bg_cfg, bg_engine).await; });

        // Crop/resize finish worker (parallel, independent from imgbb + blogger upload)
        let fin_wp = wp.clone();
        let fin_cfg = cfg.clone();
        tokio::spawn(async move { crate::finish::run_finish_worker(fin_wp, fin_cfg).await; });

        // Media library auto-scan worker (offload existing attachments)
        let scan_wp = wp.clone();
        let scan_cfg = cfg.clone();
        let scan_task = current_task.clone();
        tokio::spawn(async move { crate::scan::run_media_scan_worker(scan_wp, scan_cfg, scan_task).await; });

        Some(wp)
    } else { None };

    if !dist_dir.exists() {
        tracing::warn!("web-ui dist not found: {} (build agents/web-ui)", dist_dir.display());
    }

    let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(60)).build().unwrap_or_else(|_| reqwest::Client::new());
    let started_at = Instant::now();
    let cfg_path = std::env::var("DAEMON_CONFIG").ok();
    let config_live = Arc::new(RwLock::new((*cfg).clone()));
    let state = AppState { config: cfg, engine, db: db_shared, db_ok, current_task, webhook, http, started_at, cfg_path, config_live };
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