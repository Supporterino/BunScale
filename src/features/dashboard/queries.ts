/*
 * The dashboard read (task 3.2; DASH-1, DASH-6).
 *
 * One household read: the active household's non-archived rabbits with the
 * last-weighed instant or the never-weighed indication and the derived drop
 * flag, in the backend's sort order (DASH-1 … DASH-4). Both flags are derived
 * on read and never stored, so this hook presents the row as it arrives and
 * recomputes nothing.
 *
 * **The household comes from the active read, not from the caller**, and the
 * query is `enabled: false` while that read has no answer — the same guard as
 * the rabbit lists, because there is no household whose rows to ask for. The
 * empty household id in the disabled case is a key that is never fetched and
 * never written.
 *
 * **No `staleTime` override**: freshness and retry stay with the shared client
 * (`src/lib/query-client.ts`), so mounting the dashboard refreshes it and a
 * failure reaches the surface with its cause (NFR-8). Tuning staleness is P6's
 * subject, not P1's.
 *
 * The keys, the mutations and the `weight:changed` / `rabbit:changed`
 * invalidations are other tasks' (3.4, 3.5, 3.7); this module only reads.
 *
 * Types are derived from the `api.ts` wrapper rather than imported from
 * `src/bindings.ts` (project hard rule 2).
 */
import { queryOptions, useQuery } from '@tanstack/react-query';
import type { CommandData, CommandFailure } from '../../lib/command-result';
import { unwrapCommandResult } from '../../lib/command-result';
import { useActiveHousehold } from '../households/queries';
import { getDashboard } from './api';
import { dashboardKeys } from './keys';

/** `get_dashboard`'s whole outcome, value and typed failure. */
type DashboardOutcome = Awaited<ReturnType<typeof getDashboard>>;

/** One dashboard row: the rabbit, its last weigh instant and the drop flag. */
export type DashboardRow = CommandData<DashboardOutcome>[number];

/** A dashboard command's failure: the backend's cause, or a failed IPC call. */
type DashboardFailure<Outcome> = CommandFailure<Outcome> | Error;

/** The dashboard read's error type. */
export type DashboardRowsFailure = DashboardFailure<DashboardOutcome>;

/** The rows key's type, so the options cannot disagree with the factory. */
type DashboardRowsKey = ReturnType<typeof dashboardKeys.rows>;

/**
 * One household's dashboard rows (DASH-1 … DASH-4).
 *
 * The backend owns the order, including "never weighed last" (DASH-3), so the
 * rows are read in the order they arrive. Zero rows is a value — DASH-8's empty
 * state — not a failure.
 */
export function dashboardRowsQueryOptions(householdId: string) {
  return queryOptions<
    DashboardRow[],
    DashboardRowsFailure,
    DashboardRow[],
    DashboardRowsKey
  >({
    queryKey: dashboardKeys.rows(householdId),
    queryFn: () => getDashboard(householdId).then(unwrapCommandResult),
  });
}

/** The active household's dashboard rows, once there is an active household. */
export function useDashboardRows() {
  const householdId = useActiveHousehold().data?.id;

  return useQuery({
    ...dashboardRowsQueryOptions(householdId ?? ''),
    enabled: householdId !== undefined,
  });
}
