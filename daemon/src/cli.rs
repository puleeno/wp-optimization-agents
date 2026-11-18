//! Cross-platform CLI helpers: `wp-offload-daemon chrome <setup|status|login>`
//!
//! Opens (or reuses) a Chrome instance with remote debugging on a dedicated
//! profile so the user can sign into Blogger once. The daemon then reuses that
//! profile for uploads (attach-first, no second Chrome spawn).

use crate::chrome::default_chrome_path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const BLOGGER_HOME: &str = "https://www.blogger.com/";

pub enum Cli {
    ChromeSetup {
        port: u16,
        profile: Option<String>,
        save: bool,
        config: Option<String>,
    },
    ChromeStatus {
        port: u16,
    },
    ChromeLogin {
        port: u16,
        profile: Option<String>,
        timeout_secs: u64,
        save: bool,
        config: Option<String>,
    },
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn arg_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

/// Parse `chrome <setup|status|login> [...]` from the process args (args[0] = binary).
pub fn parse() -> Option<Cli> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] != "chrome" {
        return None;
    }
    let cmd = args.get(2).map(|s| s.as_str()).unwrap_or("help");
    let port: u16 = arg_value(&args, "--port")
        .and_then(|v| v.parse().ok())
        .unwrap_or(9222);
    match cmd {
        "setup" => Some(Cli::ChromeSetup {
            port,
            profile: arg_value(&args, "--profile"),
            save: arg_flag(&args, "--save"),
            config: arg_value(&args, "--config"),
        }),
        "status" => Some(Cli::ChromeStatus { port }),
        "login" => Some(Cli::ChromeLogin {
            port,
            profile: arg_value(&args, "--profile"),
            timeout_secs: arg_value(&args, "--timeout")
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            save: arg_flag(&args, "--save"),
            config: arg_value(&args, "--config"),
        }),
        _ => {
            print_help();
            Some(Cli::ChromeStatus { port })
        }
    }
}

pub fn print_help() {
    println!(
        "wp-offload-daemon chrome <command>\n\
         \n  setup   [--port 9222] [--profile DIR] [--save]  Mở Chrome + Blogger để đăng nhập\n\
         \n  login   [--port 9222] [--profile DIR] [--timeout 300] [--save]  Chờ tới khi đăng nhập Blogger\n\
         \n  status  [--port 9222]                            Kiểm tra port debug + trạng thái đăng nhập\n\
         \nMặc định: port 9222, profile riêng ~/.config/wp-opt-agents-chrome (không đụng profile chính).\n\
         --save: ghi chrome.user_data_dir vào file config (DAEMON_CONFIG hoặc --config)."
    );
}

pub async fn run(cli: Cli) -> i32 {
    match cli {
        Cli::ChromeSetup { port, profile, save, config } => cmd_setup(port, profile, save, config).await,
        Cli::ChromeStatus { port } => cmd_status(port).await,
        Cli::ChromeLogin { port, profile, timeout_secs, save, config } => {
            cmd_login(port, profile, timeout_secs, save, config).await
        }
    }
}

/// Marker file written after a successful `chrome login`, stored inside the profile dir.
pub fn login_marker(profile: &std::path::Path) -> PathBuf {
    profile.join(".wpoa-blogger-login-ok")
}

/// True when the user completed `chrome login` (marker present and not older than 30 days).
pub fn is_login_ok(profile: &std::path::Path) -> bool {
    let path = login_marker(profile);
    let Ok(meta) = std::fs::metadata(&path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return true;
    };
    let age = modified.elapsed().unwrap_or_default();
    age.as_secs() < 30 * 24 * 3600
}

pub fn login_marker_info(profile: &std::path::Path) -> String {
    let path = login_marker(profile);
    match std::fs::metadata(&path).and_then(|m| m.modified()) {
        Ok(t) => {
            let age = t.elapsed().unwrap_or_default().as_secs();
            format!("{} ({} phút trước)", path.display(), age / 60)
        }
        Err(_) => format!("{} (chưa có)", path.display()),
    }
}

fn write_login_marker(profile: &std::path::Path) -> Result<(), String> {
    let path = login_marker(profile);
    std::fs::write(&path, serde_json::json!({
        "logged_in_at": chrono::Utc::now().to_rfc3339(),
        "profile": profile.to_string_lossy().to_string(),
    }).to_string()).map_err(|e| format!("write login marker: {e}"))
}

