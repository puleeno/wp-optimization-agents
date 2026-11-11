use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MediaProcessor {
    pub width: i32,
    pub height: i32,
}

impl MediaProcessor {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub fn process_url(&self, url: &str) -> Result<String> {
        let parsed_url =
            Url::parse(url).map_err(|e| Error::MediaError(format!("Invalid URL: {}", e)))?;

        let mut processed_url = parsed_url;
        let mut params = processed_url.query_pairs_mut();
        params.append_pair("sz", "s800");
        params.append_pair("w", &self.width.to_string());
        params.append_pair("h", &self.height.to_string());
        drop(params);

        Ok(processed_url.to_string())
    }
}
