# WP Offload Agents — Setup Guide (macOS · Linux · Windows)

Điều khiển từ xa: **Rust daemon** (`wp-offload-daemon`) chạy HTTP API + Web UI tại cổng **2095**, đọc/ghi WordPress MySQL, upload ảnh lên **imgbb (backup)** → **CDN proxy** → **Blogger qua Chrome CDP**, rồi **crop/resize** theo từng image size và cập nhật CDN sang phiên bản mới nhất.

> Không dùng Google/Blogger API, không OAuth. Mọi thao tác Blogger thực hiện trên Chrome profile đã đăng nhập.

---

## 1. Kiến trúc & luồng xử lý

```
WP media upload (hook wp_generate_attachment_metadata)
        │  POST /api/webhooks/media-upload (Bearer token)
        ▼
┌──────────────────────── wp-offload-daemon (:2095) ────────────────────────┐
│  webhook  → imgbb upload (url | blob)  → CDN map + url_versions           │
│           → replace URL trong post_content/postmeta/options               │
│           → queue Blogger (status=queued)                                 │
│                                                                          │
│  blogger worker (Chrome CDP): queued → uploaded                           │
│  finish worker  (song song, N luồng): uploaded → finished                 │
│           → crop/resize theo attachment_id + image_size                   │
│           → CDN trỏ bản Blogger mới nhất                                   │
│                                                                          │
│  media scan worker: mỗi scan_interval_secs quét attachment chưa xử lý      │
│  audit: Chrome headless đo LCP/FCP/CLS/TTFB (POST /api/audits/run)        │
└───────────────────────────────────────────────────────────────────────────┘
        │                         │                        │
        ▼                         ▼                        ▼
   MySQL (WP DB)            imgbb (backup)          Blogger (Chrome UI)
```

**Trạng thái job Blogger:** `queued → uploaded → finished` (`failed` khi lỗi, xem `last_error`).

---

## 2. Yêu cầu

| Thành phần | Tối thiểu | Gợi ý |
|---|---|---|
| OS | macOS 12 · Ubuntu 20.04/Debian 11 · Windows 10 1809+ | bản LTS mới |
| Rust | 1.75+ | 1.80+ |
| Chrome/Chromium | 118+ | bản stable mới |
| MySQL/MariaDB | 5.7 / 10.3 | 8.0 / 10.6 |
| WordPress | 6.0+ | 6.5+ |
| PHP | 8.1+ | 8.3+ |
| Node.js | 18+ (chỉ để build UI) | 20+ |

**Chrome profile (bắt buộc):** profile đã đăng nhập Blogger/Google.
- Không bật headless cho lần đăng nhập đầu; sau đó có thể đặt `chrome.headless=true` (audit luôn headless).
- **Không mở Chrome bằng profile đó trong khi daemon chạy** — nếu Chrome đang chạy sẵn, `--remote-debugging-port` bị bỏ qua và job Blogger lỗi `DevTools not ready on port 9222`.
- Cổng debug mặc định `9222` (`chrome.remote_debugging_port`).

---

## 3. Build

### 3.1 macOS / Linux

```bash
git clone <repo> && cd nghesachxua.com/agents

# (tuỳ chọn) Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh && source "$HOME/.cargo/env"

# Web UI (bắt buộc: daemon serve dist/)
cd web-ui && npm install && npm run build && cd ..

# Daemon
cd daemon && cargo build --release
# artefact: target/release/wp-offload-daemon
```

### 3.2 Windows (PowerShell)

```powershell
# Rust: cài https://rustup.rs (winget install Rustlang.Rustup hoặc tải installer)
# Node: winget install OpenJS.NodeJS.LTS

cd agents\web-ui; npm install; npm run build
cd ..\daemon; cargo build --release
# artefact: target\release\wp-offload-daemon.exe
```

---

## 4. Cấu hình

### 4.1 File & biến môi trường

| Biến | Ý nghĩa | Mặc định |
|---|---|---|
| `DAEMON_CONFIG` | đường dẫn config JSON | bắt buộc (hoặc `WP_CONFIG_PATH` + `BLOGGER_BLOG_ID` + `SITE_URL`) |
| `WEB_UI_DIST` | thư mục SPA đã build | `<repo>/web-ui/dist` |
| `DAEMON_HOST` / `DAEMON_PORT` (`PORT`) | override host/port | theo config |
| `RUST_LOG` | mức log | `info` |

File config hỗ trợ **placeholder `${VAR}` / `${VAR:-default}`** (ưu tiên biến môi trường) và **đường dẫn tương đối** (quy chiếu thư mục chứa config, tự canonicalize):

