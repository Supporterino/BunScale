/*
 * The rabbit writes (task 3.4; RAB-1, RAB-5, RAB-6, RAB-7, RAB-10).
 *
 * One hook per command, each over the handle `./api.ts` returns: creating,
 * updating, archiving, restoring and permanently deleting a rabbit. The
 * mutation's variable is the **handle**, not a bare id or profile, so a retry
 * is `run()` on the same operation and re-sends the `request_id` it minted
 * (D0.14). A corrected input is a new intent and therefore a new handle; these
 * hooks cannot mint one for a retry. `delete` is the one exception to the bare
 * handle: its answer is the contract's `()`, so it carries no household and no
 * rabbit id, and the cache could not be scoped from it — its variable adds the
 * two ids beside the handle.
 *
 * **Only what the write changed is invalidated**, and each set is argued from
 * the contract rather than from caution:
 *
 * - `create` answers the whole `Rabbit`, and touches the active list's row and
 *   the household's dashboard rows (a row carries the name, thumbnail and
 *   derived flags, DASH-1). Nothing else can be cached for it: the id is minted
 *   by the backend and only learned on the confirming answer, so no list entry
 *   or profile of that id pre-exists (RAB-1).
 * - `update` touches the same two, plus that rabbit's profile and its weight
 *   series. The series is named because `WeightTrend` is its points beside the
 *   `drop_threshold_percent` they were judged against, and each point's flag is
 *   derived from it — a saved threshold edit invalidating only the dashboard
 *   row would leave a mounted trend or history holding the old threshold and
 *   the old alerts, the two-surface disagreement R2, D1.3 and D1.13 forbid
 *   (`../weight/queries.ts` keeps the threshold in the cached series for the
 *   same reason). `weightKeys.series` is the whole per-rabbit prefix, so every
 *   window is refreshed and no other rabbit's is.
 *   The archived list is *not* touched by either: a new rabbit is active, and
 *   `update_rabbit` refuses an archived rabbit (RAB-5, D1.5).
 * - `archive` / `restore` move one rabbit between the two exclusive lists, so
 *   both modes of that household are invalidated, beside its profile
 *   (`is_archived` changed) and the dashboard rows (the rabbit enters or leaves
 *   DASH-1's non-archived set). No weight key is touched: the entries are
 *   unchanged (RAB-6's "archiving discards nothing", RAB-9).
 * - `delete` removes an archived rabbit for good, so the archived list, that
 *   rabbit's profile and its whole weight series (`weightKeys.series`, every
 *   window) are invalidated. Neither the active list nor the dashboard rows are
 *   named: `delete_rabbit` refuses an active rabbit (RAB-10), and DASH-1's rows
 *   are the non-archived rabbits, so a deletable rabbit was never a row.
 * - Another household's keys are never named: every key carries the household
 *   id or the rabbit id the answer or the variables named (NFR-10).
 *
 * **Failures take P0's one path.** There is no optimistic write and no pending
 * or queued write state (D1): `onSuccess` is the only place the cache moves,
 * and a failure invalidates nothing. A non-`validation` failure is recorded
 * through `recordFailedSave` (SYNC-2, NFR-8) with a handle that re-enters this
 * mutation, so a confirmed retry re-sends the same `request_id` *and* runs the
 * invalidation above — the same reason `HouseholdOnboardingScreen` wraps its
 * own handle. `validation` is deliberately not recorded: it belongs to the
 * form field that owns the input and is already in the mutation's error state
 * (`SaveFailureNotice`'s header, and `settleRetryFailure` clears it for exactly
 * that reason). These hooks are therefore the **recorder** for rabbit writes —
 * a form must not call `recordFailedSave` a second time.
 *
 * Types are derived from the wrappers rather than imported from
 * `src/bindings.ts`, because only `api.ts` may import the generated bindings
 * (project hard rule 2).
 */
import type { QueryClient, UseMutationResult } from '@tanstack/react-query';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { unwrapCommandResult } from '../../lib/command-result';
import type { RequestOperation } from '../../lib/request-id';
import { dashboardKeys } from '../dashboard/keys';
import { describeSaveFailure } from '../sync/failure';
import { recordFailedSave, type SaveFailure } from '../sync/save-retry';
import { weightKeys } from '../weight/keys';
import type {
  archiveRabbit,
  createRabbit,
  deleteRabbit,
  restoreRabbit,
  updateRabbit,
} from './api';
import { rabbitKeys } from './keys';
import type { Rabbit } from './queries';

/** `create_rabbit`'s handle, as `api.ts` returns it (D0.14). */
type CreateOperation = ReturnType<typeof createRabbit>;

/** `update_rabbit`'s handle. */
type UpdateOperation = ReturnType<typeof updateRabbit>;

