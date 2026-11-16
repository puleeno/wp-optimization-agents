use crate::db::Db;
use crate::error::{Error, Result};
use crate::imgbb::ImgbbClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ImageSizePayload {
    pub size: String, // e.g. "full", "thumbnail", "medium"
    pub url: String,
    pub path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MediaUploadPayload {
    pub attachment_id: i64,
    pub post_id: Option<i64>,
    pub file: Option<String>, // local path
    pub url: Option<String>,  // full url
    pub mime_type: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub file_size: Option<i64>,
    #[serde(default)]
    pub sizes: Vec<ImageSizePayload>,
}

#[derive(Clone)]
pub struct WebhookProcessor {
    pub db: Db,
    pub cfg: Arc<crate::config::Config>,
    pub imgbb: Option<ImgbbClient>,
}

impl WebhookProcessor {
    pub fn new(db: Db, cfg: Arc<crate::config::Config>) -> Self {
        let imgbb = if cfg.imgbb.enabled && !cfg.imgbb.api_key.trim().is_empty() {
            Some(ImgbbClient::new(cfg.imgbb.api_key.clone()))
        } else {
            None
        };
        Self { db, cfg, imgbb }
    }

    pub async fn process_media_upload(&self, payload: MediaUploadPayload) -> Result<serde_json::Value> {
        info!(attachment_id = payload.attachment_id, "processing media upload webhook");
        self.db.ensure_pipeline_tables().await?;

        let mut all_sizes = payload.sizes.clone();
        if !all_sizes.iter().any(|s| s.size == "full") {
            if let Some(url) = payload.url.clone() {
                all_sizes.push(ImageSizePayload {
                    size: "full".into(),
                    url,
                    path: payload.file.clone(),
                    width: payload.width,
                    height: payload.height,
                    mime_type: payload.mime_type.clone(),
                    file_size: payload.file_size,
                });
            }
        }

        let imgbb = self.imgbb.as_ref().ok_or_else(|| Error::ConfigError("imgbb.api_key not configured/enabled".into()))?;

        let mut replacements: Vec<(String, String)> = Vec::new();
        let mut processed = Vec::new();

        for sz in &all_sizes {
            let local = sz.path.clone().or_else(|| self.url_to_local(&sz.url));
            let Some(local_path) = local else {
                warn!(size=%sz.size, url=%sz.url, "no local path");
                continue;
            };
            if !std::path::Path::new(&local_path).exists() {
                warn!(path=%local_path, "file missing");
                continue;
            }

            let imgbb_name = std::path::Path::new(&local_path)
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            let imgbb_url = match self.cfg.imgbb.upload_format.as_str() {
                "blob" | "binary" | "file" => {
                    imgbb.upload_blob(&local_path, imgbb_name.as_deref()).await?
                }
                _ => {
                    // default "url": upload using direct WordPress image URL (imgbb fetches it)
                    imgbb.upload_url(&sz.url, imgbb_name.as_deref()).await?
                }
            };

            let cdn_url = self
                .db
                .upsert_cdn_mapping(payload.attachment_id, &sz.size, &sz.url, &imgbb_url, "imgbb")
                .await?;
            self.db
                .insert_url_version(payload.attachment_id, &sz.size, "imgbb", &sz.url, &imgbb_url)
                .await?;

            replacements.push((sz.url.clone(), cdn_url.clone()));
            self.db
                .enqueue_blogger_upload(payload.attachment_id, payload.post_id, &sz.size, &imgbb_url, &local_path)
                .await?;
            processed.push(serde_json::json!({
                "size": sz.size,
                "wp_url": sz.url,
                "imgbb_url": imgbb_url,
                "cdn_url": cdn_url
            }));
        }

        if replacements.is_empty() {
            return Ok(serde_json::json!({"ok": true, "attachment_id": payload.attachment_id, "status":"no_files", "processed": []}));
        }

        let replaced_posts = self.db.replace_urls_in_posts_by_in(&replacements).await.unwrap_or(0);
        let _ = self.db.replace_urls_in_postmeta_and_options(&replacements).await;

        let mut deleted = 0usize;
        for sz in &all_sizes {
            if let Some(local_path) = sz.path.clone().or_else(|| self.url_to_local(&sz.url)) {
                if std::path::Path::new(&local_path).exists() && std::fs::remove_file(&local_path).is_ok() {
                    deleted += 1;
                }
            }
        }

        Ok(serde_json::json!({
            "ok": true,
            "attachment_id": payload.attachment_id,
            "processed": processed,
            "replaced_posts": replaced_posts,
            "deleted_files": deleted,
            "queued_for_blogger": replacements.len()
        }))
    }

    fn url_to_local(&self, url: &str) -> Option<String> {
        if let Some(idx) = url.find("/wp-content/uploads/") {
            let rel = &url[idx + "/wp-content/uploads/".len()..];
            let rel = rel.split('?').next().unwrap_or(rel);
            let p = std::path::Path::new(&self.cfg.site.uploads_dir).join(rel);
            return Some(p.to_string_lossy().to_string());
        }
        None
    }
}

/// Background worker: upload queued imgbb-backed images to Blogger, then switch CDN mapping
/// to latest Blogger URL per (attachment_id, image_size).
pub async fn run_blogger_queue_worker(
    processor: Arc<WebhookProcessor>,
    cfg: Arc<crate::config::Config>,
    _engine: Option<Arc<crate::offload::OffloadEngine>>,
) {
    use crate::blogger;
    use crate::chrome::{default_chrome_path, Cdp};
    use std::path::PathBuf;

    info!("blogger queue worker started");
    loop {
        let jobs = match processor.db.get_queued_blogger_jobs(10).await {
            Ok(j) => j,
            Err(e) => {
                warn!("queue fetch failed: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                continue;
            }
        };
        if jobs.is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            continue;
        }

        for (job_id, attachment_id, _post_id, image_size, source_url, _local_path) in jobs {
            let _ = processor.db.mark_blogger_job_status(job_id, "processing", None, None).await;

            let tmp_path = std::env::temp_dir().join(format!("wpoa_{}_{}.img", attachment_id, image_size.replace([':', '/'], "_")));
            let download_ok = async {
                let resp = reqwest::get(&source_url).await.map_err(|e| e.to_string())?;
                let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
                tokio::fs::write(&tmp_path, &bytes).await.map_err(|e| e.to_string())?;
                Ok::<(), String>(())
            }
            .await;

            if let Err(e) = download_ok {
                let _ = processor.db.mark_blogger_job_status(job_id, "failed", None, Some(&e)).await;
                continue;
            }

            let chrome = cfg
                .chrome
                .executable
                .clone()
                .unwrap_or_else(|| default_chrome_path().to_string_lossy().to_string());
            let user_data = cfg.chrome.user_data_dir.clone().unwrap_or_else(|| {
                std::env::var("HOME")
                    .map(|h| format!("{}/.config/wp-opt-agents-chrome", h))
                    .unwrap_or_else(|_| ".wp-opt-agents-chrome".into())
            });
            let profile = if cfg.chrome.profile_directory.is_empty() { "Default".to_string() } else { cfg.chrome.profile_directory.clone() };
            let cdp = match Cdp::launch_and_connect(&chrome, &PathBuf::from(&user_data), &profile, cfg.chrome.headless, cfg.chrome.remote_debugging_port, cfg.chrome.launch_timeout_secs).await {
                Ok(c) => c,
                Err(e) => {
                    let _ = processor.db.mark_blogger_job_status(job_id, "failed", None, Some(&e.to_string())).await;
                    let _ = tokio::fs::remove_file(&tmp_path).await;
                    continue;
                }
            };

            let title = format!("Media {} {}", attachment_id, image_size);
            let res = blogger::upload_images_for_post(&cdp, &cfg.blogger.blog_id, &title, None, &[tmp_path.to_string_lossy().to_string()]).await;
            cdp.close().await;
            let _ = tokio::fs::remove_file(&tmp_path).await;

            match res {
                Ok((_link, ups)) => {
                    if let Some(up) = ups.first() {
                        let blogger_url = up.blogger_url.clone();
                        let _ = processor.db.insert_url_version(attachment_id, &image_size, "blogger", &source_url, &blogger_url).await;
                        let _ = processor.db.set_cdn_target_latest_blogger(attachment_id, &image_size, &blogger_url).await;
                        let _ = processor.db.mark_blogger_job_status(job_id, "done", Some(&blogger_url), None).await;
                        info!(attachment_id, size=%image_size, "blogger queue job done; CDN switched to blogger");
                    } else {
                        let _ = processor.db.mark_blogger_job_status(job_id, "failed", None, Some("no upload result")).await;
                    }
                }
                Err(e) => {
                    let _ = processor.db.mark_blogger_job_status(job_id, "failed", None, Some(&e.to_string())).await;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
}