```json
"site": {
  "url": "${SITE_URL:-https://nghesachxua.localhost}",
  "wp_path": "${WP_PATH:-../..}",
  "uploads_dir": "${WP_UPLOADS_DIR:-../wp-content/uploads}"
},
"database": { "host": "${DB_HOST:-127.0.0.1}", "user": "${DB_USER:-root}", "password": "${DB_PASSWORD:-root}", "name": "${DB_NAME:-nghesachxua}" }
```

Mẫu đầy đủ: `agents/config.example.json`.

### 4.2 Giải thích trường

| Nhóm | Trường | Ghi chú |
|---|---|---|
| `database` | `host,port,user,password,name,table_prefix` | prefix `nsxu_` theo `wp-config.php` |
| `site` | `url`, `wp_path`, `uploads_dir` | `uploads_dir` để tìm file local khi imgbb `blob` |
| `agent` | `host`,`port`,`poll_interval_secs`,`replace_in_db`,`backup_before_replace`,`enable_old_url_redirect_301`,`daemon_api_token` | token phải trùng plugin WP |
| `agent` | `scan_enabled`,`scan_interval_secs`,`scan_batch_size` | worker tự quét media chưa xử lý |
| `agent` | `finish_workers`,`delete_local_after_offload` | luồng crop/resize song song; **tắt xoá file gốc** nếu chưa an tâm |
| `imgbb` | `enabled`,`api_key`,`upload_format` | `blob` = upload file local (local/private WP); `url` = imgbb tự tải URL (cần public) |
| `cdn` | `host`,`scheme` | domain proxy `/wp-content/uploads/...` |
| `chrome` | `executable`,`user_data_dir`,`profile_directory`,`headless`,`remote_debugging_port`,`launch_timeout_secs` | để trống executable/user_data_dir = **tự dò theo OS** |
| `blogger` | `blog_id`,`upload_via`,`one_wp_post_per_blogger_post` | `blog_id` là ID thật (không để `YOUR_...`) |
| `logging` | `level` | `info` mặc định |

### 4.3 Giá trị mặc định theo OS

| OS | Chrome executable | Chrome user data dir |
|---|---|---|
| macOS | `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome` | `~/Library/Application Support/Google/Chrome` |
| Linux | `/usr/bin/google-chrome` → `chromium` → `microsoft-edge` | `~/.config/google-chrome` |
| Windows | `C:\Program Files\Google\Chrome\Application\chrome.exe` | `%LOCALAPPDATA%\Google\Chrome\User Data` |

Để trống 2 trường này trong config ⇒ daemon tự chọn theo OS.

---

## 5. Plugin WordPress

```bash
PLUGIN_DIR="/path/to/wordpress/wp-content/plugins/wp-optimization-agents"
mkdir -p "$PLUGIN_DIR"
rsync -av agents/wp-optimization-agents-plugin/ "$PLUGIN_DIR/"
cd "$PLUGIN_DIR" && composer install --no-dev --optimize-autoloader
```

Trong **WP Admin → WP Opt Agents → Settings**:

| Trường (WP Admin) | Giá trị |
|---|---|
| **Daemon URL** | `http://<agent-host>:2095` |
| **Plugin/Daemon API Token** | trùng `agent.daemon_api_token` (trong `agents/config.json`) |
| **Notify Daemon on Media Upload** | **ON** — gửi webhook `POST /api/webhooks/media-upload` khi có media mới |

> Blogger/Chrome/imgbb/CDN **không** cấu hình trong plugin — tất cả nằm ở `agents/config.json` (hoặc `/config` trên UI).

Plugin tạo/bảo trì bảng `*_wp_opt_agents_images`, `*_wp_opt_agents_jobs`. Các bảng pipeline còn lại (`*_wp_opt_agents_cdn_map`, `*_wp_opt_agents_url_versions`, `*_wp_opt_agents_blogger_queue`, `*_wp_opt_agents_logs`, `*_wp_opt_agents_audits`) do daemon tự tạo.

Kiểm tra:
```sql
SHOW TABLES LIKE 'nsxu_wp_opt_agents%';
```

---

## 6. Chạy thử trước

```bash
DAEMON_CONFIG=$PWD/agents/config.json WEB_UI_DIST=$PWD/agents/web-ui/dist \
  RUST_LOG=info ./agents/daemon/target/release/wp-offload-daemon
```

Mở `http://localhost:2095`, kiểm tra lần lượt:

| Kiểm tra | Kỳ vọng |
|---|---|
| `GET /api/health` | `200` |
| `GET /api/metrics` | có `uptimeSeconds`, `memoryMb`, `cpuPercent`, `tokioWorkers` |
| `GET /api/config` | có `_meta.path` (đường dẫn config đang dùng), `cdn`, `imgbb`, `agent.scan_*` |
| `GET /api/chrome/preflight` | `chrome_exists`, `profile_exists`, `port_open` + `advice` |
| `GET /api/scan/status` | `imgbb_ready`, `blogger_ready`, `queue`, `pending` |
| `/scanner`, `/config`, `/daemon` | UI 200 (SPA fallback) |

