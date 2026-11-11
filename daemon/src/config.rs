use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub wordpress: WordPressConfig,
    pub blogger: BloggerConfig,
    pub chrome: ChromeConfig,
    pub media: MediaConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WordPressConfig {
    pub api_url: String,
    pub username: String,
    pub app_password: String,
    pub site_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BloggerConfig {
    pub blog_id: String,
    pub token: String,
    pub base_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChromeConfig {
    pub chrome_path: Option<String>,
    pub user_data_dir: Option<String>,
    pub headless: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MediaConfig {
    pub max_width: u32,
    pub max_height: u32,
    pub quality: u8,
    pub upload_timeout: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<String>,
    pub max_size_mb: u64,
    pub max_files: u32,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::ConfigError(format!("Failed to read config: {}", e)))?;

        let config: Config = serde_json::from_str(&content)
            .map_err(|e| Error::ConfigError(format!("Failed to parse config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.wordpress.api_url.is_empty() {
            return Err(Error::ConfigError("WordPress API URL is required".to_string()));
        }
        if self.wordpress.username.is_empty() {
            return Err(Error::ConfigError("WordPress username is required".to_string()));
        }
        if self.wordpress.app_password.is_empty() {
            return Err(Error::ConfigError(
                "WordPress app password is required".to_string(),
            ));
        }
        if self.blogger.blog_id.is_empty() {
            return Err(Error::ConfigError("Blogger blog ID is required".to_string()));
        }
        if self.blogger.token.is_empty() {
            return Err(Error::ConfigError("Blogger token is required".to_string()));
        }
        Ok(())
    }
}
