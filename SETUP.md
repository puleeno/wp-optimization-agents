# Agents Setup Guide

## Prerequisites
- **Rust** (stable, via rustup) + Cargo — `daemon/`
- **Node.js** 20+ and npm — `web-ui/`
- **Python** 3.11+ (optional, for `python/` orchestration)
- Access/credentials for WordPress REST API and Blogger/Google as required by your workflow

## Repository layout
```
agents/
  daemon/           # Rust daemon binary + library (serves UI/API on :2095)
  web-ui/           # React/Vite SPA frontend (build -> dist/)
  python/           # Python scripts/orchestration
  config.example.json
  SETUP.md
  RELEASE.md
```

## 1. Frontend (web-ui)
The UI is a static SPA bundle served by the Rust daemon.

```bash
cd agents/web-ui
npm install

# local dev (Vite dev server, default port shown by Vite; NOT 2095)
npm run dev

# production bundle for daemon
npm run build        # outputs dist/
npm run lint         # typecheck
```

Notes:
- Port **2095 is reserved for the Rust daemon**. Do not bind Vite to 2095.
- Rust serves `web-ui/dist/index.html` with SPA fallback for client routes.

## 2. Rust daemon
The daemon listens on `0.0.0.0:2095` by default and serves the web-ui bundle plus API endpoints.

### Build & run (development)
```bash
cd agents/daemon
cargo check
cargo run
# with config:
DAEMON_CONFIG=../config.example.json cargo run
```

### Release build
```bash
cd agents/daemon
cargo build --release
# binary: agents/daemon/target/release/wp-offload-daemon
```

### Environment variables
| Variable | Default | Description |
|---|---|---|
| `DAEMON_HOST` | `0.0.0.0` | Bind host |
| `DAEMON_PORT` / `PORT` | `2095` | HTTP port (UI + API) |
| `DAEMON_CONFIG` | *(unset)* | Path to JSON config file. If unset, daemon starts with placeholder config and warns (UI/API health available; full operations require real config). |
| `WEB_UI_DIST` | auto-detect | Path to built frontend `dist/` directory |
| `RUST_LOG` | `info` | Log level/filter (tracing), e.g. `debug`, `wp_offload_ffi=debug` |

`WEB_UI_DIST` resolution when unset: `<cwd>/web-ui/dist` or `<cwd>/../web-ui/dist`.

### HTTP endpoints
- `GET /` — dashboard SPA (from `dist/`, SPA fallback)
- `GET /health`, `GET /api/health` — health/status JSON
- `GET /api/metrics`, `GET /api/status` — basic metrics/status JSON

## 3. Configuration
Copy and edit the example:
```bash
cp agents/config.example.json agents/config.json
# edit config.json with real credentials/IDs
export DAEMON_CONFIG=/absolute/path/to/agents/config.json
```

Config schema (`config.rs`): `wordpress` (api_url, username, app_password, site_url), `blogger` (blog_id, token, base_url), `chrome` (chrome_path, user_data_dir, headless), `media` (max_width, max_height, quality, upload_timeout), `logging` (level, file, max_size_mb, max_files). Required string fields are validated on load.

> Keep `config.json` and secrets out of git.

## 4. Python (optional)
```bash
cd agents/python
python -m venv .venv && source .venv/bin/activate
# install deps per python/README.md or requirements when available
```
Python layer orchestrates workflows/clients against the daemon/APIs; see `python/README.md` for module details.

## 5. Verify
```bash
# after build+run
curl -s http://localhost:2095/health
curl -s http://localhost:2095/api/metrics
# open http://localhost:2095 in browser for dashboard
```

## Troubleshooting
- **UI 404 / blank**: ensure `agents/web-ui/dist` exists (`npm run build`) or set `WEB_UI_DIST`.
- **Port in use**: change `DAEMON_PORT` or free `2095` (2095 must be owned by daemon in integrated mode).
- **Config errors**: check `DAEMON_CONFIG` path and required fields; run with `RUST_LOG=debug`.
- **CORS**: daemon enables permissive CORS for API/UI; adjust if tightening for production.
