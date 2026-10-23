/*
 * One rabbit's dashboard as its own route (task 4.16; DASH-6, WT-6, D1.4).
 *
 * D1.4 puts the two range windows in the URL because they are a query window,
 * not a stored value: `validateSearch: validateWeightRangeSearch` (3.3) makes
 * `trend` and `history` typed, independent search parameters, so a reload or a
 * deep link keeps each view's window and changing one never writes the other.
 * This file is the wiring only — `RabbitDashboard` (4.11) renders the sections
 * from these values and reports one key at a time.
 *
 * **Why this renders the dashboard screen rather than `RabbitDashboard`
 * directly.** P0 has no layout route: every route renders its own `AppShell`
 * (`src/components/ui/AppShell`), and the rabbit dashboard is presented inside
 * the household dashboard's shell — the same adaptive navigation, the same
 * `data-household-entry` region, and the same archived-list surface. Mounting
 * `RabbitDashboard` bare here would drop the shell; mounting the existing screen
 * with the route's `rabbitId` and `search` keeps one shell and one route tree
 * for both form factors, and gives `frontend-logic` the prop swap 4.11
 * anticipated. No shell restructuring is needed.
 *
 * **No loader.** The screen's reads are TanStack Query hooks that present the
 * cached value first and revalidate on mount; a loader would duplicate that and
 * hold the navigation on the service (the same reasoning as `/settings`).
 *
 * **No entry guard.** HS-2's onboarding decision belongs to the entry routes
 * (`/`, `/onboarding`); a deep link to a rabbit is not a path P0 defines, and the
 * screen's own reads answer with their failure surface when the rabbit or the
 * local book cannot be read. Inventing a redirect here would be a navigation
 * rule the product spec does not state.
 */
import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { HouseholdDashboardScreen } from '../features/households/components/HouseholdDashboardScreen';
import { validateWeightRangeSearch } from '../features/weight/search';

export const Route = createFileRoute('/rabbits/$rabbitId')({
  validateSearch: validateWeightRangeSearch,
  component: RabbitDashboardRoute,
});

function RabbitDashboardRoute() {
  const { rabbitId } = Route.useParams();
  const search = Route.useSearch();
  const navigate = useNavigate();

  return (
    <HouseholdDashboardScreen
      onOpenSettings={() => {
        void navigate({ to: '/settings' });
      }}
      rabbitId={rabbitId}
      search={search}
    />
  );
}
