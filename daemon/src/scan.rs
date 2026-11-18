use crate::config::Config;
use crate::offload::CurrentTask;
use crate::webhook::WebhookProcessor;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Background worker that scans the WordPress media library and offloads
/// attachments that have not been processed yet (no CDN/blogger mapping).
pub async fn run_media_scan_worker(
    processor: Arc<WebhookProcessor>,
    cfg: Arc<Config>,
    task: Arc<RwLock<Option<CurrentTask>>>,
) {
    if !cfg.agent.scan_enabled {
        info!(target: "offload_worker", stage = "scan", "media scan disabled (agent.scan_enabled=false)");
        return;
    }
    let interval = cfg.agent.scan_interval_secs.max(30);
    let batch = (cfg.agent.scan_batch_size as usize).clamp(1, 200);
    info!(target: "offload_worker", stage = "scan", interval_secs = interval, batch = batch, "media scan worker started");

    // Kick off shortly after startup instead of waiting a full interval.
    tokio::time::sleep(std::time::Duration::from_secs(20)).await;

    loop {
        let pending = match processor.db.list_pending_attachments(batch).await {
            Ok(v) => v,
            Err(e) => {
                warn!(target: "offload_worker", stage = "scan", "scan query failed: {}", e);
                continue;
            }
        };
        if pending.is_empty() {
            // nothing left to do — back off until the next interval
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            continue;
        }

        let total_pending = processor.db.count_pending_attachments().await.unwrap_or(pending.len() as i64);
        info!(target: "offload_worker", stage = "scan", found = pending.len(), total_pending = total_pending, "media scan started");

        for (idx, attachment_id) in pending.iter().copied().enumerate() {
            set_task(&task, "scan", format!("Đang offload media #{} ({}/{})", attachment_id, idx + 1, pending.len()), idx, pending.len()).await;

            let payload = match processor.db.get_attachment_payload(attachment_id).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    warn!(target: "offload_worker", stage = "scan", attachment_id = attachment_id, "attachment payload not found");
                    continue;
                }
                Err(e) => {
                    warn!(target: "offload_worker", stage = "scan", attachment_id = attachment_id, "payload load failed: {}", e);
                    continue;
                }
            };

            if processor.imgbb.is_none() {
                warn!(target: "offload_worker", stage = "scan", attachment_id = attachment_id, "imgbb not configured (agent.imgbb.api_key) - skipping media offload");
                break;
            }

            match processor.process_media_upload(payload).await {
                Ok(v) => info!(target: "offload_worker", stage = "scan", attachment_id = attachment_id, result = %v, "attachment offloaded"),
                Err(e) => warn!(target: "offload_worker", stage = "scan", attachment_id = attachment_id, "attachment offload failed: {}", e),
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        let skipped = processor.db.count_skipped_attachments().await.unwrap_or(0);
        set_task_done(&task, format!("Scan xong {} media (bỏ qua {})", pending.len(), skipped)).await;

        // keep going immediately while there is still pending work
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
}

pub async fn scan_once(
    processor: Arc<WebhookProcessor>,
    cfg: Arc<Config>,
    task: Arc<RwLock<Option<CurrentTask>>>,
    batch: Option<usize>,
) -> Result<usize, String> {
    let batch = batch.unwrap_or(cfg.agent.scan_batch_size as usize).clamp(1, 200);
    if processor.imgbb.is_none() {
        return Err("imgbb.api_key chưa cấu hình".into());
    }
    let pending = processor.db.list_pending_attachments(batch).await.map_err(|e| e.to_string())?;
    let total = pending.len();
    info!(target: "offload_worker", stage = "scan", found = total, "manual media scan started");

    for (idx, attachment_id) in pending.iter().copied().enumerate() {
        set_task(&task, "scan", format!("Đang offload media #{} ({}/{})", attachment_id, idx + 1, total), idx, total).await;
        let payload = match processor.db.get_attachment_payload(attachment_id).await {
            Ok(Some(p)) => p,
            _ => continue,
        };
        match processor.process_media_upload(payload).await {
            Ok(_) => info!(target: "offload_worker", stage = "scan", attachment_id = attachment_id, "attachment offloaded"),
            Err(e) => warn!(target: "offload_worker", stage = "scan", attachment_id = attachment_id, "attachment offload failed: {}", e),
        }
    }
    set_task_done(&task, format!("Scan xong {} media", total)).await;
    Ok(total)
}

async fn set_task(task: &Arc<RwLock<Option<CurrentTask>>>, stage: &str, message: String, current: usize, total: usize) {
    let now = chrono::Utc::now().to_rfc3339();
    let mut guard = task.write().await;
    let started = guard.as_ref().and_then(|t| t.started_at.clone()).or_else(|| Some(now.clone()));
    *guard = Some(CurrentTask {
        active: true,
        kind: "media_scan".to_string(),
        stage: stage.to_string(),
        post_id: None,
        message,
        progress_current: current,
        progress_total: total,
        started_at: started,
        updated_at: Some(now),
    });
}

async fn set_task_done(task: &Arc<RwLock<Option<CurrentTask>>>, message: String) {
    let now = chrono::Utc::now().to_rfc3339();
    let mut guard = task.write().await;
    let started = guard.as_ref().and_then(|t| t.started_at.clone()).or_else(|| Some(now.clone()));
    *guard = Some(CurrentTask {
        active: false,
        kind: "media_scan".to_string(),
        stage: "done".to_string(),
        post_id: None,
        message,
        progress_current: 0,
        progress_total: 0,
        started_at: started,
        updated_at: Some(now),
    });
}
