# WP Offload Daemon - Release Notes v0.2.0

## Overview
Complete rewrite of the offload daemon with **zero Google/Blogger API dependency**. All uploads now go through **Chrome Web UI automation** via Chrome DevTools Protocol (CDP).

## Major Changes (v0.2.0)

### Architecture Shift
| Before (v0.1) | After (v0.2) |
|---------------|--------------|
| Blogger API v3 + OAuth2 | Chrome CDP → Blogger web UI |
| Service Account / OAuth tokens | Pre-signed Chrome profile |
| Per-image API calls | 1 WP post = 1 Blogger post (album) |
| External vault sync | Direct WP DB URL replacement |
| Plugin-only redirects | Daemon replaces + Plugin 301s |

### New Capabilities
- **Chrome CDP automation** — launches Chrome with remote debugging, attaches to targets
- **Blogger composer reuse** — opens existing post editor (`/post/edit/{blogId}/{postId}`) for updates
- **Google Picker OOPIF upload** — handles `docs.google.com/picker` iframe file uploads
- **Trusted click events** — `Input.dispatchMouseEvent` for Closure UI buttons
- **Publish flow** — handles Vietnamese/English labels, confirmation modals
- **Editor settled wait** — detects "Đã lưu"/"Saved" before closing
- **Post ID extraction** — parses `/post/edit/{blogId}/{postId}` from URL
- **WP↔Blogger linking** — stores `post_id`, `post_url`, `blog_id`, `linked_at` in postmeta

### Database Schema (Plugin v2.0.0)
```sql
CREATE TABLE nsxu_wp_opt_agents_images (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  origin_url TEXT NOT NULL,
  origin_url_hash CHAR(64) NOT NULL UNIQUE,
  blogger_url TEXT NULL,
  blogger_image_id VARCHAR(255) NULL,
  wp_post_id BIGINT UNSIGNED NOT NULL,
  wp_attachment_id BIGINT UNSIGNED NULL,
  mime_type VARCHAR(100) NULL,
  file_size BIGINT UNSIGNED NULL,
  width INT UNSIGNED NULL,
  height INT UNSIGNED NULL,
  metadata JSON NULL,
  status ENUM('pending','uploading','uploaded','replacing','replaced','failed') DEFAULT 'pending',
  error_message TEXT NULL,
  redirect_active TINYINT(1) DEFAULT 0,
  replaced_at DATETIME NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  INDEX idx_post (wp_post_id),
  INDEX idx_status (status),
  INDEX idx_redirect (redirect_active, status)
);
```

WP Postmeta keys:
- `_wpoa_blogger_post_id` — Blogger post ID
- `_wpoa_blogger_post_url` — Blogger post editor URL
- `_wpoa_blogger_album_url` — Blogger post public URL
- `_wpoa_blogger_blog_id` — Blogger blog ID
- `_wpoa_blogger_linked_at` — ISO8601 timestamp
- `_wpoa_backup_post_content` — original post_content before replacement

### URL Replacement & Redirects
1. Daemon collects `(origin_url, blogger_url)` pairs for uploaded images
2. Replaces in `nsxu_posts.post_content` (with `_wpoa_backup_post_content` backup)
3. Best-effort replacement in `nsxu_postmeta.meta_value` and `nsxu_options.option_value`
4. Marks mappings `status='replaced'`, `redirect_active=1`, `replaced_at=NOW()`
5. PHP plugin `template_redirect` hook serves **301** for any request matching `origin_url` with active redirect

### Configuration (config.json)
```json
{
  "database": { "host", "port", "username", "password", "name", "table_prefix" },
  "site": { "url", "uploads_dir" },
  "blogger": { "blog_id", "upload_via": "chrome_web_ui", "one_wp_post_per_blogger_post": true },
  "chrome": { "executable", "user_data_dir", "profile_directory", "headless", "remote_debugging_port", "launch_timeout_secs" },
  "agent": { "host", "port", "poll_interval_secs", "replace_in_db", "backup_before_replace", "enable_old_url_redirect_301", "daemon_api_token" },
  "logging": { "level", "json" }
}
```

### Dashboard UI (React 19 + Vite + Tailwind v4)
- Served by daemon at `http://host:port/` (SPA fallback)
- Tabs: Overview, Blogger Lab, Daemon Engine, WordPress Scanner, Daily Audit, Config
- Real-time status via `/api/health`, `/api/metrics`, `/api/status`
- Manual trigger: `POST /api/posts/{id}/process`

### REST API (Daemon)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` / `/api/health` | Health check |
| GET | `/api/metrics` | Prometheus-style metrics |
| GET | `/api/status` | Daemon status (DB, Chrome, poll loop) |
| POST | `/api/posts/{id}/process` | Full offload pipeline for one post |

