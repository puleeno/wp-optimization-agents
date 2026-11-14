# WP Offload Daemon - Complete Setup Guide

End-to-end setup for running the **WP Optimization Agents** daemon that offloads WordPress images to Blogger via **Chrome Web UI automation** (no Google/Blogger API, no OAuth). Covers macOS (launchd) and Linux (systemd).

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                      WP OFFLOAD DAEMON                              │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────────┐    │
│  │   Axum      │  │  Offload     │  │  Blogger Uploader       │    │
│  │   HTTP      │──│  Engine      │──│  (Chrome CDP)           │    │
│  │   API/UI    │  │  (Rust)      │  │  • 1 WP post = 1 post   │    │
│  └─────────────┘  └──────────────┘  │  • Picker upload       │    │
│         │               │           │  • Trusted clicks      │    │
│         ▼               ▼           │  • Wait settled        │    │
│  ┌─────────────────────────────────┐ │  • Extract post ID    │    │
│  │      MySQL (WordPress DB)       │ │  └─────────────────────┘    │
│  │  • nsxu_wp_opt_agents_images    │ │         │                    │
│  │  • postmeta (_wpoa_*)           │ │         ▼                    │
│  └─────────────────────────────────┘ │  ┌─────────────────────┐    │
│         │               ▲            │  │  WP Plugin          │    │
│         ▼               │            │  │  • 301 redirects    │    │
│  ┌─────────────────────────────────┐ │  │  • REST compat API  │    │
│  │  Chrome Browser (user profile)  │ │  │  • DB schema        │    │
│  │  • Signed into Blogger/Google   │ │  └─────────────────────┘    │
│  └─────────────────────────────────┘ └─────────────────────────────┘
```

**Key behaviors:**
- **No Google/Blogger API** — uses Chrome DevTools Protocol (CDP) against Blogger web UI
- **1 WordPress post = 1 Blogger post/album** — all images for a post uploaded to single Blogger post
- **WP↔Blogger linking** — stored in WP postmeta (`_wpoa_blogger_post_id`, `_wpoa_blogger_post_url`, etc.)
- **DB-driven URL replacement** — daemon rewrites `post_content` + `postmeta` + `options`, backs up original
- **301 redirects** — PHP plugin serves 301 from old WP image URLs to Blogger URLs when `redirect_active=1`

---

## Prerequisites

### System Requirements
| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **OS** | macOS 12+ / Ubuntu 20.04+ / Debian 11+ | Latest LTS |
| **Rust** | 1.75+ | 1.80+ |
| **Chrome/Chromium** | 118+ | Latest stable |
| **MySQL/MariaDB** | 5.7+ / 10.3+ | 8.0 / 10.6+ |
| **WordPress** | 6.0+ | 6.5+ |
| **PHP** | 8.1+ | 8.3+ |
| **Composer** | 2.0+ | Latest |

### WordPress Site (nghesachxua.com)
- Database: `nghesachxua` (user `root`, pass `root`, host `localhost`)
- Table prefix: `nsxu_` (from `wp-config.php`)
- Uploads directory: `/path/to/wordpress/wp-content/uploads`

### Chrome Profile (Critical)
- Must have a **Chrome profile already signed into Blogger/Google**
- Profile will be used by daemon for automated uploads
- **Do not use headless mode** for initial setup (Google sign-in requires visible browser)

---

## Step 1: Build the Daemon

```bash
cd /Users/puleeno/Projects/nghesachxua.com/agents/daemon

# Install Rust if not present
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Build release binary (~2 min)
cargo build --release

# Verify binary
./target/release/wp-offload-daemon --help
```

**Expected output:** Binary at `target/release/wp-offload-daemon` (~7 MB)

---

## Step 2: Prepare WordPress Plugin

### 2.1 Deploy Plugin
```bash
# On your WordPress server (or local wp-content/plugins)
PLUGIN_DIR="/path/to/wordpress/wp-content/plugins/wp-optimization-agents"
mkdir -p "$PLUGIN_DIR"

# Copy plugin source
rsync -av /Users/puleeno/Projects/nghesachxua.com/agents/wp-optimization-agents-plugin/ "$PLUGIN_DIR/"

