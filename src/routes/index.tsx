/*
 * The app's entry route: HS-2's onboarding guard (task 3.3).
 *
 * This is the one place the "no household is a conclusion, not an absence of
 * data" rule becomes navigation (design.md, "IPC contract"; SYNC-3). The
 * decision itself is `resolveHouseholdEntry` (`src/features/households/entry.ts`),
 * kept out of this file so the rule is testable without a router; the route does
 * three things with it:
 *
 * 1. **onboarding** — the book holds no household *and* a fresh status is
 *    reachable with a usable account — so redirect to `/onboarding`. This is the
 *    only path to first-run onboarding (HS-2).
 * 2. **household** — the book holds one — so the active household's dashboard is
 *    presented and onboarding is skipped, online or not (HS-2, SYNC-3).
 * 3. **service-required** — the book holds none and the service/account cannot
 *    confirm it — so onboarding is *not* started and the connectivity-required
 *    slot is rendered instead (task 4.4 owns that surface).
 *
 * The redirect is thrown from `beforeLoad` rather than navigated from an effect,
 * as `tanstack-router-conventions` requires. Both components below are
 * placeholders owned by `frontend-ui`; this file only stays the thin glue
 * between the decision and them.
 */
import { createFileRoute, redirect } from '@tanstack/react-router';
import {
  EntryErrorPlaceholder,
  EntryPendingPlaceholder,
  HouseholdEntryScreen,
} from '../features/households/components/HouseholdEntryScreens';
import { resolveHouseholdEntry } from '../features/households/entry';

export const Route = createFileRoute('/')({
  beforeLoad: async ({ context }) => {
    const householdEntry = await resolveHouseholdEntry(context.queryClient);

    if (householdEntry.kind === 'onboarding') {
      throw redirect({ to: '/onboarding' });
    }

    return { householdEntry };
  },
  component: HouseholdEntryRoute,
  pendingComponent: EntryPendingPlaceholder,
  errorComponent: EntryErrorPlaceholder,
});

function HouseholdEntryRoute() {
  const { householdEntry } = Route.useRouteContext();

  return <HouseholdEntryScreen entry={householdEntry} />;
}