### Plugin Compatibility REST API
Bearer token: `daemon_api_token` from settings
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/wp-opt-agents/v1/config` | Plugin config for daemon |
| POST | `/wp-opt-agents/v1/posts/{id}/scan` | Scan/register post images |
| GET | `/wp-opt-agents/v1/posts/{id}/images` | List image mappings |
| GET | `/wp-opt-agents/v1/posts/{id}/status` | Post offload status |
| POST | `/wp-opt-agents/v1/posts/{id}/link-blogger` | Save WP↔Blogger link |
| POST | `/wp-opt-agents/v1/images/register` | Register image mapping |
| POST | `/wp-opt-agents/v1/images/{id}/uploaded` | Mark uploaded |
| POST | `/wp-opt-agents/v1/images/{id}/replaced` | Mark replaced + redirect |
| POST | `/wp-opt-agents/v1/images/{id}/fail` | Mark failed |
| POST | `/wp-opt-agents/v1/posts/{id}/replace-content` | Replace URLs in content |
| GET | `/wp-opt-agents/v1/stats` | Global statistics |

## Migration from v0.1
1. **Backup database** and plugin settings
2. **Update plugin** to v0.2.0 (runs DB upgrade to v2.0.0)
3. **Delete old OAuth/Service Account credentials** — no longer used
4. **Create Chrome profile** signed into Blogger
5. **Write config.json** with Chrome paths, Blogger blog ID
6. **Build & install daemon** v0.2.0
7. **Test** with `POST /api/posts/{id}/process`

## Known Limitations
- **Headless Chrome** — Google sign-in/Picker often fails in headless; use `headless: false`
- **Profile locking** — Only one daemon instance per Chrome profile (remote debugging port)
- **Blogger UI locale** — Selectors cover VN/EN; other locales may need updates in `blogger.rs`
- **Daily post quota** — Blogger limits ~50 posts/day; daemon detects quota notice and errors
- **Large uploads** — No chunking; very large images may timeout (configure `launch_timeout_secs`)

## Files Changed
```
agents/daemon/
├── Cargo.toml                    # v0.2.0, new deps
├── src/
│   ├── main.rs                   # Entry, config loading
│   ├── lib.rs                    # Library exports
│   ├── config.rs                 # New Config struct (DB, Site, Blogger, Chrome, Agent, Logging)
│   ├── error.rs                  # Error types (Config, Database, Chrome, Blogger, Replace, ...)
│   ├── chrome.rs                 # CDP client (launch, WS, trusted clicks, picker, evaluate)
│   ├── blogger.rs                # Blogger upload (composer, picker, publish, wait settled)
│   ├── db.rs                     # MySQL pool, mappings, postmeta, replacements
│   ├── offload.rs                # Orchestration engine (poll loop + manual trigger)
│   └── main_lib.rs               # Axum server (static UI + API)
├── macos/com.puleeno.wp-offload-daemon.plist
├── linux/wp-offload-daemon.service
└── install.sh

agents/wp-optimization-agents-plugin/
├── wp-optimization-agents.php    # v0.2.0, DB v2.0.0
├── src/Database/Schema.php       # New tables, postmeta keys
├── src/Repository/ImageRepository.php  # Hash, CRUD, redirect lookup
├── src/Service/OffloadService.php      # Replace content, 301 redirect, link blogger
├── src/Service/SettingsService.php     # New defaults (no auth_type)
├── src/Admin/Settings.php        # Chrome/Blogger fields only
├── src/Admin/Pages/Renderer.php  # Updated dashboard text
└── README.md                     # Rewritten for new architecture

agents/web-ui/                    # Dashboard (built to dist/)
agents/config.example.json        # Example config
agents/SETUP.md                   # Complete setup guide
agents/RELEASE.md                 # This file
```

## Verification Checklist
- [ ] `cargo build --release` succeeds
- [ ] Plugin activates, creates `nsxu_wp_opt_agents_images` + `jobs` tables
- [ ] Plugin settings save without "Auth Type" field
- [ ] Daemon starts, serves dashboard at `:2095`
- [ ] `GET /api/health` returns `{"status":"ok","db_connected":true,"chrome_ready":true}`
- [ ] Chrome launches with profile, CDP connects
- [ ] `POST /api/posts/{id}/process` uploads images to Blogger
- [ ] Blogger post created/updated with all images
- [ ] WP postmeta `_wpoa_*` populated
- [ ] `post_content` URLs replaced, backup meta created
- [ ] Mappings marked `replaced` + `redirect_active=1`
- [ ] Old image URL returns `301` → Blogger URL
- [ ] Service persists across reboot (launchd/systemd)