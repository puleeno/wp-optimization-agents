pub mod config;
pub mod error;
pub mod main_lib;
pub mod media_processor;

pub use config::Config;
pub use error::Error;
pub use main_lib::{run_server, OffloadDaemon, OffloadManager};
pub use media_processor::MediaProcessor;

#[cfg(test)]
mod tests {
    use super::config::{BloggerConfig, ChromeConfig, Config, LoggingConfig, MediaConfig, WordPressConfig};
    use super::media_processor::MediaProcessor;

    #[test]
    fn test_config_validation() {
        let config = Config {
            wordpress: WordPressConfig {
                api_url: "https://example.com".to_string(),
                username: "admin".to_string(),
                app_password: "password".to_string(),
                site_url: "https://example.com".to_string(),
            },
            blogger: BloggerConfig {
                blog_id: "123".to_string(),
                token: "token".to_string(),
                base_url: "https://www.blogger.com/api".to_string(),
            },
            chrome: ChromeConfig {
                chrome_path: None,
                user_data_dir: None,
                headless: true,
            },
            media: MediaConfig {
                max_width: 1200,
                max_height: 800,
                quality: 85,
                upload_timeout: 30,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: None,
                max_size_mb: 10,
                max_files: 5,
            },
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_media_processor() {
        let processor = MediaProcessor::new(800, 600);
        let out = processor
            .process_url("https://example.com/image.jpg")
            .expect("url");
        assert!(out.contains("w=800"));
        assert!(out.contains("h=600"));
    }
}
