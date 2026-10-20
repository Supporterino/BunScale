/*
 * The one failure surface, from the frontend's side (task 3.4; D0.6, NFR-8).
 *
 * Every command resolves to `{ status: 'error', error }` carrying the backend's
 * `CommandError`, and every product failure has exactly one cause from the
 * frozen, exhaustive `SaveCause` list. This module is where a cause becomes
 * **copy**: one message key per cause, in a `Record` keyed by the cause type, so
 * a cause added to the contract turns into a compile error here instead of a
 * silent fall-through to a generic message.
 *
 * The backend's own `message_key` is deliberately not what the UI renders. It is
 * per-cause too (`src-tauri/src/ipc/failure.rs`), but it also names the stub
 * placeholder `error.not_implemented`, which is not product copy; deriving the
 * key from the cause means a stub — or any future unclassified value — can only
 * ever be shown as `unknown`. The cost is stated rather than hidden: this map
 * and `failure.rs` must stay in step, the backend's per-cause distinctness is
 * asserted in task 5.10, and renaming a key on either side is a contract change
 * rather than an implementation detail.
 *
 * `retryable` is not re-derived here either. It is the backend's per-cause
 * policy carried on the failure itself, and a second policy in the frontend
 * could only disagree with it.
 */
import type { CommandFailure } from '../../lib/command-result';
import type { getServiceStatus } from './api';

/** A wrapper outcome, used only to derive the contract's failure types. */
type StatusOutcome = Awaited<ReturnType<typeof getServiceStatus>>;

/** The contract's `CommandError`, derived from a wrapper (project hard rule 2). */
export type SaveCommandError = CommandFailure<StatusOutcome>;

/** The contract's frozen, exhaustive failure taxonomy. */
export type SaveCause = SaveCommandError['cause'];

/** The i18n key of each cause's copy; task 3.7 resolves the strings. */
export const SAVE_CAUSE_MESSAGE_KEYS = {
  connectivity: 'error.connectivity',
  no_account: 'error.no_account',
  allowance_exceeded: 'error.allowance_exceeded',
  rate_limited: 'error.rate_limited',
  timeout: 'error.timeout',
  validation: 'error.validation',
  unknown: 'error.unknown',
} as const satisfies Record<SaveCause, string>;

/** Every message key this surface can produce — one per cause. */
export type SaveMessageKey = (typeof SAVE_CAUSE_MESSAGE_KEYS)[SaveCause];

/** What a failure is shown as: the cause, its key, and whether to offer a retry. */
export type SaveFailureDescription = {
  /** The cause the copy is derived from; unclassified values become `unknown`. */
  readonly cause: SaveCause;
  /** The key for that cause — never the backend's raw `message_key`. */
  readonly messageKey: SaveMessageKey;
  /**
   * Whether the backend named this failure retryable. `false` for anything that
   * did not arrive as a `CommandError`, because no cause was reported and no
   * retry can be promised (D0.6).
   */
  readonly retryable: boolean;
  /** The backend's failure object, or `null` when the value was not one. */
  readonly failure: SaveCommandError | null;
};

/**
 * Whether `value` is the contract's `CommandError` rather than an IPC-level
 * rejection. The union's error slot is nominal (`src/lib/request-id.ts`), and
 * Tauri can put a plain string there for an unknown or denied command, so this
 * has to be checked at runtime before `cause` is read.
 */
export function isSaveCommandError(value: unknown): value is SaveCommandError {
  return (
    typeof value === 'object' &&
    value !== null &&
    'cause' in value &&
    isSaveCause(value.cause) &&
    'retryable' in value &&
    typeof value.retryable === 'boolean' &&
    'message_key' in value &&
    typeof value.message_key === 'string' &&
    'request_id' in value &&
    (value.request_id === null || typeof value.request_id === 'string')
  );
}

/*
 * The causes are read from the map rather than from a second hand-written list,
 * so the map stays the one place a cause is enumerated.
 */
function isSaveCause(value: unknown): value is SaveCause {
  return (
    typeof value === 'string' && Object.hasOwn(SAVE_CAUSE_MESSAGE_KEYS, value)
  );
}

/**
 * Describe a failed operation for the failure surface.
 *
 * A product failure keeps its cause and the backend's `retryable` flag; anything
 * else — an `Error` from a broken IPC call, a string from a denied command — is
 * the unclassified failure (`unknown`, not retryable) rather than a guessed
 * cause. It is still a *failure*: nothing here can make a save look completed.
 */
export function describeSaveFailure(error: unknown): SaveFailureDescription {
  if (!isSaveCommandError(error)) {
    return {
      cause: 'unknown',
      messageKey: SAVE_CAUSE_MESSAGE_KEYS.unknown,
      retryable: false,
      failure: null,
    };
  }

  return {
    cause: error.cause,
    messageKey: SAVE_CAUSE_MESSAGE_KEYS[error.cause],
    retryable: error.retryable,
    failure: error,
  };
}
