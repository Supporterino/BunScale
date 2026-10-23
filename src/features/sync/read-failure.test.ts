/*
 * Task 3.10's evidence for the read-side failure mapper (NFR-8, D1.16).
 *
 * Every fixture is a failure a P1 read wrapper can hand the surface: D1.16's
 * unknown-content refusal, a cause-specific refusal with no surface key, a key
 * the catalog does not know, a transport cause whose copy promises another
 * attempt, and a value that is not the contract's failure at all. The point is
 * that the surface key the backend named survives, while a key the catalog
 * lacks never reaches `t`.
 */
import { describe, expect, it } from 'vitest';
import { P0_MESSAGES } from '../i18n/messages';
import { describeSaveFailure, SAVE_CAUSE_MESSAGE_KEYS } from './failure';
import { describeReadFailure } from './read-failure';

/** D1.16's refusal: rows loaded, one row's content not (task 3.8). */
const CONTENT_UNLOADED = {
  cause: 'unknown',
  retryable: false,
  message_key: 'error.household_content_unloaded',
  request_id: null,
};

/** A refusal whose copy is its cause's own, with no surface key. */
const VALIDATION = {
  cause: 'validation',
  retryable: false,
  message_key: 'error.validation',
  request_id: 'request-1',
};

describe('the read failure copy', () => {
  it("keeps D1.16's surface key and offers a refresh", () => {
    expect(describeReadFailure(CONTENT_UNLOADED)).toEqual({
      condition: 'content_unloaded',
      messageKey: 'error.household_content_unloaded',
      refreshable: true,
      failure: CONTENT_UNLOADED,
    });
  });

  it("falls back to the cause's own key when the failure carries no surface key", () => {
    expect(describeReadFailure(VALIDATION)).toEqual({
      condition: 'cause',
      messageKey: 'error.validation',
      refreshable: false,
      failure: VALIDATION,
    });
  });

  it('never renders a key the catalog does not know', () => {
    /*
     * P0's stub placeholder is a real `CommandError` key and not product copy.
     * The read mapper must fall back to the cause, exactly as the write mapper
     * does, and must not echo the key.
     */
    const stub = {
      cause: 'unknown',
      retryable: false,
      message_key: 'error.not_implemented',
      request_id: 'request-1',
    };

    expect(describeReadFailure(stub)).toEqual({
      condition: 'cause',
      messageKey: 'error.unknown',
      refreshable: true,
      failure: stub,
    });

    /* The write half is unchanged: it still derives copy from the cause only. */
    expect(describeSaveFailure(CONTENT_UNLOADED).messageKey).toBe(
      'error.unknown',
    );

    for (const failure of [
      CONTENT_UNLOADED,
      VALIDATION,
      stub,
      new Error('the bridge broke'),
      null,
    ]) {
      expect(P0_MESSAGES).toHaveProperty(
        describeReadFailure(failure).messageKey,
      );
    }
  });

  it("offers the refresh every non-validation cause's copy promises", () => {
    for (const [cause, messageKey] of Object.entries(SAVE_CAUSE_MESSAGE_KEYS)) {
      const described = describeReadFailure({
        cause,
        retryable: false,
        message_key: messageKey,
        request_id: null,
      });

      expect(described.refreshable).toBe(cause !== 'validation');
    }
  });

  it('treats anything that is not the contract failure as unclassified', () => {
    const unclassified = {
      condition: 'unclassified',
      messageKey: 'error.unknown',
      refreshable: true,
      failure: null,
    };

    expect(describeReadFailure(new Error('the bridge broke'))).toEqual(
      unclassified,
    );
    expect(describeReadFailure('command not found')).toEqual(unclassified);
    expect(describeReadFailure(null)).toEqual(unclassified);
    expect(describeReadFailure({ cause: 'not_a_cause' })).toEqual(unclassified);
  });
});
