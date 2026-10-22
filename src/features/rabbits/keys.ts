/*
 * Rabbit query keys (task 3.2; DASH-1, DASH-6, RAB-6).
 *
 * One hierarchical namespace for the reads the rabbit screens and the dashboard
 * share, so task 3.4 can invalidate exactly what a write or a `rabbit:changed`
 * event touched: `rabbitKeys.all` for the feature, `rabbitKeys.lists()` for
 * every loaded list, `[...rabbitKeys.lists(), householdId]` for both lists of
 * one household, or `rabbitKeys.detail(rabbitId)` for one profile.
 *
 * The isolation rule is why a list key carries its household id (NFR-10): one
 * household's rows live under that household's entry, so switching households
 * asks for a different key and can neither render nor be served from the
 * previous household's cached rows.
 *
 * The two list modes are **distinct keys**, not one key with a flag. The frozen
 * `list_rabbits(household_id, include_archived)` answers the active list for
 * `false` and the archived list for `true` — never both — so `mode` is part of
 * the cache identity. Sharing one key would let the archived list's rows
 * replace the dashboard's active rows (and the reverse) whenever the two
 * screens mounted.
 *
 * A profile is keyed by its rabbit id alone: `get_rabbit` takes no household
 * and a rabbit belongs to exactly one, so the id already names the one value.
 *
 * Ids are the contract's `HouseholdId` / `RabbitId`, opaque strings. The
 * factories take plain `string` so this module can live outside `api.ts` and its
 * bindings import (project hard rule 2) without weakening anything: both are
 * string aliases, not branded types.
 */

/** Which of `list_rabbits`' two exclusive answers a list key names. */
export type RabbitListMode = 'active' | 'archived';

/**
 * Every rabbit query key. Hierarchical, so a prefix invalidates a subtree
 * (`invalidateQueries({ queryKey: rabbitKeys.all })` is the whole feature).
 */
export const rabbitKeys = {
  /** The whole feature: the invalidation root. */
  all: ['rabbits'] as const,

  /** Every loaded rabbit list, whatever household and mode it belongs to. */
  lists: () => [...rabbitKeys.all, 'list'] as const,

  /**
   * One household's active list or archived list — the scope that keeps
   * households apart and the two modes from replacing each other.
   */
  list: (householdId: string, mode: RabbitListMode) =>
    [...rabbitKeys.lists(), householdId, mode] as const,

  /** Every loaded rabbit profile. */
  details: () => [...rabbitKeys.all, 'detail'] as const,

  /** One rabbit's full profile (DASH-6, RAB-5). */
  detail: (rabbitId: string) => [...rabbitKeys.details(), rabbitId] as const,
};
