use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    pub site: SiteConfig,
    pub blogger: BloggerConfig,
    pub chrome: ChromeConfig,
    pub agent: AgentConfig,
    #[serde(default)]
    pub imgbb: ImgbbConfig,
    #[serde(default)]
    pub cdn: CdnConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Directory of the config file (used to resolve relative paths); not serialized.
    #[serde(skip)]
    pub base_dir: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub name: String,
    pub table_prefix: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SiteConfig {
    pub url: String,
    #[serde(default)]
    pub wp_path: String,
    #[serde(default)]
    pub uploads_dir: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BloggerConfig {
    pub blog_id: String,
    #[serde(default = "default_upload_via")]
    pub upload_via: String,
    #[serde(default = "default_true")]
    pub one_wp_post_per_blogger_post: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChromeConfig {
    #[serde(default)]
    pub executable: Option<String>,
    #[serde(default)]
    pub user_data_dir: Option<String>,
    #[serde(default = "default_profile_dir")]
    pub profile_directory: String,
    #[serde(default)]
    pub headless: bool,
    #[serde(default = "default_debug_port")]
    pub remote_debugging_port: u16,
    #[serde(default = "default_launch_timeout")]
    pub launch_timeout_secs: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AgentConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_agent_port")]
    pub port: u16,
    #[serde(default = "default_poll")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_true")]
    pub replace_in_db: bool,
    #[serde(default = "default_true")]
    pub enable_old_url_redirect_301: bool,
    #[serde(default = "default_true")]
    pub backup_before_replace: bool,
    #[serde(default)]
    pub daemon_api_token: String,
    /// Automatic media-library scan worker (offload existing attachments)
    #[serde(default = "default_true")]
    pub scan_enabled: bool,
    #[serde(default = "default_scan_interval")]
    pub scan_interval_secs: u64,
    #[serde(default = "default_scan_batch")]
    pub scan_batch_size: u64,
    /// Parallel crop/resize finish workers (independent from imgbb + blogger upload)
    #[serde(default = "default_finish_workers")]
    pub finish_workers: u64,
    /// Delete local physical files after a successful offload (imgbb + CDN + Blogger queue)
    #[serde(default = "default_true")]
    pub delete_local_after_offload: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ImgbbConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// "url" = upload via direct/public WP image URL, "blob" = upload binary/file bytes from local path
    #[serde(default = "default_imgbb_upload_format")]
    pub upload_format: String,
}

impl Default for ImgbbConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            enabled: true,
            upload_format: default_imgbb_upload_format(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CdnConfig {
    #[serde(default = "default_cdn_host")]
    pub host: String,
    #[serde(default = "default_cdn_scheme")]
    pub scheme: String,
}

impl Default for CdnConfig {
    fn default() -> Self {
        Self { host: default_cdn_host(), scheme: default_cdn_scheme() }
    }
}

/// Expand `${VAR}` and `${VAR:-default}` placeholders using environment variables.
pub fn expand_env(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some(close_rel) = input[i + 2..].find('}') {
                let close = i + 2 + close_rel;
                let inner = &input[i + 2..close];
                let (name, default) = match inner.split_once(":-") {
                    Some((n, d)) => (n.trim(), Some(d)),
                    None => (inner.trim(), None),
                };
                let value = std::env::var(name)
                    .ok()
                    .filter(|v| !v.is_empty())
                    .or_else(|| default.map(|d| d.to_string()))
                    .unwrap_or_default();
                out.push_str(&value);
                i = close + 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn default_upload_via() -> String { "chrome_web_ui".into() }
fn default_profile_dir() -> String { "Default".into() }
fn default_debug_port() -> u16 { 9222 }
fn default_launch_timeout() -> u64 { 30 }
fn default_host() -> String { "0.0.0.0".into() }
fn default_agent_port() -> u16 { 2095 }
fn default_poll() -> u64 { 60 }
fn default_true() -> bool { true }
fn default_log_level() -> String { "info".into() }
fn default_cdn_scheme() -> String { "https".into() }
fn default_cdn_host() -> String { "cdn.nghesachxua.localhost".into() }
fn default_imgbb_upload_format() -> String { "url".into() }
fn default_scan_interval() -> u64 { 900 }
fn default_scan_batch() -> u64 { 20 }
fn default_finish_workers() -> u64 { 4 }

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::ConfigError(format!("Failed to read config {}: {}", path, e)))?;
        let mut cfg: Config = serde_json::from_str(&content)
            .map_err(|e| Error::ConfigError(format!("Failed to parse config {}: {}", path, e)))?;
        cfg.base_dir = Path::new(path)
            .parent()
            .map(|p| p.to_path_buf())
            .or_else(|| Some(PathBuf::from(".")));
        cfg.normalize();
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn from_wp_config(wp_config_path: &Path) -> Result<Self> {
        // Minimal helper: parse defines from wp-config.php (best-effort) - callers can override via JSON
        let content = std::fs::read_to_string(wp_config_path)
            .map_err(|e| Error::ConfigError(format!("read wp-config: {e}")))?;
        let get = |key: &str| -> Option<String> {
            let re = regex::Regex::new(&format!(r#"define\(\s*'{}'\s*,\s*'([^']*)'\s*\)"#, regex::escape(key))).ok()?;
            re.captures(&content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
        };
        let prefix_re = regex::Regex::new(r#"\$table_prefix\s*=\s*'([^']*)'"#).ok();
        let prefix = prefix_re.and_then(|r| r.captures(&content)).and_then(|c| c.get(1)).map(|m| m.as_str().to_string()).unwrap_or_else(|| "wp_".into());
        let wp_path = wp_config_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let mut cfg = Config {
            database: DatabaseConfig {
                host: get("DB_HOST").unwrap_or_else(|| "127.0.0.1".into()),
                port: 3306,
                user: get("DB_USER").unwrap_or_default(),
                password: get("DB_PASSWORD").unwrap_or_default(),
                name: get("DB_NAME").unwrap_or_default(),
                table_prefix: prefix,
            },
            site: SiteConfig {
                url: String::new(),
                wp_path: wp_path.to_string_lossy().to_string(),
                uploads_dir: wp_path.join("wp-content/uploads").to_string_lossy().to_string(),
            },
            blogger: BloggerConfig { blog_id: String::new(), upload_via: default_upload_via(), one_wp_post_per_blogger_post: true },
            chrome: ChromeConfig { executable: None, user_data_dir: None, profile_directory: default_profile_dir(), headless: false, remote_debugging_port: default_debug_port(), launch_timeout_secs: default_launch_timeout() },
            agent: AgentConfig { host: default_host(), port: default_agent_port(), poll_interval_secs: default_poll(), replace_in_db: true, enable_old_url_redirect_301: true, backup_before_replace: true, daemon_api_token: String::new(), scan_enabled: true, scan_interval_secs: default_scan_interval(), scan_batch_size: default_scan_batch(), finish_workers: default_finish_workers(), delete_local_after_offload: true },
            imgbb: ImgbbConfig::default(),
            cdn: CdnConfig::default(),
            logging: LoggingConfig { level: default_log_level() },
            base_dir: Some(wp_path.clone()),
        };
        cfg.normalize();
        Ok(cfg)
    }

    pub fn normalize(&mut self) {
        if self.site.url.ends_with('/') { self.site.url.pop(); }

        // expand ${VAR:-default} placeholders so one config works on macOS + Linux
        self.site.url = expand_env(&self.site.url);
        self.site.wp_path = expand_env(&self.site.wp_path);
        self.site.uploads_dir = expand_env(&self.site.uploads_dir);
        self.database.host = expand_env(&self.database.host);
        self.database.name = expand_env(&self.database.name);
        self.database.user = expand_env(&self.database.user);
        self.database.password = expand_env(&self.database.password);
        self.agent.daemon_api_token = expand_env(&self.agent.daemon_api_token);
        self.blogger.blog_id = expand_env(&self.blogger.blog_id);
        self.imgbb.api_key = expand_env(&self.imgbb.api_key);
        self.cdn.host = expand_env(&self.cdn.host);
        if let Some(exe) = self.chrome.executable.as_ref() {
            let expanded = expand_env(exe);
            self.chrome.executable = if expanded.trim().is_empty() { None } else { Some(expanded) };
        }
        if let Some(dir) = self.chrome.user_data_dir.as_ref() {
            let expanded = expand_env(dir);
            self.chrome.user_data_dir = if expanded.trim().is_empty() { None } else { Some(expanded) };
        }

        // resolve relative paths against the config file directory
        self.site.wp_path = self.resolve_path(&self.site.wp_path);
        if self.site.uploads_dir.trim().is_empty() {
            self.site.uploads_dir = self.resolve_path("wp-content/uploads");
        } else {
            self.site.uploads_dir = self.resolve_path(&self.site.uploads_dir);
        }
        if let Some(dir) = self.chrome.user_data_dir.clone() {
            self.chrome.user_data_dir = Some(self.resolve_path(&dir));
        }
        // split host:port in DB_HOST
        if let Some((h, p)) = self.database.host.split_once(':') {
            if let Ok(port) = p.parse::<u16>() { self.database.port = port; self.database.host = h.to_string(); }
        }
        if self.site.uploads_dir.is_empty() && !self.site.wp_path.is_empty() {
            self.site.uploads_dir = PathBuf::from(&self.site.wp_path).join("wp-content/uploads").to_string_lossy().to_string();
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.database.host.is_empty() { return Err(Error::ConfigError("database.host is required".into())); }
        if self.database.name.is_empty() { return Err(Error::ConfigError("database.name is required".into())); }
        if self.database.user.is_empty() { return Err(Error::ConfigError("database.user is required".into())); }
        if self.blogger.blog_id.is_empty() { return Err(Error::ConfigError("blogger.blog_id is required".into())); }
        Ok(())
    }

    /// Resolve a possibly-relative path against the config file directory.
    pub fn resolve_path(&self, p: &str) -> String {
        let expanded = expand_env(p);
        let trimmed = expanded.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        let path = Path::new(trimmed);
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else {
            match &self.base_dir {
                Some(base) => base.join(path),
                None => return trimmed.to_string(),
            }
        };
        // clean up "..", "." segments when the path exists on disk
        match std::fs::canonicalize(&joined) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => joined.to_string_lossy().to_string(),
        }
    }

    pub fn mysql_opts(&self) -> mysql_async::OptsBuilder {
        mysql_async::OptsBuilder::default()
            .ip_or_hostname(self.database.host.clone())
            .tcp_port(self.database.port)
            .user(Some(self.database.user.clone()))
            .pass(Some(self.database.password.clone()))
            .db_name(Some(self.database.name.clone()))
    }

    pub fn t(&self, name: &str) -> String {
        format!("{}{}", self.database.table_prefix, name)
    }
}
