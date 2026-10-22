/*
 * The ranged weight reads (task 3.3; WT-6, WT-7, D1.4).
 *
 * `get_weight_entries` serves both ranged views that WT-6 defines: the trend
 * chart and the weight-only history call the one frozen read, each with its own
 * `WeightRange`. The hooks below therefore hold no range of their own — each
 * takes the window its caller read from the route's typed search parameters
 * (`./search`), which is what lets the two views sit at different ranges at the
 * same time. Nothing here stores a selection, so changing one view's range
 * cannot move the other's.
 *
 * **One cache entry per (rabbit, range).** Both hooks use the same options
 * factory and so the same key, because for one rabbit the two views are asking
 * exactly the same question: the same window is one read, one answer, and one
 * entry. Their independence is the independence of the arguments, not of the
 * data — a trend at 30 days and a history at 30 days legitimately share.
 *
 * **No cross-rabbit placeholder.** `placeholderData` is left unset on purpose:
 * carrying the previous rabbit's or the previous range's points into a new key
 * would render one series under another rabbit's or another window's label
 * while it loads, which is what the scoped key exists to prevent (NFR-10).
 *
 * Freshness and retry stay with the shared client (`src/lib/query-client.ts`):
 * no `staleTime` override, so mounting a screen refreshes it, and a failure
 * reaches the surface with its cause instead of being retried invisibly
 * (NFR-8). This task adds **reads only**: the weight mutations are task 3.5 and
 * the `weight:changed` subscription is task 3.7.
 *
 * Types are derived from the wrapper rather than imported from
 * `src/bindings.ts`, because only `api.ts` may import the generated bindings
 * (project hard rule 2) — and a derived type cannot drift from the wrapper it
 * describes.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';
import type { CommandData, CommandFailure } from '../../lib/command-result';
import { unwrapCommandResult } from '../../lib/command-result';
import { getWeightEntries } from './api';
import { weightKeys } from './keys';
import type { WeightRange } from './search';

/** `get_weight_entries`' whole outcome, value and typed failure. */
type WeightSeriesOutcome = Awaited<ReturnType<typeof getWeightEntries>>;

/**
 * One rabbit's series for one window: the points inside the window, oldest
 * first, with the threshold they were judged against (WT-6, WT-7).
 */
export type WeightTrend = CommandData<WeightSeriesOutcome>;

/**
 * A weight read's error: the backend's `CommandError` in the normal case, or an
 * `Error` when the IPC call itself failed. It is a union on purpose, so a
 * consumer has to narrow before reading `cause`, `retryable` or `message_key`.
 */
export type WeightSeriesFailure = CommandFailure<WeightSeriesOutcome> | Error;

/** The range key's type, so the options cannot disagree with the factory. */
type WeightRangeKey = ReturnType<typeof weightKeys.range>;

/**
 * One rabbit's series in one window (WT-6, WT-7).
 *
 * The range is passed to the backend as a query window, never stored: the
 * window limits which points come back, and the drop flags are still derived
 * over the rabbit's full entry set (I3), so a point can be flagged on the basis
 * of a predecessor outside the window. `'all'` is the absence of a lower bound
 * (D1.4), so full history returns every entry unchanged rather than a computed
 * period.
 *
 * The whole object is the cached value — points and threshold together — so a
 * point is always judged against the threshold that produced it (D1.3). A blank
 * or unknown rabbit id is the backend's `validation` failure, not an empty
 * series.
 */
export function weightSeriesQueryOptions(rabbitId: string, range: WeightRange) {
  return queryOptions<
    WeightTrend,
    WeightSeriesFailure,
    WeightTrend,
    WeightRangeKey
  >({
    queryKey: weightKeys.range(rabbitId, range),
    queryFn: () => getWeightEntries(rabbitId, range).then(unwrapCommandResult),
  });
}

/**
 * The trend chart's series, at the route's trend range (WT-6, DASH-7).
 *
 * Pass `search.trend` from `./search`: this hook has no default of its own and
 * reads no other view's state, so the chart cannot move the history.
 */
export function useWeightTrend(rabbitId: string, range: WeightRange) {
  return useQuery(weightSeriesQueryOptions(rabbitId, range));
}

/**
 * The weight-only history's series, at the route's history range (WT-6, DASH-7).
 *
 * The same read as the trend, with the independent window: pass `search.history`
 * from `./search`. The listing renders `data.points`, which are already ordered
 * oldest first, so a newest-first view reverses them.
 */
export function useWeightHistory(rabbitId: string, range: WeightRange) {
  return useQuery(weightSeriesQueryOptions(rabbitId, range));
}
