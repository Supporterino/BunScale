/*
 * Retry state for a failed save (task 3.4; SYNC-2, NFR-8, D0.14).
 *
 * SYNC-2's rule is not "try again" but "the save is not presented as saved, the
 * cause is named, and the retry re-issues the *same* operation". The operation
 * identity (`request_id`) is what makes that retry safe: it is semantic, and the
 * shared service deduplicates on it (D0.14), so a second attempt that minted a
 * new id would be a second write. `RequestOperation` from
 * `src/lib/request-id.ts` already holds the minted id, which is why this store
 * holds the **handle** and not the arguments: `retryFailedSave()` calls
 * `operation.run()` again, so the id is re-sent by construction.
 *
 * Why a module store rather than component state: a failed save outlives the
 * form that started it. The surface that offers the retry (task 4.5) may be
 * rendered elsewhere — or after a navigation — and the operation must survive
 * with it; the alternative would be a new dependency (the conventions allow a
 * small store only when justified) or a prop chain through the screens. React's
 * `useSyncExternalStore` is the dependency-free way to make one module-scope slot
 * observable, so that is what this is.
 *
 * One slot is deliberate: P0 has one write that reaches the service
 * (`create_household`), so a second concurrent failed save would replace the
 * first rather than queue behind it. The limit is stated rather than implied; a
 * queue is P1's if item writes make it reachable.
 *
 * Clear paths, stated exactly: the slot is cleared by an `ok` for the attempt
 * the slot still holds — and then that attempt's recorded success callback
 * runs — by an explicit dismissal, which is **refused while a retry is running**
 * because the operation is in flight and may land, or by a retry the backend
 * refuses with `validation`, which is the form field's own error rather than a
 * save the shell presents (task 7.7). Every other outcome, including a typed
 * `CommandError`, an IPC-level `Error` and a value that is neither, keeps a
 * failure on record (`unknown` when unclassifiable). A result that arrives for
 * an attempt the slot no longer holds is dropped: a newer failure owns the
 * surface by then, and adopting the older one's success would claim a save the
 * newer attempt never completed.
 */
import { useSyncExternalStore } from 'react';
import type { CommandResult, RequestOperation } from '../../lib/request-id';
import type { SaveCause, SaveCommandError, SaveMessageKey } from './failure';
import { describeSaveFailure } from './failure';

/** The error type a save operation reports through (the wrappers' union). */
export type SaveFailure = SaveCommandError | Error;

/** One failed save, with everything a retry surface needs to render and retry. */
export type FailedSave = {
  /** The failed operation; re-running it re-sends the same `request_id`. */
  readonly operation: RequestOperation<unknown, SaveFailure>;
  /** The backend's failure object, or `null` when the value was not one. */
  readonly failure: SaveCommandError | null;
  /** The cause its copy comes from (never the backend's raw key). */
  readonly cause: SaveCause;
  /** The copy key the surface resolves (task 3.7). */
  readonly messageKey: SaveMessageKey;
  /** Whether the backend says re-issuing can succeed. */
  readonly retryable: boolean;
};

/** What a surface subscribes to. */
export type SaveRetryState = {
  /** The save waiting for a retry, or `null` when there is none. */
  readonly failure: FailedSave | null;
  /** Whether a retry is running right now. */
  readonly retrying: boolean;
};

let state: SaveRetryState = { failure: null, retrying: false };
const listeners = new Set<() => void>();

/*
 * The recorded success callback. The store cannot be generic — one slot holds
 * operations of different value types — so the callback is remembered as
 * `unknown`-taking, and each recorder adapts its own `T`. The cast is sound by
 * construction: a callback is only ever called with the value of the operation
 * it was recorded with, which is its `T`.
 */
let onSaved: ((value: unknown) => void) | null = null;

function publish(next: SaveRetryState): void {
  state = next;

  for (const listener of listeners) {
    listener();
  }
}

/** The current state, for a non-React reader (and for the hook below). */
export function getSaveRetryState(): SaveRetryState {
  return state;
}

/** Subscribe to changes; returns the unsubscribe a `useEffect` must call. */
export function subscribeToSaveRetry(listener: () => void): () => void {
  listeners.add(listener);

  return () => {
    listeners.delete(listener);
  };
}

/** The retry state as a component reads it. */
export function useSaveRetry(): SaveRetryState {
  return useSyncExternalStore(
    subscribeToSaveRetry,
    getSaveRetryState,
    getSaveRetryState,
  );
}

/**
 * Record a save that did not complete.
 *
 * `onSaved` is what the first attempt's success would have done (task 4.1's
 * navigation, for instance); it runs only after a retry is confirmed by the
 * backend, never before. Recording a second failure replaces the first: there is
 * one slot (see the header), and the newer failure supersedes any retry already
 * in flight — that attempt's result is dropped when it arrives.
 */
