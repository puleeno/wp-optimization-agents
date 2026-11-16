use crate::blogger;
use crate::chrome::{default_chrome_path, Cdp};
use crate::config::Config;
use crate::db::Db;
use crate::error::Result;
use crate::wp_api::{WpApiClient, WpImage, WpPostStatusResponse};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

pub struct OffloadEngine {
    cfg: Arc<Config>,
    wp_api: WpApiClient,
    db: Db,
}

impl OffloadEngine {
    pub async fn new(cfg: Arc<Config>) -> Result<Self> {
        let base_url = format!("{}/wp-json/wp-opt-agents/v1", cfg.site.url.trim_end_matches('/'));
        let token = cfg.agent.daemon_api_token.clone();
        let wp_api = WpApiClient::new(base_url, token);
        let db = Db::new((*cfg).clone())?;
        db.ping().await?;
        Ok(Self { cfg, wp_api, db })
    }

    pub fn db(&self) -> &Db { &self.db }

    /// Process one WP post using plugin REST API:
    /// 1. Get post status (images) from plugin
    /// 2. For each pending image, get sizes from plugin
    /// 3. Upload all images to Blogger via Chrome (1 WP post = 1 Blogger post)
    /// 4. Mark images uploaded in plugin with blogger URLs
    /// 5. Link blogger post in plugin
    /// 6. Mark images replaced in plugin (triggers 301 redirects + URL replacement)
    pub async fn process_post(&self, post_id: i64) -> Result<serde_json::Value> {
        info!(post_id, "processing post via WP REST API");

        // 1. Get post status from plugin
        let post_status = self.wp_api.get_post_status(post_id).await?;
        if post_status.images.is_empty() {
            return Ok(serde_json::json!({"post_id": post_id, "status":"no_images", "uploaded":0, "replaced":0}));
        }

        // Filter images needing upload
        let to_upload: Vec<WpImage> = post_status.images.into_iter()
            .filter(|im| im.status != "replaced" && im.blogger_url.as_deref().unwrap_or("").is_empty())
            .collect();

        if to_upload.is_empty() {
            return Ok(serde_json::json!({"post_id": post_id, "status":"all_uploaded", "uploaded":0, "replaced":0}));
        }

        // 2. Resolve local paths and upload
        let paths: Vec<String> = to_upload.iter()
            .filter_map(|im| self.origin_to_local(&im.origin_url))
            .filter(|p| std::path::Path::new(p).exists())
            .collect();

        if paths.is_empty() {
            warn!(post_id, "no local files found for images");
            return Ok(serde_json::json!({"post_id": post_id, "status":"no_local_files", "uploaded":0, "replaced":0}));
        }

        // 3. Upload to Blogger
        let cdp = self.launch_chrome().await?;
        let title = self.get_post_title_via_api(post_id).await.unwrap_or_else(|_| format!("Post {post_id}"));
        let existing_post_id = self.get_existing_blogger_post_id(post_id).await?;

        let existing = if existing_post_id.is_empty() { None } else { Some(existing_post_id.as_str()) };
        let res = blogger::upload_images_for_post(&cdp, &self.cfg.blogger.blog_id, &title, existing, &paths).await;
        
        let (blogger_post_id, blogger_post_url, uploaded_images) = match res {
            Ok((link, ups)) => (link.post_id, link.post_url, ups),
            Err(e) => {
                // Mark failed in plugin
                for im in &to_upload {
                    let _ = self.wp_api.mark_image_failed(im.id, &e.to_string()).await;
                }
                cdp.close().await;
                return Err(e);
            }
        };
        cdp.close().await;

        // 4. Mark images uploaded in plugin with blogger URLs
        for (i, up) in uploaded_images.iter().enumerate() {
            if i < to_upload.len() {
                let im = &to_upload[i];
                let _ = self.wp_api.mark_image_uploaded(
                    im.id,
                    &up.blogger_url,
                    up.blogger_image_id.as_deref(),
                    None
                ).await;
            }
        }

        // 5. Link blogger post in plugin
        let blogger_album_url = format!("https://www.blogger.com/blog/post/{}", blogger_post_id);
        let _ = self.wp_api.link_blogger_post(
            post_id,
            &blogger_post_id,
            &blogger_post_url,
            &blogger_album_url,
            &self.cfg.blogger.blog_id
        ).await;

        // 6. Mark images replaced in plugin (triggers 301 redirects + URL replacement in WP)
        let mut replaced_count = 0;
        for im in &to_upload {
            if let Err(e) = self.wp_api.mark_image_replaced(im.id).await {
                warn!(image_id = im.id, error = %e, "failed to mark replaced in plugin");
            } else {
                replaced_count += 1;
            }
        }

        Ok(serde_json::json!({
            "post_id": post_id,
            "blogger": {
                "post_id": blogger_post_id,
                "post_url": blogger_post_url,
            },
            "uploaded_now": to_upload.len(),
            "replaced_mappings": replaced_count,
        }))
    }

    async fn get_post_title_via_api(&self, post_id: i64) -> Result<String> {
        // Could add a REST endpoint for this, fallback to default
        Ok(format!("Post {post_id}"))
    }

    async fn get_existing_blogger_post_id(&self, _post_id: i64) -> Result<String> {
        // Could add a REST endpoint for this
        Ok(String::new())
    }

    fn origin_to_local(&self, url: &str) -> Option<String> {
        if let Some(idx) = url.find("/wp-content/uploads/") {
            let rel = &url[idx + "/wp-content/uploads/".len()..];
            let rel = rel.split('?').next().unwrap_or(rel);
            let p = PathBuf::from(&self.cfg.site.uploads_dir).join(rel);
            return Some(p.to_string_lossy().to_string());
        }
        None
    }

    async fn launch_chrome(&self) -> Result<Cdp> {
        let chrome = self.cfg.chrome.executable.clone().unwrap_or_else(|| default_chrome_path().to_string_lossy().to_string());
        let user_data = self.cfg.chrome.user_data_dir.clone()
            .unwrap_or_else(|| dirs_next_fallback().join("wp-opt-agents-chrome").to_string_lossy().to_string());
        let profile = if self.cfg.chrome.profile_directory.is_empty() { "Default".to_string() } else { self.cfg.chrome.profile_directory.clone() };
        Cdp::launch_and_connect(&chrome, &PathBuf::from(user_data), &profile, self.cfg.chrome.headless, self.cfg.chrome.remote_debugging_port, self.cfg.chrome.launch_timeout_secs).await
    }

    pub async fn run_poll_loop(self: Arc<Self>) {
        let interval = std::cmp::max(10, self.cfg.agent.poll_interval_secs);
        info!("offload poll loop started ({}s)", interval);
        loop {
            // Poll posts from plugin stats endpoint
            match self.poll_candidate_posts().await {
                Ok(ids) => {
                    for pid in ids {
                        match self.process_post(pid).await {
                            Ok(r) => info!(post_id=pid, result=%r, "post processed"),
                            Err(e) => warn!(post_id=pid, error=%e, "post process failed"),
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
                Err(e) => warn!("poll list posts failed: {}", e),
            }
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        }
    }

    async fn poll_candidate_posts(&self) -> Result<Vec<i64>> {
        // Get stats to find posts with pending images
        // For now, return empty - could add a REST endpoint for this
        Ok(vec![])
    }
}

fn dirs_next_fallback() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") { PathBuf::from(home).join(".config") } else { PathBuf::from(".") }
}