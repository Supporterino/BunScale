/*
 * Household query keys (task 3.2; HS-3, HS-4, SYNC-1, NFR-10).
 *
 * One hierarchical namespace, so task 3.6 can invalidate exactly what an event
 * touched: `householdKeys.all` for the feature, `householdKeys.collections()`
 * for every loaded collection, or `householdKeys.collection(id)` for the one a
 * `household:changed` event names.
 *
 * The isolation rule is why a collection key carries its household id (NFR-10):
 * one household's rows live under that household's entry, so switching
 * households asks for a different key and can neither render nor be served from
 * the previous household's cached rows. The `list` and `active` keys are
 * deliberately not scoped that way — they describe the person's whole held set
 * rather than one household's data, and the active household cannot be keyed by
 * its own value.
 *
 * Ids are the contract's `HouseholdId`, an opaque string. The factory takes a
 * plain `string` so it can live outside `api.ts` and its bindings import
 * (project hard rule 2) without weakening anything: `HouseholdId` is a string
 * alias, not a branded type.
 */

/**
 * Every household query key. Hierarchical, so a prefix invalidates a subtree
 * (`invalidateQueries({ queryKey: householdKeys.all })` is the whole feature).
 */
export const householdKeys = {
  /** The whole feature: the invalidation root. */
  all: ['households'] as const,

  /** The held households, with the active one indicated (HS-3, HS-4). */
  list: () => [...householdKeys.all, 'list'] as const,

  /** The active household itself, or `null` (HS-2, HS-3, DM-3). */
  active: () => [...householdKeys.all, 'active'] as const,

  /** Every loaded collection, whatever household it belongs to. */
  collections: () => [...householdKeys.all, 'collection'] as const,

  /** One household's collection — the scope that keeps households apart. */
  collection: (householdId: string) =>
    [...householdKeys.collections(), householdId] as const,
};
