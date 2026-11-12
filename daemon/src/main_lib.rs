use crate::chrome::default_chrome_path;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::offload::OffloadEngine;
use axum::{extract::{Path, State}, response::IntoResponse, routing::{get, post}, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

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
struct MetricsResponse {
    status: &'static str,
    pid: u32,
    db: bool,
}

pub fn create_router(state: AppState, dist_dir: PathBuf) -> Router {
    let index = dist_dir.join("index.html");
    let static_files = ServeDir::new(dist_dir.clone()).not_found_service(ServeFile::new(index));
    Router::new()
        .route("/health", get(health))
        .route("/api/health", get(health))
        .route("/api/metrics", get(metrics))
        .route("/api/status", get(metrics))
        .route("/api/posts/:id/process", post(process_post))
        .nest_service("/", static_files)
        .with_state(state)
        .layer(CorsLayer::permissive())
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(HealthResponse { status: "ok", service: "wp-offload-daemon", db: state.db_ok })
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    Json(MetricsResponse { status: "ok", pid: std::process::id(), db: state.db_ok })
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
