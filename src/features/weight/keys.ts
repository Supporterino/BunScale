/*
 * Weight query keys (task 3.3; WT-6, D1.4).
 *
 * One hierarchical namespace for the ranged weight reads, so task 3.5 can
 * invalidate exactly what a write changed: `weightKeys.all` is the whole
 * feature, `weightKeys.series(rabbitId)` is one rabbit's every window (the unit
 * a weight write invalidates), and `weightKeys.range(rabbitId, range)` is one
 * cache entry.
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
 * rabbit's trend and history at every range).
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
};
