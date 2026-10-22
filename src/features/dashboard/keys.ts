/*
 * Dashboard query keys (task 3.2; DASH-1, DASH-6).
 *
 * One namespace for the dashboard read, kept apart from `rabbitKeys` and
 * `weightKeys` because the rows are a projection of their own: a `weight:changed`
 * event invalidates this read beside the rabbit's trend, history and detail
 * (design.md, Events), and task 3.5 must be able to name it without reaching
 * into another feature's keys.
 *
 * The rows are household-scoped (DASH-1: the active household's non-archived
 * rabbits), so the key carries the household id: another household's rows are a
 * different entry and can never be served under this one (NFR-10).
 */

/**
 * Every dashboard query key. Hierarchical, so a prefix invalidates the subtree
 * (`dashboardKeys.all` is the whole feature).
 */
export const dashboardKeys = {
  /** The whole feature: the invalidation root. */
  all: ['dashboard'] as const,

  /** One household's dashboard rows (DASH-1 … DASH-4). */
  rows: (householdId: string) =>
    [...dashboardKeys.all, 'rows', householdId] as const,
};
