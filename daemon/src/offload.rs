use crate::blogger;
use crate::chrome::{default_chrome_path, Cdp};
use crate::config::Config;
use crate::db::Db;
use crate::error::Result;
use crate::wp_api::{WpApiClient, WpImage, WpPostStatusResponse};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct CurrentTask {
    pub active: bool,
    pub kind: String,
    pub stage: String,
    pub post_id: Option<i64>,
    pub message: String,
    pub progress_current: usize,
    pub progress_total: usize,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
}

pub struct OffloadEngine {
    cfg: Arc<Config>,
    wp_api: WpApiClient,
    db: Db,
    current_task: Arc<RwLock<Option<CurrentTask>>>,
}

impl OffloadEngine {
    pub async fn new(cfg: Arc<Config>, current_task: Arc<RwLock<Option<CurrentTask>>>) -> Result<Self> {
        let base_url = format!("{}/wp-json/wp-opt-agents/v1", cfg.site.url.trim_end_matches('/'));
        let token = cfg.agent.daemon_api_token.clone();
        let wp_api = WpApiClient::new(base_url, token);
        let db = Db::new((*cfg).clone())?;
        db.ping().await?;
        Ok(Self { cfg, wp_api, db, current_task })
    }

    pub fn db(&self) -> &Db { &self.db }

    async fn set_task(&self, task: Option<CurrentTask>) {
        let mut guard = self.current_task.write().await;
        *guard = task;
    }

    async fn update_stage(&self, stage: &str, message: String, progress_current: usize, progress_total: usize, post_id: Option<i64>) {
        let now = chrono::Utc::now().to_rfc3339();
        let mut guard = self.current_task.write().await;
        let started = guard.as_ref().and_then(|t| t.started_at.clone()).or_else(|| Some(now.clone()));
        *guard = Some(CurrentTask {
            active: true,
            kind: "offload_post".to_string(),
            stage: stage.to_string(),
            post_id,
            message,
            progress_current,
            progress_total,
            started_at: started,
            updated_at: Some(now),
        });
    }

    /// Process one WP post using plugin REST API:
    /// 1. Get post status (images) from plugin
    /// 2. For each pending image, get sizes from plugin
    /// 3. Upload all images to Blogger via Chrome (1 WP post = 1 Blogger post)
    /// 4. Mark images uploaded in plugin with blogger URLs
    /// 5. Link blogger post in plugin
    /// 6. Mark images replaced in plugin (triggers 301 redirects + URL replacement)
    pub async fn process_post(&self, post_id: i64) -> Result<serde_json::Value> {
        info!(target: "offload_worker", stage = "post_process", post_id = post_id, "processing post via WP REST API");
        self.update_stage("fetching", format!("Đang lấy trạng thái ảnh của bài viết #{}", post_id), 0, 1, Some(post_id)).await;

        // 1. Get post status from plugin
        let post_status = match self.wp_api.get_post_status(post_id).await {
            Ok(v) => v,
            Err(e) => { self.set_task(None).await; return Err(e); }
        };
        if post_status.images.is_empty() {
            self.set_task(None).await;
            return Ok(serde_json::json!({"post_id": post_id, "status":"no_images", "uploaded":0, "replaced":0}));
        }

        // Filter images needing upload
        let to_upload: Vec<WpImage> = post_status.images.into_iter()
            .filter(|im| im.status != "replaced" && im.blogger_url.as_deref().unwrap_or("").is_empty())
            .collect();

        if to_upload.is_empty() {
            self.set_task(None).await;
            return Ok(serde_json::json!({"post_id": post_id, "status":"all_uploaded", "uploaded":0, "replaced":0}));
        }

        // 2. Resolve local paths and upload
        let paths: Vec<String> = to_upload.iter()
            .filter_map(|im| self.origin_to_local(&im.origin_url))
            .filter(|p| std::path::Path::new(p).exists())
            .collect();

        if paths.is_empty() {
            warn!(target: "offload_worker", stage = "post_process", post_id = post_id, "no local files found for images");
            self.set_task(None).await;
            return Ok(serde_json::json!({"post_id": post_id, "status":"no_local_files", "uploaded":0, "replaced":0}));
        }

        self.update_stage("launching_chrome", format!("Đang khởi động Chrome để upload {} ảnh (post #{})", paths.len(), post_id), 0, paths.len(), Some(post_id)).await;

        // 3. Upload to Blogger
        let cdp = match self.launch_chrome().await {
            Ok(v) => v,
            Err(e) => { self.set_task(None).await; return Err(e); }
        };
        let title = self.get_post_title_via_api(post_id).await.unwrap_or_else(|_| format!("Post {post_id}"));
        let existing_post_id = self.get_existing_blogger_post_id(post_id).await.unwrap_or_default();

        self.update_stage("uploading", format!("Đang upload ảnh lên Blogger (post #{}, {} ảnh)", post_id, paths.len()), 0, paths.len(), Some(post_id)).await;

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
                self.set_task(None).await;
                return Err(e);
            }
        };
        cdp.close().await;

        // 4. Mark images uploaded in plugin with blogger URLs
        self.update_stage("saving_upload", format!("Đang lưu URL Blogger vào plugin (post #{})", post_id), paths.len(), paths.len(), Some(post_id)).await;
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
        self.update_stage("linking", format!("Đang liên kết WP post #{} ↔ Blogger post", post_id), paths.len(), paths.len(), Some(post_id)).await;
        let blogger_album_url = format!("https://www.blogger.com/blog/post/{}", blogger_post_id);
        let _ = self.wp_api.link_blogger_post(
            post_id,
            &blogger_post_id,
            &blogger_post_url,
            &blogger_album_url,
            &self.cfg.blogger.blog_id
        ).await;

        // 6. Mark images replaced in plugin (triggers 301 redirects + URL replacement in WP)
        self.update_stage("replacing", format!("Đang thay URL trong WP DB và bật redirect (post #{})", post_id), 0, to_upload.len(), Some(post_id)).await;
        let mut replaced_count = 0;
        for (idx, im) in to_upload.iter().enumerate() {
            if let Err(e) = self.wp_api.mark_image_replaced(im.id).await {
                warn!(target: "offload_worker", stage = "replace", post_id = post_id, "failed to mark replaced in plugin: {}", e);
            } else {
                replaced_count += 1;
            }
            self.update_stage("replacing", format!("Đã xử lý {}/{} ảnh (post #{})", idx + 1, to_upload.len(), post_id), idx + 1, to_upload.len(), Some(post_id)).await;
        }

        self.set_task(None).await;
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
            self.set_task(Some(CurrentTask {
                active: false,
                kind: "idle".to_string(),
                stage: "waiting".to_string(),
                post_id: None,
                message: "Đang chờ tác vụ offload tiếp theo...".to_string(),
                progress_current: 0,
                progress_total: 0,
                started_at: None,
                updated_at: Some(chrono::Utc::now().to_rfc3339()),
            })).await;
            match self.poll_candidate_posts().await {
                Ok(ids) => {
                    for pid in ids {
                        match self.process_post(pid).await {
                            Ok(r) => info!(target: "offload_worker", stage = "post_process", post_id = pid, result = %r, "post processed"),
                            Err(e) => warn!(target: "offload_worker", stage = "post_process", post_id = pid, "post process failed: {}", e),
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
                Err(e) => warn!(target: "offload_worker", stage = "poll", "poll list posts failed: {}", e),
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