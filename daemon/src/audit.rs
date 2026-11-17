use crate::chrome::{default_chrome_path, Cdp};
use crate::config::Config;
use crate::db::Db;
use crate::error::Result;
use crate::offload::CurrentTask;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

const LCP_OBSERVER: &str = r#"
window.__wpoa = window.__wpoa || { lcp: 0, cls: 0 };
try {
  new PerformanceObserver((l) => {
    for (const e of l.getEntries()) { window.__wpoa.lcp = e.startTime; }
  }).observe({ type: 'largest-contentful-paint', buffered: true });
  new PerformanceObserver((l) => {
    for (const e of l.getEntries()) { if (!e.hadRecentInput) { window.__wpoa.cls += e.value; } }
  }).observe({ type: 'layout-shift', buffered: true });
} catch (e) {}
"#;

const METRICS_JS: &str = r#"
(() => {
  const nav = (performance.getEntriesByType('navigation') || [])[0] || {};
  const paints = performance.getEntriesByType('paint') || [];
  let fcpEntry = (performance.getEntriesByName('first-contentful-paint') || [])[0] || {};
  if (!fcpEntry.startTime) {
    const p = paints.find((x) => x.name === 'first-contentful-paint');
    if (p) fcpEntry = p;
  }
  const res = performance.getEntriesByType('resource') || [];
  let transfer = nav.transferSize || 0;
  for (const r of res) transfer += (r.transferSize || 0);
  const imgs = Array.from(document.images || []).map((i) => i.currentSrc || i.src || '');
  const cdn = imgs.filter((s) => /cdn\.|-[0-9a-f]{12,}\.[a-z0-9]+(\?|$)/i.test(s)).length;
  const w = window.__wpoa || {};
  let lcp = w.lcp || 0;
  if (!lcp) {
    const l = performance.getEntriesByType('largest-contentful-paint') || [];
    if (l.length) lcp = l[l.length - 1].startTime;
  }
  let cls = w.cls || 0;
  if (!cls) {
    let acc = 0;
    for (const e of performance.getEntriesByType('layout-shift') || []) { if (!e.hadRecentInput) acc += e.value; }
    cls = acc;
  }
  return JSON.stringify({
    lcp: lcp,
    cls: cls,
    fcp: fcpEntry.startTime || 0,
    ttfb: nav.responseStart || 0,
    dcl: nav.domContentLoadedEventEnd || 0,
    load: nav.loadEventEnd || 0,
    transfer: transfer,
    requests: res.length + 1,
    images: imgs.length,
    cdnImages: cdn,
    title: document.title || ''
  });
})()
"#;

#[derive(Debug, Clone, Serialize)]
pub struct AuditResult {
    pub url: String,
    pub device: String,
    pub ok: bool,
    pub lcp: f64,
    pub cls: f64,
    pub fcp: f64,
    pub ttfb_ms: u64,
    pub load_ms: u64,
    pub transfer_kb: f64,
    pub requests: u32,
    pub images: u32,
    pub cdn_images: u32,
    pub score: u32,
    pub status: String,
    pub notes: String,
}

pub struct AuditRunner {
    pub db: Db,
    pub cfg: Arc<Config>,
    pub task: Arc<RwLock<Option<CurrentTask>>>,
}

impl AuditRunner {
    pub fn new(db: Db, cfg: Arc<Config>, task: Arc<RwLock<Option<CurrentTask>>>) -> Self {
        Self { db, cfg, task }
    }

    async fn set_stage(&self, stage: &str, message: String, current: usize, total: usize) {
        let now = chrono::Utc::now().to_rfc3339();
        let mut guard = self.task.write().await;
        let started = guard.as_ref().and_then(|t| t.started_at.clone()).or_else(|| Some(now.clone()));
        *guard = Some(CurrentTask {
            active: true,
            kind: "daily_audit".to_string(),
            stage: stage.to_string(),
            post_id: None,
            message,
            progress_current: current,
            progress_total: total,
            started_at: started,
            updated_at: Some(now),
        });
    }

    async fn set_finish(&self, stage: &str, message: String) {
        let now = chrono::Utc::now().to_rfc3339();
        let mut guard = self.task.write().await;
        let started = guard.as_ref().and_then(|t| t.started_at.clone()).or_else(|| Some(now.clone()));
        *guard = Some(CurrentTask {
            active: false,
            kind: "daily_audit".to_string(),
            stage: stage.to_string(),
            post_id: None,
            message,
            progress_current: 0,
            progress_total: 0,
            started_at: started,
            updated_at: Some(now),
        });
    }