# Install PHP dependencies
cd "$PLUGIN_DIR"
composer install --no-dev --optimize-autoloader
```

### 2.2 Activate & Configure
1. **WP Admin → Plugins → Activate "WP Optimization Agents"**
   - Creates/upgrades tables: `nsxu_wp_opt_agents_images`, `nsxu_wp_opt_agents_jobs` (DB v2.0.0)

2. **WP Admin → WP Opt Agents → Settings**
   Configure these required fields:

   | Section | Field | Value |
   |---------|-------|-------|
   | **Blogger / Chrome** | Blogger Blog ID | `1234567890123456789` (from Blogger URL) |
   | | Chrome Profile | `Default` (or your profile name) |
   | | Chrome Executable | `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome` (macOS) or `/usr/bin/google-chrome` (Linux) |
   | | Chrome User Data Dir | `/Users/you/Library/Application Support/Google/Chrome` (macOS) or `/home/you/.config/google-chrome` (Linux) |
   | | Headless Chrome | **OFF** (required for Google sign-in) |
   | **Media Offload** | Replace URLs in WP DB | **ON** |
   | | Backup Before Rewrite | **ON** |
   | | 301 Redirect Old URL → New | **ON** |
   | **Agent Daemon** | Daemon Port | `2095` |
   | | Poll Interval (seconds) | `60` |
   | | API Token | (generate a random string, e.g., `openssl rand -hex 32`) |

3. **Click "Save Settings"** — this writes clean options without legacy keys

### 2.3 Verify Plugin Tables
```sql
-- In MySQL
USE nghesachxua;
SHOW TABLES LIKE 'nsxu_wp_opt_agents%';
-- Should show: nsxu_wp_opt_agents_images, nsxu_wp_opt_agents_jobs

DESCRIBE nsxu_wp_opt_agents_images;
-- Should have: id, origin_url, origin_url_hash, blogger_url, blogger_image_id,
--              wp_post_id, wp_attachment_id, status, redirect_active, replaced_at, ...
```

---

## Step 3: Create Daemon Configuration

Create `/Users/puleeno/Projects/nghesachxua.com/agents/config.json`:

```json
{
  "database": {
    "host": "localhost",
    "port": 3306,
    "username": "root",
    "password": "root",
    "name": "nghesachxua",
    "table_prefix": "nsxu_"
  },
  "site": {
    "url": "https://nghesachxua.localhost",
    "uploads_dir": "/Users/puleeno/Projects/nghesachxua.com/wp-content/uploads"
  },
  "blogger": {
    "blog_id": "1234567890123456789",
    "upload_via": "chrome_web_ui",
    "one_wp_post_per_blogger_post": true
  },
  "chrome": {
    "executable": "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "user_data_dir": "/Users/puleeno/Library/Application Support/Google/Chrome",
    "profile_directory": "Default",
    "headless": false,
    "remote_debugging_port": 9222,
    "launch_timeout_secs": 30
  },
  "agent": {
    "host": "127.0.0.1",
    "port": 2095,
    "poll_interval_secs": 60,
    "replace_in_db": true,
    "backup_before_replace": true,
    "enable_old_url_redirect_301": true,
    "daemon_api_token": "your-generated-token-here"
  },
  "logging": {
    "level": "info",
    "json": false
  }
}
```

**Linux paths:**
```json
"chrome": {
  "executable": "/usr/bin/google-chrome",
  "user_data_dir": "/home/www-data/.config/google-chrome",
  "profile_directory": "Default"
}
```

---

## Step 4: Install & Run as Service

### macOS (launchd)

```bash
cd /Users/puleeno/Projects/nghesachxua.com/agents/daemon

# Make install script executable
chmod +x install.sh

# Install (requires sudo)
sudo ./install.sh install
```

**What it does:**
- Copies binary to `/usr/local/bin/wp-offload-daemon`
- Installs `/Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist`
- Loads and starts service

**Manage service:**
```bash
# Status
launchctl list | grep wp-offload

# Logs
tail -f /var/log/wp-offload-daemon/stdout.log
tail -f /var/log/wp-offload-daemon/stderr.log

# Stop
sudo launchctl stop com.puleeno.wp-offload-daemon

# Start
sudo launchctl start com.puleeno.wp-offload-daemon

