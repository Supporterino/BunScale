import type { QueryClient } from '@tanstack/react-query';
import { createRootRouteWithContext, Outlet } from '@tanstack/react-router';

/*
 * The root of the file-based route tree: every route under `src/routes/` is a
 * child of this one, and it is the only route that declares the router context
 * (`createRootRouteWithContext`, per `tanstack-router-conventions`). The context
 * type is what `src/router.tsx` must satisfy when it calls `createRouter`, so a
 * route reading `context.queryClient` cannot be written before the cache exists.
 *
 * `NotFoundScreen` is the root-level not-found surface that
 * `tanstack-router-conventions` requires. It is deliberately unstyled, semantic
 * markup only: it is a scaffolding placeholder, and `frontend-ui` owns its
 * presentation (the design tokens are task 0.20, the primitives 0.21). No
 * `errorComponent` or `pendingComponent` is declared yet because no route has a
 * `loader` — the conventions attach those two to routes that do.
 */
export const Route = createRootRouteWithContext<{ queryClient: QueryClient }>()(
  {
    component: RootLayout,
    notFoundComponent: NotFoundScreen,
  },
);

/*
 * The root layout is a pass-through: it exists so route layout (adaptive
 * navigation, fixed bars, safe areas) has one place to live, and
 * `frontend-conventions` puts all of that in the UI layer rather than here.
 */
function RootLayout() {
  return <Outlet />;
}

function NotFoundScreen() {
  return (
    <main>
      <h1>Not found</h1>
      <p>This screen does not exist.</p>
    </main>
  );
}
