/*
 * The weight writes (task 3.5; WT-1, WT-3, WT-5, DM-1).
 *
 * One hook per command — create, update, delete — each over the handle
 * `./api.ts` returns (D0.14). Three facts shape them:
 *
 * **The display-unit boundary is here, at the entry** (WT-3, DM-1, I1). No
 * weight command takes or returns a unit: grams are canonical on the wire, so
 * the mutation's variable is what the form holds — the text the person typed
 * plus the unit that text is in — and the command is called with
 * `parseDisplayWeight`'s whole-gram result. A caller never builds a
 * `WeightEntryDraft` itself; doing so would put the conversion in a second place
 * and leave two definitions of what a stored value means. The unit travels
 * **with the intent** rather than being read when the command runs: a retry
 * re-sends the number the person typed, and if the device preference changed in
 * between, re-reading it would store a different weight than the one they
 * confirmed. A value `parseDisplayWeight` refuses (sub-gram precision, a comma,
 * a unit suffix, an empty field, wider than the wire type) is reported as the
 * same `validation` cause the command would use, so the form has one path for
 * both and no command is reached (see `unreadableWeight`).
 *
 * **The identity has to survive the parse.** `create_weight_entry` and
 * `update_weight_entry` bind their draft when the handle is minted, so the
 * handle can only be built after the parse — while a retry must re-send the same
 * `request_id` (D0.14: a second id is a second write, R6). The handle is
 * therefore minted once per intent object and remembered by that object's
 * identity (`operationFor`), then dropped once the write lands: `mutate` and the
 * recorded retry pass the same variables object, so the retry hits the same
 * handle, while a corrected value — or a fresh submission — is a new intent and
 * gets its own id. `delete` needs no parse and uses the same
 * shape for uniformity.
 *
 * **Only what the write changed is invalidated**, and the set is the same for
 * all three because the three change the same two reads:
 *
 * - `weightKeys.series(rabbitId)` — every window. A new point can enter any of
 *   them, a corrected value changes the points and the flags derived from them,
 *   and a corrected timestamp can move a point between windows or out of one
 *   (WT-6, WT-7). Naming a range would leave the other windows stale.
 * - `dashboardKeys.rows(householdId)` — a row carries `last_weighed_at` and the
 *   derived drop flag (DASH-1, D1.3, D1.13), and every one of the three writes
 *   can move them (a create enters, a delete can remove the newest entry, an
 *   edit can restamp it).
 *
 * Nothing else is named: the rabbit's own keys are untouched, because a weight
 * write changes no profile field (a `Rabbit` carries no weight value) and no
 * list row (the name and thumbnail it shows are unchanged). Another rabbit's
 * series and another household's rows are never touched (NFR-10).
 *
 * **The entry detail is reached through the series, not named again (task
 * 4.19).** `./keys.ts` keys one entry's detail under its rabbit's series prefix,
 * so the `weightKeys.series(rabbitId)` invalidation above already refetches the
 * detail an `update` or a `delete` touched. An explicit `weightKeys.detail(...)`
 * call here would invalidate a subset of what is already invalid, and reading as
 * if it did more is worse than not having it.
 *
 * **Failures take P0's one path**, exactly as task 3.4's rabbit writes do (D1,
 * SYNC-2): no optimistic write and no pending state, `onSuccess` is the only
 * place the cache moves, and a failed write invalidates nothing. A
 * non-`validation` failure is recorded through `recordFailedSave` with a handle
 * that re-enters this mutation, so a confirmed retry re-sends the same
 * `request_id` *and* runs the invalidation above. `validation` is deliberately
 * not recorded: it belongs to the form field that owns the input and is already
 * in the mutation's error state (task 3.6). These hooks are therefore the
 * **recorder** for weight writes — a form must not call `recordFailedSave` a
 * second time.
 *
 * Types are derived from the wrappers rather than imported from
 * `src/bindings.ts`, because only `api.ts` may import the generated bindings
 * (project hard rule 2).
 */
