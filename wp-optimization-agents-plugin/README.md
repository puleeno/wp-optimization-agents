# WP Optimization Agents Plugin

WordPress plugin companion for the `wp-offload-daemon` agent that offloads post images to Blogger via the **Blogger web UI (Chrome automation)** — no Google/Blogger API or OAuth tokens. The Rust daemon reads the WordPress DB directly, manages offload mappings, replaces image URLs in the DB, and records WP↔Blogger post links; this plugin provides compatible DB schema, 301 redirects for replaced URLs, admin settings, and optional compatibility REST endpoints.

## Architecture (current)

- **Upload path:** Chrome browser automation against Blogger composer (CDP), not Google/Blogger API.
- **1 WP post = 1 Blogger post/album:** all pending images for a WP post are uploaded to a single Blogger post; the link is stored on the WP post.
- **DB is the source of truth:** daemon connects to WP MySQL (`{$prefix}wp_opt_agents_images`) to track origin/blogger URLs, attachment/post IDs, status, and redirect state.
- **URL replacement:** daemon rewrites old image URLs to Blogger URLs in post content (with backup meta) and marks mappings replaced with `redirect_active=1`.
- **Old URL handling:** plugin serves `301` redirects from original image URLs to Blogger URLs when `redirect_active` mappings exist (and redirect is enabled).
- **Daemon/UI:** Rust daemon (`wp-offload-daemon`) serves the dashboard UI and API on port `2095` (configurable).

## Installation

1. Place plugin in `wp-content/plugins/wp-optimization-agents`.
2. Run `composer install` in the plugin directory (if dependencies are used).
3. Activate plugin in WordPress admin (creates/upgrades tables via `dbDelta`).
4. Configure at **WP Opt Agents > Settings**.

On activation/upgrade (DB version `2.0.0`) the plugin ensures:

- `{$prefix}wp_opt_agents_images` — image offload mappings (`origin_url`, `origin_url_hash`, `blogger_image_url`, `wp_post_id`, `wp_attachment_id`, status `pending|uploading|uploaded|replacing|replaced|failed`, `redirect_active`, timestamps/metadata).
- `{$prefix}wp_opt_agents_jobs` — job tracking.

WP↔Blogger post link is stored as post meta on the WP post: `_wpoa_blogger_post_id`, `_wpoa_blogger_post_url`, `_wpoa_blogger_album_url`, `_wpoa_blogger_blog_id`, `_wpoa_blogger_linked_at`.

## Settings

### Blogger / Chrome web UI
- **Blogger Blog ID** — target blog for uploads.
- **Upload mode** — Chrome web UI (no API).
- **Chrome executable / user data dir / profile directory / headless** — used by daemon/browser automation.

### Agent / Offload
- **Daemon host/port** (default `127.0.0.1:2095`), poll interval.
- **Replace in DB** after upload, **backup before replace**.
- **Enable old URL 301 redirect** to Blogger URL (served by plugin from mapping table).
- **Daemon API token** — Bearer token for compatibility REST endpoints.

### Daily Audit (optional)
- Enable/configure Lighthouse audit targets/schedule/thresholds and notifications if used by the agent stack.

## Compatibility REST API

Optional endpoints for daemon/plugin compatibility (Bearer `Authorization: Bearer <daemon_api_token>`):

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/wp-opt-agents/v1/config` | Plugin/agent configuration |
| POST | `/wp-opt-agents/v1/posts/{id}/scan` | Scan/register post images |
| GET | `/wp-opt-agents/v1/posts/{id}/images` | List post image mappings |
| GET | `/wp-opt-agents/v1/posts/{id}/status` | Post offload status |
| POST | `/wp-opt-agents/v1/posts/{id}/link-blogger` | Save WP↔Blogger post link |
| POST | `/wp-opt-agents/v1/images/register` | Register image mapping (`/images` alias) |
| POST | `/wp-opt-agents/v1/images/{id}/uploaded` | Mark image uploaded |
| POST | `/wp-opt-agents/v1/images/{id}/replaced` | Mark image replaced (`redirect_active`) |
| POST | `/wp-opt-agents/v1/images/{id}/fail` | Mark image failed (`/sync` legacy alias supported) |
| POST | `/wp-opt-agents/v1/posts/{id}/replace-content` | Replace image URLs in post content |
| GET | `/wp-opt-agents/v1/stats` | Offload statistics |

> Note: the primary offload flow is driven by the Rust daemon via direct DB access and Chrome automation; REST routes are for compatibility/integration.

## Redirect behavior

When a request path matches an `origin_url` with an active replaced mapping (`replaced` + `redirect_active=1`), the plugin issues a `301` redirect to the stored `blogger_image_url`. Disable via the **Enable old URL 301 redirect** setting.

## Development

```bash
composer install
composer test
composer cs
composer cbf
```

## License

GPL-2.0-or-later
