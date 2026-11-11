# wp-offload-agents

WordPress media offload agents (Rust daemon + Python orchestration + web dashboard).

## Components
- `daemon/` — Rust daemon/service (`wp-offload-daemon`): core logic + HTTP server on port **2095** serving API and the web UI bundle.
- `web-ui/` — React/Vite dashboard frontend (SPA). Built to `web-ui/dist/` and served by the Rust daemon.
- `python/` — Python orchestration/scripts (API clients, workflows, monitoring). See `python/README.md`.

## Documents
- [SETUP.md](SETUP.md) — environment setup, build, run, configuration.
- [RELEASE.md](RELEASE.md) — release checklist, versioning, build artifacts, deployment/rollback.

## Quick start
```bash
# 1) Build frontend bundle
cd agents/web-ui && npm install && npm run build

# 2) Run daemon (serves UI + API on :2095)
cd ../daemon
DAEMON_CONFIG=../config.example.json cargo run
# open http://localhost:2095
```
