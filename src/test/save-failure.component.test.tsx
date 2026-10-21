/*
 * The save-failure surface at the component layer (task 5.13; SYNC-2, NFR-8).
 *
 * `SaveFailureNotice` is the one presentational surface for a save that did not
 * complete, so the three acceptance scenarios about what a person sees are
 * asserted here as copy and actions:
 *
 *   - "Save refused without connectivity" — a `connectivity` failure states that
 *     the change was not saved and names connectivity. It is assertive, and no
 *     part of it reads as applied;
 *   - "Save failure names its cause and offers a retry" — a retry is offered only
 *     for the causes the backend marks retryable (connectivity, no account,
 *     allowance exceeded, rate limited, timeout) and never for `validation` or
 *     `unknown`, where dismissal is the action instead;
 *   - "A failed save is visible and specific" — every cause produces its own
 *     copy, so a classified cause can never fall back to "something went wrong".
 *
 * `retrying` is asserted as a state of that same retry offer rather than as a
 * fourth scenario (task 7.9): while an attempt is in flight the control keeps
 * its node and its label and takes the `Button`'s busy form (`disabled` and
 * `aria-busy`) instead of being withdrawn, and the notice stays the only live
 * region; a non-retryable cause has no control at all, `retrying` or not. The
 * file renders `SaveFailureNotice` by props — that is the honest layer for the
 * prop; the store-to-surface forwarding (`SaveFailureSurface` reads `retrying`
 * from `useSaveRetry` and hands it down) is a wiring fact this file does not
 * render, and the store itself is `save-retry.test.tsx`'s.
 *
 * The cause list is read from `SAVE_CAUSE_MESSAGE_KEYS`, the map that defines the
 * taxonomy, so a cause added to the contract is covered by these specs rather
 * than silently skipped. Every fixture carries `error.unknown` as the backend's
 * `message_key` on purpose: the surface must derive its copy from the cause, not
 * echo the backend's string (`features/sync/failure.ts` states the rule).
 *
 * What is NOT assertable at this layer, and who owns it:
 *   - which screen mounts the surface in a real flow (onboarding's create form
 *     today), and that the refused entry never reaches the shared collection,
 *     are route and backend facts: Rust 5.1–5.10 assert the write, and E2E 5.16
 *     asserts the visible failure in a launch, including that the retry re-sends
 *     the operation under its own request id (D0.14);
 *   - the notice's layout and its presentation at each form factor are browser
 *     facts; E2E 5.16 runs at both viewports.
 *
 * The suite runs at both component baselines (390×844 and 1280×800) through
 * `describeAtBothBaselines`: the assertions do not branch on the viewport, but
 * the harness contract is that a component spec runs at both. Both platforms
 * share this layer: nothing here is per-OS.
 */

import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { t } from '../features/i18n';
import { SaveFailureNotice } from '../features/sync/components/SaveFailureNotice';
import {
  SAVE_CAUSE_MESSAGE_KEYS,
  type SaveCause,
  type SaveCommandError,
} from '../features/sync/failure';
import { describeAtBothBaselines } from './setup';

/** The connectivity copy, verbatim: what "connectivity is required" reads as. */
const CONNECTIVITY_BODY =
  'No connection. Check your internet connection and try again.';

/** The catalog's generic wording, reserved for a failure with no cause. */
const GENERIC = t('error.unknown');

/** The backend's retry policy, one flag per cause (`CommandError::retryable`). */
const RETRYABLE_BY_CAUSE: Record<SaveCause, boolean> = {
  connectivity: true,
  no_account: true,
  allowance_exceeded: true,
  rate_limited: true,
  timeout: true,
  validation: false,
  unknown: false,
};

/** Every cause the contract can report, read from the map that defines them. */
const CAUSES = Object.keys(SAVE_CAUSE_MESSAGE_KEYS) as SaveCause[];

const RETRYABLE_CAUSES = CAUSES.filter((cause) => RETRYABLE_BY_CAUSE[cause]);

const NON_RETRYABLE_CAUSES = CAUSES.filter(
  (cause) => !RETRYABLE_BY_CAUSE[cause],
);

/**
 * A product failure as the backend reports it.
 *
 * `message_key` is a generic key rather than the cause's own, so a spec fails if
 * the surface stops deriving the copy from `cause` and renders the backend's
 * raw key instead.
 */
function failure(cause: SaveCause): SaveCommandError {
  return {
    cause,
    message_key: 'error.unknown',
    request_id: `request-${cause}`,
    retryable: RETRYABLE_BY_CAUSE[cause],
  };
}