# Uninstall
sudo ./install.sh uninstall
```

### Linux (systemd)

```bash
# Copy project to server
rsync -av /Users/puleeno/Projects/nghesachxua.com/ user@server:/opt/nghesachxua.com/

# On server
cd /opt/nghesachxua.com/agents/daemon
chmod +x install.sh
sudo ./install.sh install
```

**Manage service:**
```bash
# Status
systemctl status wp-offload-daemon

# Logs
journalctl -u wp-offload-daemon -f

# Restart
sudo systemctl restart wp-offload-daemon

# Stop
sudo systemctl stop wp-offload-daemon

# Uninstall
sudo ./install.sh uninstall
```

---

## Step 5: Verify Dashboard & API

### 5.1 Dashboard UI
Open browser: **http://localhost:2095**

You should see the React dashboard with tabs:
- **Overview** — stats, quick actions
- **Blogger Lab** — manual upload testing
- **Daemon Engine** — poll loop status
- **WordPress Scanner** — post/image discovery
- **Daily Audit** — Lighthouse integration
- **Config** — live config view

### 5.2 Health Check
```bash
curl http://localhost:2095/health
# {"status":"ok"}

curl http://localhost:2095/api/health
# {"status":"ok","uptime_secs":...,"db_connected":true,"chrome_ready":true}
```

### 5.3 Process a Post (Manual Trigger)
```bash
# Get a post ID with local images
curl -X POST http://localhost:2095/api/posts/123/process

# Response:
# {
#   "post_id": 123,
#   "blogger": {"post_id": "987654321", "post_url": "https://www.blogger.com/blog/post/edit/..."},
#   "uploaded_now": 5,
#   "replacements_applied": 10,
#   "replaced_mappings": 5,
#   "total_images": 5
# }
```

---

## Step 6: Test End-to-End Flow

### 6.1 Find a Test Post
```sql
-- Posts with local images in content
SELECT p.ID, p.post_title, p.post_content
FROM nsxu_posts p
WHERE p.post_content LIKE '%wp-content/uploads%'
  AND p.post_status = 'publish'
LIMIT 5;
```

### 6.2 Run Offload
```bash
# Replace 123 with actual post ID
curl -X POST http://localhost:2095/api/posts/123/process
```

### 6.3 Verify Results
1. **Dashboard → Blogger Lab** — shows uploaded images
2. **MySQL** — check mappings:
   ```sql
   SELECT origin_url, blogger_url, status, redirect_active
   FROM nsxu_wp_opt_agents_images
   WHERE wp_post_id = 123;
   ```
3. **WP Postmeta** — check link:
   ```sql
   SELECT meta_key, meta_value
   FROM nsxu_postmeta
   WHERE post_id = 123 AND meta_key LIKE '_wpoa_%';
   ```
4. **Blogger** — open the post URL, verify all images present
5. **301 Redirect Test**:
   ```bash
   curl -I "https://nghesachxua.localhost/wp-content/uploads/2024/01/test-image.jpg"
   # Should return 301 → Blogger URL
   ```

---

## Troubleshooting

### Chrome Won't Launch / CDP Connection Failed
```bash
# Check Chrome path
ls -la "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

# Check user data dir exists
ls -la "/Users/puleeno/Library/Application Support/Google/Chrome/Default"

# Port 9222 free?
lsof -i :9222
# Kill any existing Chrome with remote debugging
pkill -f "remote-debugging-port=9222"
```

### "Google sign-in required" in Logs
- Ensure Chrome profile is **already signed into Blogger**
- Run Chrome manually with that profile, go to blogger.com, verify signed in
- **Do not use headless mode** until sign-in is confirmed working
- Daemon uses existing profile — it does NOT handle OAuth flow

### Database Connection Failed
```bash
# Test MySQL connectivity
mysql -h localhost -u root -proot nghesachxua -e "SELECT 1"

# Check config.json database section matches wp-config.php
```

### Images Not Uploading / Picker Failed
- Blogger UI selectors may have changed (localized VN/EN)
- Check `blogger.rs` selectors for `Tải lên từ máy tính` / `Upload from computer`
- Increase timeouts in config if network slow
- Check logs for `picker_set_file` success/failure

### 301 Redirect Not Working
```sql
-- Verify mapping has redirect_active=1 and status=replaced
SELECT * FROM nsxu_wp_opt_agents_images
WHERE origin_url = '...' AND redirect_active = 1 AND status = 'replaced';

