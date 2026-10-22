/*
 * The rabbit reads (task 3.2; DASH-1, DASH-6, RAB-6).
 *
 * The rabbit reads sit on `./api` and one key namespace (`./keys`): the active
 * list, the archived list and one rabbit's profile. The household's dashboard
 * rows are the same kind of read but live in `../dashboard/queries`, beside
 * their own keys. This task adds **reads only**: the rabbit mutations are task
 * 3.4, the weight reads are task 3.3, and the `rabbit:changed` subscription
 * that invalidates these after a write is task 3.7. Nothing here writes the
 * cache.
 *
 * **Freshness and retry stay with the shared client** (`src/lib/query-client.ts`):
 * no `staleTime` override, so mounting a screen refreshes it and a failure
 * reaches the surface with its cause instead of being retried invisibly
 * (NFR-8). Tuning staleness is P6's subject; inventing a P1 policy here would
 * make that change reason about two.
 *
 * **The household comes from the active read, not from the caller.**
 * `useActiveHousehold` is P0's single source for which household is active, so
 * a list hook cannot be pointed at a household the rest of the app does not
 * treat as active. While that read has no answer — first mount, or no household
 * at all — the list query is `enabled: false` and names no household's rows;
 * it fetches once the active household arrives.
 *
 * **No cross-household placeholder.** `placeholderData` is left unset on
 * purpose: carrying the previous household's rows into the new key would render
 * one household's list under another's id while it loads, which is exactly what
 * the household-scoped key exists to prevent (NFR-10). The same holds between
 * the two list modes.
 *
 * Types are derived from the wrappers rather than imported from
 * `src/bindings.ts`, because only `api.ts` may import the generated bindings
 * (project hard rule 2) — and a derived type cannot drift from the wrapper it
 * describes.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';
import type { CommandData, CommandFailure } from '../../lib/command-result';
import { unwrapCommandResult } from '../../lib/command-result';
import { useActiveHousehold } from '../households/queries';
import { getRabbit, listRabbits } from './api';
import { type RabbitListMode, rabbitKeys } from './keys';

/** `list_rabbits`' whole outcome, value and typed failure. */
type RabbitListOutcome = Awaited<ReturnType<typeof listRabbits>>;

/** `get_rabbit`'s whole outcome. */
type RabbitProfileOutcome = Awaited<ReturnType<typeof getRabbit>>;

/** One row of a household's active or archived list (DASH-1, RAB-6). */
export type RabbitSummary = CommandData<RabbitListOutcome>[number];

/** One rabbit's full profile (DASH-6, RAB-5). */
export type Rabbit = CommandData<RabbitProfileOutcome>;

/**
 * A rabbit command's error: the backend's `CommandError` in the normal case,
 * or an `Error` when the IPC call itself failed. It is a union so a consumer
 * has to narrow before reading `cause`, `retryable` or `message_key` — the
 * `CommandError` slot is nominal and can hold a plain string at runtime.
 */
type RabbitFailure<Outcome> = CommandFailure<Outcome> | Error;

/** The list read's error type. */
export type RabbitListFailure = RabbitFailure<RabbitListOutcome>;

/** The profile read's error type. */
export type RabbitProfileFailure = RabbitFailure<RabbitProfileOutcome>;

/** The list key's type, so the options cannot disagree with the factory. */
type RabbitListKey = ReturnType<typeof rabbitKeys.list>;

/** The profile key's type. */
type RabbitDetailKey = ReturnType<typeof rabbitKeys.detail>;

/**
 * One household's list, in the mode the key names (DASH-1, RAB-6).
 *
 * `mode: 'archived'` is the contract's `include_archived: true`, which selects
 * the archived list **only** — never both lists. The empty active list is a
 * value, not a failure: it is what DASH-8's empty state is keyed on, and a
 * household whose rabbits are all archived still has a reachable archived list.
 */
export function rabbitListQueryOptions(
  householdId: string,
  mode: RabbitListMode,
) {
  return queryOptions<
    RabbitSummary[],
    RabbitListFailure,
    RabbitSummary[],
    RabbitListKey
  >({
    queryKey: rabbitKeys.list(householdId, mode),
    queryFn: () =>
      listRabbits(householdId, mode === 'archived').then(unwrapCommandResult),
  });
}

/**
 * The active household's list, in one of its two modes.
 *
 * The empty household id in the disabled case is a key that is never fetched
 * and never written: `enabled` is false until `useActiveHousehold` answers, and
 * once it does the key becomes that household's real entry. Keeping the options
 * in one factory is what stops the hook and a loader from disagreeing about the
 * key or the flag.
 */
export function useRabbitList(mode: RabbitListMode) {
  const householdId = useActiveHousehold().data?.id;

  return useQuery({
    ...rabbitListQueryOptions(householdId ?? '', mode),
    enabled: householdId !== undefined,
  });
}

/** The active household's non-archived rabbits (DASH-1). */
export function useActiveRabbits() {
  return useRabbitList('active');
}

/** The active household's archived rabbits (RAB-6). */
export function useArchivedRabbits() {
  return useRabbitList('archived');
}

/**
 * One rabbit's full profile (DASH-6, RAB-5).
 *
 * Keyed by the rabbit id alone: `get_rabbit` takes no household and a rabbit
 * belongs to exactly one household, so the id already names the value. An
 * unknown or blank id is the backend's `validation` failure, not a `null`.
 */
export function rabbitProfileQueryOptions(rabbitId: string) {
  return queryOptions<Rabbit, RabbitProfileFailure, Rabbit, RabbitDetailKey>({
    queryKey: rabbitKeys.detail(rabbitId),
    queryFn: () => getRabbit(rabbitId).then(unwrapCommandResult),
  });
}

/** The profile of the rabbit the route names. */
export function useRabbitProfile(rabbitId: string) {
  return useQuery(rabbitProfileQueryOptions(rabbitId));
}
