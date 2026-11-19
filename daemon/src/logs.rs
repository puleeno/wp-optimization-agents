use chrono::Utc;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

const MAX_MEMORY: usize = 2000;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub subsystem: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
}

impl LogEntry {
    pub fn new(level: &str, subsystem: &str, message: &str) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            subsystem: subsystem.to_string(),
            message: message.to_string(),
            attachment_id: None,
            post_id: None,
            task_id: None,
            stage: None,
            provider: None,
            target: None,
            size: None,
        }
    }
}

static LOG_BUF: OnceLock<Mutex<VecDeque<LogEntry>>> = OnceLock::new();
static LOG_TX: OnceLock<Mutex<Option<UnboundedSender<LogEntry>>>> = OnceLock::new();

fn buf() -> &'static Mutex<VecDeque<LogEntry>> {
    LOG_BUF.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_MEMORY)))
}

fn tx_slot() -> &'static Mutex<Option<UnboundedSender<LogEntry>>> {
    LOG_TX.get_or_init(|| Mutex::new(None))
}

pub fn set_sender(tx: UnboundedSender<LogEntry>) {
    if let Ok(mut g) = tx_slot().lock() {
        *g = Some(tx);
    }
}

pub fn push(entry: LogEntry) {
    if let Ok(mut g) = buf().lock() {
        if g.len() >= MAX_MEMORY {
            g.pop_front();
        }
        g.push_back(entry.clone());
    }
    if let Ok(g) = tx_slot().lock() {
        if let Some(tx) = g.as_ref() {
            let _ = tx.send(entry);
        }
    }
}

pub fn recent(limit: usize) -> Vec<LogEntry> {
    if let Ok(g) = buf().lock() {
        let n = g.len().saturating_sub(limit);
        g.iter().skip(n).cloned().collect()
    } else {
        Vec::new()
    }
}

#[derive(Default)]
pub struct CaptureLayer;

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = meta.level().to_string();
        let subsystem = map_subsystem(meta.target());

        let mut message = String::new();
        let mut fields: Vec<(String, String)> = Vec::new();
        let mut visitor = FieldVisitor {
            message: &mut message,
            fields: &mut fields,
        };
        event.record(&mut visitor);

        if message.is_empty() {
            message = meta.target().to_string();
        }

        let get = |k: &str| -> Option<String> {
            fields
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(k))
                .map(|(_, v)| v.clone())
        };
        let _ = &fields;
        let get_i64 = |k: &str| -> Option<i64> { get(k).and_then(|v| v.parse::<i64>().ok()) };

        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level,
            subsystem,
            message,
            attachment_id: get_i64("attachment_id"),
            post_id: get_i64("post_id"),
            task_id: get("task_id"),
            stage: get("stage"),
            provider: get("provider"),
            target: get("target").or_else(|| get("url")).or_else(|| get("path")),
            size: get("size").or_else(|| get("image_size")),
        };
        push(entry);
    }
}

fn map_subsystem(target: &str) -> String {
    if !target.contains("::") {
        let t = target.to_lowercase();
        return match t.as_str() {
            "api_server" => "api_server".to_string(),
            "webhook" => "webhook".to_string(),
            "blogger_client" => "blogger_client".to_string(),
            "chrome_cdp" => "chrome_cdp".to_string(),
            "imgbb_backup" => "imgbb_backup".to_string(),
            "offload_worker" => "offload_worker".to_string(),
            "wp_client" => "wp_client".to_string(),
            other => other.to_string(),
        };
    }
    let seg = target.rsplit("::").next().unwrap_or(target).to_lowercase();
    match seg.as_str() {
        "webhook" => "webhook",
        "offload" => "offload_worker",
        "main_lib" => "api_server",
        "db" => "wp_db",
        "imgbb" => "imgbb_backup",
        "blogger" => "blogger_client",
        "chrome" | "cdp" => "chrome_cdp",
        "wp_api" => "wp_client",
        "logs" => "log_store",
        "chrome_audit" => "chrome_audit",
        _ => {
            if seg.contains("blogger") {
                "blogger_client"
            } else if seg.contains("chrome") || seg.contains("cdp") {
                "chrome_cdp"
            } else if seg.contains("imgbb") {
                "imgbb_backup"
            } else if seg.contains("webhook") {
                "webhook"
            } else if seg.contains("offload") {
                "offload_worker"
            } else {
                "rust_daemon"
            }
        }
    }
    .to_string()
}

struct FieldVisitor<'a> {
    message: &'a mut String,
    fields: &'a mut Vec<(String, String)>,
}

impl<'a> tracing::field::Visit for FieldVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let raw = format!("{:?}", value);
        let val = if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
            raw[1..raw.len() - 1].to_string()
        } else {
            raw
        };
        if field.name() == "message" {
            self.message.push_str(&val);
            return;
        }
        if self.fields.iter().any(|(n, _)| n == field.name()) {
            return;
        }
        self.fields.push((field.name().to_string(), val));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
            return;
        }
        if self.fields.iter().any(|(n, _)| n == field.name()) {
            return;
        }
        self.fields.push((field.name().to_string(), value.to_string()));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if self.fields.iter().any(|(n, _)| n == field.name()) {
            return;
        }
        self.fields.push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if self.fields.iter().any(|(n, _)| n == field.name()) {
            return;
        }
        self.fields.push((field.name().to_string(), value.to_string()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if self.fields.iter().any(|(n, _)| n == field.name()) {
            return;
        }
        self.fields.push((field.name().to_string(), value.to_string()));
    }
}

/// Start DB-backed log writer: consumes entries from an unbounded channel and
/// persists them to `wp_opt_agents_logs` so logs survive daemon restarts.
pub async fn spawn_writer(db: crate::db::Db) {
    let (tx, mut rx) = unbounded_channel::<LogEntry>();
    set_sender(tx);
    if let Err(e) = db.prune_logs(7, 200_000).await {
        eprintln!("log prune failed: {e}");
    }
    tokio::spawn(async move {
        let mut pending: Vec<LogEntry> = Vec::new();
        loop {
            tokio::select! {
                maybe = rx.recv() => match maybe {
                    Some(entry) => {
                        pending.push(entry);
                        if pending.len() >= 50 {
                            flush(&db, &mut pending).await;
                        }
                    }
                    None => break,
                },
                _ = tokio::time::sleep(std::time::Duration::from_millis(400)) => {
                    if !pending.is_empty() {
                        flush(&db, &mut pending).await;
                    }
                }
            }
        }
        if !pending.is_empty() {
            flush(&db, &mut pending).await;
        }
    });
}

async fn flush(db: &crate::db::Db, pending: &mut Vec<LogEntry>) {
    if let Err(e) = db.insert_logs(pending).await {
        eprintln!("log persist failed: {e}");
    }
    pending.clear();
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct LogFilter {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub level: Option<String>,
    pub subsystem: Option<String>,
    pub q: Option<String>,
    pub attachment_id: Option<i64>,
    pub post_id: Option<i64>,
    pub task_id: Option<String>,
}

pub async fn query(db: &crate::db::Db, f: &LogFilter) -> crate::error::Result<(Vec<LogEntry>, i64)> {
    db.query_logs(f).await
}
