use crate::error::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WpImageSize {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WpImageSizesResponse {
    pub image_id: i64,
    pub origin_url: String,
    pub attachment_id: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub sizes: HashMap<String, WpImageSize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WpPostStatusResponse {
    pub post_id: i64,
    pub total_images: i32,
    pub replaced_images: i32,
    pub uploaded_images: i32,
    pub pending_images: i32,
    pub failed_images: i32,
    pub images: Vec<WpImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WpImage {
    pub id: i64,
    pub attachment_id: Option<i64>,
    pub origin_url: String,
    pub blogger_url: Option<String>,
    pub blogger_image_id: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub redirect_active: i32,
    pub replaced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WpApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WpApiClient {
    client: Client,
    base_url: String,
    token: String,
}

impl WpApiClient {
    pub fn new(base_url: String, token: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();
        Self { client, base_url, token }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    pub async fn get_post_status(&self, post_id: i64) -> Result<WpPostStatusResponse> {
        let url = format!("{}/posts/{}/status", self.base_url, post_id);
        debug!("GET {}", url);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| Error::NetworkError(format!("WP API request failed: {}", e)))?;
        
        let status = resp.status();
        let text = resp.text().await.map_err(|e| Error::NetworkError(format!("WP API read failed: {}", e)))?;
        
        if !status.is_success() {
            return Err(Error::BloggerError(format!("WP API error {}: {}", status, text)));
        }
        
        let api_resp: WpApiResponse<WpPostStatusResponse> = serde_json::from_str(&text)
            .map_err(|e| Error::BloggerError(format!("WP API parse failed: {}", e)))?;
        
        api_resp.data.ok_or_else(|| Error::BloggerError("WP API missing data".into()))
    }

    pub async fn get_image_sizes(&self, image_id: i64) -> Result<WpImageSizesResponse> {
        let url = format!("{}/images/{}/sizes", self.base_url, image_id);
        debug!("GET {}", url);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| Error::NetworkError(format!("WP API request failed: {}", e)))?;
        
        let status = resp.status();
        let text = resp.text().await.map_err(|e| Error::NetworkError(format!("WP API read failed: {}", e)))?;
        
        if !status.is_success() {
            return Err(Error::BloggerError(format!("WP API error {}: {}", status, text)));
        }
        
        let api_resp: WpApiResponse<WpImageSizesResponse> = serde_json::from_str(&text)
            .map_err(|e| Error::BloggerError(format!("WP API parse failed: {}", e)))?;
        
        api_resp.data.ok_or_else(|| Error::BloggerError("WP API missing data".into()))
    }

    pub async fn mark_image_uploaded(&self, image_id: i64, blogger_url: &str, blogger_image_id: Option<&str>, transformed_params: Option<&str>) -> Result<()> {
        let url = format!("{}/images/{}/uploaded", self.base_url, image_id);
        debug!("POST {}", url);
        
        let mut body = serde_json::json!({
            "blogger_url": blogger_url,
        });
        if let Some(id) = blogger_image_id {
            body["blogger_image_id"] = serde_json::json!(id);
        }
        if let Some(params) = transformed_params {
            body["transformed_params"] = serde_json::json!(params);
        }
        
        let resp = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::NetworkError(format!("WP API request failed: {}", e)))?;
        
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::BloggerError(format!("WP API error {}: {}", status, text)));
        }
        
        Ok(())
    }

    pub async fn mark_image_replaced(&self, image_id: i64) -> Result<()> {
        let url = format!("{}/images/{}/replaced", self.base_url, image_id);
        debug!("POST {}", url);
        
        let resp = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| Error::NetworkError(format!("WP API request failed: {}", e)))?;
        
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::BloggerError(format!("WP API error {}: {}", status, text)));
        }
        
        Ok(())
    }

    pub async fn mark_image_failed(&self, image_id: i64, error: &str) -> Result<()> {
        let url = format!("{}/images/{}/fail", self.base_url, image_id);
        debug!("POST {}", url);
        
        let body = serde_json::json!({
            "error": error,
        });
        
        let resp = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::NetworkError(format!("WP API request failed: {}", e)))?;
        
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::BloggerError(format!("WP API error {}: {}", status, text)));
        }
        
        Ok(())
    }

    pub async fn link_blogger_post(&self, post_id: i64, blogger_post_id: &str, blogger_post_url: &str, blogger_album_url: &str, blogger_blog_id: &str) -> Result<()> {
        let url = format!("{}/posts/{}/link-blogger", self.base_url, post_id);
        debug!("POST {}", url);
        
        let body = serde_json::json!({
            "blogger_post_id": blogger_post_id,
            "blogger_post_url": blogger_post_url,
            "blogger_album_url": blogger_album_url,
            "blogger_blog_id": blogger_blog_id,
        });
        
        let resp = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::NetworkError(format!("WP API request failed: {}", e)))?;
        
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::BloggerError(format!("WP API error {}: {}", status, text)));
        }
        
        Ok(())
    }
}