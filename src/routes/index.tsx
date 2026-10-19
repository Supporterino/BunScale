import { createFileRoute } from '@tanstack/react-router';
import { DesignSystemPreview } from '../features/design-preview/components/DesignSystemPreview';

/*
 * The index route of the scaffold: one route inside the root route, so
 * `src/routes/` is a real file-based tree and the router plugin has something to
 * render into `src/routeTree.gen.ts` (TOOL-4). It exists to prove the tree is
 * wired end to end — `index.html` → `src/main.tsx` → `src/router.tsx` → this
 * route — not to present a screen.
 *
 * The placeholder screen now lives in
 * `src/features/design-preview/components/`, and this file keeps only
 * `createFileRoute` and the `component` reference, per
 * `tanstack-router-conventions` ("Route files are thin"). It renders the
 * design-system preview that task 0.21 delivers and task 0.25 will assert at
 * both the mobile and desktop baselines.
 */
export const Route = createFileRoute('/')({
  component: DesignSystemPreview,
});
