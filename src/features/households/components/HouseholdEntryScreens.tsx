/*
 * Placeholder surfaces for the routes task 3.3 adds (HS-2, SYNC-3, DM-3).
 *
 * These exist so the entry guard has something real to render and so each P0
 * screen can replace one in place: `frontend-ui` owns their design, copy and
 * layout (tasks 4.1, 4.2 and 4.4), and this file deliberately carries none of
 * the three — bare semantic elements, no class names, no primitives. The one
 * thing it does carry beyond the decision is `data-household-entry`, a stable
 * attribute naming which state the guard reached, so a test can assert the
 * decision without depending on placeholder copy that will change.
 *
 * Which surface is which:
 *
 * - `HouseholdOnboardingScreen` — HS-2's first-run household creation (task
 *   4.1). It lives in its own module and is re-exported below as
 *   `OnboardingPlaceholder`, the name the route imports today, so this task did
 *   not have to change a route file it does not own.
 * - `DashboardPlaceholder` — the active household's dashboard (task 4.2). It
 *   reads the active-household query, which is what proves the entry context and
 *   the cache agree.
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
import { useActiveHousehold } from '../queries';
import { HouseholdOnboardingScreen } from './HouseholdOnboardingScreen';

/** The entry route's screen: the dashboard, or the connectivity slot. */
export function HouseholdEntryScreen({ entry }: { entry: HouseholdEntry }) {
  switch (entry.kind) {
    case 'household':
      return <DashboardPlaceholder />;
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
 * form, keyboard measurement and copy stay together; re-exported under the name
 * the route already imports, which keeps this task out of a route file
 * `frontend-ui` does not own. The route should import
 * `HouseholdOnboardingScreen` directly the next time its owner touches it.
 */
export {
  HouseholdOnboardingScreen,
  HouseholdOnboardingScreen as OnboardingPlaceholder,
} from './HouseholdOnboardingScreen';

/** The active household's dashboard, until task 4.2 builds the shell. */
function DashboardPlaceholder() {
  const active = useActiveHousehold();
  const name = active.data?.name ?? 'no active household';

  return (
    <main data-household-entry="household">
      <h1>Household dashboard</h1>
      <p>Active household: {name}.</p>
      <p>The dashboard shell lands here (task 4.2).</p>
    </main>
  );
}

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
