use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use wp_offload_ffi::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let host = std::env::var("DAEMON_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("DAEMON_PORT")
        .ok()
        .or_else(|| std::env::var("PORT").ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(2095);

    let dist_dir = wp_offload_ffi::main_lib::resolve_dist_dir();

    // Config is required for core daemon operations; server can still start to serve UI.
    let config = if let Ok(path) = std::env::var("DAEMON_CONFIG") {
        match Config::load(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to load config {path}: {e}");
                return Err(e.into());
            }
        }
    } else {
        tracing::warn!("DAEMON_CONFIG not set; using placeholder config. Set DAEMON_CONFIG for full operation.");
        Config {
            wordpress: wp_offload_ffi::config::WordPressConfig {
                api_url: "https://example.com/wp-json".into(),
                username: "placeholder".into(),
                app_password: "placeholder".into(),
                site_url: "https://example.com".into(),
            },
            blogger: wp_offload_ffi::config::BloggerConfig {
                blog_id: "placeholder".into(),
                token: "placeholder".into(),
                base_url: "https://www.blogger.com/api".into(),
            },
            chrome: wp_offload_ffi::config::ChromeConfig {
                chrome_path: None,
                user_data_dir: None,
                headless: true,
            },
            media: wp_offload_ffi::config::MediaConfig {
                max_width: 1200,
                max_height: 800,
                quality: 85,
                upload_timeout: 30,
            },
            logging: wp_offload_ffi::config::LoggingConfig {
                level: "info".into(),
                file: None,
                max_size_mb: 10,
                max_files: 5,
            },
        }
    };

    wp_offload_ffi::main_lib::run_server(config, host, port, dist_dir).await?;
    Ok(())
}
