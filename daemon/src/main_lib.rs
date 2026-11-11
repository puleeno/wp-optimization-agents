use crate::config::Config;
use crate::error::{Error, Result};
use crate::media_processor::MediaProcessor;
use axum::{
    extract::State,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Debug)]
pub struct OffloadDaemon {
    config: Config,
    client: reqwest::Client,
}

impl OffloadDaemon {
    pub fn new(config: Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.media.upload_timeout))
            .build()
            .map_err(|e| Error::NetworkError(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub fn manager(&self) -> OffloadManager {
        OffloadManager {
            inner: Arc::new(RwLock::new(OffloadManagerState { running: false })),
        }
    }
}

#[derive(Debug, Default)]
struct OffloadManagerState {
    running: bool,
}

#[derive(Clone)]
pub struct OffloadManager {
    inner: Arc<RwLock<OffloadManagerState>>,
}

impl OffloadManager {
    pub async fn start(&self) {
        self.inner.write().await.running = true;
    }

    pub async fn stop(&self) {
        self.inner.write().await.running = false;
    }

    pub async fn is_running(&self) -> bool {
        self.inner.read().await.running
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub manager: OffloadManager,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct MetricsResponse {
    status: &'static str,
    daemon_running: bool,
    pid: u32,
}

pub fn create_router(state: AppState, dist_dir: PathBuf) -> Router {
    let index = dist_dir.join("index.html");
    let static_files = ServeDir::new(dist_dir.clone()).not_found_service(ServeFile::new(index));

    Router::new()
        .route("/health", get(health))
        .route("/api/health", get(health))
        .route("/api/metrics", get(metrics))
        .route("/api/status", get(metrics))
        .nest_service("/", static_files)
        .with_state(state)
        .layer(CorsLayer::permissive())
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        service: "wp-offload-daemon",
    })
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let running = state.manager.is_running().await;
    Json(MetricsResponse {
        status: "ok",
        daemon_running: running,
        pid: std::process::id(),
    })
}

pub async fn run_server(config: Config, host: String, port: u16, dist_dir: PathBuf) -> Result<()> {
    let daemon = OffloadDaemon::new(config.clone())?;
    let manager = daemon.manager();
    manager.start().await;

    if !dist_dir.exists() {
        tracing::warn!(
            "web-ui dist directory not found: {}. Frontend assets will 404 until built.",
            dist_dir.display()
        );
    }

    let state = AppState {
        config: Arc::new(config),
        manager,
    };

    let app = create_router(state, dist_dir);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| Error::ConfigError(format!("Invalid bind address: {e}")))?;

    tracing::info!("wp-offload-daemon listening on http://{addr} (serving web-ui + API)");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| Error::NetworkError(format!("Failed to bind {addr}: {e}")))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| Error::NetworkError(format!("Server error: {e}")))?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}

pub fn resolve_dist_dir() -> PathBuf {
    if let Ok(p) = std::env::var("WEB_UI_DIST") {
        return PathBuf::from(p);
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let candidate = cwd.join("web-ui").join("dist");
    if candidate.exists() {
        return candidate;
    }
    let candidate2 = cwd.join("..").join("web-ui").join("dist");
    candidate2
}

pub fn process_media_url(url: &str, width: i32, height: i32) -> Result<String> {
    MediaProcessor::new(width, height).process_url(url)
}
