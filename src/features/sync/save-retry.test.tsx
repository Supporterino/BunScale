/*
 * Task 3.4's evidence for the failed-save retry state (SYNC-2, NFR-8, D0.14).
 *
 * The property that matters is identity: a retry must re-issue the operation it
 * was recorded with, so both attempts carry one `request_id` and the shared
 * service can deduplicate them (D0.14). The rest is the surface's honesty — a
 * failed save stays failed until the backend answers `ok`, a later failure
 * updates the cause instead of clearing the record, and nothing is ever shown as
 * saved when no answer arrived (a rejected `run()` included).
 *
 * One cause settles as a clear rather than a record: a retry the backend refuses
 * with `validation` belongs to the form field that owns the input, so the slot is
 * dropped without a second notice (task 7.7) — and, like every other outcome,
 * only for the attempt the slot still holds, so a superseded retry that settles
 * late as `validation` cannot clear a newer failure.
 *
 * The interleaving specs are the other half: the slot can move on while an
 * attempt is in flight. A newer failure supersedes it (and its late result is
 * dropped, whichever way it settles), and a dismissal is refused while a retry
 * is running because that operation may land — so a late success can never
 * clear a newer failure, call a callback that has moved on, or resurrect a
 * dismissed record.
 *
 * The last group renders the store-backed mount itself (`SaveFailureSurface`)
 * rather than the hook: the surface is the only reader that turns the store's
 * `retrying` into the notice's busy form, so that forwarding is a fact about
 * this file's fixture arrangement (the record, the `deferred()` operation and
 * `resetSaveRetry` all live here). `save-failure.component.test.tsx` owns the
 * notice's `retrying` prop and deliberately does not render the mount; deleting
 * `retrying={retrying}` from the mount must fail here. The cases are:
 *
 *   - "shows the store retrying and unmounts on success" — with a retryable
 *     record on a deferred operation and `retryFailedSave()` started but not
 *     awaited, the store's `retrying` reaches the notice: one `alert`, no
 *     `status`, and the `Try again` control present, labelled, disabled and
 *     `aria-busy`. The attempt then succeeds, and the surface unmounts;
 *   - "re-enables the retry when the attempt fails" — the same arrangement, the
 *     attempt failing instead, so the control comes back enabled without
 *     `aria-busy` (the forwarding of `retrying: false`, not just of `true`).
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import {
  act,
  render,
  renderHook,
  screen,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CommandResult } from '../../lib/request-id';
import { createHousehold } from '../households/api';
import type { Household } from '../households/queries';
import { SaveFailureSurface } from './components/SaveFailureSurface';
import type { SaveCommandError } from './failure';
import type { SaveFailure } from './save-retry';
import {
  dismissFailedSave,
  getSaveRetryState,
  recordFailedSave,
  resetSaveRetry,
  retryFailedSave,
  useSaveRetry,
} from './save-retry';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

const HOUSEHOLD: Household = {
  id: 'household-a',
  name: 'Household A',
  created_at: AT,
  owner_member_id: 'member-a',
  is_active: true,
  is_owner: true,
};

/** The backend's connectivity failure, carrying the operation's identity. */
function connectivity(requestId: string): SaveCommandError {
  return {
    cause: 'connectivity',
    retryable: true,
    message_key: 'error.connectivity',
    request_id: requestId,
  };
}

/** The backend's rate-limited failure. */
function rateLimited(requestId: string): SaveCommandError {
  return {
    cause: 'rate_limited',
    retryable: true,
    message_key: 'error.rate_limited',
    request_id: requestId,
  };
}

/** The backend's `validation` refusal: the form field's own error. */
function validation(requestId: string): SaveCommandError {
  return {
    cause: 'validation',
    retryable: false,
    message_key: 'error.validation',
    request_id: requestId,
  };
}

/** A save operation the test controls, for the paths `api.ts` cannot fake. */
function operation(
  run: () => Promise<CommandResult<unknown, SaveFailure>>,
  requestId = 'request-1',
) {
  return { requestId, run };
}

