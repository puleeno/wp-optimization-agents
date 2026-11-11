# WP Optimization Agents Plugin

WordPress plugin providing REST API endpoints for the WP Optimization Agents daemon to offload images to Blogger CDN and optimize WordPress performance.

## Features

- **REST API** - Secure endpoints for daemon communication
- **Image Offload Tracking** - Database tracking of offloaded images with status
- **Post Content Replacement** - Automatic URL replacement in post content
- **Settings Management** - Comprehensive admin UI for configuration
- **Daily Audit Integration** - Support for automated Lighthouse audits

## Installation

1. Place plugin in `wp-content/plugins/wp-optimization-agents`
2. Run `composer install` in plugin directory
3. Activate plugin in WordPress admin
4. Configure settings at **WP Opt Agents > Settings**

## API Endpoints

All endpoints require `Authorization: Bearer <token>` header.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/wp-opt-agents/v1/config` | Get plugin configuration for daemon |
| POST | `/wp-opt-agents/v1/posts/{id}/scan` | Scan post for local images |
| GET | `/wp-opt-agents/v1/posts/{id}/images` | Get offload status for post images |
| POST | `/wp-opt-agents/v1/images` | Register images for offload |
| POST | `/wp-opt-agents/v1/images/{id}/sync` | Mark image as synced with Blogger URL |
| POST | `/wp-opt-agents/v1/images/{id}/fail` | Mark image as failed |
| POST | `/wp-opt-agents/v1/posts/{id}/replace-content` | Replace image URLs in post content |
| GET | `/wp-opt-agents/v1/posts/{id}/status` | Get post offload status |
| GET | `/wp-opt-agents/v1/stats` | Get global offload statistics |

## Configuration

Settings available in admin panel:

### Blogger Configuration
- **Blog ID** - Your Blogger blog ID
- **Album Name** - Album to store offloaded images
- **Auth Type** - OAuth 2.0 or Service Account
- **CDN Domain Override** - Custom CDN domain (optional)

### Media Offload Settings
- **Auto WebP Conversion** - Enable automatic WebP
- **Default Quality** - Image quality (1-100)
- **Sync Chunk Size** - Images per batch
- **Replace Post Content** - Auto-replace URLs after offload
- **Backup Before Rewrite** - Create backup before changes

### Daemon Settings
- **Port** - Daemon HTTP port (default 2095)
- **Log Level** - debug/info/warn/error
- **Worker Threads** - Tokio worker threads
- **Poll Interval** - Daemon poll interval (seconds)
- **Auto Retry Limit** - Max retries for failed images
- **API Token** - Bearer token for daemon auth

### Daily Audit
- **Enable Daily Audit** - Run automated Lighthouse audits
- **Cron Schedule** - Cron expression (default 03:00 UTC)
- **Target URLs** - URLs to audit (one per line)
- **Slack Webhook** - Notification webhook
- **Alert LCP Threshold** - Alert if LCP exceeds (ms)

## Architecture

### Design Patterns
- **Repository Pattern** - Data access abstraction (`ImageRepository`, `SettingsRepository`)
- **Service Layer** - Business logic encapsulation (`OffloadService`, `SettingsService`)
- **Dependency Injection** - Interfaces for all dependencies
- **Factory/Service Locator** - `Plugin` class manages service instances
- **Single Responsibility** - Each class has one purpose

### Database Schema
Table: `wp_wp_opt_agents_images`
- Tracks each image offload operation
- Status: pending → processing → synced/failed
- Stores Blogger URL, transformed params, metadata

## Development

```bash
# Install dependencies
composer install

# Run tests
composer test

# Code style check
composer cs

# Auto-fix code style
composer cbf
```

## Daemon Integration

The Rust daemon should:
1. Call `GET /config` on startup to get settings
2. Use `POST /posts/{id}/scan` to find images
3. Call `POST /images` to register images
4. Process images and call `POST /images/{id}/sync` or `/fail`
5. Call `POST /posts/{id}/replace-content` to update post

## License

GPL-2.0-or-later