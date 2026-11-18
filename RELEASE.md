# Deploy & Release — WP Offload Agents

Quy trình build → cài đặt → xác minh → rollback cho `wp-offload-daemon` (macOS · Linux · Windows).
Setup chi tiết (plugin, config, vận hành): xem [SETUP.md](SETUP.md).

---

## 1. Thành phần & artifact

| Thành phần | Artifact | Ghi chú |
|---|---|---|
| Daemon (Rust) | `agents/daemon/target/release/wp-offload-daemon[.exe]` | serve API + Web UI, cổng 2095 |
| Web UI (React) | `agents/web-ui/dist/` | daemon serve trực tiếp (`WEB_UI_DIST`) |
| Plugin WP (PHP) | `agents/wp-optimization-agents-plugin/` | cài vào `wp-content/plugins/` |
| Config | `agents/config.json` | portable: `${VAR:-default}` + path tương đối |
| Service | `macos/com.puleeno.wp-offload-daemon.plist`, `linux/wp-offload-daemon.service` | Windows: NSSM/Task Scheduler |

---

## 2. Build

```bash
# macOS / Linux
cd agents/web-ui && npm install && npm run build
cd ../daemon && cargo build --release      # → target/release/wp-offload-daemon
```

```powershell
# Windows (PowerShell)
cd agents\web-ui; npm install; npm run build
cd ..\daemon; cargo build --release        # → target\release\wp-offload-daemon.exe
```

Kiểm tra nhanh trước khi đóng gói:
```bash
./target/release/wp-offload-daemon chrome status     # CLI có mặt
cargo test --lib                                     # unit tests (finish/php)
```

---

## 3. Chuẩn bị trước deploy

1. **Config portable** (không chứa path macOS):
```bash
cp agents/config.example.json agents/config.json
# sửa nếu cần: site.url, database.*, agent.daemon_api_token, imgbb.api_key, blogger.blog_id
```
2. **Giá trị bắt buộc**: `imgbb.api_key` (nên `upload_format="blob"`), `blogger.blog_id` (ID thật), `agent.daemon_api_token` (trùng plugin WP).
3. **Chrome**: cài Google Chrome; đăng nhập Blogger 1 lần bằng CLI (mục 6).
4. **Firewall**: mở cổng 2095 (hoặc cổng `agent.port`) nếu truy cập từ máy khác.
5. **Backup**: dump DB WP + copy `wp-content/uploads` nếu bật `delete_local_after_offload=false`.

---

## 4. Deploy

### 4.1 macOS (launchd)

```bash
# build xong, cài service
cd agents/daemon
sudo ./install.sh install
```
`install.sh` copy binary → `/usr/local/bin/wp-offload-daemon`, cài `/Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist`, load + start.

Nếu repo không nằm ở `/Users/puleeno/Projects/nghesachxua.com`, sửa trong plist: `DAEMON_CONFIG`, `WEB_UI_DIST`, `WorkingDirectory`.

Deploy thủ công (không dùng script):
```bash
sudo cp agents/daemon/target/release/wp-offload-daemon /usr/local/bin/wp-offload-daemon
sudo launchctl bootout system /Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist 2>/dev/null || true
sudo launchctl bootstrap system /Library/LaunchDaemons/com.puleeno.wp-offload-daemon.plist
```

### 4.2 Linux (systemd)

```bash
sudo rsync -av agents/ /opt/wp-opt-agents/
cd /opt/wp-opt-agents/daemon && sudo ./install.sh install
```
Unit: `/etc/systemd/system/wp-offload-daemon.service` (mẫu: `agents/daemon/linux/wp-offload-daemon.service`).
Với `User=www-data`, đặt `chrome.user_data_dir=/var/lib/wp-offload-daemon/chrome` và `chown -R www-data`; giữ `ProtectHome=false`, `PrivateTmp=false` (Chrome cần `/dev/shm` + profile ghi được).

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now wp-offload-daemon
```

### 4.3 Windows

```powershell
# NSSM (khuyến nghị)
nssm install wp-offload-daemon C:\wp-opt-agents\wp-offload-daemon.exe
nssm set wp-offload-daemon AppDirectory C:\wp-opt-agents
nssm set wp-offload-daemon AppEnvironmentExtra DAEMON_CONFIG=C:\wp-opt-agents\config.json WEB_UI_DIST=C:\wp-opt-agents\web-ui\dist RUST_LOG=info
nssm set wp-offload-daemon Start SERVICE_AUTO_START
nssm start wp-offload-daemon
```
Hoặc Task Scheduler: *At startup* → `wp-offload-daemon.exe`, *Start in* `C:\wp-opt-agents`, tick *Run whether user is logged on or not*.

### 4.4 Cập nhật (upgrade) không downtime lớn

```bash
# macOS
sudo cp agents/daemon/target/release/wp-offload-daemon /usr/local/bin/wp-offload-daemon
sudo launchctl kickstart -k system/com.puleeno.wp-offload-daemon

# Linux
sudo cp agents/daemon/target/release/wp-offload-daemon /usr/local/bin/wp-offload-daemon
sudo systemctl restart wp-offload-daemon