import type { QueryClient, UseMutationResult } from '@tanstack/react-query';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { CommandData, CommandFailure } from '../../lib/command-result';
import { unwrapCommandResult } from '../../lib/command-result';
import type { RequestOperation } from '../../lib/request-id';
import { dashboardKeys } from '../dashboard/keys';
import type { SaveCommandError } from '../sync/failure';
import { describeSaveFailure, SAVE_CAUSE_MESSAGE_KEYS } from '../sync/failure';
import type { SaveFailure } from '../sync/save-retry';
import { recordFailedSave } from '../sync/save-retry';
import {
  createWeightEntry,
  deleteWeightEntry,
  parseDisplayWeight,
  updateWeightEntry,
} from './api';
import { weightKeys } from './keys';

/** `create_weight_entry`'s handle, as `api.ts` returns it (D0.14). */
type CreateOperation = ReturnType<typeof createWeightEntry>;

/** The contract's `RabbitId`, derived from the wrapper that takes it. */
type RabbitId = Parameters<typeof createWeightEntry>[0];

/** The contract's `WeightEntryId`, derived from the wrapper that takes it. */
type WeightEntryId = Parameters<typeof updateWeightEntry>[0];

/** The contract's `WeightEntryDraft`, derived from the wrapper that takes it. */
type WeightEntryDraft = Parameters<typeof createWeightEntry>[1];

/** The saved entry create and update answer with. */
type WeightEntry = CommandData<Awaited<ReturnType<CreateOperation['run']>>>;

/**
 * The display unit the person is typing in, derived from the conversion whose
 * parameter it is — the same `WeightUnit` SET-4 resolves and `api.ts` parses.
 */
type WeightUnit = Parameters<typeof parseDisplayWeight>[1];

/** A weight command's failure, derived from a wrapper rather than bindings. */
type WeightCommandError = CommandFailure<
  Awaited<ReturnType<CreateOperation['run']>>
>;

/**
 * What the form holds, and what the entry boundary converts (WT-3, DM-1).
 *
 * `text` is what the person typed, in `unit`; everything else is the draft's
 * other field. `timestamp: null` records the moment of creation on create and
 * **keeps the entry's existing timestamp** on update (WT-1, WT-5) — an edit
 * never restamps a back-dated entry.
 */
export type WeightEntryInput = {
  /** The typed value, unparsed. The unit is applied here, never sent. */
  readonly text: string;
  /** The display unit `text` is in, as SET-4 resolved it for this intent. */
  readonly unit: WeightUnit;
  /** The person's instant, or `null` for "now" / "keep" (WT-1, WT-5). */
  readonly timestamp: WeightEntryDraft['timestamp'];
  /** The optional note, as typed. */
  readonly note: WeightEntryDraft['note'];
  /** WT-4's confirmation for a value above the plausibility bound. */
  readonly outlierConfirmed: WeightEntryDraft['outlier_confirmed'];
};

/**
 * The scope a weight write's answer cannot carry.
 *
 * `WeightEntry` has no household id and `delete_weight_entry` answers `()`, so
 * the ids that scope the invalidation are the caller's own intent, captured when
 * `mutate` is called: a household or rabbit switched afterwards cannot move the
 * invalidation to the wrong cache entries (NFR-10). `householdId` is a plain
 * string like every key factory's.
 */
type WriteScope = {
  /** Scopes the dashboard rows key (DASH-1). */
  readonly householdId: string;
  /** Scopes the weight series prefix (WT-6). */
  readonly rabbitId: RabbitId;
};

/** A new entry: the person's draft, scoped to its rabbit and household. */
export type CreateWeightEntryVariables = WriteScope & {
  readonly input: WeightEntryInput;
};

/** A correction: the entry's id and the whole replacement draft. */
export type UpdateWeightEntryVariables = WriteScope & {
  readonly entryId: WeightEntryId;
  readonly input: WeightEntryInput;
};

/** A removal: the entry's id, and the rabbit and household it belonged to. */
export type DeleteWeightEntryVariables = WriteScope & {
  readonly entryId: WeightEntryId;
};