fn config_path(explicit: Option<String>) -> Option<String> {
    explicit
        .or_else(|| std::env::var("DAEMON_CONFIG").ok())
        .filter(|p| !p.trim().is_empty())
}

fn profile_path(explicit: Option<String>) -> PathBuf {
    if let Some(p) = explicit.filter(|p| !p.trim().is_empty()) {
        return PathBuf::from(p);
    }
    crate::chrome::default_automation_profile()
}

fn chrome_binary() -> PathBuf {
    // config may pin an executable; fall back to OS detection
    if let Some(path) = config_path(None) {
        if let Ok(cfg) = crate::config::Config::load(&path) {
            if let Some(exe) = cfg.chrome.executable.clone() {
                let p = PathBuf::from(&exe);
                if p.exists() {
                    return p;
                }
            }
        }
    }
    default_chrome_path()
}

fn mac_app_name(chrome: &PathBuf) -> Option<String> {
    let s = chrome.to_string_lossy();
    if s.contains("Google Chrome.app") {
        Some("Google Chrome".to_string())
    } else if s.contains("Chromium.app") {
        Some("Chromium".to_string())
    } else if s.contains("Microsoft Edge.app") {
        Some("Microsoft Edge".to_string())
    } else {
        None
    }
}

fn launch_chrome(chrome: &PathBuf, profile: &PathBuf, port: u16) -> Result<(), String> {
    let _ = std::fs::create_dir_all(profile);
    let args_vec = vec![
        format!("--remote-debugging-port={}", port),
        format!("--user-data-dir={}", profile.display()),
        "--remote-allow-origins=*".to_string(),
        "--no-first-run".to_string(),
        "--no-default-browser-check".to_string(),
        BLOGGER_HOME.to_string(),
    ];

    #[cfg(target_os = "macos")]
    {
        if let Some(app) = mac_app_name(chrome) {
            let mut cmd = Command::new("open");
            cmd.arg("-na").arg(&app).arg("--args");
            for a in &args_vec {
                cmd.arg(a);
            }
            let st = cmd.status().map_err(|e| format!("open Chrome: {e}"))?;
            if st.success() {
                return Ok(());
            }
        }
    }

    let mut cmd = Command::new(chrome);
    for a in &args_vec {
        cmd.arg(a);
    }
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x00000008); // DETACHED_PROCESS
    }
    cmd.spawn().map_err(|e| format!("spawn Chrome: {e}"))?;
    Ok(())
}

async fn port_open(port: u16) -> bool {
    tokio::time::timeout(
        Duration::from_millis(700),
        tokio::net::TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false)
}

async fn wait_port(port: u16, timeout_secs: u64) -> bool {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        if port_open(port).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    false
}

async fn blogger_targets(port: u16) -> Vec<(String, String)> {
    let url = format!("http://127.0.0.1:{}/json/list", port);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let Ok(resp) = client.get(&url).send().await else {
        return Vec::new();
    };
    let Ok(list) = resp.json::<Vec<serde_json::Value>>().await else {
        return Vec::new();
    };
    list.into_iter()
        .filter_map(|v| {
            let u = v.get("url").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let t = v.get("title").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if u.is_empty() { None } else { Some((u, t)) }
        })
        .collect()
}

fn is_logged_in(targets: &[(String, String)]) -> bool {
    // Only the Blogger dashboard (blog list / post editor) proves a signed-in session;
    // public pages like /about/ are readable anonymously.
    targets.iter().any(|(u, _)| {
        let lower = u.to_lowercase();
        let on_dashboard = lower.contains("blogger.com/blog")
            || lower.contains("draft.blogger.com")
            || lower.contains("blogger.com/~")
            || lower.contains("blogger.com/profile")
            || lower.contains("blogger.com/my-blogs");
        on_dashboard
            && !lower.contains("accounts.google.com")
            && !lower.contains("servicelogin")
            && !lower.contains("signin")
            && !lower.contains("/about")
    })
}

