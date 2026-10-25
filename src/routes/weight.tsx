/*
 * The weight-form route D1.10 requires (task 4.18; WT-2, DASH-9, D1.10).
 *
 * Both quick paths open this one route with the rabbit bound through typed
 * search parameters — `rabbit`, the shared `mode` and the `from` origin — and
 * the screen behind it is 4.9's form. The route file is the thin half:
 * `WeightFormScreen` lives in
 * `src/features/weight/components/WeightFormScreen.tsx` and its validator in
 * `src/features/weight/search.ts`, beside the range pair, because a route holds
 * the wiring while the feature owns the reads and the surface.
 *
 * No loader and no guard, matching the rabbit route's reasoning: the screen's
 * reads are TanStack Query hooks that present cached data first and revalidate
 * on mount, and their failures are the screen's own read surfaces. A deep link
 * with an unknown rabbit is answered by the profile read's `validation`
 * refusal rather than by a redirect the product spec does not state.
 *
 * It is not under an adaptive layout fork: `WeightFormScreen` renders the same
 * `AppShell` every other surface uses, so iOS and macOS share this node.
 */
import { createFileRoute } from '@tanstack/react-router';
import { WeightFormScreen } from '../features/weight/components/WeightFormScreen';
import { validateWeightFormSearch } from '../features/weight/search';

export const Route = createFileRoute('/weight')({
  validateSearch: validateWeightFormSearch,
  component: WeightFormRoute,
});

function WeightFormRoute() {
  return <WeightFormScreen search={Route.useSearch()} />;
}
