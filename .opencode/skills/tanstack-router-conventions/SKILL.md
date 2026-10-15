---
name: tanstack-router-conventions
description: TanStack Router (React, file-based, Vite) conventions for a Tauri app: history, typed search params, loaders, guards, context. Load for any routing work.
---
# TanStack Router conventions

## Setup

- Vite + `@tanstack/router-plugin/vite` (file-based routing). **No TanStack Start / SSR.**
- **Hash history** (`createHashHistory`) or SPA fallback: Tauri serves static assets, so browser history breaks on reload/deep link.
- `routeTree.gen.ts` is generated and committed; never hand-edit.
- `createRootRouteWithContext<{ queryClient: QueryClient }>()`; pass context in `createRouter`.

## Routes

- `src/routes/` mirrors URL structure. Layout routes `_layout.tsx`; pathless groups `(group)`.
- Route files are thin: `createFileRoute` + `component` imported from a feature.
- Use `Link`, `useNavigate`, `redirect` with typed `to`/`params`/`search`. No string-built URLs.

## Data

- Loader prefetches via `queryClient.ensureQueryData(featureQueryOptions(...))`; component reads with `useSuspenseQuery`.
- Loader dependencies declared with `loaderDeps` for search-driven data.
- `pendingComponent` and `errorComponent` on routes with loaders; `notFoundComponent` at root.

## Search params

- `validateSearch` with Zod (with defaults). URL is the state for filters, tabs, selection that should survive reload.
- Use `search` updater functions, not manual merges.

## Guards

- `beforeLoad` for redirects (e.g., onboarding incomplete). Throw `redirect`, don't navigate in effects.

## Adaptive layout

- One route tree for iOS and macOS. A layout route (`_app`) renders bottom tabs on compact and sidebar on expanded; routes do not fork per form factor.
- No reliance on native swipe-back or browser back: every pushed screen has an in-UI back control using `router.history.back()` or an explicit parent `to`.
- Page transitions: `transform`/`opacity` only; respect reduced motion.
- Desktop shortcuts (e.g. ⌘[ back) registered only on `pointer: fine`.

## Splitting

- Automatic code splitting via plugin. Keep heavy components lazy.

## Tests

- Render with `createMemoryHistory` and a test router; mock IPC with `mockIPC`.
