/*
 * The household collection read and its loaded-data-first cache (task 3.2;
 * SYNC-1, SYNC-3).
 *
 * `load_household` already answers from this device's copy first and reports
 * where the answer came from: `source` is `service` when the refresh that
 * accompanied the read succeeded, `local` when the previously loaded copy was
 * returned unchanged, and `loaded_at` is `null` only for a collection this
 * device has never loaded (D0.2). The frontend's job is to keep that value
 * whole, which is what makes the three SYNC-3 states distinguishable:
 *
 * - **The cached value is the contract's `HouseholdCollection`,** never a bare
 *   item array. A read of "no items" is therefore still two different values —
 *   `loaded_at: null` (nothing was ever loaded) and an empty `items` with a
 *   `loaded_at` (loaded, and currently empty) — and `collectionPresence` names
 *   which one it is so a surface cannot render the first as the second.
 * - **A refresh replaces, it never appends.** The whole collection is the unit
 *   of cache data, so a refetch stores the object the backend returned: there
 *   is no merge step that could keep an item twice (SYNC-1), and no
 *   `structuralSharing` override that would make old rows survive a replacement.
 * - **Previously loaded rows stay on screen while a refresh runs, and after it
 *   fails.** Re-using a key keeps the cached data visible during the refetch,
 *   and TanStack Query keeps the last successful data when a refetch rejects —
 *   which is SYNC-3's "previously loaded data is shown while unreachable" once
 *   the backend reports `source: local`.
 * - **No cross-household placeholder.** `placeholderData` is left unset on
 *   purpose: carrying the previous data into a new key would render one
 *   household's rows under another household's id for the duration of the load,
 *   which is exactly what the household-scoped key exists to prevent (NFR-10).
 *
 * Freshness and retry policy stay with the shared client
 * (`src/lib/query-client.ts`): no `staleTime` override here, so mounting a
 * household's screen refreshes it — SYNC-3's "a start with connectivity
 * refreshes loaded data" — and a failure reaches the surface with its cause
 * instead of being retried invisibly.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';
import type { CommandData, CommandFailure } from '../../lib/command-result';
import { unwrapCommandResult } from '../../lib/command-result';
import { loadHousehold } from './api';
import { householdKeys } from './keys';

/*
 * The read's types are derived from the wrapper rather than imported from
 * `src/bindings.ts`, because only `api.ts` may import the generated bindings
 * (project hard rule 2) — and a derived type cannot drift from the wrapper it
 * describes.
 */

/** `load_household`'s whole outcome, value and typed failure. */
type CollectionOutcome = Awaited<ReturnType<typeof loadHousehold>>;

/** One household's collection, as a successful read resolves it. */
export type HouseholdCollection = CommandData<CollectionOutcome>;

/**
 * The read's error type: the backend's `CommandError` in the normal case, or an
 * `Error` when the IPC call itself failed — the only rejection `typedError`
 * re-throws instead of converting into the union. It is a union on purpose, so
 * a consumer has to narrow before reading `cause`, `retryable` or
 * `message_key`. That union is still narrower than what can arrive: the
 * `CommandError` type is nominal, and an unknown or denied command puts a plain
 * string in the slot at runtime, so the check has to be a runtime one (see
 * `src/lib/request-id.ts`).
 */
export type HouseholdCollectionFailure =
  | CommandFailure<CollectionOutcome>
  | Error;

/** The collection key's type, so the options cannot disagree with the factory. */
type CollectionKey = ReturnType<typeof householdKeys.collection>;

/**
 * The collection read for one household (SYNC-1, SYNC-3).
 *
 * Keyed by the household it belongs to, so a screen for another household has
 * its own cache entry and the rows of the two can never mix (NFR-10). The
 * queryFn unwraps the wrapper's `Result`: a successful read replaces the cache
 * entry with the returned collection, and a failure is re-thrown unchanged — a
 * `CommandError` for a command failure, an `Error` for a failed IPC call — so
 * the cause reaches the failure surface (NFR-8).
 */
export function householdCollectionQueryOptions(householdId: string) {
  return queryOptions<
    HouseholdCollection,
    HouseholdCollectionFailure,
    HouseholdCollection,
    CollectionKey
  >({
    queryKey: householdKeys.collection(householdId),
    queryFn: () => loadHousehold(householdId).then(unwrapCommandResult),
  });
}

/**
 * The loaded collection of one household, reading the cached copy first.
 *
 * `data` is the last loaded collection while a refresh runs; `isFetching`
 * reports the refresh itself. `data` stays defined if that refresh fails, which
 * is what lets the surface keep showing previously loaded rows (SYNC-3).
 */
export function useHouseholdCollection(householdId: string) {
  return useQuery(householdCollectionQueryOptions(householdId));
}

/**
 * Which of SYNC-3's states a collection read is in.
 *
 * `never-loaded` is the explicit "nothing was ever loaded on this device" value
 * — never an empty collection, and never by itself a claim about connectivity:
 * pairing it with a service status is the guard's and the connectivity
 * surface's job (tasks 3.3 and 4.4).
 */
export type CollectionPresence =
  | { kind: 'never-loaded' }
  | { kind: 'loaded'; source: HouseholdCollection['source']; loadedAt: string };

/**
 * Derive the presence of one read.
 *
 * It takes a completed read: a query whose `data` is still `undefined` has not
 * produced a collection yet, which is a fourth state owned by the query's
 * `status` rather than by this helper.
 *
 * The contract pairs `source: local` with `loaded_at: null` for a collection
 * this device has never loaded, and a `service` read always records the load
 * instant, so `loaded_at` is the discriminator. `loadedAt` is the contract's
 * `Instant`, an RFC 3339 string; it is never re-parsed here.
 */
export function collectionPresence(
  collection: HouseholdCollection,
): CollectionPresence {
  if (collection.loaded_at === null) {
    return { kind: 'never-loaded' };
  }

  return {
    kind: 'loaded',
    source: collection.source,
    loadedAt: collection.loaded_at,
  };
}