---

## 7. Chrome Login CLI (macOS · Linux · Windows)

Dùng binary daemon để mở Chrome + đăng nhập Blogger một lần, sau đó daemon tái sử dụng profile đó (attach-first, không mở Chrome thứ 2).

```bash
# Xem trợ giúp
wp-offload-daemon chrome

# Mở Chrome với profile riêng + port debug, vào trang Blogger
wp-offload-daemon chrome setup                 # mặc định port 9222, profile ~/.config/wp-opt-agents-chrome
wp-offload-daemon chrome setup --port 9223 --profile /path/to/profile

# Kiểm tra port debug + trạng thái đăng nhập Blogger
wp-offload-daemon chrome status

# Chờ tới khi đăng nhập thành công rồi ghi chrome.user_data_dir vào config
wp-offload-daemon chrome login --save          # --config /path/config.json nếu không đặt DAEMON_CONFIG
```

**Đặc điểm:**
- Dùng **profile riêng** (`chrome.user_data_dir` mặc định `~/.config/wp-opt-agents-chrome`; Windows `%LOCALAPPDATA%\wp-opt-agents-chrome`) → không đụng profile Chrome chính, không bị khoá profile.
- Tự dò Chrome theo OS nếu `chrome.executable` để trống (macOS `Google Chrome`, Linux `google-chrome`/`chromium`, Windows `chrome.exe`); macOS dùng `open -na` để mở đúng GUI session.
- `--save` chỉ ghi `chrome.user_data_dir` vào `DAEMON_CONFIG` (hoặc `--config`), **không** ghi secret khác.
- Sau khi login: daemon sẽ **attach vào Chrome đang chạy** nếu port debug đã mở; nếu không, daemon tự mở Chrome với profile này.

**Windows:** chạy trong PowerShell (nếu là service dạng SYSTEM, hãy chạy CLI bằng chính user sẽ dùng Chrome):
```powershell
.\wp-offload-daemon.exe chrome setup
.\wp-offload-daemon.exe chrome login --save --config C:\wp-opt-agents\config.json
```

---

## 8. Cài như service

### 8.1 macOS (launchd)

```bash
cd agents/daemon
sudo ./install.sh install          # copy binary → /usr/local/bin, cài plist, load service
```

`~/Library/LaunchDaemons` → file hệ thống: `/Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist` (sửa `DAEMON_CONFIG`, `WEB_UI_DIST`, `WorkingDirectory` nếu repo không nằm ở `/Users/puleeno/Projects/nghesachxua.com`).

```bash
launchctl list | grep wp-offload
tail -f /var/log/wp-offload-daemon/stdout.log
sudo launchctl bootout system /Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist
sudo launchctl bootstrap system /Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist
# hoặc: sudo launchctl kickstart -k system/com.puleeno.wp-offload-daemon
```

### 8.2 Linux (systemd)

```bash
sudo rsync -av agents/ /opt/wp-opt-agents/
cd /opt/wp-opt-agents/daemon && sudo ./install.sh install
```

Unit mẫu `agents/daemon/linux/wp-offload-daemon.service` — cần chỉnh cho production:

```ini
[Service]
User=www-data
Group=www-data
WorkingDirectory=/opt/wp-opt-agents
ExecStart=/usr/local/bin/wp-offload-daemon
Environment=RUST_LOG=info
Environment=DAEMON_CONFIG=/opt/wp-opt-agents/agents/config.json
Environment=WEB_UI_DIST=/opt/wp-opt-agents/agents/web-ui/dist
Restart=on-failure
RestartSec=10
# Chrome cần profile ghi được + /dev/shm; nếu dùng profile ở /home/* hãy tắt ProtectHome
ProtectHome=false
PrivateTmp=false
ReadWritePaths=/opt/wp-opt-agents /var/log/wp-offload-daemon /var/lib/wp-offload-daemon
```

