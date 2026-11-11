// Media processing module with URL transformation
use reqwest::Client;
use url::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MediaProcessor {
    pub width: i32,
    pub height: i32,
}

impl MediaProcessor {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
    
    pub async fn process_url(&self, url: &str) -> Result<String, Error> {
        // Process URL for Blogger upload
        // This handles resizing and cropping via Blogger URL parameters
        
        let parsed_url = Url::parse(url)
            .map_err(|e| Error::MediaError(format!("Invalid URL: {}", e)))?;
        
        // Build Blogger URL with transformation parameters
        let mut processed_url = parsed_url.clone();
        
        // Add query parameters for image processing
        let mut params = processed_url.query_pairs_mut();
        params.append_pair("sz", "s800"); // Default size
        params.append_pair("w", &self.width.to_string());
        params.append_pair("h", &self.height.to_string());
        
        Ok(processed_url.to_string())
    }
}
