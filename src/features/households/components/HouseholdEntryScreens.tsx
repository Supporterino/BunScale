/*
 * The entry route's screen compositions (HS-2, SYNC-3, DM-3).
 *
 * Tasks 3.3 placed the routes; tasks 4.1, 4.2 and 4.4 own the screens. The two
 * real screens now live in their own modules and are composed here, because the
 * route already imports this file; the remaining slots are still placeholders
 * until 4.4/4.5. The one thing this file carries beyond the decision is
 * `data-household-entry`, the stable name of the entry state the guard reached,
 * which the specs assert instead of copy that will change. The dashboard's is on
 * its `main`, through `AppShell`'s `mainData`.
 *
 * Which surface is which:
 *
 * - `HouseholdOnboardingScreen` — HS-2's first-run household creation (task
 *   4.1), imported directly by the `/onboarding` route.
 * - `HouseholdDashboardScreen` — the P0 dashboard shell (task 4.2): adaptive
 *   navigation, the gear control's SET-1 seam, and the app-event subscription.
 * - `ServiceRequiredPlaceholder` — the explicit connectivity-required state
 *   (task 4.4). The guard reaches it whenever the service or the account cannot
 *   confirm "no household", and it carries the observed status so 4.4 can render
 *   the cause rather than a generic message (NFR-8). It is a slot, not a screen:
 *   task 3.3 must not invent 4.4's surface.
 * - `EntryPendingPlaceholder` / `EntryErrorPlaceholder` — the route's pending and
 *   error surfaces, which `tanstack-router-conventions` requires on a route whose
 *   `beforeLoad` reads data. A failed read (an IPC defect, or a local book that
 *   cannot be read) lands here; it deliberately never lands in onboarding.
 */

import type { ServiceStatus } from '../../sync/entry-status';
import type { HouseholdEntry } from '../entry';
import { HouseholdDashboardScreen } from './HouseholdDashboardScreen';
import { HouseholdOnboardingScreen } from './HouseholdOnboardingScreen';

/**
 * The entry route's screen: the dashboard, or the connectivity slot.
 *
 * `onOpenSettings` is the dashboard shell's SET-1 destination, supplied by
 * `src/routes/index.tsx` as `navigate({ to: '/settings' })`. It is required so
 * the gear can never render as an enabled control with no destination.
 */
export function HouseholdEntryScreen({
  entry,
  onOpenSettings,
}: {
  entry: HouseholdEntry;
  onOpenSettings: () => void;
}) {
  switch (entry.kind) {
    case 'household':
      return <HouseholdDashboardScreen onOpenSettings={onOpenSettings} />;
    case 'service-required':
      return <ServiceRequiredPlaceholder status={entry.status} />;
    case 'onboarding':
      // The route redirects on this entry; rendering it is a defensive
      // fallback rather than a state this screen is designed for.
      return <HouseholdOnboardingScreen />;
  }
}

/*
 * HS-2's first-run surface (task 4.1). Built in its own module so the screen's
 * form, keyboard measurement and copy stay together; the `/onboarding` route
 * imports that module directly (Q43).
 */

/** SYNC-3's connectivity-required slot, until task 4.4 builds it. */
function ServiceRequiredPlaceholder({
  status,
}: {
  status: ServiceStatus | null;
}) {
  const state = status?.state ?? 'unobserved';
  const cause = status?.cause ?? 'none';

  return (
    <main
      data-household-entry="service-required"
      data-service-state={state}
      data-service-cause={cause}
    >
      <h1>Connectivity required</h1>
      <p>
        The explicit connectivity-required surface lands here (task 4.4). The
        guard read the service state as {state}, with cause {cause}.
      </p>
    </main>
  );
}

/** The entry route's pending surface (`beforeLoad` may probe for up to 5 s). */
export function EntryPendingPlaceholder() {
  return (
    <main data-household-entry="pending" aria-busy="true">
      <h1>Loading</h1>
      <p>The household book is being read.</p>
    </main>
  );
}

/** The entry route's error surface; task 3.4 maps causes to copy. */
export function EntryErrorPlaceholder({ error }: { error: unknown }) {
  return (
    <main data-household-entry="error" data-failure={failureName(error)}>
      <h1>Could not load the household</h1>
      <p>The cause-specific failure surface lands here (tasks 3.4 and 4.5).</p>
    </main>
  );
}

/*
 * The failure's cause, for the attribute above — never rendered as copy, because
 * the copy is task 3.4's cause map and NFR-8 forbids a bare message standing in
 * for it. A `CommandError` is a plain object (see `src/lib/request-id.ts`), so
 * this narrows at runtime instead of trusting the router's `Error` typing.
 */
function failureName(error: unknown): string {
  if (typeof error === 'object' && error !== null && 'cause' in error) {
    return typeof error.cause === 'string' ? error.cause : 'unknown';
  }

  return error instanceof Error ? 'ipc-error' : 'unknown';
}
