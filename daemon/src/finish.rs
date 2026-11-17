use crate::config::Config;
use crate::webhook::WebhookProcessor;
use std::sync::Arc;
use tracing::{info, warn};

/// Build the final resized/cropped image URL from an uploaded Blogger URL.
///
/// Blogger / googleusercontent URLs accept a size suffix:
///   `.../photo.jpg=s0`        -> original
///   `.../photo.jpg=s1024`     -> fit within 1024px (keeps aspect ratio)
///   `.../photo.jpg=s1024-c`   -> cropped to 1024x1024
///   `.../photo.jpg=w800-h600` -> explicit crop box
/// We map WP's registered size (width/height + crop flag) onto that suffix.
pub fn build_sized_url(base: &str, width: u32, height: u32, crop: bool) -> String {
    let base = base.trim();
    if base.is_empty() {
        return String::new();
    }
    let (head, existing) = match base.split_once('=') {
        Some((h, v)) => (h.to_string(), v.trim().to_string()),
        None => (base.to_string(), String::new()),
    };

    // explicit crop box when we know both dimensions and cropping is requested
    let suffix = if crop && width > 0 && height > 0 {
        format!("w{}-h{}", width, height)
    } else if width > 0 {
        format!("s{}", width)
    } else if !existing.is_empty() {
        existing
    } else {
        "s0".to_string()
    };

    format!("{}={}", head, suffix)
}

/// Finish worker: independent of the imgbb/webhook thread and of the Blogger upload
/// thread. It picks jobs whose Blogger upload is done, computes the resized/cropped
/// variant for the exact WP image size, records it as the newest URL version and
/// points the CDN mapping (and any direct Blogger URL in content) at it.
pub async fn run_finish_worker(processor: Arc<WebhookProcessor>, cfg: Arc<Config>) {
    let workers = cfg.agent.finish_workers.max(1) as usize;
    info!(target: "offload_worker", stage = "finish", workers = workers, "finish (crop/resize) worker started");

    loop {
        let jobs = match processor.db.get_uploaded_blogger_jobs(workers).await {
            Ok(j) => j,
            Err(e) => {
                warn!(target: "offload_worker", stage = "finish", "fetch uploaded jobs failed: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };
        if jobs.is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            continue;
        }

        let mut handles = Vec::new();
        for (job_id, attachment_id, post_id, image_size, blogger_url) in jobs {
            let p = processor.clone();
            handles.push(tokio::spawn(async move {
                finish_one(p, job_id, attachment_id, post_id, image_size, blogger_url).await;
            }));
        }
        for h in handles {
            let _ = h.await;
        }
    }
}

async fn finish_one(
    processor: Arc<WebhookProcessor>,
    job_id: i64,
    attachment_id: i64,
    post_id: Option<i64>,
    image_size: String,
    blogger_url: String,
) {
    let spec = processor.db.get_size_spec(attachment_id, &image_size).await.ok().flatten();
    let (w, h, crop) = spec.unwrap_or((0, 0, false));
    let sized = build_sized_url(&blogger_url, w, h, crop);

    info!(target: "offload_worker", stage = "finish", provider = "blogger", attachment_id = attachment_id, size = %image_size, target = %sized, width = w, height = h, crop = crop, "sized variant ready");

    if let Err(e) = processor
        .db
        .insert_url_version(attachment_id, &image_size, "blogger", &blogger_url, &sized)
        .await
    {
        warn!(target: "offload_worker", stage = "finish", attachment_id = attachment_id, size = %image_size, "insert version failed: {}", e);
    }

    if let Err(e) = processor
        .db
        .set_cdn_target_latest_blogger(attachment_id, &image_size, &sized)
        .await
    {
        warn!(target: "offload_worker", stage = "finish", attachment_id = attachment_id, size = %image_size, "cdn switch failed: {}", e);
    }

    // catch any content that still references the direct Blogger URL
    match processor
        .db
        .replace_urls_in_posts_by_in(&[(blogger_url.clone(), sized.clone())])
        .await
    {
        Ok(0) => {}
        Ok(n) => info!(target: "offload_worker", stage = "finish", provider = "wp_db", attachment_id = attachment_id, size = %image_size, replaced = n, "direct blogger urls replaced with sized variant"),
        Err(e) => warn!(target: "offload_worker", stage = "finish", attachment_id = attachment_id, size = %image_size, "content replace failed: {}", e),
    }

    let _ = processor
        .db
        .mark_blogger_job_status(job_id, "finished", Some(&sized), None)
        .await;

    if post_id.is_some() {
        info!(target: "offload_worker", stage = "finish", provider = "blogger", attachment_id = attachment_id, size = %image_size, target = %sized, "finish done");
    }
}

#[cfg(test)]
mod tests {
    use super::build_sized_url;

    #[test]
    fn sizes_googleusercontent() {
        let base = "https://lh3.googleusercontent.com/pw/ABC=s0";
        assert_eq!(build_sized_url(base, 300, 300, false), "https://lh3.googleusercontent.com/pw/ABC=s300");
        assert_eq!(build_sized_url(base, 150, 150, true), "https://lh3.googleusercontent.com/pw/ABC=w150-h150");
    }

    #[test]
    fn sizes_blogspot() {
        let base = "https://bp.blogspot.com/XYZ/photo.png=s1600";
        assert_eq!(build_sized_url(base, 1024, 0, false), "https://bp.blogspot.com/XYZ/photo.png=s1024");
        assert_eq!(build_sized_url(base, 768, 768, true), "https://bp.blogspot.com/XYZ/photo.png=w768-h768");
    }

    #[test]
    fn appends_when_no_suffix() {
        let base = "https://example.com/img/photo.jpg";
        assert_eq!(build_sized_url(base, 640, 0, false), "https://example.com/img/photo.jpg=s640");
    }
}