-- Check plugin setting
SELECT option_value FROM nsxu_options WHERE option_name = 'wp_opt_agents_settings';
-- JSON should contain "enable_old_url_redirect_301":true
```

### Service Won't Start (macOS)
```bash
# Check plist syntax
plutil -lint /Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist

# Check logs
tail -f /var/log/wp-offload-daemon/stderr.log

# Common: binary path wrong, config file missing, permissions
```

### Service Won't Start (Linux)
```bash
# Check service file
systemctl cat wp-offload-daemon

# Check journal
journalctl -u wp-offload-daemon -n 50 --no-pager

# Common: User www-data lacks Chrome profile access, config path wrong
```

---

## Configuration Reference

### Environment Variables (override config.json)
| Variable | Description | Default |
|----------|-------------|---------|
| `DAEMON_CONFIG` | Path to config.json | — |
| `DAEMON_HOST` | Bind host | `127.0.0.1` |
| `DAEMON_PORT` / `PORT` | Bind port | `2095` |
| `WEB_UI_DIST` | Dashboard build directory | — |
| `WP_CONFIG_PATH` | Path to wp-config.php (bootstrap) | — |
| `BLOGGER_BLOG_ID` | Blogger blog ID (bootstrap) | — |
| `SITE_URL` | WordPress site URL (bootstrap) | — |
| `RUST_LOG` | Log level (trace/debug/info/warn/error) | `info` |

### Config.json Sections
```json
{
  "database": { "host", "port", "username", "password", "name", "table_prefix" },
  "site": { "url", "uploads_dir" },
  "blogger": { "blog_id", "upload_via", "one_wp_post_per_blogger_post" },
  "chrome": { "executable", "user_data_dir", "profile_directory", "headless", "remote_debugging_port", "launch_timeout_secs" },
  "agent": { "host", "port", "poll_interval_secs", "replace_in_db", "backup_before_replace", "enable_old_url_redirect_301", "daemon_api_token" },
  "logging": { "level", "json" }
}
```

---

## Security Notes

1. **Chrome profile** — contains Google auth cookies; restrict access to daemon user only
2. **Database credentials** — config.json should be `chmod 600`, owned by daemon user
3. **API token** — used for plugin REST compatibility; generate strong random value
4. **Network** — daemon binds to `127.0.0.1:2095` by default; use reverse proxy (nginx) for external access with TLS
5. **Logs** — may contain URLs; rotate and restrict access

---

## Updating

```bash
# Pull latest changes
cd /Users/puleeno/Projects/nghesachxua.com
git pull

# Rebuild
cd agents/daemon
cargo build --release

# Restart service
# macOS:
sudo launchctl stop com.puleeno.wp-offload-daemon
sudo launchctl start com.puleeno.wp-offload-daemon

# Linux:
sudo systemctl restart wp-offload-daemon
```

---

## File Locations Summary

| File | Purpose |
|------|---------|
| `agents/daemon/target/release/wp-offload-daemon` | Release binary |
| `agents/daemon/macos/com.puleeno.wp-offload-daemon.plist` | macOS launchd service |
| `agents/daemon/linux/wp-offload-daemon.service` | Linux systemd service |
| `agents/daemon/install.sh` | Cross-platform installer |
| `agents/config.json` | Daemon configuration |
| `agents/web-ui/dist/` | Dashboard UI (served by daemon) |
| `wp-content/plugins/wp-optimization-agents/` | WordPress plugin |
| `/var/log/wp-offload-daemon/` | Daemon logs (macOS) |
| `journalctl -u wp-offload-daemon` | Daemon logs (Linux) |

---

## Support

- **Issues**: Check logs first (`RUST_LOG=debug` for verbose)
- **Blogger UI changes**: Update selectors in `agents/daemon/src/blogger.rs`
- **WP schema changes**: Update `agents/wp-optimization-agents-plugin/src/Database/Schema.php`
- **Dashboard**: Rebuild with `cd agents/web-ui && npm run build`