/**
 * The refusal for typed text that is not a weight this product can store
 * (WT-3, WT-4).
 *
 * `parseDisplayWeight` is the only gate before the command. Its `null` covers
 * sub-gram precision, a comma, a unit suffix, an empty field and a value wider
 * than the wire type — all of them "this input cannot be stored", which is the
 * command's `validation` cause. Reporting it in that shape gives the form one
 * validation path for both its own parse and the command's refusals, and it is
 * left in the mutation's error rather than recorded for the shell (SYNC-2's
 * rule). `request_id` is `null` because no command was reached: there is no
 * operation to re-issue, and claiming one would suggest a write was sent.
 *
 * A non-positive value is deliberately **not** refused here: `-5` parses to
 * `-5000` g and the command refuses it (WT-4), so the form and the boundary
 * refuse it by the one rule they share rather than the boundary re-deriving it.
 */
function unreadableWeight(): SaveCommandError {
  return {
    cause: 'validation',
    retryable: false,
    message_key: SAVE_CAUSE_MESSAGE_KEYS.validation,
    request_id: null,
  };
}

/**
 * The contract's draft: the typed value as whole grams, everything else as given.
 *
 * This is the display-unit boundary (WT-3, DM-1, I1). `value_grams` is the
 * parse's exact integer — `1.2` in kilograms becomes `1200`, not a float
 * product — and nothing on the draft names a unit.
 */
function draftOf(input: WeightEntryInput): WeightEntryDraft {
  const grams = parseDisplayWeight(input.text, input.unit);

  if (grams === null) {
    throw unreadableWeight();
  }

  return {
    value_grams: grams,
    timestamp: input.timestamp,
    note: input.note,
    outlier_confirmed: input.outlierConfirmed,
  };
}

/*
 * The handle of each intent, minted when it is first attempted.
 *
 * A `WeakMap` keyed by the variables object rather than a field on it: the
 * object is the caller's and may be frozen or shared, and the entry has to
 * disappear with it. The types are widened because one map serves all three
 * hooks; each slot is only ever read by `operationFor` with the variables type
 * that created it, so the cast back is sound by construction.
 */
const operations = new WeakMap<
  object,
  RequestOperation<unknown, WeightCommandError>
>();

/**
 * The handle for one intent, minted on the first attempt and re-used by its
 * retry (D0.14).
 *
 * `mutationFn` runs again for a retry — `recordFailedSave` re-enters the
 * mutation rather than calling the bare handle — so building a handle
 * unconditionally would mint a second `request_id` and turn a retry into a
 * second write. The variables object is the retry's identity: TanStack hands the
 * same object back, and a corrected value is a new object.
 */
function operationFor<Value>(
  intent: object,
  create: () => RequestOperation<Value, WeightCommandError>,
): RequestOperation<Value, WeightCommandError> {
  const existing = operations.get(intent);

  if (existing !== undefined) {
    return existing as RequestOperation<Value, WeightCommandError>;
  }

  const operation = create();
  operations.set(intent, operation);

  return operation;
}

/**
 * What a confirmed weight write supersedes: that rabbit's whole series and the
 * household's dashboard rows.
 *
 * One function rather than three because the three writes change the same two
 * reads only (`weightKeys.series` covers every window, so the trend and the
 * history are both refreshed and no other rabbit's is).
 */
function invalidateWeightWrite(client: QueryClient, scope: WriteScope): void {
  void client.invalidateQueries({
    queryKey: weightKeys.series(scope.rabbitId),
  });
  void client.invalidateQueries({
    queryKey: dashboardKeys.rows(scope.householdId),
  });
}

/**
 * One weight write's shared plumbing: run the handle, invalidate on the
 * confirmed post-state, record a failure P0's retry surface can present.
 *
 * `onConfirmed` is only reached on success, which is what keeps a failed write
 * from touching the cache at all (D1). The recorded retry calls this mutation's
 * `mutateAsync`, not the bare handle, so a retry that lands invalidates as a
 * first-attempt success does, and a retry refused as `validation` lands in the
 * mutation's own `error` where the form field shows it. `onError` is defined
 * before `useMutation` returns the result it belongs to, so the retry reaches
 * that result through the binding this very initializer creates.
 *
 * The intent's handle is dropped once the write is confirmed: it has landed, so
 * there is nothing left to re-issue, and a form that submits the same object
 * again is making a second entry rather than re-issuing the first — which the
 * shared service would otherwise deduplicate on the old `request_id`.
 */