/** `archive_rabbit`'s handle. */
type ArchiveOperation = ReturnType<typeof archiveRabbit>;

/** `restore_rabbit`'s handle. */
type RestoreOperation = ReturnType<typeof restoreRabbit>;

/** `delete_rabbit`'s handle. */
type DeleteOperation = ReturnType<typeof deleteRabbit>;

/** A write's handle: `run()` again re-sends the request id it minted. */
type WriteHandle<Value> = RequestOperation<Value, SaveFailure>;

/**
 * One delete intent: the handle and the ids its `()` answer cannot carry.
 *
 * The household scopes the archived list key, and the rabbit id scopes the
 * profile and the weight series. Both are the caller's own intent, captured
 * when `mutate` is called, so a household switched after the call cannot move
 * the invalidation to the wrong cache entries.
 */
type DeleteVariables = {
  readonly householdId: string;
  readonly rabbitId: string;
  readonly operation: DeleteOperation;
};

/**
 * The four writes whose mutation variable is the handle itself.
 *
 * They answer the whole `Rabbit`, so the invalidation reads the ids from the
 * confirmed value and the variable needs to carry nothing else. `delete` cannot
 * use it (see `DeleteVariables`).
 */
function sameOperation<Value>(
  operation: WriteHandle<Value>,
): WriteHandle<Value> {
  return operation;
}

/**
 * One rabbit write's shared plumbing: run the handle, invalidate on the
 * confirmed post-state, record a failure P0's retry surface can present.
 *
 * `onConfirmed` is only reached on success, which is what keeps a failed write
 * from touching the cache at all (D1). The recorded retry calls this mutation's
 * `mutateAsync`, not the bare handle, so a retry that lands runs `onConfirmed`
 * and a retry refused as `validation` lands in the mutation's own `error` where
 * the form field shows it.
 *
 * `onError` is defined before `useMutation` returns the result it belongs to,
 * so the retry reaches that result through the binding this very initializer
 * creates. Reading it there is safe: a mutation can only be started through
 * that object, so it exists before any handler runs.
 */
function useRabbitWrite<Value, Variables>(
  operationOf: (variables: Variables) => WriteHandle<Value>,
  onConfirmed: (
    client: QueryClient,
    value: Value,
    variables: Variables,
  ) => void,
): UseMutationResult<Value, SaveFailure, Variables> {
  const queryClient = useQueryClient();

  const mutation: UseMutationResult<Value, SaveFailure, Variables> =
    useMutation<Value, SaveFailure, Variables>({
      mutationFn: (variables) =>
        operationOf(variables).run().then(unwrapCommandResult),
      onSuccess: (value, variables) => {
        onConfirmed(queryClient, value, variables);
      },
      onError: (error, variables) => {
        if (describeSaveFailure(error).cause === 'validation') {
          return;
        }

        recordFailedSave<Value>(
          {
            requestId: operationOf(variables).requestId,
            run: () =>
              mutation.mutateAsync(variables).then((value) => ({
                status: 'ok' as const,
                data: value,
              })),
          },
          error,
        );
      },
    });

  return mutation;
}

/**
 * A created rabbit: its active-list row and the household's dashboard row.
 *
 * Both read a projection of the confirmed `Rabbit` (RAB-1, DASH-1). There is
 * nothing else to invalidate: the id is minted by this command and only learned
 * from this answer, so neither a profile nor a list entry of it can already be
 * cached, and the rabbit is active, so it is not in the archived list.
 */
function invalidateCreatedRabbit(client: QueryClient, rabbit: Rabbit): void {
  void client.invalidateQueries({
    queryKey: rabbitKeys.list(rabbit.household_id, 'active'),
  });
  void client.invalidateQueries({
    queryKey: dashboardKeys.rows(rabbit.household_id),
  });
}

/**
 * An updated rabbit: its active-list row, its profile, its dashboard row and
 * its weight series.
 *
 * The series belongs to this set because a `WeightTrend` is its points beside
 * the `drop_threshold_percent` they were judged against (D1.3, D1.13): a saved
 * threshold edit must not leave a mounted trend or history presenting alerts
 * derived from the previous threshold. `weightKeys.series` is the per-rabbit
 * prefix, so both windows are refreshed and no other rabbit's is. The archived
 * list is not named — `update_rabbit` refuses an archived rabbit (RAB-5).
 */
function invalidateUpdatedRabbit(client: QueryClient, rabbit: Rabbit): void {
  void client.invalidateQueries({
    queryKey: rabbitKeys.list(rabbit.household_id, 'active'),
  });
  void client.invalidateQueries({ queryKey: rabbitKeys.detail(rabbit.id) });
  void client.invalidateQueries({
    queryKey: dashboardKeys.rows(rabbit.household_id),
  });
  void client.invalidateQueries({ queryKey: weightKeys.series(rabbit.id) });
}

