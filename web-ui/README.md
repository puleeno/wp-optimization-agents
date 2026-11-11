# WP Optimization Agents - Web UI

Dashboard UI for monitoring WordPress-to-Blogger media offload, daemon status, performance audits, and configuration.

Based on the reference implementation in `/Users/puleeno/Downloads/wp-optimization-agents---blogger-offload-&-daily-performance-dashboard`.

## Stack
- React 19 + TypeScript
- Vite
- Tailwind CSS v4
- lucide-react

## Run
```bash
cd agents/web-ui
npm install
npm run dev
```

Dev server: `http://localhost:2095` (configured in `vite.config.ts` and scripts).

## Build
```bash
npm run build
npm run preview
```

## Structure
- `src/App.tsx` - app shell, state, handlers
- `src/components/` - dashboard tabs and layout
- `src/data/mockData.ts` - sample/mock data
- `src/types/dashboard.ts` - shared types
- `src/utils/bloggerUrl.ts` - Blogger URL transform helpers
- `index.html`, `src/main.tsx`, `src/index.css` - entry and styles

## Notes
- Current implementation uses local/mock state for metrics, logs, posts, audits, and config.
- Integration with Rust daemon/Python orchestrator APIs can replace mock handlers/data when endpoints are available.
- `.env.example` documents intended backend/monitoring settings.