# Windows
nssm stop wp-offload-daemon
copy /Y agents\daemon\target\release\wp-offload-daemon.exe C:\wp-opt-agents\
nssm start wp-offload-daemon
```
UI: deploy `agents/web-ui/dist` (rsync/xcopy) rồi hard-refresh — daemon serve từ đĩa nên bundle mới có hiệu lực ngay.

---

## 5. Xác minh sau deploy

| Kiểm tra | Lệnh | Kỳ vọng |
|---|---|---|
| Service | `launchctl list \| grep wp-offload` · `systemctl status wp-offload-daemon` | `running`/`active` |
| Health | `curl -s localhost:2095/api/health` | 200 |
| Metrics | `curl -s localhost:2095/api/metrics` | `uptimeSeconds`, `memoryMb`, `cpuPercent`, `tokioWorkers` |
| Config | `curl -s localhost:2095/api/config` | có `_meta.path`, `cdn.host`, `imgbb`, `agent.scan_*` |
| Chrome | `curl -s localhost:2095/api/chrome/preflight` | `chrome_exists`, `profile_exists`, `port_open` + advice |
| Pipeline | `curl -s localhost:2095/api/scan/status` | `imgbb_ready`, `blogger_ready`, `queue`, `pending` |
| Logs | `curl -s "localhost:2095/api/logs?limit=5"` | có dòng thật |
| UI | mở `http://<host>:2095/` | `/overview`, `/crawl/wp-media`, `/daemon`, `/config` |

Smoke test pipeline:
```bash
# 1 media (dùng attachment có file local)
curl -s -X POST localhost:2095/api/attachments/<id>/process
# quét batch ngay
curl -s -X POST localhost:2095/api/scan/run -d '{"batch":5}' -H 'Content-Type: application/json'
# theo dõi
curl -s "localhost:2095/api/logs?q=scan&limit=10"
```

---

## 6. Đăng nhập Blogger (bắt buộc trước khi chạy job)

```bash
# mở Chrome profile riêng + vào blogger.com
wp-offload-daemon chrome setup
# chờ tới khi đăng nhập thành công và ghi chrome.user_data_dir vào config
wp-offload-daemon chrome login --save --config /path/to/config.json
wp-offload-daemon chrome status      # Blogger login: OK
```
Nếu dùng service (Linux `www-data`), chạy CLI bằng chính user đó để profile đúng.

---

## 7. Rollback

```bash
# macOS: giữ binary cũ trước khi update
sudo cp /usr/local/bin/wp-offload-daemon /usr/local/bin/wp-offload-daemon.bak
# rollback
sudo cp /usr/local/bin/wp-offload-daemon.bak /usr/local/bin/wp-offload-daemon
sudo launchctl kickstart -k system/com.puleeno.wp-offload-daemon
```
- Plugin WP: rollback bằng cách restore thư mục plugin + `wp-content/uploads` (nếu đã xoá file) + DB dump.
- Config: giữ bản `.bak` khi deploy (`cp agents/config.json agents/config.json.bak`).
- Dữ liệu pipeline không tự xoá khi rollback binary: `cdn_map` / `url_versions` / `blogger_queue` giữ nguyên, bản mới vẫn đọc được.

---

## 8. Troubleshooting deploy

| Triệu chứng | Nguyên nhân | Xử lý |
|---|---|---|
| `Address already in use (2095)` | service cũ chưa tắt | `launchctl kickstart -k ...` / `systemctl restart` |
| UI cũ, thiếu field | bundle cache | hard-refresh; kiểm tra `WEB_UI_DIST` trỏ đúng `dist` |
| `/api/logs` 404/500 | binary cũ | deploy binary mới |
| `/api/attachments` 500 `ONLY_FULL_GROUP_BY` | binary cũ | deploy binary mới |
| imgbb 400 `Can't download remote image` | `upload_format=url` với WP local | đặt `blob`, restart daemon |
| Blogger `DevTools not ready` | Chrome chưa mở port debug | mục 6 (chrome setup/login) |
| Job `processing` treo | daemon crash giữa job | tự reset sau 600s; hoặc set `status='queued'` |
| Service không đọc được config | sai quyền/`ProtectHome` | `chown` đúng user, `ProtectHome=false`, kiểm tra `DAEMON_CONFIG` |

---

## 9. Checklist release

- [ ] `npm run build` (web-ui) + `cargo build --release` (daemon) OK
- [ ] `cargo test --lib` pass
- [ ] `agents/config.json` portable (`${VAR}`, không path cứng), `imgbb.upload_format=blob`
- [ ] `blogger.blog_id` là ID thật; `agent.daemon_api_token` trùng plugin
- [ ] `wp-optimization-agents-plugin` đã `composer install`, settings đúng
- [ ] Service lên (`running`/`active`), `GET /api/health` = 200
- [ ] `GET /api/chrome/preflight`: port 9222 open, Blogger login OK
- [ ] `POST /api/scan/run` xử lý 1 attachment: imgbb → CDN → Blogger → finish (cdn_map có `target_provider=blogger`)
- [ ] `GET /api/logs` có log thật từng bước (scan/imgbb_upload/cdn_map/blogger_job/finish)
