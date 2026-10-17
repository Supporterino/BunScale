import { createFileRoute } from '@tanstack/react-router';

/*
 * The index route of the scaffold: one route inside the root route, so
 * `src/routes/` is a real file-based tree and the router plugin has something to
 * render into `src/routeTree.gen.ts` (TOOL-4). It exists to prove the tree is
 * wired end to end — `index.html` → `src/main.tsx` → `src/router.tsx` → this
 * component — not to present a screen.
 *
 * The markup is deliberately bare and unstyled: design tokens are task 0.20 and
 * the design-system primitives plus the real placeholder screen are task 0.21,
 * which replaces this component. The copy here is scaffolding text, not a P0
 * string, so task 3.7's i18n scaffold has nothing to wrap at this point. Once
 * the first real screen lands, this file keeps only `createFileRoute` and the
 * `component` reference and imports the component from a feature, per
 * `tanstack-router-conventions` ("Route files are thin").
 */
export const Route = createFileRoute('/')({
  component: IndexRoute,
});

function IndexRoute() {
  return (
    <main>
      <h1>BunScale</h1>
      <p>Scaffold route. The first screen arrives with the P0 UI tasks.</p>
    </main>
  );
}
