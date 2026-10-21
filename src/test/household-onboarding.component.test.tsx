/*
 * Household onboarding at the component layer (task 5.12; HS-1, D0.14).
 *
 * Two acceptance scenarios are asserted here as behaviour a person can see:
 *
 *   - "Create the first household" — a valid name reaches `create_household`
 *     exactly once with that name and one request id, and the new household's
 *     dashboard is what is presented afterwards. The route is the app's own
 *     (`createTestRouter` over the committed tree), so the dashboard is reached
 *     the way it is in a launch: `navigate('/')` re-runs the entry guard against
 *     a fresh read, not a faked callback;
 *   - "Blank name is rejected" — an empty name and a whitespace-only name are
 *     refused before any command is sent, the inline prompt is the field's own
 *     live region (`role="alert"`) linked through `aria-describedby`, and the
 *     field is marked `aria-invalid`. The backend's `validation` refusal gets
 *     the same field treatment in the same words rather than a notice.
 *
 * Plus the retry identity of a failed create (D0.14): a retryable failure then
 * a retry re-sends the SAME `request_id`. Q29 records that no earlier task owned
 * this assertion for the screens, so task 5.12 folds it in here (and in the
 * Settings create form, which keeps the same handle discipline).
 *
 * What is NOT assertable at this layer, and who owns it:
 *   - "the creator is recorded as its first member" and "that household is the
 *     active household" are domain facts behind the command, which takes only a
 *     name; Rust 5.1 asserts them. The component asserts the observable half:
 *     the created household is the one the dashboard is titled with;
 *   - geometry (the 44 pt targets, the form cap) needs a layout engine —
 *     `getBoundingClientRect()` returns zeroes under jsdom — so E2E 5.14 owns
 *     it. The same holds for the visual presentation of the two baselines.
 *
 * `describeAtBothBaselines` runs the whole suite at 390×844 and 1280×800: no
 * assertion here branches on the viewport, and the arranged layout is E2E's
 * claim, but the two baselines are part of the harness contract for every
 * frontend spec.
 *
 * IPC is mocked at `invoke` (`testing-strategy`), and the event plugin is
 * mocked as well because both screens mount `useAppEvents`, whose `listen`
 * calls travel through the same bridge.
 */

import { RouterProvider } from '@tanstack/react-router';
import type { InvokeArgs } from '@tauri-apps/api/core';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { cleanup, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type {
  Household,
  HouseholdSummary,
} from '../features/households/queries';
import type { ServiceStatus } from '../features/sync/entry-status';
import {
  createTestQueryClient,
  createTestRouter,
  describeAtBothBaselines,
  renderWithQueryClient,
} from './setup';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

/** The household the person creates; also the backend's post-state. */
const CREATED: Household = {
  id: 'household-garden',
  name: 'Garden household',
  created_at: AT,
  owner_member_id: 'member-1',
  is_active: true,
  is_owner: true,
};

/** A fresh observation that lets the guard reach onboarding (SYNC-3). */
const REACHABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: true,
  cause: null,
  observed_at: AT,
};

/** One `invoke` call the app made, as the backend would have received it. */
type MockCall = {
  readonly command: string;
  readonly args: Record<string, unknown>;
};

/** One mocked command's answer; a throw is a rejected `invoke`. */
type MockHandler = (args: Record<string, unknown>) => unknown;

/** The contract's `Household` as one row of the held list. */
function summary(household: Household): HouseholdSummary {
  return {
    id: household.id,
    name: household.name,
    created_at: household.created_at,
    is_active: true,
    is_owner: household.is_owner,
    member_count: 1,
  };
}

/** The normalized payload of one `invoke` call. */
function payloadOf(args: InvokeArgs | undefined): Record<string, unknown> {
  return (args ?? {}) as Record<string, unknown>;
}

let calls: MockCall[];
let handlers: Map<string, MockHandler>;

/** Every `create_household` call, in order. */
function createCalls(): MockCall[] {
  return calls.filter((call) => call.command === 'create_household');
}

