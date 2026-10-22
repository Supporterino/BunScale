/*
 * The derived drop alert, read rather than recomputed (task 3.7; WT-7, DASH-4,
 * D1.3, D1.12, D1.13).
 *
 * Both flags arrive on the contract's payload: a `WeightTrend` point and a
 * `WeightEntry` carry `has_weight_drop_alert`, and a `DashboardRow` carries the
 * same flag for the rabbit (its derivation over that rabbit's entries). The
 * backend orders the entries by timestamp, takes the earlier-created entry as
 * the previous one on a tie (D1.12) and derives the flags over the current
 * entry set on every read (2.13, 2.14; D1.13). A second derivation here would
 * be the two-surface disagreement R2 forbids: it would have to reproduce the
 * tie rule, the archived rules and the fact that a window's earliest point can
 * be flagged from a predecessor outside the window (I3). Nothing in this module
 * looks at `value_grams` or `drop_threshold_percent`; it reads the flag.
 *
 * **A threshold edit re-derives without a full reload.** The rabbit write's
 * invalidation (3.4) refreshes `weightKeys.series` — every window — and the
 * dashboard rows. Each refreshed `WeightTrend` carries the new
 * `drop_threshold_percent` beside the points judged against it, so a flag and
 * the threshold that produced it are one payload and cannot be paired wrongly.
 * A re-read after an edit or a deletion carries the re-derived flags around the
 * change for the same reason. The accessors below present that payload and keep
 * no alert state of their own.
 *
 * **No surface consumes these yet.** 4.10 (the entry detail's drop-alert
 * marker), 4.11 (the trend and history markers) and 4.12 (the dashboard row
 * flag) are the consumers, so on its own this module changes no user-visible
 * behaviour: the re-derive outcome rests on 3.4/3.9's invalidation, proved by
 * `rabbits/mutations.test.ts`'s "update invalidates the row, the profile, both
 * weight windows and the dashboard row".
 *
 * **Constraint for the entry-detail hook (task 4.10).** When a
 * `weightKeys.detail(entryId)` hook lands, key it under
 * `weightKeys.series(rabbitId)` — or invalidate `weightKeys.all` on a rabbit
 * update. A detail key outside the series prefix keeps the old
 * `has_weight_drop_alert` after a threshold edit, which breaks WT-7 on that
 * surface.
 *
 * `entryHasDropAlert` serves both the detail view (a `WeightEntry`) and the
 * trend/history markers (a `WeightEntrySummary`); only the flag field is named,
 * so either payload satisfies it. `dashboardRowHasDropAlert` is the row
 * counterpart, and it reads the row's flag instead of folding the points this
 * device happens to hold — folding would re-introduce the derivation above.
 *
 * Types come from the query wrappers (`./queries`, `../dashboard/queries`)
 * rather than `src/bindings.ts`, because only an `api.ts` may import the
 * generated bindings (project hard rule 2).
 */
import type { DashboardRow } from '../dashboard/queries';
import type { WeightTrend } from './queries';

/**
 * Whether one weight entry carries the drop alert (WT-7).
 *
 * Accepts a full `WeightEntry` or one `WeightTrend` point; both carry the
 * contract's derived flag. `true` is presented as the warning — the selector
 * never asks whether the value "should" be flagged, because that question is
 * the backend's and a different answer here is exactly the disagreement R2
 * forbids.
 */
export function entryHasDropAlert(
  entry: Pick<WeightTrend['points'][number], 'has_weight_drop_alert'>,
): boolean {
  return entry.has_weight_drop_alert;
}

/**
 * Whether a dashboard row carries the weight-drop flag (DASH-4, WT-7).
 *
 * The row's flag is the backend's derivation over that rabbit's entries, not a
 * fold of the points this device has cached: a papered-over empty cache would
 * read as "no alert" and clear a flag the service knows about.
 */
export function dashboardRowHasDropAlert(
  row: Pick<DashboardRow, 'has_weight_drop_alert'>,
): boolean {
  return row.has_weight_drop_alert;
}
