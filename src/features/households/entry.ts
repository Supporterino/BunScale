/*
 * The household entry decision (task 3.3; HS-2, HS-3, DM-3, SYNC-3).
 *
 * HS-2 asks that first-run onboarding be presented when the person has no
 * household and never again once one exists. The trap the contract names is
 * that **"no household" is a conclusion, not an absence of data**: an empty
 * `listHouseholds()` and a `null` active household are also exactly what an
 * unreachable service with no local copy looks like (SYNC-3). So the decision
 * has two halves and this module keeps them apart:
 *
 * - a **household exists** — the device's household book says so (D0.2,
 *   persisted across launches). That answer needs no network, so it is the
 *   dashboard regardless of the service's state: previously loaded data is
 *   shown while unreachable.
 * - **no household** — boarding is only allowed on a fresh reachable-and-
 *   identified status. `unknown` (nothing observed yet), `unreachable`, and
 *   `reachable` with an unusable account all mean "cannot tell", and none of
 *   them may start onboarding; they resolve to `service-required`, which task
 *   4.4 turns into the explicit connectivity-required surface.
 *
 * Status freshness comes from `src/features/sync/entry-status.ts`, which task
 * 3.4 replaces with its store; the decision below is pure, so the rule is
 * testable without a service, a query cache or a router.
 */
import type { QueryClient } from '@tanstack/react-query';
import type { ServiceStatus } from '../sync/entry-status';
import { readEntryServiceStatus } from '../sync/entry-status';
import type { Household, HouseholdSummary } from './queries';
import {
  activeHouseholdQueryOptions,
  householdListQueryOptions,
} from './queries';

/** What the device's household book says (D0.2, HS-4). */
export type HouseholdPresence = {
  /** Every household the person holds, in the order they were acquired. */
  households: readonly HouseholdSummary[];
  /** The active one, or `null` when none is. */
  activeHousehold: Household | null;
};

/**
 * The entry decision for one launch or navigation.
 *
 * `service-required` carries the status the decision was made on so the surface
 * can name the cause (4.4); `null` means no status was consulted at all — which
 * only happens when a caller skips the read, and can never produce onboarding.
 */
export type HouseholdEntry =
  | { kind: 'household' }
  | { kind: 'onboarding' }
  | { kind: 'service-required'; status: ServiceStatus | null };

/** Whether the book holds at least one household. */
export function hasHousehold(presence: HouseholdPresence): boolean {
  return presence.households.length > 0 || presence.activeHousehold !== null;
}

/*
 * A status may only decide onboarding when it is a *current* observation of a
 * reachable service with a usable account (R3, D0.7 criterion 4). `observed_at`
 * is checked explicitly even though `state: reachable` already implies an
 * observation, so a future change to `ServiceStatus` cannot silently let an
 * unobserved value onboard someone.
 */
function isFreshReachableAccount(status: ServiceStatus): boolean {
  return (
    status.observed_at !== null &&
    status.state === 'reachable' &&
    status.account_valid === true
  );
}

/**
 * Decide where the app starts, from the book and a status.
 *
 * Pure and total: every combination produces exactly one of the three entries.
 * `status` is `ServiceStatus | null` because the household check comes first —
 * when a household exists no status is read at all — and because "no status"
 * must be an explicitly safe value rather than an accidental `reachable`.
 */
export function householdEntryDecision(
  presence: HouseholdPresence,
  status: ServiceStatus | null,
): HouseholdEntry {
  if (hasHousehold(presence)) {
    return { kind: 'household' };
  }

  if (status !== null && isFreshReachableAccount(status)) {
    return { kind: 'onboarding' };
  }

  return { kind: 'service-required', status };
}

/**
 * Read the book and, only when it holds no household, the service status, then
 * decide (HS-2, SYNC-3).
 *
 * The order is load-bearing. The two household reads are local (D0.2) and the
 * status read may probe the network for up to `REACHABILITY_PROBE_TIMEOUT`, so
 * a person who already holds a household reaches their dashboard without
 * waiting on the service: no status is read, and `householdEntryDecision`
 * cannot consult one.
 *
 * **The book is read fresh, not cached.** `fetchQuery` rather than
 * `ensureQueryData`: the latter returns whatever the cache holds while it is
 * considered fresh — it does not revalidate unless asked — so a book that was
 * empty at launch would stay empty in the decision's hands even after a
 * household was created (4.1) and the list invalidated (3.6). That is exactly
 * HS-2's failure: onboarding would be presented again although a household
 * exists. `staleTime: 0` states the requirement rather than inheriting it from
 * the client's defaults, so a future `staleTime` there cannot make this
 * decision stale.
 *
 * A failed household read is **not** turned into "no household": it rejects,
 * and the route's error surface shows it. That is the whole point of the rule
 * this task implements — absent data must never present onboarding.
 *
 * The decision is therefore made per navigation on a read taken at that moment.
 * A household can only leave the held set in P3, and until then the screens'
 * reads are kept fresh by invalidation (task 3.6) rather than by re-deciding in
 * place.
 */
export async function resolveHouseholdEntry(
  queryClient: QueryClient,
): Promise<HouseholdEntry> {
  const [households, activeHousehold] = await Promise.all([
    queryClient.fetchQuery({
      ...householdListQueryOptions(),
      staleTime: 0,
    }),
    queryClient.fetchQuery({
      ...activeHouseholdQueryOptions(),
      staleTime: 0,
    }),
  ]);
  const presence: HouseholdPresence = { households, activeHousehold };
  const status = hasHousehold(presence)
    ? null
    : await readEntryServiceStatus(queryClient);

  return householdEntryDecision(presence, status);
}
