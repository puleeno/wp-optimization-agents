use crate::db::Db;
use crate::error::{Error, Result};
use crate::imgbb::ImgbbClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

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
        info!(target: "webhook", stage = "receive", attachment_id = payload.attachment_id, post_id = payload.post_id, sizes = payload.sizes.len(), "media upload webhook received");
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

        let imgbb = match self.imgbb.as_ref() {
            Some(c) => c,
            None => {
                error!(target: "webhook", stage = "config", provider = "imgbb", attachment_id = payload.attachment_id, "imgbb.api_key not configured/enabled - cannot process media upload");
                return Err(Error::ConfigError("imgbb.api_key not configured/enabled".into()));
            }
        };

        let mut replacements: Vec<(String, String)> = Vec::new();
        let mut processed = Vec::new();
        let mut failed_sizes = 0usize;

        for sz in &all_sizes {
            let local = sz.path.clone().or_else(|| self.url_to_local(&sz.url));
            let existing = local
                .as_deref()
                .filter(|p| std::path::Path::new(p).exists())
                .map(|p| p.to_string());
            if existing.is_none() {
                warn!(target: "webhook", stage = "local_path", provider = "imgbb", attachment_id = payload.attachment_id, size = %sz.size, target = %sz.url, "file missing on disk - sẽ thử upload bằng URL");
            }
            let local_path = existing.clone().unwrap_or_default();

            let imgbb_name = std::path::Path::new(&local_path)
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .or_else(|| sz.url.split('/').last().map(|s| s.to_string()));

            info!(target: "webhook", stage = "imgbb_upload", provider = "imgbb", attachment_id = payload.attachment_id, size = %sz.size, target = %sz.url, mode = if existing.is_some() { self.cfg.imgbb.upload_format.clone() } else { "url".to_string() }, "uploading size to imgbb");
            let started = std::time::Instant::now();
            let use_blob = existing.is_some()
                && matches!(self.cfg.imgbb.upload_format.as_str(), "blob" | "binary" | "file");
            let mut imgbb_res = if use_blob {
                imgbb.upload_blob(&local_path, imgbb_name.as_deref()).await
            } else {
                imgbb.upload_url(&sz.url, imgbb_name.as_deref()).await
            };

            // Smart fallback: if blob upload is impossible (local file missing) and the WP URL is
            // publicly reachable, retry with the URL form instead of giving up on this size.
            if let Err(blob_err) = &imgbb_res {
                if use_blob {
                    warn!(target: "webhook", stage = "imgbb_upload", provider = "imgbb", attachment_id = payload.attachment_id, size = %sz.size, target = %sz.url, "blob upload failed ({}), thử lại bằng URL", blob_err);
                    imgbb_res = imgbb.upload_url(&sz.url, imgbb_name.as_deref()).await;
                }
            }

            let imgbb_url = match imgbb_res {
                Ok(u) => {
                    info!(target: "webhook", stage = "imgbb_upload", provider = "imgbb", attachment_id = payload.attachment_id, size = %sz.size, target = %u, elapsed_ms = started.elapsed().as_millis() as u64, "imgbb upload ok");
                    u
                }
                Err(e) => {
                    error!(target: "webhook", stage = "imgbb_upload", provider = "imgbb", attachment_id = payload.attachment_id, size = %sz.size, target = %sz.url, "imgbb upload failed: {}", e);
                    failed_sizes += 1;
                    continue;
                }
            };

            let cdn_url = self
                .db
                .upsert_cdn_mapping(payload.attachment_id, &sz.size, &sz.url, &imgbb_url, "imgbb")
                .await?;
            info!(target: "webhook", stage = "cdn_map", provider = "imgbb", attachment_id = payload.attachment_id, size = %sz.size, target = %cdn_url, "cdn mapping -> imgbb (pending blogger)");
            self.db
                .insert_url_version(payload.attachment_id, &sz.size, "imgbb", &sz.url, &imgbb_url)
                .await?;

            replacements.push((sz.url.clone(), cdn_url.clone()));
            self.db
                .enqueue_blogger_upload(payload.attachment_id, payload.post_id, &sz.size, &imgbb_url, &local_path)
                .await?;
            info!(target: "webhook", stage = "queue", provider = "blogger", attachment_id = payload.attachment_id, size = %sz.size, target = %imgbb_url, "queued for blogger upload");
            processed.push(serde_json::json!({
                "size": sz.size,
                "wp_url": sz.url,
                "imgbb_url": imgbb_url,
                "cdn_url": cdn_url
            }));
        }

        if replacements.is_empty() {
            let reason = if failed_sizes > 0 {
                format!("imgbb failed for {} size(s)", failed_sizes)
            } else {
                "no local file and WP URL not reachable".to_string()
            };
            warn!(target: "webhook", stage = "skip", provider = "imgbb", attachment_id = payload.attachment_id, "{}", reason);
            let _ = self
                .db
                .mark_attachment_skipped(payload.attachment_id, payload.post_id, &reason)
                .await;
            return Ok(serde_json::json!({
                "ok": true,
                "attachment_id": payload.attachment_id,
                "status": "skipped",
                "reason": reason,
                "processed": []
            }));
        }

        let replaced_posts = self.db.replace_urls_in_posts_by_in(&replacements).await.unwrap_or(0);
        info!(target: "webhook", stage = "replace", provider = "wp_db", attachment_id = payload.attachment_id, replaced_posts = replaced_posts, "wp post content urls replaced");
        match self.db.replace_urls_in_postmeta_and_options(&replacements).await {
            Ok(n) if n > 0 => info!(target: "webhook", stage = "replace", provider = "wp_db", attachment_id = payload.attachment_id, replaced_meta = n, "wp postmeta/options urls replaced"),
            Ok(_) => debug!(target: "webhook", stage = "replace", provider = "wp_db", attachment_id = payload.attachment_id, "no postmeta/options replacements"),
            Err(e) => warn!(target: "webhook", stage = "replace", provider = "wp_db", attachment_id = payload.attachment_id, "postmeta/options replace failed: {}", e),
        }

        let mut deleted = 0usize;
        if self.cfg.agent.delete_local_after_offload {
            for sz in &all_sizes {
                if let Some(local_path) = sz.path.clone().or_else(|| self.url_to_local(&sz.url)) {
                    if std::path::Path::new(&local_path).exists() && std::fs::remove_file(&local_path).is_ok() {
                        deleted += 1;
                    }
                }
            }
        } else if !all_sizes.is_empty() {
            info!(target: "webhook", stage = "cleanup", provider = "wp_fs", attachment_id = payload.attachment_id, "delete_local_after_offload=false - local files kept");
        }

        info!(target: "webhook", stage = "done", provider = "imgbb", attachment_id = payload.attachment_id, queued = replacements.len(), failed_sizes = failed_sizes, replaced_posts = replaced_posts, deleted_files = deleted, "media upload processed");

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

    // Profile that holds the Blogger session (same one used by `chrome setup/login`).
    let profile_dir = cfg
        .chrome
        .user_data_dir
        .clone()
        .unwrap_or_else(|| crate::chrome::default_user_data_dir().to_string_lossy().to_string());
    let profile_path = PathBuf::from(&profile_dir);

    info!("blogger queue worker started");
    if let Err(e) = processor.db.reset_stale_blogger_jobs(600).await {
        warn!("reset stale blogger jobs failed: {}", e);
    }
    let mut cycles: u64 = 0;
    let mut last_login_warn: Option<std::time::Instant> = None;
    loop {
        cycles += 1;

        // Login gate: never launch Chrome / touch jobs until `chrome login` has been done.
        if !crate::cli::is_login_ok(&profile_path) {
            let now = std::time::Instant::now();
            let should_log = last_login_warn.map(|t| now.duration_since(t).as_secs() > 60).unwrap_or(true);
            if should_log {
                warn!(target: "webhook", stage = "blogger_login", provider = "chrome_cdp", profile = %profile_dir, "Blogger chưa đăng nhập - TẠM DỪNG queue. Chạy: wp-offload-daemon chrome login --save");
                last_login_warn = Some(now);
            }
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            continue;
        }
        if cycles % 30 == 0 {
            if let Err(e) = processor.db.reset_stale_blogger_jobs(600).await {
                warn!("reset stale blogger jobs failed: {}", e);
            }
        }
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

        for (job_id, attachment_id, _post_id, image_size, source_url, local_path) in jobs {
            info!(target: "webhook", stage = "blogger_job", provider = "blogger", attachment_id = attachment_id, size = %image_size, target = %source_url, job_id = job_id, "blogger job picked");
            let _ = processor.db.mark_blogger_job_status(job_id, "processing", None, None).await;

            let tmp_path = std::env::temp_dir().join(format!("wpoa_{}_{}.img", attachment_id, image_size.replace([':', '/'], "_")));

            // 1) prefer the local file we just uploaded to imgbb (avoids fetching i.ibb.co)
            let mut prepared = false;
            if let Some(p) = local_path.clone() {
                if std::path::Path::new(&p).exists() {
                    match std::fs::copy(&p, &tmp_path) {
                        Ok(_) => {
                            prepared = true;
                            debug!(target: "webhook", stage = "blogger_job", provider = "blogger", attachment_id = attachment_id, size = %image_size, path = %p, "using local file for blogger upload");
                        }
                        Err(e) => warn!(target: "webhook", stage = "blogger_job", provider = "blogger", attachment_id = attachment_id, size = %image_size, path = %p, "copy local file failed: {}", e),
                    }
                }
            }

            // 2) otherwise download the source URL with retries
            if !prepared {
                let mut last_err = String::new();
                for attempt in 1..=3 {
                    let res = async {
                        let resp = reqwest::get(&source_url).await.map_err(|e| e.to_string())?;
                        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
                        tokio::fs::write(&tmp_path, &bytes).await.map_err(|e| e.to_string())?;
                        Ok::<(), String>(())
                    }
                    .await;
                    match res {
                        Ok(()) => {
                            prepared = true;
                            debug!(target: "webhook", stage = "blogger_job", provider = "blogger", attachment_id = attachment_id, size = %image_size, "source downloaded for blogger upload");
                            break;
                        }
                        Err(e) => {
                            last_err = e;
                            if attempt < 3 {
                                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                            }
                        }
                    }
                }
                if !prepared {
                    warn!(target: "webhook", stage = "blogger_job", provider = "blogger", attachment_id = attachment_id, size = %image_size, target = %source_url, "blogger job source unavailable: {}", last_err);
                    let _ = processor.db.mark_blogger_job_status(job_id, "failed", None, Some(&last_err)).await;
                    continue;
                }
            }

            let chrome = cfg
                .chrome
                .executable
                .clone()
                .unwrap_or_else(|| default_chrome_path().to_string_lossy().to_string());
            let user_data = cfg.chrome.user_data_dir.clone()
                .unwrap_or_else(|| crate::chrome::default_user_data_dir().to_string_lossy().to_string());
            let profile = if cfg.chrome.profile_directory.is_empty() { "Default".to_string() } else { cfg.chrome.profile_directory.clone() };
            let mut cdp_opt = None;
            let mut last_err = String::new();
            for attempt in 1..=2 {
                match Cdp::launch_and_connect(&chrome, &PathBuf::from(&user_data), &profile, cfg.chrome.headless, cfg.chrome.remote_debugging_port, cfg.chrome.launch_timeout_secs).await {
                    Ok(c) => {
                        cdp_opt = Some(c);
                        break;
                    }
                    Err(e) => {
                        last_err = e.to_string();
                        if attempt == 1 {
                            warn!(target: "webhook", stage = "blogger_job", provider = "chrome_cdp", attachment_id = attachment_id, size = %image_size, "chrome launch attempt 1 failed ({}) - retrying", e);
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        }
                    }
                }
            }
            let cdp = match cdp_opt {
                Some(c) => c,
                None => {
                    let hint = format!(
                        "{} | profile={} port={} (neu Chrome dang mo san: hay tat han Cmd+Q roi thu lai, hoac mo Chrome voi --remote-debugging-port={})",
                        last_err, user_data, cfg.chrome.remote_debugging_port, cfg.chrome.remote_debugging_port
                    );
                    warn!(target: "webhook", stage = "blogger_job", provider = "chrome_cdp", attachment_id = attachment_id, size = %image_size, "{}", hint);
                    let _ = processor.db.mark_blogger_job_status(job_id, "failed", None, Some(&hint)).await;
                    let _ = tokio::fs::remove_file(&tmp_path).await;
                    continue;
                }
            };
            info!(target: "webhook", stage = "blogger_job", provider = "chrome_cdp", attachment_id = attachment_id, size = %image_size, "chrome cdp connected; uploading to blogger");

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
                        // Hand off to the finish worker (crop/resize) instead of marking done here.
                        let _ = processor.db.mark_blogger_job_status(job_id, "uploaded", Some(&blogger_url), None).await;
                        info!(target: "webhook", stage = "blogger_job", provider = "blogger", attachment_id = attachment_id, size = %image_size, target = %blogger_url, "blogger upload done; queued for crop/resize finish");
                    } else {
                        warn!(target: "webhook", stage = "blogger_job", provider = "blogger", attachment_id = attachment_id, size = %image_size, "blogger upload returned no result");
                        let _ = processor.db.mark_blogger_job_status(job_id, "failed", None, Some("no upload result")).await;
                    }
                }
                Err(e) => {
                    let msg = e.to_string();
                    let looks_like_login = msg.to_lowercase().contains("sign in")
                        || msg.to_lowercase().contains("signin")
                        || msg.to_lowercase().contains("login")
                        || msg.to_lowercase().contains("composer");
                    if looks_like_login {
                        warn!(target: "webhook", stage = "blogger_login", provider = "chrome_cdp", attachment_id = attachment_id, size = %image_size, "Blogger yêu cầu đăng nhập - dừng queue, job được giữ ở trạng thái queued");
                        // put the job back so it retries after login, don't burn it as failed
                        let _ = processor.db.mark_blogger_job_status(job_id, "queued", None, Some("waiting for blogger login")).await;
                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                        break;
                    }
                    error!(target: "webhook", stage = "blogger_job", provider = "blogger", attachment_id = attachment_id, size = %image_size, "blogger upload failed: {}", e);
                    let _ = processor.db.mark_blogger_job_status(job_id, "failed", None, Some(&msg)).await;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
}
