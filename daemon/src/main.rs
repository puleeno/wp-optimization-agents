use std::path::Path;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use wp_offload_ffi::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let host = std::env::var("DAEMON_HOST").ok();
    let port: Option<u16> = std::env::var("DAEMON_PORT").ok().or_else(|| std::env::var("PORT").ok()).and_then(|v| v.parse().ok());
    let dist_dir = wp_offload_ffi::main_lib::resolve_dist_dir();

    let config = if let Ok(path) = std::env::var("DAEMON_CONFIG") {
        Config::load(&path)?
    } else if let Ok(wp) = std::env::var("WP_CONFIG_PATH") {
        // best-effort bootstrap from wp-config.php; requires blogger blog id via env
        let mut c = Config::from_wp_config(Path::new(&wp))?;
        c.blogger.blog_id = std::env::var("BLOGGER_BLOG_ID").unwrap_or_default();
        c.site.url = std::env::var("SITE_URL").unwrap_or_default();
        if c.blogger.blog_id.is_empty() {
            tracing::warn!("BLOGGER_BLOG_ID not set; daemon will start without engine until valid config provided");
        }
        c
    } else {
        tracing::error!("Set DAEMON_CONFIG=/path/to/agents/config.json (or WP_CONFIG_PATH + BLOGGER_BLOG_ID + SITE_URL)");
        return Err("missing DAEMON_CONFIG".into());
    };

    wp_offload_ffi::main_lib::run_server(config, host, port, dist_dir).await?;
    Ok(())
}
