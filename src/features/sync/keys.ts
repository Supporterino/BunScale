/*
 * Sync query keys (task 3.4).
 *
 * One namespace, so task 3.6 can invalidate the status when
 * `service:status-changed` arrives without reaching into the household
 * namespace: a service failure says nothing about a household's rows.
 */

/** Every sync query key, hierarchical so a prefix invalidates the subtree. */
export const syncKeys = {
  /** The whole sync feature. */
  all: ['sync'] as const,

  /** The last observed service/account status (NFR-1, R3). */
  status: () => [...syncKeys.all, 'status'] as const,
};