    pub async fn run(&self, urls: Vec<String>) -> Result<Vec<AuditResult>> {
        let targets: Vec<String> = if urls.is_empty() {
            vec![self.cfg.site.url.clone()]
        } else {
            urls
        };
        let total = targets.len();
        self.set_stage("launching_chrome", format!("Khởi tạo Chrome headless cho {} URL", total), 0, total).await;

        let chrome = self.cfg.chrome.executable.clone()
            .unwrap_or_else(|| default_chrome_path().to_string_lossy().to_string());
        // Use a dedicated temp profile + separate debug port so we never clash with
        // the user's already-running Chrome instance (which ignores --remote-debugging-port).
        let user_data = std::env::temp_dir().join(format!("wpoa-audit-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&user_data);
        let profile = "Default".to_string();
        let port = self.cfg.chrome.remote_debugging_port.saturating_add(1);

        let cdp = match Cdp::launch_and_connect(&chrome, &user_data, &profile, true, port, self.cfg.chrome.launch_timeout_secs).await {
            Ok(c) => c,
            Err(e) => {
                self.set_finish("failed", format!("Không khởi chạy được Chrome headless: {}", e)).await;
                return Err(e);
            }
        };

        let _ = cdp.call("Page.enable", serde_json::json!({})).await;
        let _ = cdp.call("Performance.enable", serde_json::json!({})).await;
        if let Err(e) = cdp.call("Page.addScriptToEvaluateOnNewDocument", serde_json::json!({ "source": LCP_OBSERVER })).await {
            tracing::debug!(target: "api_server", stage = "audit", "inject audit observers failed: {}", e);
        }

        let mut results = Vec::new();
        for (idx, url) in targets.iter().enumerate() {
            self.set_stage("auditing", format!("Đang đo {}", url), idx, total).await;
            info!(target: "api_server", stage = "audit", target = %url, "audit started");
            let res = Self::audit_one(&cdp, url).await;
            match &res {
                Ok(r) => info!(target: "api_server", stage = "audit", target = %url, score = r.score, lcp = r.lcp, cls = r.cls, "audit done"),
                Err(e) => warn!(target: "api_server", stage = "audit", target = %url, "audit failed: {}", e),
            }
            let mut record = res.unwrap_or_else(|e| AuditResult {
                url: url.clone(),
                device: "mobile".into(),
                ok: false,
                lcp: 0.0, cls: 0.0, fcp: 0.0, ttfb_ms: 0, load_ms: 0, transfer_kb: 0.0,
                requests: 0, images: 0, cdn_images: 0, score: 0, status: "failed".into(),
                notes: format!("audit error: {}", e),
            });
            record.device = "mobile".into();
            if let Err(e) = self.db.insert_audit(&record).await {
                warn!(target: "db", stage = "audit", "insert audit failed: {}", e);
            }
            results.push(record);
            self.set_stage("auditing", format!("Đã đo {}/{}", idx + 1, total), idx + 1, total).await;
        }

        cdp.close().await;
        self.set_finish("done", format!("Hoàn tất audit {} URL", results.len())).await;
        info!(target: "api_server", stage = "audit", count = results.len(), "daily audit finished");
        Ok(results)
    }

    async fn audit_one(cdp: &Cdp, url: &str) -> Result<AuditResult> {
        cdp.call("Page.navigate", serde_json::json!({ "url": url })).await?;

        let mut ready = false;
        for _ in 0..60 {
            cdp.wait_ms(300).await;
            if let Ok(v) = cdp.evaluate("document.readyState", false).await {
                if v.as_str() == Some("complete") {
                    ready = true;
                    break;
                }
            }
        }
        if !ready {
            return Err(crate::error::Error::NetworkError("page load timeout".into()));
        }

        cdp.wait_ms(1200).await;
        let raw = cdp.evaluate(METRICS_JS, false).await?;
        let raw = raw.as_str().unwrap_or("{}");
        let m: serde_json::Value = serde_json::from_str(raw).unwrap_or_else(|_| serde_json::json!({}));

        let lcp = m.get("lcp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let cls = m.get("cls").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let fcp = m.get("fcp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ttfb_ms = m.get("ttfb").and_then(|v| v.as_f64()).unwrap_or(0.0) as u64;
        let load_ms = m.get("load").and_then(|v| v.as_f64()).unwrap_or(0.0) as u64;
        let transfer_kb = m.get("transfer").and_then(|v| v.as_f64()).unwrap_or(0.0) / 1024.0;
        let requests = m.get("requests").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let images = m.get("images").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let cdn_images = m.get("cdnImages").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let (score, status, notes) = score_of(lcp, cls, fcp, ttfb_ms, cdn_images, images);

        Ok(AuditResult {
            url: url.to_string(),
            device: "mobile".into(),
            ok: true,
            lcp: (lcp / 1000.0 * 100.0).round() / 100.0,
            cls: (cls * 1000.0).round() / 1000.0,
            fcp: (fcp / 1000.0 * 100.0).round() / 100.0,
            ttfb_ms,
            load_ms,
            transfer_kb: (transfer_kb * 10.0).round() / 10.0,
            requests,
            images,
            cdn_images,
            score,
            status,
            notes,
        })
    }
}

fn score_of(lcp_ms: f64, cls: f64, fcp_ms: f64, ttfb_ms: u64, cdn_images: u32, images: u32) -> (u32, String, String) {
    let mut score: f64 = 100.0;
    if lcp_ms > 2500.0 { score -= ((lcp_ms - 2500.0) / 100.0).min(40.0); }
    if cls > 0.1 { score -= ((cls - 0.1) * 100.0).min(20.0); }
    if fcp_ms > 1800.0 { score -= ((fcp_ms - 1800.0) / 150.0).min(20.0); }
    if ttfb_ms > 800 { score -= (((ttfb_ms as f64) - 800.0) / 80.0).min(15.0); }
    let score = score.clamp(0.0, 100.0).round() as u32;

    let status = if score >= 90 { "passed" } else if score >= 70 { "warning" } else { "failed" };
    let mut notes = format!("LCP {}ms · FCP {}ms · CLS {} · TTFB {}ms", lcp_ms.round(), fcp_ms.round(), cls, ttfb_ms);
    if images > 0 && cdn_images < images {
        notes.push_str(&format!(" · {} ảnh chưa qua CDN/offload", images - cdn_images));
    }
    (score, status.to_string(), notes)
}