describeAtBothBaselines('Save failure surface (SYNC-2, NFR-8)', () => {
  describe('"Save refused without connectivity"', () => {
    it('names connectivity and never looks saved', async () => {
      const dismiss = vi.fn();
      const retry = vi.fn();

      render(
        <SaveFailureNotice
          error={failure('connectivity')}
          onDismiss={dismiss}
          onRetry={retry}
        />,
      );

      const notice = screen.getByRole('alert');

      expect(
        within(notice).getByRole('heading', { name: 'Not saved' }),
      ).toBeVisible();
      expect(notice).toHaveTextContent(CONNECTIVITY_BODY);

      // Assertive, not a polite status, and no applied-looking copy anywhere.
      expect(screen.queryByRole('status')).toBeNull();
      expect(notice).not.toHaveTextContent(/has been saved/i);

      const user = userEvent.setup();

      await user.click(
        within(notice).getByRole('button', { name: 'Try again' }),
      );

      expect(retry).toHaveBeenCalledTimes(1);
      expect(dismiss).not.toHaveBeenCalled();
    });
  });

  describe('"Save failure names its cause and offers a retry"', () => {
    for (const cause of RETRYABLE_CAUSES) {
      it(`offers a retry for ${cause}`, async () => {
        const dismiss = vi.fn();
        const retry = vi.fn();

        render(
          <SaveFailureNotice
            error={failure(cause)}
            onDismiss={dismiss}
            onRetry={retry}
          />,
        );

        const notice = screen.getByRole('alert');
        const user = userEvent.setup();

        expect(notice).toHaveTextContent(t(SAVE_CAUSE_MESSAGE_KEYS[cause]));

        await user.click(
          within(notice).getByRole('button', { name: 'Try again' }),
        );

        expect(retry).toHaveBeenCalledTimes(1);
        expect(dismiss).not.toHaveBeenCalled();
      });
    }

    for (const cause of NON_RETRYABLE_CAUSES) {
      it(`offers a dismissal instead of a retry for ${cause}`, async () => {
        const dismiss = vi.fn();
        const retry = vi.fn();

        render(
          <SaveFailureNotice
            error={failure(cause)}
            onDismiss={dismiss}
            onRetry={retry}
          />,
        );

        const notice = screen.getByRole('alert');

        expect(
          within(notice).queryByRole('button', { name: 'Try again' }),
        ).toBeNull();

        const user = userEvent.setup();

        await user.click(
          within(notice).getByRole('button', { name: 'Dismiss' }),
        );

        expect(dismiss).toHaveBeenCalledTimes(1);
        expect(retry).not.toHaveBeenCalled();
      });
    }

    /*
     * Task 7.9: the in-flight state is `Button`'s `loading` form, not an
     * absent action. The control the person pressed stays where it is, with the
     * label that says what is running; the form disables it and exposes
     * `aria-busy`, which is what keeps the attempt from landing twice and from
     * being announced by a second live region.
     */
    it('keeps the retry control in its busy form while the retry runs', () => {
      const retry = vi.fn();

      render(
        <SaveFailureNotice
          error={failure('connectivity')}
          onRetry={retry}
          retrying
        />,
      );

      const notice = screen.getByRole('alert');
      const control = within(notice).getByRole('button', { name: 'Try again' });

      // Still there, still labelled: the node is not replaced or unmounted.
      expect(control).toBeVisible();
      expect(control).toHaveTextContent('Try again');

      // The Button's loading contract: unreachable and announced as busy. Focus
      // is deliberately not asserted — the native `disabled` releases it.
      expect(control).toBeDisabled();
      expect(control).toHaveAttribute('aria-busy', 'true');

      // The busy form adds no live region: one alert, no polite status.
      expect(screen.getAllByRole('alert')).toHaveLength(1);
      expect(screen.queryByRole('status')).toBeNull();
    });

    // The shell mount passes no dismissal (SaveFailureSurface), so a
    // non-retryable record is copy alone — and `retrying` cannot conjure a
    // control into that state.
    it('offers no control for a non-retryable cause while retrying', () => {
      render(<SaveFailureNotice error={failure('validation')} retrying />);

      const notice = screen.getByRole('alert');

      expect(notice).toHaveTextContent(t(SAVE_CAUSE_MESSAGE_KEYS.validation));
      expect(within(notice).queryAllByRole('button')).toHaveLength(0);
      expect(screen.queryByRole('status')).toBeNull();
    });
  });

  describe('"A failed save is visible and specific"', () => {
    it('names every cause distinctly and never generically', () => {
      const notices = CAUSES.map((cause) => (
        <SaveFailureNotice error={failure(cause)} key={cause} />
      ));

      render(<div>{notices}</div>);

      const alerts = screen.getAllByRole('alert');

      expect(alerts).toHaveLength(CAUSES.length);

      // One visible, cause-specific message per cause: no two read the same.
      const copies = alerts.map((alert) => alert.textContent);

      expect(new Set(copies).size).toBe(CAUSES.length);

      for (const cause of CAUSES) {
        const copy = t(SAVE_CAUSE_MESSAGE_KEYS[cause]);
        const alert = screen.getByText(copy).closest('[role="alert"]');

        if (alert === null) {
          throw new Error(`${cause} rendered no failure notice`);
        }

        expect(alert).toBeVisible();
        expect(alert).toHaveTextContent(t('error.notSaved'));
        expect(alert).not.toHaveTextContent(/has been saved/i);

        if (cause !== 'unknown') {
          // A classified cause never falls back to the generic wording.
          expect(copy).not.toBe(GENERIC);
        }
      }
    });
  });
});
