/*
 * Settings & Profile query keys (task 3.5; SET-2, SET-4).
 *
 * One namespace for the reads that belong to this screen and to the device:
 * the identity (SET-2) and the effective display weight unit (SET-4). Keeping
 * them together means a later invalidation can name the one read it changed
 * without reaching into another feature.
 *
 * The household list is deliberately **not** here. It is the person's held set,
 * not a Settings preference: it lives in `src/features/households/keys.ts` with
 * the active household and the switch, because those three describe one fact
 * and the switch has to write the same keys (HS-4, DM-3). `./households`
 * composes the two household reads for this screen and adds no key of its own.
 *
 * The display unit is device-scoped rather than household-scoped (D0.2, R7), so
 * nothing here carries a household id: switching households cannot change the
 * unit, and changing the unit cannot invalidate a household read.
 */

/** Every Settings & Profile query key, hierarchical so a prefix invalidates the subtree. */
export const settingsKeys = {
  /** The whole feature: the invalidation root. */
  all: ['settings'] as const,

  /** The current person's identity as SET-2 resolves it. */
  identity: () => [...settingsKeys.all, 'identity'] as const,

  /**
   * The effective per-device display unit (SET-4).
   *
   * This is the one value every weight display derives from. P0 has no cached
   * formatted weight — stored values stay canonical and conversion happens at
   * the entry/display boundary (D0.4) — so there is no second cache entry to
   * invalidate alongside it.
   */
  displayWeightUnit: () =>
    [...settingsKeys.all, 'display-weight-unit'] as const,
};
