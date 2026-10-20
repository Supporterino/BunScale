/*
 * Task 3.4's evidence for the one failure surface (D0.6, NFR-8): every cause
 * maps to exactly one message key, the key comes from the cause rather than from
 * the backend's raw field, and anything that is not the contract's failure is
 * the unclassified cause instead of a guessed one.
 */
import { describe, expect, it } from 'vitest';
import type { SaveCause, SaveMessageKey } from './failure';
import {
  describeSaveFailure,
  isSaveCommandError,
  SAVE_CAUSE_MESSAGE_KEYS,
} from './failure';

/** The frozen taxonomy, as the map enumerates it. */
const CAUSES = Object.entries(SAVE_CAUSE_MESSAGE_KEYS) as Array<
  [SaveCause, SaveMessageKey]
>;

describe('the failure surface copy', () => {
  it('names one distinct message key per cause', () => {
    const keys = CAUSES.map(([, key]) => key);

    expect(new Set(keys).size).toBe(keys.length);
    expect(keys.every((key) => key.startsWith('error.'))).toBe(true);
    expect(CAUSES.length).toBeGreaterThan(0);
  });

  it('derives the copy from the cause, not from the backend key', () => {
    /*
     * A stub's failure is a real `CommandError` with `cause: unknown` and the
     * placeholder key `error.not_implemented`. The surface must not render that
     * placeholder, so the cause decides the key.
     */
    const stub = {
      cause: 'unknown',
      retryable: false,
      message_key: 'error.not_implemented',
      request_id: 'request-1',
    };

    expect(describeSaveFailure(stub)).toEqual({
      cause: 'unknown',
      messageKey: 'error.unknown',
      retryable: false,
      failure: stub,
    });
  });

  it('maps every cause to its own key', () => {
    for (const [cause, messageKey] of CAUSES) {
      const described = describeSaveFailure({
        cause,
        retryable: true,
        message_key: 'error.some_other_key',
        request_id: null,
      });

      expect(described.cause).toBe(cause);
      expect(described.messageKey).toBe(messageKey);
      expect(described.failure).not.toBeNull();
      expect(described.retryable).toBe(true);
    }
  });

  it('treats anything that is not the contract failure as unclassified', () => {
    const unclassified = {
      cause: 'unknown',
      messageKey: 'error.unknown',
      retryable: false,
      failure: null,
    };

    expect(describeSaveFailure(new Error('the bridge broke'))).toEqual(
      unclassified,
    );
    expect(describeSaveFailure('command not found')).toEqual(unclassified);
    expect(describeSaveFailure(null)).toEqual(unclassified);
    expect(describeSaveFailure({ cause: 'not_a_cause' })).toEqual(unclassified);
  });

  it('recognises only a fully shaped contract failure', () => {
    expect(
      isSaveCommandError({
        cause: 'connectivity',
        retryable: true,
        message_key: 'error.connectivity',
        request_id: null,
      }),
    ).toBe(true);
    expect(isSaveCommandError({ cause: 'connectivity' })).toBe(false);
    expect(isSaveCommandError({ cause: 'nope', retryable: true })).toBe(false);
  });
});
