use crate::error::{Error, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};

type WsWrite = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;
type WsRead = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

#[derive(Clone)]
pub struct Cdp {
    inner: Arc<CdpInner>,
}

struct CdpInner {
    write: Mutex<WsWrite>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    id: AtomicU64,
    session_id: String,
    _child: Mutex<Option<Child>>,
}

impl Cdp {
    pub async fn launch_and_connect(
        chrome_path: &str,
        user_data_dir: &Path,
        profile_dir: &str,
        headless: bool,
        debug_port: u16,
        timeout_secs: u64,
    ) -> Result<Self> {
        std::fs::create_dir_all(user_data_dir).ok();
        let port_file = user_data_dir.join("DevToolsActivePort");
        let _ = std::fs::remove_file(&port_file);

        let mut cmd = Command::new(chrome_path);
        cmd.arg(format!("--user-data-dir={}", user_data_dir.display()));
        cmd.arg(format!("--profile-directory={}", profile_dir));
        cmd.arg(format!("--remote-debugging-port={}", debug_port));
        cmd.arg("--remote-allow-origins=*");
        cmd.arg("--no-first-run");
        cmd.arg("--no-default-browser-check");
        cmd.arg("--disable-background-networking");
        if headless { cmd.arg("--headless=new"); }
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        let child = cmd.spawn().map_err(|e| Error::ChromeError(format!("spawn chrome: {e}")))?;

        let ws_url = wait_for_ws_url(&port_file, debug_port, timeout_secs).await?;
        tracing::info!("connecting CDP {}", ws_url);
        let (stream, _) = connect_async(&ws_url).await.map_err(|e| Error::ChromeError(format!("ws connect: {e}")))?;
        let (write, read) = stream.split();
        let pending = Arc::new(Mutex::new(HashMap::<u64, oneshot::Sender<Value>>::new()));
        spawn_reader(read, pending.clone());

        let browser = CdpBrowser { write: Mutex::new(write), pending: pending.clone(), id: AtomicU64::new(1) };
        // create target + attach
        let t = browser.send("Target.createTarget", json!({"url":"about:blank"}), None).await?;
        let target_id = t.get("targetId").and_then(|v| v.as_str()).ok_or_else(|| Error::ChromeError("no targetId".into()))?;
        let att = browser.send("Target.attachToTarget", json!({"targetId": target_id, "flatten": true}), None).await?;
        let session_id = att.get("sessionId").and_then(|v| v.as_str()).ok_or_else(|| Error::ChromeError("no sessionId".into()))?.to_string();

        let inner = Arc::new(CdpInner {
            write: browser.write,
            pending,
            id: browser.id,
            session_id,
            _child: Mutex::new(Some(child)),
        });
        let cdp = Cdp { inner };
        let _ = cdp.call("Page.enable", json!({})).await;
        let _ = cdp.call("Runtime.enable", json!({})).await;
        Ok(cdp)
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        self.send_raw(method, params, Some(self.inner.session_id.clone())).await
    }

    pub async fn call_target(&self, session_id: &str, method: &str, params: Value) -> Result<Value> {
        self.send_raw(method, params, Some(session_id.to_string())).await
    }

    pub async fn browser_call(&self, method: &str, params: Value) -> Result<Value> {
        self.send_raw(method, params, None).await
    }

    async fn send_raw(&self, method: &str, params: Value, session_id: Option<String>) -> Result<Value> {
        let id = self.inner.id.fetch_add(1, Ordering::Relaxed);
        let mut msg = json!({"id": id, "method": method, "params": params});
        if let Some(sid) = session_id { msg["sessionId"] = json!(sid); }
        let (tx, rx) = oneshot::channel();
        self.inner.pending.lock().await.insert(id, tx);
        let text = msg.to_string();
        self.inner.write.lock().await.send(Message::Text(text.into())).await
            .map_err(|e| Error::ChromeError(format!("ws send: {e}")))?;
        let v = tokio::time::timeout(Duration::from_secs(45), rx).await
            .map_err(|_| Error::ChromeError(format!("timeout {method}")))?
            .map_err(|_| Error::ChromeError(format!("channel closed {method}")))?;
        if let Some(err) = v.get("error") {
            return Err(Error::ChromeError(format!("{method}: {}", err.get("message").and_then(|m| m.as_str()).unwrap_or("cdp error"))));
        }
        Ok(v.get("result").cloned().unwrap_or(Value::Null))
    }

    pub async fn goto(&self, url: &str, wait_ms: u64) -> Result<()> {
        self.call("Page.navigate", json!({"url": url})).await?;
        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        Ok(())
    }

    pub async fn current_url(&self) -> Result<String> {
        let v = self.evaluate("location.href", false).await?;
        Ok(v.as_str().unwrap_or("").to_string())
    }