/** A promise whose settlement the test controls. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};

  const promise = new Promise<T>((settle) => {
    resolve = settle;
  });

  return { promise, resolve };
}

describe('the failed-save retry state', () => {
  let answers: Array<() => unknown>;
  let calls: Array<{ command: string; args: unknown }>;

  beforeEach(() => {
    resetSaveRetry();
    answers = [];
    calls = [];

    mockIPC((command, args) => {
      calls.push({ command, args });

      const answer = answers.shift();

      if (answer === undefined) {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return answer();
    });
  });

  afterEach(() => {
    clearMocks();
  });

  it('retries with the same request id and clears only on success', async () => {
    const saved = vi.fn();
    const create = createHousehold('Household A');

    answers.push(() => {
      throw connectivity(create.requestId);
    });

    const attempt = await create.run();

    expect(attempt.status).toBe('error');

    if (attempt.status === 'error') {
      recordFailedSave(create, attempt.error, saved);
    }

    const recorded = getSaveRetryState().failure;

    expect(recorded?.cause).toBe('connectivity');
    expect(recorded?.messageKey).toBe('error.connectivity');
    expect(recorded?.retryable).toBe(true);
    expect(recorded?.operation).toBe(create);
    expect(saved).not.toHaveBeenCalled();

    answers.push(() => HOUSEHOLD);

    await retryFailedSave();

    expect(
      calls.map((call) => (call.args as { requestId?: string }).requestId),
    ).toEqual([create.requestId, create.requestId]);
    expect(getSaveRetryState()).toEqual({ failure: null, retrying: false });
    expect(saved).toHaveBeenCalledWith(HOUSEHOLD);
  });

  it('keeps the record and updates the cause when the retry fails too', async () => {
    const saved = vi.fn();
    const create = createHousehold('Household A');

    recordFailedSave(create, connectivity(create.requestId), saved);

    answers.push(() => {
      throw {
        cause: 'rate_limited',
        retryable: true,
        message_key: 'error.rate_limited',
        request_id: create.requestId,
      };
    });

    await retryFailedSave();

    const recorded = getSaveRetryState().failure;

    expect(recorded?.cause).toBe('rate_limited');
    expect(recorded?.messageKey).toBe('error.rate_limited');
    expect(recorded?.operation).toBe(create);
    expect(saved).not.toHaveBeenCalled();
  });

  it('clears the record when the retry is refused as validation', async () => {
    const saved = vi.fn();
    const refused = operation(async () => ({
      status: 'error',
      error: validation('request-1'),
    }));

    recordFailedSave(refused, connectivity(refused.requestId), saved);

    await retryFailedSave();

    // The refusal is the form field's to present, so the shell's record goes
    // rather than becoming a second notice for one refusal. A clear is not a
    // confirmation: the recorded success callback does not run.
    expect(getSaveRetryState()).toEqual({ failure: null, retrying: false });
    expect(saved).not.toHaveBeenCalled();
  });

  it('treats a rejected retry as an unclassified failure, never a success', async () => {
    const failing = operation(async () => {
      throw new Error('the bridge broke');
    });

    recordFailedSave(failing, connectivity(failing.requestId));

    await retryFailedSave();

    const recorded = getSaveRetryState().failure;

    expect(recorded?.cause).toBe('unknown');
    expect(recorded?.messageKey).toBe('error.unknown');
    expect(recorded?.retryable).toBe(false);
    expect(recorded?.failure).toBeNull();
    expect(recorded?.operation).toBe(failing);
  });

  it('runs one retry at a time', async () => {
    const runs = vi.fn();
    const pending = deferred<CommandResult<unknown, SaveFailure>>();
    const slow = operation(() => {
      runs();

      return pending.promise;
    });

    recordFailedSave(slow, connectivity(slow.requestId));

    const first = retryFailedSave();
    const second = retryFailedSave();

    expect(getSaveRetryState().retrying).toBe(true);
    expect(runs).toHaveBeenCalledTimes(1);

    pending.resolve({ status: 'ok', data: 'saved' });
    await first;
    await second;

    expect(getSaveRetryState()).toEqual({ failure: null, retrying: false });
  });

  it('keeps a newer failure when an older attempt finishes', async () => {
    const olderSaved = vi.fn();
    const newerSaved = vi.fn();
    const pending = deferred<CommandResult<unknown, SaveFailure>>();
    const older = operation(() => pending.promise, 'request-older');
    const newer = operation(
      async () => ({ status: 'ok', data: null }),
      'request-newer',
    );

    recordFailedSave(older, connectivity(older.requestId), olderSaved);
    const retry = retryFailedSave();

    // A newer save fails while the older attempt is still in flight: the newer
    // failure owns the surface from now on.
    recordFailedSave(newer, connectivity(newer.requestId), newerSaved);

    // The older attempt then succeeds. It must not clear the newer failure, nor
    // run either callback: that write is not the one the surface is showing.
    pending.resolve({ status: 'ok', data: 'older' });
    await retry;

    expect(getSaveRetryState().failure?.operation).toBe(newer);
    expect(getSaveRetryState().failure?.cause).toBe('connectivity');
    expect(olderSaved).not.toHaveBeenCalled();
    expect(newerSaved).not.toHaveBeenCalled();
  });

  it('does not resurrect a superseded failure from an older attempt', async () => {
    const pending = deferred<CommandResult<unknown, SaveFailure>>();
    const older = operation(() => pending.promise, 'request-older');
    const newer = operation(
      async () => ({ status: 'ok', data: null }),
      'request-newer',
    );

    recordFailedSave(older, connectivity(older.requestId));
    const retry = retryFailedSave();

    recordFailedSave(newer, rateLimited(newer.requestId));

    pending.resolve({
      status: 'error',
      error: connectivity(older.requestId),
    });
    await retry;

    const recorded = getSaveRetryState().failure;

    expect(recorded?.operation).toBe(newer);
    expect(recorded?.cause).toBe('rate_limited');
  });

  it('does not clear a newer record when a superseded retry settles as validation', async () => {
    const pending = deferred<CommandResult<unknown, SaveFailure>>();
    const older = operation(() => pending.promise, 'request-older');
    const newer = operation(
      async () => ({ status: 'ok', data: null }),
      'request-newer',
    );

    recordFailedSave(older, connectivity(older.requestId));
    const retry = retryFailedSave();

    recordFailedSave(newer, rateLimited(newer.requestId));

    // The older attempt is refused as validation after the newer failure owns
    // the slot. `validation` clears the attempt it belongs to, but this is not
    // that attempt: the identity guard wins and the newer record stands.
    pending.resolve({ status: 'error', error: validation(older.requestId) });
    await retry;

    const recorded = getSaveRetryState().failure;

    expect(recorded?.operation).toBe(newer);
    expect(recorded?.cause).toBe('rate_limited');
    expect(recorded?.messageKey).toBe('error.rate_limited');
    expect(getSaveRetryState().retrying).toBe(false);
  });

  it('refuses a dismissal while a retry is running', async () => {
    const saved = vi.fn();
    const pending = deferred<CommandResult<unknown, SaveFailure>>();
    const slow = operation(() => pending.promise);

    recordFailedSave(slow, connectivity(slow.requestId), saved);
    const retry = retryFailedSave();

    dismissFailedSave();

    // The operation is in flight and may land, so the slot and its callback
    // stay: a dismissal cannot discard a confirmed write.
    expect(getSaveRetryState().failure?.operation).toBe(slow);
    expect(getSaveRetryState().retrying).toBe(true);

    pending.resolve({ status: 'ok', data: 'saved' });
    await retry;

    expect(getSaveRetryState()).toEqual({ failure: null, retrying: false });
    expect(saved).toHaveBeenCalledWith('saved');
  });

  it('exposes the record to a component and clears it on dismiss', () => {
    const idle = operation(async () => ({ status: 'ok', data: null }));
    const { result } = renderHook(() => useSaveRetry());

    expect(result.current).toEqual({ failure: null, retrying: false });

    act(() => {
      recordFailedSave(idle, connectivity(idle.requestId));
    });

    expect(result.current.failure?.cause).toBe('connectivity');
    expect(result.current.retrying).toBe(false);

    act(() => {
      dismissFailedSave();
    });

    expect(result.current).toEqual({ failure: null, retrying: false });
  });

  /*
   * The wiring half of task 7.9, and the reason it lives here: the notice's
   * `retrying` prop is pinned by `save-failure.component.test.tsx`, but only
   * `SaveFailureSurface` reads that flag from the store. These render the mount
   * with this file's own record/`deferred()` arrangement, so removing
   * `retrying={retrying}` from the mount leaves the control enabled and fails.
   */
  describe('the store-backed surface forwards an in-flight retry', () => {
    it('shows the store retrying and unmounts on success', async () => {
      const pending = deferred<CommandResult<unknown, SaveFailure>>();
      const slow = operation(() => pending.promise);

      recordFailedSave(slow, connectivity(slow.requestId));

      render(<SaveFailureSurface />);

      // Before the attempt: the same control is usable, so the busy form below
      // can only come from the store's `retrying`.
      expect(
        within(screen.getByRole('alert')).getByRole('button', {
          name: 'Try again',
        }),
      ).toBeEnabled();

      let retry: Promise<void> | undefined;

      act(() => {
        retry = retryFailedSave();
      });

      const notice = screen.getByRole('alert');
      const control = within(notice).getByRole('button', { name: 'Try again' });

      // The control is not withdrawn while busy: same node, same label, the
      // Button's loading form.
      expect(control).toBeVisible();
      expect(control).toHaveTextContent('Try again');
      expect(control).toBeDisabled();
      expect(control).toHaveAttribute('aria-busy', 'true');

      // The busy form adds no second live region.
      expect(screen.getAllByRole('alert')).toHaveLength(1);
      expect(screen.queryByRole('status')).toBeNull();

      await act(async () => {
        pending.resolve({ status: 'ok', data: 'saved' });
        await retry;
      });

      // The retry succeeded, so the record is gone and the surface with it.
      expect(screen.queryByRole('alert')).toBeNull();
    });

    it('re-enables the retry when the attempt fails', async () => {
      const pending = deferred<CommandResult<unknown, SaveFailure>>();
      const slow = operation(() => pending.promise);

      recordFailedSave(slow, connectivity(slow.requestId));

      render(<SaveFailureSurface />);

      let retry: Promise<void> | undefined;

      act(() => {
        retry = retryFailedSave();
      });

      expect(
        within(screen.getByRole('alert')).getByRole('button', {
          name: 'Try again',
        }),
      ).toBeDisabled();

      await act(async () => {
        pending.resolve({
          status: 'error',
          error: rateLimited(slow.requestId),
        });
        await retry;
      });

      const notice = screen.getByRole('alert');
      const control = within(notice).getByRole('button', { name: 'Try again' });

      // The failure keeps the record and the offer: `retrying` went back to
      // false, so the forwarding must carry that too.
      expect(control).toHaveTextContent('Try again');
      expect(control).toBeEnabled();
      expect(control).not.toHaveAttribute('aria-busy');
      expect(screen.getAllByRole('alert')).toHaveLength(1);
    });
  });
});
