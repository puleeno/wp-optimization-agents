# WP Optimization Agents - Web UI

Dashboard frontend (SPA) for monitoring WordPress-to-Blogger media offload, daemon status, performance audits, and configuration.

Based on the reference implementation in `/Users/puleeno/Downloads/wp-optimization-agents---blogger-offload-&-daily-performance-dashboard`.

## Integration
- **Rust daemon owns port `2095`** and serves this UI as static assets.
- Build output: `dist/` (bundle) — Rust should serve `dist/index.html` and bundled assets from there.
- Vite dev/preview ports are not bound to `2095`.

## Stack
- React 19 + TypeScript
- Vite
- Tailwind CSS v4
- lucide-react

## Develop
```bash
cd agents/web-ui
npm install
npm run dev
```
Use the Vite dev server URL shown in terminal for local UI development.

## Build (for Rust)
```bash
npm run build
```
Produces `dist/` for embedding/serving by the Rust daemon.

## Structure
- `src/App.tsx` - app shell, state, handlers
- `src/components/` - dashboard tabs and layout
- `src/data/mockData.ts` - sample/mock data
- `src/types/dashboard.ts` - shared types
- `src/utils/bloggerUrl.ts` - Blogger URL transform helpers
- `index.html`, `src/main.tsx`, `src/index.css` - entry and styles

## Notes
- Current implementation uses local/mock state for metrics, logs, posts, audits, and config.
- When Rust APIs are available, replace mock handlers/data with real endpoints (served on `:2095` by daemon).