beforeEach(() => {
  calls = [];
  handlers = new Map();

  mockIPC(
    (command, args) => {
      const payload = payloadOf(args);

      calls.push({ command, args: payload });

      const handler = handlers.get(command);

      if (handler === undefined) {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return handler(payload);
    },
    { shouldMockEvents: true },
  );
});

afterEach(async () => {
  /*
   * Unmount first: both screens release their `listen` subscriptions on
   * unmount, and those calls need the mocked bridge. The awaits let the
   * released promises settle before `clearMocks` removes `invoke`.
   */
  cleanup();
  await Promise.resolve();
  await Promise.resolve();

  clearMocks();
});

/** Boot the app's real route tree at `path`, as a launch would. */
async function launch(path: string) {
  const client = createTestQueryClient();
  const router = createTestRouter({ client, initialEntries: [path] });

  await router.load();
  renderWithQueryClient(<RouterProvider router={router} />, { client });

  return router;
}

/** The guard's answers for "no household, service confirms it" (SYNC-3). */
function answerNoHousehold(): void {
  handlers.set('list_households', () => []);
  handlers.set('get_active_household', () => null);
  handlers.set('get_service_status', () => REACHABLE);
}

describeAtBothBaselines('Household onboarding (HS-1)', () => {
  describe('"Create the first household"', () => {
    it('creates the named household and presents its dashboard', async () => {
      let households: HouseholdSummary[] = [];
      let active: Household | null = null;

      handlers.set('list_households', () => households);
      handlers.set('get_active_household', () => active);
      handlers.set('get_service_status', () => REACHABLE);
      handlers.set('create_household', () => {
        // The backend's post-state: the book holds it and it is active.
        active = CREATED;
        households = [summary(CREATED)];

        return CREATED;
      });

      const router = await launch('/onboarding');

      await screen.findByRole('heading', { name: 'Create your household' });

      const user = userEvent.setup();
      const field = screen.getByRole('textbox', { name: /Household name/ });
      const submit = screen.getByRole('button', { name: 'Create household' });

      await user.type(field, CREATED.name);
      await user.click(submit);

      await waitFor(() => {
        expect(router.state.location.pathname).toBe('/');
      });

      expect(
        await screen.findByRole('heading', { name: CREATED.name }),
      ).toBeVisible();
      expect(screen.getByRole('main')).toHaveAttribute(
        'data-household-entry',
        'household',
      );
      expect(
        screen.queryByRole('heading', { name: 'Create your household' }),
      ).toBeNull();

      const creates = createCalls();

      expect(creates).toHaveLength(1);
      expect(creates[0].args.name).toBe(CREATED.name);

      // One operation identity: the command was sent once, under one id.
      const requestIds = new Set(creates.map((call) => call.args.requestId));

      expect(requestIds.size).toBe(1);

      const [requestId] = [...requestIds];

      expect(typeof requestId).toBe('string');
      expect(requestId).not.toBe('');
    });
  });

  describe('"Blank name is rejected"', () => {
    it('refuses an empty name without calling create_household', async () => {
      answerNoHousehold();
      handlers.set('create_household', () => CREATED);

      const router = await launch('/onboarding');

      await screen.findByRole('heading', { name: 'Create your household' });

      const user = userEvent.setup();
      const field = screen.getByRole('textbox', { name: /Household name/ });
      const submit = screen.getByRole('button', { name: 'Create household' });

      await user.click(submit);

      const prompt = await screen.findByRole('alert');

      expect(prompt).toHaveTextContent('Enter a name for your household.');
      expect(field).toHaveAttribute('aria-invalid', 'true');
      expect(field.getAttribute('aria-describedby')).toContain(prompt.id);
      expect(createCalls()).toHaveLength(0);
      expect(router.state.location.pathname).toBe('/onboarding');
    });

    it('refuses a whitespace-only name without a command', async () => {
      answerNoHousehold();
      handlers.set('create_household', () => CREATED);

      const router = await launch('/onboarding');

      await screen.findByRole('heading', { name: 'Create your household' });

      const user = userEvent.setup();
      const field = screen.getByRole('textbox', { name: /Household name/ });
      const submit = screen.getByRole('button', { name: 'Create household' });

      await user.type(field, '   ');
      await user.click(submit);

      const prompt = await screen.findByRole('alert');

      expect(prompt).toHaveTextContent('Enter a name for your household.');
      expect(field).toHaveAttribute('aria-invalid', 'true');
      expect(createCalls()).toHaveLength(0);
      expect(router.state.location.pathname).toBe('/onboarding');
    });

    it('shows a backend validation refusal inline', async () => {
      answerNoHousehold();
      handlers.set('create_household', (args) => {
        throw {
          cause: 'validation',
          retryable: false,
          message_key: 'error.validation',
          request_id: args.requestId,
        };
      });

      const router = await launch('/onboarding');

      await screen.findByRole('heading', { name: 'Create your household' });

      const user = userEvent.setup();
      const field = screen.getByRole('textbox', { name: /Household name/ });
      const submit = screen.getByRole('button', { name: 'Create household' });

      await user.type(field, CREATED.name);
      await user.click(submit);

      // The prompt is the field's own live region, in the blank-name words —
      // not the `error.*` notice copy, which is reserved for other causes.
      const prompt = await screen.findByRole('alert');

      expect(prompt).toHaveTextContent('Enter a name for your household.');
      expect(field).toHaveAttribute('aria-invalid', 'true');
      expect(screen.queryByText('Not saved')).toBeNull();
      expect(createCalls()).toHaveLength(1);
      expect(router.state.location.pathname).toBe('/onboarding');
    });
  });

  /*
   * D0.14's identity rule, folded in by task 5.12 because Q29 records it as
   * unowned: the screen keeps the failed attempt's handle and "Try again"
   * re-runs it, so both attempts carry one `request_id`.
   */
  describe('Retry identity (D0.14, Q29)', () => {
    it('re-sends the same request_id when retried', async () => {
      let households: HouseholdSummary[] = [];
      let active: Household | null = null;
      let attempts = 0;

      handlers.set('list_households', () => households);
      handlers.set('get_active_household', () => active);
      handlers.set('get_service_status', () => REACHABLE);
      handlers.set('create_household', (args) => {
        attempts += 1;

        if (attempts === 1) {
          throw {
            cause: 'connectivity',
            retryable: true,
            message_key: 'error.connectivity',
            request_id: args.requestId,
          };
        }

        active = CREATED;
        households = [summary(CREATED)];

        return CREATED;
      });

      const router = await launch('/onboarding');

      await screen.findByRole('heading', { name: 'Create your household' });

      const user = userEvent.setup();
      const field = screen.getByRole('textbox', { name: /Household name/ });
      const submit = screen.getByRole('button', { name: 'Create household' });

      await user.type(field, CREATED.name);
      await user.click(submit);

      const notice = await screen.findByRole('alert');

      expect(
        within(notice).getByRole('heading', { name: 'Not saved' }),
      ).toBeVisible();
      expect(notice).toHaveTextContent(
        'No connection. Check your internet connection and try again.',
      );

      // A failed create never looks like a created household.
      expect(router.state.location.pathname).toBe('/onboarding');
      expect(
        screen.getByRole('heading', { name: 'Create your household' }),
      ).toBeVisible();

      await user.click(screen.getByRole('button', { name: 'Try again' }));

      await waitFor(() => {
        expect(router.state.location.pathname).toBe('/');
      });

      await screen.findByRole('heading', { name: CREATED.name });

      const creates = createCalls();

      expect(creates).toHaveLength(2);
      expect(creates[0].args.name).toBe(CREATED.name);
      expect(creates[1].args.name).toBe(CREATED.name);
      expect(creates[1].args.requestId).toBe(creates[0].args.requestId);
    });
  });
});
