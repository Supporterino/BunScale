/*
 * The read-side failure mapper (task 3.10; NFR-8, D1.16, SYNC-3).
 *
 * `describeSaveFailure` derives copy from the cause alone and deliberately
 * discards `message_key`, which is right for a write: a cause can only be
 * shown as its own copy. A **read** can carry a surface key on a frozen cause
 * (D1.16's `error.household_content_unloaded` on `unknown`), so deriving the
 * key from the cause would collapse every P1 read refusal into the
 * operation-neutral `error.unknown`. This module is the read half and does not
 * change the write half: `describeSaveFailure` keeps its behaviour.
 *
 * The rule, in order:
 *
 * 1. `message_key` wins when the catalog knows it, so a surface key the
 *    backend named is what the screen renders.
 * 2. Otherwise the cause's own key is the fallback (`SAVE_CAUSE_MESSAGE_KEYS`),
 *    so a classified cause is never reduced to a generic message.
 * 3. A key the catalog does not know — a stub's `error.not_implemented`, a
 *    future rename — is never rendered; it falls back to the cause, and an
 *    unrecognisable value to `error.unknown`.
 *
 * Three read-side conditions are named, because the surface's copy differs:
 *
 * - `content_unloaded` — D1.16's unknown-content refusal: rows are loaded, one
 *   row's content is not, and a fresh read can fill it.
 * - `cause` — a classified cause with no surface key; its own copy is correct
 *   (`validation`'s "check it and try again", a transport cause's "try again").
 * - `unclassified` — the value was not a contract failure at all (an `Error`
 *   from a broken IPC call, a string from a denied command).
 *
 * A never-loaded household is **not** a failure and is not mapped here:
 * `load_household` answers `Ok` with `source: local` and `loaded_at: null`, and
 * a list read answers an empty list, so the surface reads that state from the
 * value it holds (task 4.15), never from a rejection.
 *
 * **Never a retry.** A read takes no `request_id` (design.md's command table),
 * so there is no reissued operation to offer. The surface's action is
 * a refresh — a fresh read — which is why the description says `refreshable`
 * and never exposes the failure's `retryable` flag: a write retry here would
 * re-run an operation the read does not have.
 */
import type { MessageKey } from '../i18n/messages';
import { P0_MESSAGES } from '../i18n/messages';
import type { SaveCommandError } from './failure';
import { isSaveCommandError, SAVE_CAUSE_MESSAGE_KEYS } from './failure';

/** D1.16's surface key on the frozen `unknown` cause (task 3.8). */
const CONTENT_UNLOADED_MESSAGE_KEY =
  'error.household_content_unloaded' as const;

/** How a read failure must be presented. */
export type ReadFailureCondition =
  | 'content_unloaded'
  | 'cause'
  | 'unclassified';

/** What a read failure is shown as: its condition, its copy, and the value. */
export type ReadFailureDescription = {
  /** The condition the copy belongs to. */
  readonly condition: ReadFailureCondition;
  /** The catalog key the surface resolves; always a key the catalog knows. */
  readonly messageKey: MessageKey;
  /**
   * Whether a fresh read can change the answer. `false` only for `validation`,
   * whose copy asks the person to correct the input instead; every other
   * cause's copy promises another attempt, and a read's action is a fresh read.
   */
  readonly refreshable: boolean;
  /** The backend's failure object, or `null` when the value was not one. */
  readonly failure: SaveCommandError | null;
};

/** Whether the catalog can answer for `key`; `t` would otherwise substitute. */
function isCatalogKey(key: string): key is MessageKey {
  return Object.hasOwn(P0_MESSAGES, key);
}

/**
 * Describe a failed read for the surface that renders it (NFR-8, D1.16).
 *
 * A recognised value keeps its surface key or its cause's copy; anything else
 * is the unclassified failure with `error.unknown`. The result can never carry
 * a key the catalog lacks, so a typo or a future rename is a generic message
 * rather than a raw key shown to the person.
 */
export function describeReadFailure(error: unknown): ReadFailureDescription {
  if (!isSaveCommandError(error)) {
    return {
      condition: 'unclassified',
      messageKey: SAVE_CAUSE_MESSAGE_KEYS.unknown,
      refreshable: true,
      failure: null,
    };
  }

  const messageKey = isCatalogKey(error.message_key)
    ? error.message_key
    : SAVE_CAUSE_MESSAGE_KEYS[error.cause];

  if (messageKey === CONTENT_UNLOADED_MESSAGE_KEY) {
    return {
      condition: 'content_unloaded',
      messageKey,
      refreshable: true,
      failure: error,
    };
  }

  return {
    condition: 'cause',
    messageKey,
    refreshable: error.cause !== 'validation',
    failure: error,
  };
}