/**
 * A rabbit that moved between the two lists: both modes, its profile, the
 * dashboard rows.
 *
 * Both list modes are named because the two keys are distinct by design
 * (`./keys`): the rabbit leaves one and enters the other, so invalidating one
 * mode alone would leave the other showing a row that is no longer there
 * (RAB-6, RAB-7).
 */
function invalidateArchivedState(client: QueryClient, rabbit: Rabbit): void {
  void client.invalidateQueries({
    queryKey: rabbitKeys.list(rabbit.household_id, 'active'),
  });
  void client.invalidateQueries({
    queryKey: rabbitKeys.list(rabbit.household_id, 'archived'),
  });
  void client.invalidateQueries({ queryKey: rabbitKeys.detail(rabbit.id) });
  void client.invalidateQueries({
    queryKey: dashboardKeys.rows(rabbit.household_id),
  });
}

/**
 * A permanently deleted rabbit: the archived list, its profile, its weight
 * series.
 *
 * The weight series is invalidated as a whole (`weightKeys.series`) rather than
 * per range, because every window of that rabbit's history is gone; naming a
 * range would leave the other windows cached (RAB-10). Neither the active list
 * nor the dashboard rows are named: `delete_rabbit` refuses an active rabbit
 * (RAB-10), and DASH-1's rows are the non-archived rabbits, so the deleted
 * rabbit was never a row.
 */
function invalidateDeleted(
  client: QueryClient,
  variables: DeleteVariables,
): void {
  void client.invalidateQueries({
    queryKey: rabbitKeys.list(variables.householdId, 'archived'),
  });
  void client.invalidateQueries({
    queryKey: rabbitKeys.detail(variables.rabbitId),
  });
  void client.invalidateQueries({
    queryKey: weightKeys.series(variables.rabbitId),
  });
}

/**
 * Add a rabbit to a household (RAB-1, RAB-2, RAB-3).
 *
 * Start it with `mutate(createRabbit(householdId, profile))`; re-submitting the
 * same handle is the retry, and a corrected profile is a new handle. A blank
 * name is refused by the backend as `validation` (RAB-2) and stays in the
 * mutation's error for the field; every other cause is recorded for the shell.
 */
export function useCreateRabbit() {
  return useRabbitWrite<Rabbit, CreateOperation>(
    sameOperation,
    (client, rabbit) => {
      invalidateCreatedRabbit(client, rabbit);
    },
  );
}

/**
 * Replace an active rabbit's editable field set (RAB-5, WT-7).
 *
 * A whole-profile replacement, so the form submits every field. The threshold
 * is one of them, so the dashboard rows (whose drop flag is derived from it on
 * read, D1.3) and that rabbit's weight series (points plus the threshold they
 * were judged against, D1.13) are invalidated with the profile. Writes to an
 * archived rabbit are refused as `validation` (D1.5), which is why no archived
 * list is named.
 */
export function useUpdateRabbit() {
  return useRabbitWrite<Rabbit, UpdateOperation>(
    sameOperation,
    (client, rabbit) => {
      invalidateUpdatedRabbit(client, rabbit);
    },
  );
}

/**
 * Archive a rabbit: the reversible soft delete (RAB-6, RAB-9).
 *
 * Start it with `mutate(archiveRabbit(rabbitId))`. Nothing is discarded, so the
 * history stays cached and reachable; only the archived marker, the two lists
 * and the dashboard row move.
 */
export function useArchiveRabbit() {
  return useRabbitWrite<Rabbit, ArchiveOperation>(
    sameOperation,
    (client, rabbit) => {
      invalidateArchivedState(client, rabbit);
    },
  );
}

/** Return an archived rabbit to the active list (RAB-7). */
export function useRestoreRabbit() {
  return useRabbitWrite<Rabbit, RestoreOperation>(
    sameOperation,
    (client, rabbit) => {
      invalidateArchivedState(client, rabbit);
    },
  );
}

/**
 * Remove an archived rabbit and its data for good (RAB-10).
 *
 * Start it with `mutate({ householdId, rabbitId, operation: deleteRabbit(rabbitId) })`;
 * the caller's confirmation (task 4.5) is the only path that may call this. The
 * contract restricts deletion to an archived rabbit, so an active rabbit is
 * refused as `validation` and stays active.
 */
export function useDeleteRabbit() {
  return useRabbitWrite<null, DeleteVariables>(
    (variables) => variables.operation,
    (client, _value, variables) => {
      invalidateDeleted(client, variables);
    },
  );
}