Với `User=www-data`, đặt `chrome.user_data_dir=/var/lib/wp-offload-daemon/chrome` (và `mkdir -p` + `chown www-data`), hoặc chạy service bằng user của bạn.

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now wp-offload-daemon
journalctl -u wp-offload-daemon -f
```

### 8.3 Windows

Không có `install.sh` cho Windows. Chạy 2 cách:

**A. Task Scheduler (đơn giản):** tạo task *At startup* → Action `Start a program` = `C:\wp-opt-agents\wp-offload-daemon.exe`, *Start in* = `C:\wp-opt-agents`, tick *Run whether user is logged on or not*.

**B. NSSM (khuyến nghị):**
```powershell
winget install NSSM.NSSM
nssm install wp-offload-daemon C:\wp-opt-agents\wp-offload-daemon.exe
nssm set wp-offload-daemon AppDirectory C:\wp-opt-agents
nssm set wp-offload-daemon AppEnvironmentExtra DAEMON_CONFIG=C:\wp-opt-agents\config.json WEB_UI_DIST=C:\wp-opt-agents\web-ui\dist RUST_LOG=info
nssm set wp-offload-daemon Start SERVICE_AUTO_START
nssm start wp-offload-daemon
```

Lưu ý Windows:
- `chrome.user_data_dir` nên trỏ `C:\Users\<user>\AppData\Local\Google\Chrome\User Data`; service chạy `LocalSystem` có thể không thấy profile của user → dùng task "Run only when user is logged on" hoặc tách profile riêng.
- Nếu chạy dạng service, đặt `chrome.headless=true` và dùng profile riêng đã đăng nhập Blogger.

---

## 9. Vận hành

### 9.1 API chính

| Endpoint | Mô tả |
|---|---|
| `GET /api/health` · `GET /api/metrics` | trạng thái, uptime, RAM, CPU, tokio workers |
| `GET /api/config` · `PUT /api/config` | đọc/ghi config (merge + validate, ghi atomic, cập nhật live) |
| `GET /api/attachments?limit&offset` | danh sách media (header `x-total-count`) |
| `POST /api/attachments/:id/process` | offload 1 attachment ngay |
| `POST /api/scan/run` · `GET /api/scan/status` | chạy ngay / xem hàng đợi worker |
| `POST /api/audits/run` · `GET /api/audits` | audit tốc độ bằng Chrome headless |
| `GET /api/chrome/preflight` | kiểm tra Chrome/profile/port debug |
| `GET /api/logs?level&subsystem&q&attachment_id&limit&offset` | log chi tiết (bảng `*_wp_opt_agents_logs`) |
| `GET /api/logs/stream` · `GET /api/tasks/stream` | SSE realtime log / task |
| `GET /cdn/*path` | CDN proxy (imgbb → Blogger bản mới nhất) |

### 9.2 Web UI
- `/` tổng quan · `/blogger` · `/daemon` (log thật + SSE) · `/scanner` (media library + nút **Log**/**Quét ngay**) · `/audit` · `/config` (hiện đường dẫn config + preflight Chrome).

### 9.3 Bảo mật
- `agent.daemon_api_token` là Bearer token cho webhook → sinh bằng `openssl rand -hex 32`.
- Không commit `agents/config.json` chứa mật khẩu/API key thật (dùng `${VAR}` + biến môi trường).
- `GET /api/config` đã mask `database.password` và `imgbb.api_key`; PUT giữ nguyên key nếu gửi rỗng.

---

## 10. Troubleshooting

| Triệu chứng | Nguyên nhân & cách xử lý |
|---|---|
| `imgbb HTTP 400 "Can't download remote image"` | `imgbb.upload_format="url"` nhưng WP không public → đổi sang `blob`. |
| `attachment offload failed ... file missing on disk` | File local đã bị xoá trước đó → bật lại `delete_local_after_offload=false` và upload lại từ WP. |
| `Chrome/CDP error: DevTools not ready on port 9222` | Chrome đang mở cùng profile → tắt hẳn (Cmd+Q / kill chrome) rồi thử lại; hoặc đổi `remote_debugging_port`. Dùng `/api/chrome/preflight`. |
| Blogger job `failed` hàng loạt | `blogger.blog_id` chưa đúng (không phải `YOUR_...`), hoặc profile chưa đăng nhập Blogger. |
| `/scanner` 404 | Đang chạy daemon cũ chưa deploy → copy binary mới + restart service. |
| `/api/attachments` 500 `ONLY_FULL_GROUP_BY` | Binary cũ → deploy bản mới. |
| Config lưu nhưng reload mất | Binary cũ (chưa có live config) → deploy bản mới. |
| `/config` thiếu trường | Bundle UI cũ (cache) → Cmd+Shift+R. |
| Log trống | Binary cũ chưa có buffer log/DB → deploy bản mới. |

Kiểm tra nhanh:
```bash
curl -s localhost:2095/api/health
curl -s "localhost:2095/api/scan/status"
curl -s "localhost:2095/api/logs?level=ERROR&limit=10"
```

---

## 11. Tài liệu liên quan
- `agents/README.md` — tổng quan components
- `agents/daemon/README.md` — đặc tả daemon/kiến trúc
- `agents/RELEASE.md` — quy trình release/deploy/rollback
- `agents/config.example.json` — config mẫu production (dùng `${VAR}`)
