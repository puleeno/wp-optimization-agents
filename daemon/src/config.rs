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
    pub logging: LoggingConfig,
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
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
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

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::ConfigError(format!("Failed to read config {}: {}", path, e)))?;
        let mut cfg: Config = serde_json::from_str(&content)
            .map_err(|e| Error::ConfigError(format!("Failed to parse config {}: {}", path, e)))?;
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
            agent: AgentConfig { host: default_host(), port: default_agent_port(), poll_interval_secs: default_poll(), replace_in_db: true, enable_old_url_redirect_301: true, backup_before_replace: true },
            logging: LoggingConfig { level: default_log_level() },
        };
        cfg.normalize();
        Ok(cfg)
    }

    pub fn normalize(&mut self) {
        if self.site.url.ends_with('/') { self.site.url.pop(); }
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