export function recordFailedSave<T>(
  operation: RequestOperation<T, SaveFailure>,
  error: unknown,
  onSavedCallback?: (value: T) => void,
): FailedSave {
  const description = describeSaveFailure(error);
  const failure: FailedSave = {
    operation,
    failure: description.failure,
    cause: description.cause,
    messageKey: description.messageKey,
    retryable: description.retryable,
  };

  onSaved =
    onSavedCallback === undefined
      ? null
      : (value) => {
          onSavedCallback(value as T);
        };

  publish({ failure, retrying: false });

  return failure;
}

/**
 * Drop the recorded failure, and with it the retry.
 *
 * Refused while a retry is running: that operation is in flight and may land, so
 * dropping the slot would discard a confirmed write and the callback that
 * belongs to it. A surface has `retrying` and can disable its control until the
 * attempt settles.
 */
export function dismissFailedSave(): void {
  if (state.retrying) {
    return;
  }

  onSaved = null;
  publish({ failure: null, retrying: false });
}

/**
 * Re-issue the recorded save with the same operation handle.
 *
 * The handle is the identity: `operation.run()` sends the `request_id` minted
 * when the operation was created (D0.14). On `ok` the slot is cleared and that
 * attempt's recorded success callback runs. A `validation` refusal clears the
 * slot without recording it: that cause belongs to the form field that owns the
 * input, and recording it would put a second live region on screen (the field's
 * alert and this surface) for one refusal. Every other failure updates the
 * slot's cause and keeps it, so the surface shows the new attempt's cause; when
 * that cause is retryable the surface offers another retry. A rejected `run()`
 * (an IPC-level `Error`) is treated the same way — a save that produced no
 * answer has not succeeded — and is described as `unknown` rather than
 * re-thrown, because the surface it belongs to is this store's caller; nothing
 * is swallowed silently, it is recorded and observable.
 *
 * The attempt is captured before the `await` and the slot is compared by
 * identity afterwards. `recordFailedSave` always publishes a new record and a
 * dismissal clears the slot, so a result that no longer belongs to the current
 * attempt is dropped: it cannot clear a newer failure, resurrect a dismissed
 * one, or call a callback that has moved on.
 *
 * A call while a retry is already running is a no-op, so a double press cannot
 * start two attempts (they would share the id, but the second answer would
 * overwrite the first's state).
 */
export async function retryFailedSave(): Promise<void> {
  const attempt = state.failure;

  if (attempt === null || state.retrying) {
    return;
  }

  const onSavedForAttempt = onSaved;

  publish({ failure: attempt, retrying: true });

  let outcome: CommandResult<unknown, SaveFailure>;

  try {
    outcome = await attempt.operation.run();
  } catch (error) {
    if (state.failure !== attempt) {
      return;
    }

    settleRetryFailure(attempt, error);
    return;
  }

  // Superseded while the attempt ran (see the doc above): drop the result.
  if (state.failure !== attempt) {
    return;
  }

  if (outcome.status === 'ok') {
    onSaved = null;
    publish({ failure: null, retrying: false });
    onSavedForAttempt?.(outcome.data);

    return;
  }

  settleRetryFailure(attempt, outcome.error);
}

/**
 * Settle a retry that did not succeed.
 *
 * The caller has already established that `attempt` is still the slot's
 * record, so a superseded retry cannot clear or overwrite a newer failure here.
 * `validation` is the one cause the shell does not present: it belongs to the
 * form field, which shows it from its own mutation error. Clearing here is
 * therefore only sound for a recorder whose own surface renders that refusal:
 * the create flow's `mutateAsync` does (the mutation error drives the field).
 * A future recorder must keep that true — a bare handle would leave the
 * refusal shown nowhere. Recording it here instead would create a second live
 * region and a non-retryable notice with no action. Every other cause replaces
 * the record's description and keeps it.
 */
function settleRetryFailure(attempt: FailedSave, error: unknown): void {
  const description = describeSaveFailure(error);

  if (description.cause === 'validation') {
    onSaved = null;
    publish({ failure: null, retrying: false });

    return;
  }

  publish({
    failure: { ...attempt, ...description },
    retrying: false,
  });
}

/**
 * Discard the store's state without notifying subscribers.
 *
 * For a spec only: the store lives as long as the module, so one test's record
 * must not leak into the next. It bypasses the `retrying` refusal on purpose —
 * the only way a spec can recover from an earlier one that left an attempt
 * running — and no app path needs it.
 */
export function resetSaveRetry(): void {
  onSaved = null;
  state = { failure: null, retrying: false };
}
