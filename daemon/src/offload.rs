use crate::blogger;
use crate::chrome::{default_chrome_path, Cdp};
use crate::config::Config;
use crate::db::Db;
use crate::error::Result;
use std::path::PathBuf;
use std::sync::Arc;

pub struct OffloadEngine {
    cfg: Arc<Config>,
    db: Db,
}

impl OffloadEngine {
    pub async fn new(cfg: Arc<Config>) -> Result<Self> {
        let db = Db::new((*cfg).clone())?;
        db.ping().await?;
        Ok(Self { cfg, db })
    }

    pub fn db(&self) -> &Db { &self.db }

    /// Process one WP post: discover images -> upload pending to Blogger (1 post = 1 blogger post) -> replace URLs in DB -> mark replaced & activate redirects -> link WP<->Blogger.
    pub async fn process_post(&self, post_id: i64) -> Result<serde_json::Value> {
        tracing::info!(post_id, "processing post");
        let rows = self.db.ensure_post_images_registered(post_id).await?;
        if rows.is_empty() {
            return Ok(serde_json::json!({"post_id": post_id, "status":"no_images", "uploaded":0, "replaced":0}));
        }

        let (blogger_post_id_existing, _) = self.db.get_post_blogger_link(post_id).await?.unwrap_or((String::new(), String::new()));

        // images needing upload (have local file and no blogger_url yet)
        let mut to_upload: Vec<(i64, String, String)> = Vec::new(); // (image_map_id, origin_url, local_path)
        let all = self.db.list_images_for_post(post_id).await?;
        for im in &all {
            if im.status == "replaced" && im.blogger_url.as_deref().unwrap_or("").len() > 0 { continue; }
            if im.blogger_url.as_deref().unwrap_or("").len() > 0 { continue; } // already uploaded
            // resolve local path
            let local = self.origin_to_local(&im.origin_url);
            if let Some(lp) = local {
                if std::path::Path::new(&lp).exists() {
                    to_upload.push((im.id, im.origin_url.clone(), lp));
                } else {
                    tracing::warn!("local file missing for image {} {}", im.id, im.origin_url);
                }
            }
        }

        let mut blogger_link: Option<(String,String)> = None; // (post_id,url)
        if !to_upload.is_empty() {
            let cdp = self.launch_chrome().await?;
            let title = self.db.get_post_title(post_id).await.unwrap_or_else(|_| format!("Post {post_id}"));
            let paths: Vec<String> = to_upload.iter().map(|x| x.2.clone()).collect();
            for (id,_,_) in &to_upload { let _ = self.db.mark_image_status(*id, "uploading", None).await; }
            let existing = if blogger_post_id_existing.is_empty() { None } else { Some(blogger_post_id_existing.as_str()) };
            let res = blogger::upload_images_for_post(&cdp, &self.cfg.blogger.blog_id, &title, existing, &paths).await;
            match res {
                Ok((link, ups)) => {
                    // map by path
                    for (i, up) in ups.iter().enumerate() {
                        if let Some((img_id, _, _)) = to_upload.get(i) {
                            let _ = self.db.mark_image_uploaded(*img_id, &up.blogger_url, up.blogger_image_id.as_deref()).await;
                        }
                    }
                    let _ = self.db.save_post_blogger_link(post_id, &link.post_id, &link.post_url, &self.cfg.blogger.blog_id).await;
                    blogger_link = Some((link.post_id, link.post_url));
                }
                Err(e) => {
                    for (id,_,_) in &to_upload { let _ = self.db.mark_image_status(*id, "failed", Some(&e.to_string())).await; }
                    cdp.close().await;
                    return Err(e);
                }
            }
            cdp.close().await;
        }

        // build replacements for uploaded images and apply DB replacements
        let refreshed = self.db.list_images_for_post(post_id).await?;
        let replacements: Vec<(String,String)> = refreshed.iter()
            .filter(|im| im.blogger_url.as_deref().unwrap_or("").len() > 0 && im.status != "replaced")
            .map(|im| (im.origin_url.clone(), im.blogger_url.clone().unwrap_or_default()))
            .collect();

        let mut applied = 0usize;
        if self.cfg.agent.replace_in_db && !replacements.is_empty() {
            applied = self.db.replace_urls_in_post_content(post_id, &replacements, self.cfg.agent.backup_before_replace).await?;
            let _ = self.db.replace_urls_in_postmeta_and_options(&replacements).await;
        }

        // mark replaced + activate redirect (plugin/daemon both can rely on DB state)
        let mut replaced = 0usize;
        for im in refreshed.iter() {
            if im.blogger_url.as_deref().unwrap_or("").is_empty() { continue; }
            if im.status == "replaced" { replaced += 1; continue; }
            // if replaced in content or replacement exists, mark replaced
            if replacements.iter().any(|(o,_)| o == &im.origin_url) || !self.cfg.agent.replace_in_db {
                let _ = self.db.mark_image_replaced(im.id).await;
                replaced += 1;
            }
        }

        Ok(serde_json::json!({
            "post_id": post_id,
            "blogger": blogger_link.map(|(id,u)| serde_json::json!({"post_id": id, "post_url": u})),
            "uploaded_now": to_upload.len(),
            "replacements_applied": applied,
            "replaced_mappings": replaced,
            "total_images": refreshed.len()
        }))
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
        tracing::info!("offload poll loop started ({}s)", interval);
        loop {
            match self.db.list_candidate_post_ids(20).await {
                Ok(ids) => {
                    for pid in ids {
                        // process posts that appear to have local uploads in content
                        match self.db.extract_post_images(pid).await {
                            Ok(imgs) if !imgs.is_empty() => {
                                // only process if there is pending/upload work
                                if let Ok(rows) = self.db.ensure_post_images_registered(pid).await {
                                    let needs = rows.iter().any(|r| r.status != "replaced" || r.blogger_url.as_deref().unwrap_or("").is_empty());
                                    if needs {
                                        match self.process_post(pid).await {
                                            Ok(r) => tracing::info!(post_id=pid, result=%r, "post processed"),
                                            Err(e) => tracing::error!(post_id=pid, error=%e, "post process failed"),
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
                Err(e) => tracing::error!("poll list posts failed: {}", e),
            }
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        }
    }
}

fn dirs_next_fallback() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") { PathBuf::from(home).join(".config") } else { PathBuf::from(".") }
}