fn save_profile_to_config(config: &str, profile: &PathBuf) -> Result<(), String> {
    let content = std::fs::read_to_string(config).map_err(|e| format!("read config: {e}"))?;
    let mut val: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("parse config: {e}"))?;
    let obj = val
        .as_object_mut()
        .ok_or_else(|| "config is not an object".to_string())?;
    let chrome = obj
        .entry("chrome")
        .or_insert_with(|| serde_json::json!({}));
    if let Some(c) = chrome.as_object_mut() {
        c.insert(
            "user_data_dir".into(),
            serde_json::json!(profile.to_string_lossy().to_string()),
        );
    }
    let pretty = serde_json::to_string_pretty(&val).map_err(|e| e.to_string())?;
    let tmp = format!("{}.tmp", config);
    std::fs::write(&tmp, pretty).map_err(|e| format!("write config: {e}"))?;
    std::fs::rename(&tmp, config).map_err(|e| format!("replace config: {e}"))?;
    Ok(())
}

async fn cmd_status(port: u16) -> i32 {
    let open = wait_port(port, 2).await;
    println!("Chrome debug port {}: {}", port, if open { "OPEN" } else { "CLOSED" });
    if !open {
        println!("→ Chạy: wp-offload-daemon chrome setup");
        return 1;
    }
    let targets = blogger_targets(port).await;
    let logged = is_logged_in(&targets);
    println!("Targets: {}", targets.len());
    for (u, t) in targets.iter().take(5) {
        println!("  - {} | {}", t, u);
    }
    println!("Blogger login: {}", if logged { "OK" } else { "CHƯA đăng nhập" });
    if logged { 0 } else { 2 }
}

async fn cmd_setup(port: u16, profile: Option<String>, save: bool, config: Option<String>) -> i32 {
    let chrome = chrome_binary();
    let profile = profile_path(profile);
    if !chrome.exists() {
        eprintln!("Không tìm thấy Chrome: {}", chrome.display());
        eprintln!("→ Cài Google Chrome hoặc đặt chrome.executable trong config.");
        return 1;
    }
    println!("Chrome     : {}", chrome.display());
    println!("Profile    : {}", profile.display());
    println!("Port debug : {}", port);

    if port_open(port).await {
        println!("Đã có Chrome mở port {} — dùng lại phiên hiện tại.", port);
    } else if let Err(e) = launch_chrome(&chrome, &profile, port) {
        eprintln!("Không mở được Chrome: {e}");
        return 1;
    }

    if !wait_port(port, 25).await {
        eprintln!("Chrome không mở port debug sau 25s. Kiểm tra chrome.executable / quyền chạy.");
        return 1;
    }
    println!("Port {} OPEN. Đăng nhập Google/Blogger trong cửa sổ Chrome vừa mở.", port);
    println!("Sau khi đăng nhập: wp-offload-daemon chrome login --save");
    0
}

async fn cmd_login(port: u16, profile_opt: Option<String>, timeout_secs: u64, save: bool, config: Option<String>) -> i32 {
    let chrome = chrome_binary();
    let profile = profile_path(profile_opt);
    if !port_open(port).await {
        if let Err(e) = launch_chrome(&chrome, &profile, port) {
            eprintln!("Không mở được Chrome: {e}");
            return 1;
        }
        if !wait_port(port, 25).await {
            eprintln!("Chrome không mở port debug sau 25s.");
            return 1;
        }
    }

    println!("Chờ đăng nhập Blogger (tối đa {}s)...", timeout_secs);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs.max(10));
    loop {
        let targets = blogger_targets(port).await;
        if is_logged_in(&targets) {
            println!("Blogger login: OK");
            println!("Profile: {}", profile.display());
            if let Err(e) = write_login_marker(&profile) {
                eprintln!("Không ghi được marker đăng nhập: {e}");
            } else {
                println!("Đã ghi marker: {}", login_marker(&profile).display());
            }
            if save {
                let Some(path) = config_path(config) else {
                    eprintln!("Không có DAEMON_CONFIG/--config để ghi chrome.user_data_dir.");
                    return 0;
                };
                match save_profile_to_config(&path, &profile) {
                    Ok(()) => println!("Đã ghi chrome.user_data_dir vào {}", path),
                    Err(e) => eprintln!("Ghi config lỗi: {e}"),
                }
            }
            return 0;
        }
        if tokio::time::Instant::now() >= deadline {
            eprintln!("Hết thời gian chờ — chưa thấy trang blogger.com (chưa đăng nhập?).");
            return 2;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
