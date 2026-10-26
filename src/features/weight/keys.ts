/*
 * Weight query keys (task 3.3; WT-6, D1.4).
 *
 * One hierarchical namespace for the weight reads, so task 3.5 can invalidate
 * exactly what a write changed: `weightKeys.all` is the whole feature,
 * `weightKeys.series(rabbitId)` is one rabbit's every window (the unit a weight
 * write invalidates), `weightKeys.range(rabbitId, range)` is one cache entry,
 * and `weightKeys.detail(rabbitId, entryId)` is one entry's detail (task 4.19,
 * WT-8).
 *
 * **The range is part of the cache identity**, and that is what makes WT-6's
 * independence real rather than presentational: the trend at 30 days and the
 * history at 7 days are two entries asked with two arguments, so one view's
 * mount, refetch or failure cannot replace the other's points. One shared key
 * would make whichever view read last win.
 *
 * **The rabbit is part of it too**, so switching rabbits cannot serve the
 * previous rabbit's series while the new one loads (NFR-10's isolation rule,
 * applied to a read whose key would otherwise be only a range). The id is a
 * plain `string` because `RabbitId` is a string alias and this module stays
 * outside `api.ts`'s bindings import (project hard rule 2).
 */
import type { WeightRange } from './search';

/**
 * Every weight query key. Hierarchical, so a prefix invalidates a subtree
 * (`invalidateQueries({ queryKey: weightKeys.series(rabbitId) })` is one
 * rabbit's trend and history at every range, and its entry details with them).
 */
export const weightKeys = {
  /** The whole feature: the invalidation root. */
  all: ['weights'] as const,

  /** Every loaded window of one rabbit's series. */
  series: (rabbitId: string) =>
    [...weightKeys.all, 'series', rabbitId] as const,

  /** One rabbit's series in one window — one independent view's cache entry. */
  range: (rabbitId: string, range: WeightRange) =>
    [...weightKeys.series(rabbitId), range] as const,

  /**
   * One entry's detail, under its rabbit's series prefix (task 4.19; WT-8,
   * WT-7).
   *
   * The prefix is not cosmetic. A rabbit write — a threshold edit — invalidates
   * `weightKeys.series(rabbitId)` and nothing else of this feature, and its flag
   * is re-derived on read rather than stored (D1.3, D1.13), so a detail keyed
   * outside that prefix would keep the flag the previous threshold produced.
   * `../weight/alerts.ts` records the constraint; this factory is where it is
   * kept.
   *
   * The entry id is the identity and the rabbit id is the scope the detail read
   * cannot supply before it answers (`get_weight_entry` takes the entry only).
   * Both are plain strings for the reason above.
   *
   * **A blank rabbit is outside that reach.** The route's validator keeps an
   * absent `?rabbit=` as `''` instead of failing the link (task 4.19,
   * `./search`), so such a deep link keys the detail as
   * `['weights', 'series', '', 'detail', entryId]`: it is under no rabbit's
   * prefix, no `series(rabbitId)` invalidation reaches it, and its derived drop
   * flag can keep the value a previous threshold produced — the very staleness
   * the prefix exists to prevent. Task 4.21 must read the rabbit from the entry
   * it loaded before trusting that flag.
   */
  detail: (rabbitId: string, entryId: string) =>
    [...weightKeys.series(rabbitId), 'detail', entryId] as const,
};
