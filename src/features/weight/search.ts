/*
 * The rabbit dashboard's two range selections, as typed route search
 * parameters (task 3.3; WT-6, D1.4).
 *
 * Range selection is a read-side concern and a query window, never a stored
 * value (D1.4), so it lives in the URL rather than in the cache or a store. The
 * trend chart and the weight-only history each carry their own field — `trend`
 * and `history` — and neither is derived from the other, which is WT-6's
 * independence made structural instead of a rule a surface has to remember:
 * updating one field through the router's `search` updater writes that field
 * only, and the other view keeps reading its own.
 *
 * The default is named once (`DEFAULT_WEIGHT_RANGE`) and used for both fields,
 * so a first visit asks the backend for one window per view and the two agree
 * until the person changes one. A default held anywhere else would be a second
 * source of the same decision, and D1.4 rules that out.
 *
 * Validation is a plain function rather than a Zod schema: `zod` is not a
 * dependency (project hard rule 6), and the whole schema is four literals in
 * two optional fields. An absent, misspelled or out-of-union value becomes the
 * default instead of failing the navigation — the search string is a
 * preference, not something a person can break, and refusing the route over a
 * stale bookmark would show an error page where the default window belongs.
 *
 * `WeightRange` is derived from the wrapper that consumes it (`./api`'s
 * `getWeightEntries`), so the validator and the hooks speak the contract's own
 * union rather than a second type of their own. The **value** list below is
 * still written by hand, because a type cannot enumerate its own variants, and
 * `satisfies` only rejects an entry that is not in the union — it says nothing
 * about one being missing. Completeness is pinned by `search.test.ts`, which
 * compares the list against the frozen four with `toEqual`, so dropping a
 * window fails the suite instead of silently validating to the default.
 *
 * **The weight form's binding is validated here too (task 4.18).** That search
 * is the same kind of state — a typed URL that must never fail a navigation —
 * so `validateWeightFormSearch` sits beside the range pair it hands back on the
 * way out of the form, rather than in the screen that renders it.
 */
import type { getWeightEntries } from './api';

/** The contract's `WeightRange`, as the read in `api.ts` takes it. */
export type WeightRange = Parameters<typeof getWeightEntries>[1];

/**
 * The four windows, in the order WT-6 presents them (7 days, 30 days, 90 days,
 * full history). The range control renders this list, so the order is part of
 * the value rather than a detail of the surface. `satisfies` rejects an entry
 * that is not one of the contract's values; that all four are present is a
 * test's job, not a type's (see the header).
 */
export const WEIGHT_RANGES = [
  'days_7',
  'days_30',
  'days_90',
  'all',
] as const satisfies readonly WeightRange[];

/**
 * The window a rabbit's dashboard opens on, for the trend and the history
 * alike. One name, used by both fields below: "the default" cannot come to mean
 * two different windows.
 */
export const DEFAULT_WEIGHT_RANGE: WeightRange = 'days_30';

/** The dashboard's range state: two independent windows, one per view. */
export type WeightRangeSearch = {
  trend: WeightRange;
  history: WeightRange;
};

/** Whether an unvalidated search value names one of the four windows. */
function isWeightRange(value: unknown): value is WeightRange {
  return WEIGHT_RANGES.some((range) => range === value);
}

/** A search value as its window, or the default when it names none. */
function weightRangeOr(value: unknown): WeightRange {
  return isWeightRange(value) ? value : DEFAULT_WEIGHT_RANGE;
}

/**
 * The validator task 4.11's route passes as `validateSearch`:
 *
 * ```ts
 * validateSearch: validateWeightRangeSearch,
 * ```
 *
 * It reads the two fields it owns and returns both, so a bare
 * `/rabbits/$rabbitId` visit still yields a complete, typed range state. Any
 * other search field is left to whatever else the route validates; this
 * function does not pass it through, so a caller cannot end up with two
 * versions of the same key.
 */
export function validateWeightRangeSearch(
  search: Record<string, unknown>,
): WeightRangeSearch {
  return {
    trend: weightRangeOr(search.trend),
    history: weightRangeOr(search.history),
  };
}

/**
 * The weight form's entry kind: the one the two quick paths share (WT-2,
 * D1.10). Only quick entry exists, but the route names the kind rather than
 * implying it, so a second kind could be bound without changing the contract.
 */
export type WeightFormMode = 'quick';

/**
 * The surface that opened the weight form. Cancel returns to it and a confirmed
 * save returns to the same rabbit's context (D1.10).
 *
 * `dashboard` is the list route (`/`), whose rows are the per-rabbit path and
 * whose quick-entry control is the picker path. `rabbit` is a rabbit's own
 * dashboard, so the form gives back the windows it was opened at.
 */
export type WeightFormOrigin = 'dashboard' | 'rabbit';

/**
 * The form's typed search (task 4.18). The range pair is the rabbit
 * dashboard's contract, carried only so a `from: 'rabbit'` return lands on the
 * same windows; both are optional so a dashboard caller does not have to
 * restate a window it does not own. The validator still fills the defaults if
 * they are absent.
 */
export type WeightFormSearch = Partial<WeightRangeSearch> & {
  /** The rabbit this entry is bound to (DASH-9, WT-2). */
  rabbit: string;
  /** The quick-entry kind both paths share (D1.10). */
  mode: WeightFormMode;
  /** Where cancel and a confirmed save return (D1.10). */
  from: WeightFormOrigin;
};

/**
 * Validate the weight form's search without failing the navigation: a missing
 * or misspelled value becomes the ordinary default instead of an error page
 * over a stale bookmark. The pair comes from the frozen
 * `validateWeightRangeSearch` rather than restating its four windows here.
 *
 * A blank `rabbit` is not defaulted away: the profile read is the surface that
 * says an id is unknown (it answers `validation`), and inventing a destination
 * here would bind the form to a rabbit the person did not choose.
 */
export function validateWeightFormSearch(
  search: Record<string, unknown>,
): WeightFormSearch {
  const rabbit = typeof search.rabbit === 'string' ? search.rabbit : '';

  return {
    ...validateWeightRangeSearch(search),
    rabbit,
    mode: 'quick',
    // Without a rabbit there is no rabbit dashboard to return to; the default
    // keeps a malformed URL on the dashboard instead of a route with no id.
    from: search.from === 'rabbit' && rabbit !== '' ? 'rabbit' : 'dashboard',
  };
}
