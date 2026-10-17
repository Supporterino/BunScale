import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { queryClient } from './lib/query-client';
import { router } from './router';

/*
 * Application entry point. `index.html` references exactly this file
 * (`<script type="module" src="/src/main.tsx">`, task 0.14) and declares the
 * mount node, so the two halves are one contract: this module mounts into
 * `#root` and nothing else in `src/` touches the document. A missing node is
 * therefore a hard failure rather than a silently blank window.
 */
const mountNode = document.getElementById('root');

if (mountNode === null) {
  throw new Error(
    'Missing #root mount node: src/main.tsx mounts into the node index.html declares',
  );
}

/*
 * Provider order: the query cache is the outermost piece of app-wide state, so
 * `QueryClientProvider` sits above the router and every route reads it with
 * `useQueryClient()`. The router carries the same instance as its `context`
 * (see `src/router.tsx`), which a route reads with `Route.useRouteContext()`
 * (or the `useRouteContext` hook) — so both paths reach one cache.
 * `RouterProvider` renders the route tree from `src/routeTree.gen.ts`
 * (generated from `src/routes/`, never edited).
 *
 * `StrictMode` is deliberate and development-only. Its double-invoked effects
 * are what catch a Tauri event subscription that forgets to unlisten on
 * cleanup (`frontend-conventions`, "Events"), and the app's subscriptions are
 * effect-held from task 3.6 on; the cost is a duplicated dev render, and the
 * production build is unaffected. `frontend-conventions` states no preference
 * either way, so the choice is recorded here rather than left implicit.
 */
createRoot(mountNode).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>,
);
