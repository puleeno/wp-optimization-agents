use crate::error::{Error, Result};
use reqwest::multipart;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct ImgbbClient {
    client: reqwest::Client,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct ImgbbResponse {
    success: bool,
    data: Option<ImgbbData>,
    error: Option<ImgbbError>,
}

#[derive(Debug, Deserialize)]
struct ImgbbData {
    url: Option<String>,
    display_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImgbbError {
    message: Option<String>,
}

impl ImgbbClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            api_key,
        }
    }

    /// Upload to imgbb using a direct/public WordPress image URL.
    pub async fn upload_url(&self, image_url: &str, name: Option<&str>) -> Result<String> {
        let mut form = vec![("image".to_string(), image_url.to_string())];
        if let Some(n) = name {
            if !n.trim().is_empty() {
                form.push(("name".to_string(), n.to_string()));
            }
        }
        let resp = self
            .client
            .post("https://api.imgbb.com/1/upload")
            .query(&[("key", self.api_key.as_str())])
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::NetworkError(format!("imgbb upload url: {}", e)))?;
        self.parse_response(resp).await
    }

    /// Upload to imgbb using local binary/blob file bytes (multipart/form-data).
    pub async fn upload_blob(&self, path: &str, name: Option<&str>) -> Result<String> {
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| Error::NetworkError(format!("read file {}: {}", path, e)))?;
        let file_name = name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| path.rsplit('/').next().unwrap_or("image"));
        let part = multipart::Part::bytes(bytes).file_name(file_name.to_string());
        let mut form = multipart::Form::new().part("image", part);
        if let Some(n) = name {
            if !n.trim().is_empty() {
                form = form.text("name", n.to_string());
            }
        }
        let resp = self
            .client
            .post("https://api.imgbb.com/1/upload")
            .query(&[("key", self.api_key.as_str())])
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::NetworkError(format!("imgbb upload blob: {}", e)))?;
        self.parse_response(resp).await
    }

    async fn parse_response(&self, resp: reqwest::Response) -> Result<String> {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::NetworkError(format!("imgbb read: {}", e)))?;
        if !status.is_success() {
            return Err(Error::BloggerError(format!("imgbb HTTP {}: {}", status, text)));
        }
        let parsed: ImgbbResponse = serde_json::from_str(&text)
            .map_err(|e| Error::BloggerError(format!("imgbb parse: {} body={}", e, text)))?;
        if !parsed.success {
            let msg = parsed
                .error
                .and_then(|e| e.message)
                .unwrap_or_else(|| "unknown error".into());
            return Err(Error::BloggerError(format!("imgbb error: {}", msg)));
        }
        parsed
            .data
            .and_then(|d| d.display_url.or(d.url))
            .ok_or_else(|| Error::BloggerError("imgbb missing url".into()))
    }
}
