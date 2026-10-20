/*
 * First-run household creation: the route HS-2 presents when the person holds
 * no household (task 3.3 places the route; task 4.1 builds the screen).
 *
 * The guard is the second half of HS-2's rule — "once at least one household
 * exists, the onboarding flow SHALL NOT be presented again". Reaching this
 * route is not proof that onboarding is due: a deep link, a back navigation or
 * a create that already landed on another device would all arrive here with a
 * household in the book. So the route re-runs the same entry decision and
 * redirects away unless onboarding is still the correct answer. The decision is
 * shared with `/`, which is what keeps the two from ever disagreeing.
 *
 * The connectivity-required case redirects to `/` as well: there the slot for
 * task 4.4 renders, which is the honest surface for "cannot tell yet" — never
 * this screen.
 */
import { createFileRoute, redirect } from '@tanstack/react-router';
import {
  EntryErrorPlaceholder,
  EntryPendingPlaceholder,
  OnboardingPlaceholder,
} from '../features/households/components/HouseholdEntryScreens';
import { resolveHouseholdEntry } from '../features/households/entry';

export const Route = createFileRoute('/onboarding')({
  beforeLoad: async ({ context }) => {
    const householdEntry = await resolveHouseholdEntry(context.queryClient);

    if (householdEntry.kind !== 'onboarding') {
      throw redirect({ to: '/' });
    }
  },
  component: OnboardingPlaceholder,
  pendingComponent: EntryPendingPlaceholder,
  errorComponent: EntryErrorPlaceholder,
});
