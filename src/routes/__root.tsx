import type { QueryClient } from '@tanstack/react-query';
import { createRootRouteWithContext, Outlet } from '@tanstack/react-router';
import { NotFoundScreen } from '../features/design-preview/components/NotFoundScreen';
import { SaveFailureSurface } from '../features/sync/components/SaveFailureSurface';

/*
 * The root of the file-based route tree: every route under `src/routes/` is a
 * child of this one, and it is the only route that declares the router context
 * (`createRootRouteWithContext`, per `tanstack-router-conventions`). The context
 * type is what `src/router.tsx` must satisfy when it calls `createRouter`, so a
 * route reading `context.queryClient` cannot be written before the cache exists.
 *
 * `NotFoundScreen` is the root-level not-found surface that
 * `tanstack-router-conventions` requires; it lives in the design-preview
 * feature. No `errorComponent` or `pendingComponent` is declared yet because no
 * route has a `loader` — the conventions attach those two to routes that do.
 */
export const Route = createRootRouteWithContext<{ queryClient: QueryClient }>()(
  {
    component: RootLayout,
    notFoundComponent: NotFoundScreen,
  },
);

/*
 * The root layout is a pass-through with one shell-level surface: the
 * store-backed save failure (task 7.7). It is mounted **after** `<Outlet />`
 * because the surface is fixed to the bottom edge: document order is Tab
 * order, so the route's heading and field come first and the notice's action
 * follows them, matching the visual order (design-system convention 9). It is
 * still mounted once for every route, so a save that failed on the screen that
 * started it is presented after a navigation. A single slot means the surface
 * shows exactly one notice; the styling hook is `bs-save-failure` (placement
 * and safe-area treatment belong to `frontend-ui`). All other layout (adaptive
 * navigation, fixed bars, safe areas) lives in the UI layer, not here.
 */
function RootLayout() {
  return (
    <>
      <Outlet />
      <SaveFailureSurface />
    </>
  );
}
