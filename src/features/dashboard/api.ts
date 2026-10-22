/*
 * Dashboard command (DASH-1 … DASH-4): the only module that imports its
 * generated bindings. Project hard rule 2 keeps `src/bindings.ts` out of routes,
 * hooks and components, so the dashboard read goes through this function.
 *
 * One read, one household: the rows are the household's rabbits with the values
 * the list presents — the last weigh instant or the never-weighed indication,
 * and the weight-drop flag. Both flags are derived on read and never stored
 * (D1.3), and both arrive on the row rather than being recomputed from an
 * absence, so the row indicator and the entry flag cannot disagree (DASH-2,
 * DASH-4, WT-7).
 *
 * The read is served from this device's copy, so it takes no request id and
 * reports no service cause: an unreachable service is a status fact (SYNC-3),
 * not a dashboard failure.
 */
import type { CommandError, DashboardRow, HouseholdId } from '../../bindings';
import { commands } from '../../bindings';
import type { CommandResult } from '../../lib/request-id';

/** One read's outcome: the contract's `Result`, typed to this domain. */
type Outcome<T> = Promise<CommandResult<T, CommandError>>;

/**
 * The household's dashboard rows (DASH-1 … DASH-4).
 *
 * The backend owns the sort order, including "never weighed last" (DASH-3), so
 * the rows are presented in the order they come back. A household whose rabbits
 * are all archived answers zero rows, which is DASH-8's empty state — the
 * archived list is a screen of its own, not a row here (DASH-8, RAB-6).
 *
 * Each row's `rabbit` carries grams nowhere and a name and thumbnail only: a
 * weight value is read through the weight commands, and a thumbnail is presented
 * with a placeholder when it is `null`.
 */
export function getDashboard(
  householdId: HouseholdId,
): Outcome<DashboardRow[]> {
  return commands.getDashboard(householdId);
}