    pub async fn evaluate(&self, expr: &str, await_promise: bool) -> Result<Value> {
        let r = self.call("Runtime.evaluate", json!({
            "expression": expr,
            "returnByValue": true,
            "awaitPromise": await_promise
        })).await?;
        if let Some(ex) = r.get("exceptionDetails") {
            return Err(Error::ChromeError(format!("js exception: {ex}")));
        }
        Ok(r.get("result").and_then(|x| x.get("value")).cloned().unwrap_or(Value::Null))
    }

    pub async fn wait_ms(&self, ms: u64) { tokio::time::sleep(Duration::from_millis(ms)).await; }

    pub async fn wait_for_visible(&self, selector: &str, timeout_ms: u64) -> Result<bool> {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        let sel = serde_json::to_string(selector)?;
        loop {
            let js = format!(r#"(function(){{const e=document.querySelector({sel}); if(!e) return false; const r=e.getBoundingClientRect(), s=getComputedStyle(e); return r.width>0&&r.height>0&&s.visibility!=='hidden'&&s.display!=='none';}})()"#);
            if self.evaluate(&js, false).await.unwrap_or(Value::Bool(false)).as_bool().unwrap_or(false) { return Ok(true); }
            if std::time::Instant::now() >= deadline { return Ok(false); }
            self.wait_ms(300).await;
        }
    }

    pub async fn click_selector(&self, selector: &str) -> Result<bool> {
        let sel = serde_json::to_string(selector)?;
        let js = format!(r#"(function(){{const e=document.querySelector({sel}); if(!e) return false; e.click(); return true;}})()"#);
        Ok(self.evaluate(&js, false).await.unwrap_or(Value::Bool(false)).as_bool().unwrap_or(false))
    }

    pub async fn real_click_selector(&self, selector: &str) -> Result<bool> {
        let sel = serde_json::to_string(selector)?;
        let js = format!(r#"(function(){{
            const els=document.querySelectorAll({sel});
            for(const el of els){{ const r=el.getBoundingClientRect(); if(r.width<=2||r.height<=2||r.x<0||r.y<0) continue; return {{x:r.x+r.width/2,y:r.y+r.height/2}}; }}
            return null;
        }})()"#);
        let v = self.evaluate(&js, false).await.unwrap_or(Value::Null);
        if let (Some(x), Some(y)) = (v.get("x").and_then(|n| n.as_f64()), v.get("y").and_then(|n| n.as_f64())) {
            self.trusted_click(x, y).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn real_click_text(&self, text: &str) -> Result<bool> {
        let t = serde_json::to_string(text)?;
        let js = format!(r#"(function(){{
            const want={t}; let best=null;
            const nodes=document.querySelectorAll('a,button,div,span,li,[role=button],[role=menuitem],input');
            for(const el of nodes){{ const label=(el.getAttribute('aria-label')||'')+' '+(el.textContent||''); if(!label.includes(want)) continue; const r=el.getBoundingClientRect(); if(r.width<=2||r.height<=2||r.x<0||r.y<0) continue; const area=r.width*r.height; if(!best||area<best.area) best={{x:r.x+r.width/2,y:r.y+r.height/2,area}}; }}
            if(!best) return null; return {{x:best.x,y:best.y}};
        }})()"#);
        let v = self.evaluate(&js, false).await.unwrap_or(Value::Null);
        if let (Some(x), Some(y)) = (v.get("x").and_then(|n| n.as_f64()), v.get("y").and_then(|n| n.as_f64())) {
            self.trusted_click(x, y).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn trusted_click(&self, x: f64, y: f64) -> Result<()> {
        self.call("Input.dispatchMouseEvent", json!({"type":"mouseMoved","x":x,"y":y})).await?;
        self.call("Input.dispatchMouseEvent", json!({"type":"mousePressed","x":x,"y":y,"button":"left","buttons":1,"clickCount":1})).await?;
        self.call("Input.dispatchMouseEvent", json!({"type":"mouseReleased","x":x,"y":y,"button":"left","buttons":0,"clickCount":1})).await?;
        Ok(())
    }

    pub async fn set_file_input_files(&self, selector: &str, files: &[String]) -> Result<bool> {
        let sel = serde_json::to_string(selector)?;
        let js = format!(r#"(function(){{const e=document.querySelector({sel}); if(!e) return null; window.__wpoa_file=e; return true;}})()"#);
        let found = self.evaluate(&js, false).await.unwrap_or(Value::Null);
        if !found.as_bool().unwrap_or(false) { return Ok(false); }
        let obj = self.call("Runtime.evaluate", json!({"expression":"window.__wpoa_file","returnByValue":false})).await?;
        let object_id = obj.get("result").and_then(|r| r.get("objectId")).and_then(|v| v.as_str());
        if let Some(oid) = object_id {
            self.call("DOM.setFileInputFiles", json!({"files": files, "objectId": oid})).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn picker_set_file(&self, files: &[String]) -> Result<bool> {
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            let targets = self.browser_call("Target.getTargets", json!({})).await?;
            let list = targets.get("targetInfos").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let mut picker_id: Option<String> = None;
            for t in &list {
                let url = t.get("url").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                if url.contains("docs.google.com/picker") || url.contains("picker") || url.contains("/upload") {
                    picker_id = t.get("targetId").and_then(|v| v.as_str()).map(|s| s.to_string());
                    if picker_id.is_some() { break; }
                }
            }
            if let Some(tid) = picker_id {
                let att = self.browser_call("Target.attachToTarget", json!({"targetId": tid, "flatten": true})).await?;
                if let Some(sid) = att.get("sessionId").and_then(|v| v.as_str()) {
                    let rt = self.call_target(sid, "Runtime.evaluate", json!({"expression":"document.querySelector('input[type=file]')","returnByValue":false})).await;
                    if let Ok(rtv) = rt {
                        if let Some(oid) = rtv.get("result").and_then(|r| r.get("objectId")).and_then(|v| v.as_str()) {
                            self.call_target(sid, "DOM.setFileInputFiles", json!({"files": files, "objectId": oid})).await?;
                            return Ok(true);
                        }
                    }
                }
            }
            if std::time::Instant::now() >= deadline { return Ok(false); }
            self.wait_ms(500).await;
        }
    }

    pub async fn body_imgs(&self) -> Result<Vec<String>> {
        let js = r#"(function(){const out=[];const walk=(doc)=>{try{doc.querySelectorAll('img').forEach(im=>{if(im.src) out.push(im.src);}); doc.querySelectorAll('iframe').forEach(f=>{try{if(f.contentDocument) walk(f.contentDocument);}catch(e){}});}catch(e){}}; walk(document); return out;})()"#;
        let v = self.evaluate(js, false).await.unwrap_or(Value::Null);
        Ok(v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default())
    }

    pub async fn close(self) {
        let _ = self.call("Browser.close", json!({})).await;
        if let Some(mut c) = self.inner._child.lock().await.take() { let _ = c.kill(); }
    }
}

struct CdpBrowser {
    write: Mutex<WsWrite>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    id: AtomicU64,
}
impl CdpBrowser {
    async fn send(&self, method: &str, params: Value, session_id: Option<&str>) -> Result<Value> {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        let mut msg = json!({"id": id, "method": method, "params": params});
        if let Some(s) = session_id { msg["sessionId"] = json!(s); }
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        self.write.lock().await.send(Message::Text(msg.to_string().into())).await.map_err(|e| Error::ChromeError(e.to_string()))?;
        let v = tokio::time::timeout(Duration::from_secs(30), rx).await.map_err(|_| Error::ChromeError("timeout".into()))?.map_err(|_| Error::ChromeError("closed".into()))?;
        if let Some(err) = v.get("error") { return Err(Error::ChromeError(err.to_string())); }
        Ok(v.get("result").cloned().unwrap_or(Value::Null))
    }
}

fn spawn_reader(mut read: WsRead, pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>) {
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            let Ok(Message::Text(txt)) = msg else { continue };
            let Ok(v) = serde_json::from_str::<Value>(&txt) else { continue };
            if let Some(id) = v.get("id").and_then(|x| x.as_u64()) {
                if let Some(tx) = pending.lock().await.remove(&id) { let _ = tx.send(v); }
            }
        }
    });
}

async fn wait_for_ws_url(port_file: &Path, port: u16, timeout_secs: u64) -> Result<String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if port_file.exists() {
            if let Ok(content) = std::fs::read_to_string(port_file) {
                let mut lines = content.lines();
                if let Some(p) = lines.next() {
                    let p = p.trim();
                    if !p.is_empty() {
                        // fetch ws url from /json/version
                        let url = format!("http://127.0.0.1:{}/json/version", p.parse::<u16>().unwrap_or(port));
                        if let Ok(resp) = reqwest::get(&url).await {
                            if let Ok(v) = resp.json::<Value>().await {
                                if let Some(ws) = v.get("webSocketDebuggerUrl").and_then(|x| x.as_str()) {
                                    return Ok(ws.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        // fallback try default port
        let url = format!("http://127.0.0.1:{port}/json/version");
        if let Ok(resp) = reqwest::get(&url).await {
            if let Ok(v) = resp.json::<Value>().await {
                if let Some(ws) = v.get("webSocketDebuggerUrl").and_then(|x| x.as_str()) { return Ok(ws.to_string()); }
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err(Error::ChromeError(format!("DevTools not ready on port {port}")));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

pub fn default_chrome_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let p = PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
        if p.exists() { return p; }
    }
    #[cfg(target_os = "linux")]
    {
        for c in ["/usr/bin/google-chrome", "/usr/bin/google-chrome-stable", "/usr/bin/chromium", "/usr/bin/chromium-browser"] {
            let p = PathBuf::from(c); if p.exists() { return p; }
        }
    }
    PathBuf::from("google-chrome")
}
