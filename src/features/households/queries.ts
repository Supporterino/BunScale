/*
 * The household reads and their loaded-data-first cache (tasks 3.2 and 3.3;
 * HS-3, HS-4, DM-3, SYNC-1, SYNC-3).
 *
 * The held set (`householdListQueryOptions`), the active household
 * (`activeHouseholdQueryOptions`), that household's collection, and the switch
 * that changes which one is active (`useSwitchActiveHousehold`) live here
 * together because they share one key namespace (`./keys`) and one contract's
 * worth of types. The set, the active state and the switch are task 3.3's; the
 * collection's loaded-data-first cache is task 3.2's and is unchanged.
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
import {
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import type { CommandData, CommandFailure } from '../../lib/command-result';
import { unwrapCommandResult } from '../../lib/command-result';
import {
  type createHousehold,
  getActiveHousehold,
  listHouseholds,
  loadHousehold,
  type setActiveHousehold,
} from './api';
import { householdKeys } from './keys';

/*
 * The read's types are derived from the wrapper rather than imported from
 * `src/bindings.ts`, because only `api.ts` may import the generated bindings
 * (project hard rule 2) — and a derived type cannot drift from the wrapper it
 * describes.
 */

/** `load_household`'s whole outcome, value and typed failure. */
type CollectionOutcome = Awaited<ReturnType<typeof loadHousehold>>;

/** `list_households`' whole outcome. */
type HouseholdListOutcome = Awaited<ReturnType<typeof listHouseholds>>;

/** `get_active_household`'s whole outcome. */
type ActiveHouseholdOutcome = Awaited<ReturnType<typeof getActiveHousehold>>;

/** `set_active_household`'s handle, as `api.ts` returns it (D0.14). */
type SwitchOperation = ReturnType<typeof setActiveHousehold>;

/** `set_active_household`'s outcome, for the switch's error type below. */
type SwitchOutcome = Awaited<ReturnType<SwitchOperation['run']>>;

/** `create_household`'s handle, as `api.ts` returns it (D0.14). */
type CreateOperation = ReturnType<typeof createHousehold>;

/** `create_household`'s outcome, for the creation's error type below. */
type CreateOutcome = Awaited<ReturnType<CreateOperation['run']>>;

/** One row of the held-household list (HS-4). */
export type HouseholdSummary = CommandData<HouseholdListOutcome>[number];

/** One household; the active read's non-null value (HS-1, HS-3, DM-3). */
export type Household = NonNullable<CommandData<ActiveHouseholdOutcome>>;

/** One household's collection, as a successful read resolves it. */
export type HouseholdCollection = CommandData<CollectionOutcome>;

/**
 * A household command's error: the backend's `CommandError` in the normal
 * case, or an `Error` when the IPC call itself failed — the only rejection
 * `typedError` re-throws instead of converting into the union. It is a union on
 * purpose, so a consumer has to narrow before reading `cause`, `retryable` or
 * `message_key`. That union is still narrower than what can arrive: the
 * `CommandError` type is nominal, and an unknown or denied command puts a plain
 * string in the slot at runtime, so the check has to be a runtime one (see
 * `src/lib/request-id.ts`).
 */
type HouseholdFailure<Outcome> = CommandFailure<Outcome> | Error;

/** The collection read's error type. */
export type HouseholdCollectionFailure = HouseholdFailure<CollectionOutcome>;

/** The held-list read's error type. */
export type HouseholdListFailure = HouseholdFailure<HouseholdListOutcome>;

/** The active-household read's error type. */
export type ActiveHouseholdFailure = HouseholdFailure<ActiveHouseholdOutcome>;

/** The collection key's type, so the options cannot disagree with the factory. */
type CollectionKey = ReturnType<typeof householdKeys.collection>;

/** The held-list key's type. */
type HouseholdListKey = ReturnType<typeof householdKeys.list>;

/** The active-household key's type. */
type ActiveHouseholdKey = ReturnType<typeof householdKeys.active>;

/**
 * The held-household list, with the active one indicated (HS-3, HS-4, DM-3).
 *
 * Served from this device's household book (D0.2), so it is a local read: an
 * empty list is a value, not a failure, and on its own it proves nothing about
 * whether the person holds a household (SYNC-3 — see `./entry`).
 */
export function householdListQueryOptions() {
  return queryOptions<
    HouseholdSummary[],
    HouseholdListFailure,
    HouseholdSummary[],
    HouseholdListKey
  >({
    queryKey: householdKeys.list(),
    queryFn: () => listHouseholds().then(unwrapCommandResult),
  });
}

/** The held households, reading the cached copy first. */
export function useHouseholdList() {
  return useQuery(householdListQueryOptions());
}

