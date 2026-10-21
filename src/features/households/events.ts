/*
 * The `household:changed` invalidation policy (task 3.6; SYNC-1, HS-3, HS-4,
 * DM-3).
 *
 * `household:changed` is a report that something in the device's household book
 * changed, not a data payload: the contract carries `{ household_id, kind }`
 * and no items. This module therefore does not merge an event into a cache; it
 * maps the report to the cache operations whose answer the backend has
 * superseded. It is **pure policy**: the subscription itself lives in
 * `./api.ts`, the feature's only generated-bindings seam (project hard rule 2),
 * and this module imports no `src/bindings.ts` at all — the payload type is the
 * `HouseholdChange` exported by that seam.
 *
 * The cache operation is narrowed by `kind` so an event never touches a query
 * the change did not affect:
 *
 * - `created`, `switched`, `removed` change the **held set or its selection**,
 *   so the `list` + `active` pair is invalidated. Both are invalidated together
 *   because the list's `is_active` flags and the active household describe the
 *   same fact (HS-4, DM-3); refreshing one without the other would leave the
 *   surface's marker and the active state at different vintages.
 * - `refreshed` names one household whose collection was reloaded from the
 *   service, so only that household's **collection key** is invalidated. The
 *   held set did not change, the active household did not change, and no other
 *   household's rows were reloaded.
 * - `removed` additionally **removes** the departed household's collection
 *   entry. Its rows are no longer held by this person, so invalidating them
 *   would leave data nobody can reach cached for the rest of the process;
 *   removing the entry is the honest operation. Only that key is removed — the
 *   other households' collections are untouched.
 * - `householdKeys.all` is deliberately never used here. It is the whole
 *   feature, including every collection, so invalidating it for a `switched`
 *   event would refetch rows the event did not touch — exactly the broadening
 *   the switch spec rejects (task 3.3's `switch.test.tsx`).
 *
 * Both platforms (iOS 16.4, macOS 13.3) go through the same Tauri event plugin;
 * nothing here branches on platform, and nothing here knows the bridge exists.
 */
import type { QueryClient } from '@tanstack/react-query';
import type { HouseholdChange } from './api';
import { householdKeys } from './keys';

/**
 * Apply one reported change to the cache.
 *
 * A pure mapping from `kind` to query keys, kept separate from the listener so
 * the policy is testable without an event bridge. Every branch returns; an
 * unknown kind cannot arrive because the contract's union is exhaustive and
 * TypeScript checks the switch.
 */
export function applyHouseholdChange(
  queryClient: QueryClient,
  change: HouseholdChange,
): void {
  switch (change.kind) {
    case 'created':
    case 'switched':
      void queryClient.invalidateQueries({ queryKey: householdKeys.list() });
      void queryClient.invalidateQueries({
        queryKey: householdKeys.active(),
      });
      return;

    case 'removed':
      void queryClient.invalidateQueries({ queryKey: householdKeys.list() });
      void queryClient.invalidateQueries({
        queryKey: householdKeys.active(),
      });
      queryClient.removeQueries({
        queryKey: householdKeys.collection(change.household_id),
      });
      return;

    case 'refreshed':
      void queryClient.invalidateQueries({
        queryKey: householdKeys.collection(change.household_id),
      });
      return;
  }
}
