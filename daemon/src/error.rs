use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Chrome/CDP error: {0}")]
    ChromeError(String),

    #[error("Blogger upload error: {0}")]
    BloggerError(String),

    #[error("Replace error: {0}")]
    ReplaceError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Other: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<mysql_async::Error> for Error {
    fn from(e: mysql_async::Error) -> Self {
        Error::DatabaseError(e.to_string())
    }
}

impl From<mysql_async::FromRowError> for Error {
    fn from(e: mysql_async::FromRowError) -> Self {
        Error::DatabaseError(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Other(e.to_string())
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::Other(e.to_string())
    }
}
