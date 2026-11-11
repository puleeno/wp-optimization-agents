use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("WordPress API error: {0}")]
    WordPressApiError(String),

    #[error("Blogger API error: {0}")]
    BloggerApiError(String),

    #[error("Chrome automation error: {0}")]
    ChromeAutomationError(String),

    #[error("Media processing error: {0}")]
    MediaError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("File system error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
