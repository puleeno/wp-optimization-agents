# Agents Release Guide

## Scope
Release of `agents` components: Rust daemon (`wp-offload-daemon`), web-ui bundle (`web-ui/dist`), and Python scripts.

## Versioning
- Bump versions coherently when releasing:
  - `agents/daemon/Cargo.toml` (`package.version`)
  - `agents/web-ui/package.json` (`version`)
  - `agents/python` package/version markers if/when added
- Tag releases in git, e.g. `agents-vX.Y.Z` (or repo convention). Keep changelog/notes with tag or PR.

## Pre-release checklist
- [ ] Code reviewed/merged; `agents/` builds clean on target platforms.
- [ ] `web-ui`: `npm ci` (or `npm install`), `npm run lint` (tsc), `npm run build` succeeds; smoke-test UI.
- [ ] `daemon`: `cargo fmt --check` (if enforced), `cargo clippy` (if enforced), `cargo test`, `cargo build --release` succeeds.
- [ ] `config.example.json` matches `daemon/src/config.rs` schema; no secrets committed.
- [ ] `WEB_UI_DIST`/dist packaging verified: daemon serves `dist/index.html` and assets correctly on `:2095`.
- [ ] API smoke: `GET /health`, `/api/metrics` return 200; SPA routes fallback to `index.html`.
- [ ] Docs updated (`README.md`, `SETUP.md`, this file) for any behavior/env changes.

## Build release artifacts

### 1. Web UI bundle
```bash
cd agents/web-ui
npm ci
npm run lint
npm run build
# artifact: agents/web-ui/dist/
```

### 2. Daemon binary
```bash
cd agents/daemon
cargo build --release
# artifact: agents/daemon/target/release/wp-offload-daemon (platform-specific)
```

Cross-compilation (if needed): use appropriate Rust targets/toolchains per platform (e.g. `--target x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, etc.).

### 3. Python (if distributed)
Package per `python/` project conventions (wheel/sdist) when defined; include pinned deps/lock if applicable.

## Packaging & deployment
A typical deploy unit:
- `wp-offload-daemon` binary
- `web-ui/dist/` directory (same host, path known to daemon)
- `config.json` (environment-specific, not in artifact repo)

Run:
```bash
export DAEMON_HOST=0.0.0.0
export DAEMON_PORT=2095
export DAEMON_CONFIG=/etc/wp-offload/config.json
export WEB_UI_DIST=/opt/wp-offload/web-ui/dist
/opt/wp-offload/bin/wp-offload-daemon
```

Service managers:
- **systemd (Linux)**: `ExecStart` with env/config paths; `Restart=on-failure`; run as non-root user with access to dist/config.
- **launchd (macOS)** / **Windows Service**: equivalent service wrappers pointing to the binary and env.
- **Docker**: multi-stage build (build web-ui + daemon) copying `dist/` and binary into runtime image; expose 2095.

Reverse proxy (optional): terminate TLS in front and proxy to `127.0.0.1:2095`; preserve WebSocket upgrade if/when WS endpoints are added.

## Post-release verification
- `curl -f http://<host>:2095/health`
- Open dashboard, verify key tabs render and static assets load (no 404s).
- Confirm logs (`RUST_LOG=info`) show bind address and dist path.
- If config-driven features enabled, run a minimal end-to-end offload/audit flow in staging.

## Rollback
- Keep previous binary + matching `dist/` bundle.
- Redeploy prior version and restart service.
- Revert config only if schema changed incompatibly; prefer backward-compatible config evolution.

## Security notes
- Do not ship real credentials in images/artifacts; inject `DAEMON_CONFIG`/secrets via env/secret store.
- Restrict network exposure of `:2095` as appropriate; place behind auth/TLS proxy for non-local deployments.
- Review CORS and any future write endpoints before public exposure.