function useWeightWrite<Value, Variables extends object>(
  operationOf: (
    variables: Variables,
  ) => RequestOperation<Value, WeightCommandError>,
  onConfirmed: (client: QueryClient, variables: Variables) => void,
): UseMutationResult<Value, SaveFailure, Variables> {
  const queryClient = useQueryClient();

  const mutation: UseMutationResult<Value, SaveFailure, Variables> =
    useMutation<Value, SaveFailure, Variables>({
      mutationFn: (variables) =>
        operationOf(variables).run().then(unwrapCommandResult),
      onSuccess: (_value, variables) => {
        operations.delete(variables);
        onConfirmed(queryClient, variables);
      },
      onError: (error, variables) => {
        const description = describeSaveFailure(error);

        if (description.cause === 'validation') {
          return;
        }

        recordFailedSave<Value>(
          {
            /*
             * The echoed id first: a failure names the operation the command
             * actually ran under, and that is the identity the retry surface
             * must present (D0.14). The map is the fallback, not the source — a
             * concurrent attempt of the same intent can have succeeded and
             * dropped the slot, and minting a replacement id for the record
             * would name an operation that never ran. Every write carries its
             * id, so the fallback covers only a value that arrived without one.
             */
            requestId:
              description.failure?.request_id ??
              operationOf(variables).requestId,
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
 * Record a weight entry (WT-1, WT-3, DM-1).
 *
 * Start it with `mutate({ householdId, rabbitId, input })`, where `input.text`
 * is what the person typed and `input.unit` is the unit SET-4 resolved for it.
 * The draft is built here — `text` becomes whole grams and no unit is sent. A
 * retry is the recorded retry's `mutateAsync` on the same variables object, so
 * it re-sends the `request_id` the first attempt minted; a corrected value is a
 * new variables object and a new operation.
 *
 * A value of zero or less, an unconfirmed value above
 * `PLAUSIBILITY_BOUND_GRAMS`, or a write to an archived rabbit is the command's
 * `validation` refusal and stays in this mutation's error (WT-4, D1.5); every
 * other cause is recorded for the shell (SYNC-2).
 */
export function useCreateWeightEntry() {
  return useWeightWrite<WeightEntry, CreateWeightEntryVariables>(
    (variables) =>
      operationFor(variables, () =>
        createWeightEntry(variables.rabbitId, draftOf(variables.input)),
      ),
    (client, variables) => {
      invalidateWeightWrite(client, variables);
    },
  );
}

/**
 * Correct a past entry (WT-5, DM-1).
 *
 * Start it with `mutate({ householdId, rabbitId, entryId, input })`. The draft
 * is the whole editable set, as the command's replacement semantics require; a
 * `timestamp: null` keeps the entry's existing instant, so correcting a value
 * never restamps a back-dated entry (WT-5). The invalidation set is create's:
 * that rabbit's series, which the detail the correction started from sits under,
 * so the corrected point, the flags derived from it, the dashboard row and the
 * entry's own detail all refresh without a second key being named.
 */
export function useUpdateWeightEntry() {
  return useWeightWrite<WeightEntry, UpdateWeightEntryVariables>(
    (variables) =>
      operationFor(variables, () =>
        updateWeightEntry(variables.entryId, draftOf(variables.input)),
      ),
    (client, variables) => {
      invalidateWeightWrite(client, variables);
    },
  );
}

/**
 * Remove a weight entry (WT-5).
 *
 * Start it with `mutate({ householdId, rabbitId, entryId })`. The answer is the
 * contract's `()`, so the ids come from the intent (`WriteScope`): the series
 * whose points and flags the removal changes — its entry details included, which
 * then cannot answer for an entry that no longer exists — and the dashboard row
 * whose last-weighed instant or alert it can move. An unknown entry, or one of
 * an archived rabbit, is refused as `validation` and stays in this mutation's
 * error (D1.5).
 */
export function useDeleteWeightEntry() {
  return useWeightWrite<null, DeleteWeightEntryVariables>(
    (variables) =>
      operationFor(variables, () => deleteWeightEntry(variables.entryId)),
    (client, variables) => {
      invalidateWeightWrite(client, variables);
    },
  );
}