/**
 * The active household, or `null` when there is none (HS-2, HS-3, DM-3).
 *
 * At most one household is active; `null` is the onboarding trigger only once
 * the entry guard has a fresh reachable status to pair it with (SYNC-3).
 */
export function activeHouseholdQueryOptions() {
  return queryOptions<
    Household | null,
    ActiveHouseholdFailure,
    Household | null,
    ActiveHouseholdKey
  >({
    queryKey: householdKeys.active(),
    queryFn: () => getActiveHousehold().then(unwrapCommandResult),
  });
}

/** The active household, reading the cached copy first. */
export function useActiveHousehold() {
  return useQuery(activeHouseholdQueryOptions());
}

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

/**
 * Switch the active household (HS-3, HS-4, DM-3).
 *
 * The mutation's variable is the **operation handle** `setActiveHousehold`
 * returns, not a bare id: a switch is one operation identity (task 3.1), so a
 * retry re-runs the handle it was given instead of minting a second one. Start
 * a switch with `mutate(setActiveHousehold(householdId))`; keep the handle if a
 * retry is offered — `set_active_household`'s only failure is `validation` (the
 * person does not hold the household), which is not retryable, so a surface
 * rarely has to.
 *
 * On success the command's post-state is written to the active-household cache
 * **and** to the held list's `is_active` flags, then the list is invalidated, so
 * `is_active` moves to exactly the household the backend returned at the moment
 * it confirms (DM-3) instead of after a refetch — a surface that marks the
 * active row from `is_active` cannot show two active households in between. No
 * collection cache is touched: collections are keyed by household, so the newly
 * active household's rows are a different entry and the previous household's
 * entry stays its own (NFR-10).
 */
export function useSwitchActiveHousehold() {
  const queryClient = useQueryClient();

  return useMutation<
    Household,
    HouseholdFailure<SwitchOutcome>,
    SwitchOperation
  >({
    mutationFn: (operation) => operation.run().then(unwrapCommandResult),
    onSuccess: (household) => {
      queryClient.setQueryData<Household | null>(
        householdKeys.active(),
        household,
      );

      /*
       * Rewritten only when a held list is already cached: a switch does not
       * invent a held set, and the invalidation below still makes the next read
       * the authoritative one.
       */
      queryClient.setQueryData<HouseholdSummary[]>(
        householdKeys.list(),
        (current) =>
          current?.map((row) => ({
            ...row,
            is_active: row.id === household.id,
          })),
      );

      void queryClient.invalidateQueries({ queryKey: householdKeys.list() });
    },
  });
}

/**
 * Create a household and make it active (HS-1, HS-2, HS-4).
 *
 * The mutation's variable is the **operation handle** `createHousehold`
 * returns, not a bare name: `create_household` is idempotent on `request_id`
 * (D0.14), so a retry must re-run the handle of the attempt that failed rather
 * than mint a second create. Start a creation with
 * `mutate(createHousehold(name))`, and re-submit the same handle to retry.
 * A corrected name is a new intent and therefore a new handle; the component
 * owns that distinction (task 4.1's `HouseholdOnboardingScreen`).
 *
 * On success the command's post-state is written to `householdKeys.active()`
 * **and** to the held list when it is already cached, then the pair is
 * invalidated — the same shape as `useSwitchActiveHousehold`, because both
 * commands answer with the active household and both change which row is
 * `is_active` (HS-4, DM-3).
 *
 * The list write rewrites the cached rows' flags; it does **not** append the
 * new household, because the command does not return a `HouseholdSummary`
 * (`member_count` is not part of it) and inventing one would be a summary the
 * book never confirmed. Every cached row is marked inactive at that instant
 * (the returned household is the active one, and it is not yet among them), so
 * a surface marking rows from the stale list cannot keep the previous household
 * active; the invalidation's read supplies the new row.
 *
 * No collection is touched. A new household has no loaded collection on this
 * device, and no other household's collection changed (HS-3, NFR-10).
 */
export function useCreateHousehold() {
  const queryClient = useQueryClient();

  return useMutation<
    Household,
    HouseholdFailure<CreateOutcome>,
    CreateOperation
  >({
    mutationFn: (operation) => operation.run().then(unwrapCommandResult),
    onSuccess: (household) => {
      queryClient.setQueryData<Household | null>(
        householdKeys.active(),
        household,
      );

      /*
       * Rewritten only when a held list is already cached: creation does not
       * invent a held set, and the invalidation below makes the next read the
       * authoritative one.
       */
      queryClient.setQueryData<HouseholdSummary[]>(
        householdKeys.list(),
        (current) =>
          current?.map((row) => ({
            ...row,
            is_active: row.id === household.id,
          })),
      );

      void queryClient.invalidateQueries({ queryKey: householdKeys.active() });
      void queryClient.invalidateQueries({ queryKey: householdKeys.list() });
    },
  });
}
