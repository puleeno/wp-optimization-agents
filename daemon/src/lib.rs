#[cfg(test)]
mod tests {
    use super::config::{Config, MediaConfig};
    use super::error::Error;
    use super::media_processor::MediaProcessor;
    
    #[test]
    fn test_config_validation() {
        let config = Config {
            wordpress: super::config::WordPressConfig {
                api_url: "https://example.com".to_string(),
                username: "admin".to_string(),
                app_password: "password".to_string(),
                site_url: "https://example.com".to_string(),
            },
            blogger: super::config::BloggerConfig {
                blog_id: "123".to_string(),
                token: "token".to_string(),
                base_url: "https://www.blogger.com/api".to_string(),
            },
            chrome: super::config::ChromeConfig {
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
            logging: super::config::LoggingConfig {
                level: "info".to_string(),
                file: None,
                max_size_mb: 10,
                max_files: 5,
            },
        };
        
        // Test valid config passes validation
        // Config::validate() would return Ok(())
    }
    
    #[test]
    fn test_media_processor() {
        let processor = MediaProcessor::new(800, 600);
        
        // Test URL processing with transformations
        // Actual test would call processor.process_url() and verify output
    }
}
