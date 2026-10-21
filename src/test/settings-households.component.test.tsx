/*
 * Settings & Profile's household section at the component layer (task 5.12;
 * HS-4, D0.14).
 *
 * Two acceptance scenarios are asserted here:
 *
 *   - "All households are listed with the active one indicated" — every row the
 *     book holds is rendered, exactly one row carries the active marker
 *     (`aria-current="true"` plus the `Active` badge), and the owner badge
 *     follows `is_owner` rather than the active flag;
 *   - "Create an additional household without onboarding" — the inline form
 *     adds the household, marks it active, closes, and leaves the location on
 *     `/settings`: no `/onboarding`, no trip through the dashboard.
 *
 * The create form's retry identity (D0.14) is asserted as well. Q29 records that
 * no earlier task owned a screen-level assertion for it; the Settings form keeps
 * its own handle discipline ("one operation per intent, reused on retry while
 * the name is unchanged"), so the retry is worth its own test here and not only
 * on onboarding.
 *
 * What is NOT assertable at this layer, and who owns it:
 *   - switching the active household is HS-3 and already covered at the
 *     mutation layer (`src/features/households/switch.test.tsx`); this file only
 *     asserts the marker the list renders;
 *   - "the person is its first member" is a domain fact behind
 *     `create_household`, owned by Rust 5.1;
 *   - geometry (44 pt rows) and the tab-bar-vs-sidebar arrangement need a layout
 *     engine and a real browser; E2E 5.14/5.17 own them. jsdom reports zeroes
 *     for `getBoundingClientRect` and loads no app stylesheet.
 *
 * The suite runs at both component baselines through `describeAtBothBaselines`;
 * nothing here branches on the viewport, and the arranged layout stays E2E's
 * claim.
 *
 * IPC is mocked at `invoke` and the event plugin is mocked because the screen
 * mounts `useAppEvents`.
 */

import { RouterProvider } from '@tanstack/react-router';
import type { InvokeArgs } from '@tauri-apps/api/core';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { cleanup, screen, within } from '@testing-library/react';
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

/** The household that is active when Settings opens. */
const HOME: HouseholdSummary = {
  id: 'household-home',
  name: 'Home household',
  created_at: AT,
  is_active: true,
  is_owner: false,
  member_count: 2,
};

/** The same household as the active read answers it (HS-3, DM-3). */
const HOME_ACTIVE: Household = {
  id: HOME.id,
  name: HOME.name,
  created_at: HOME.created_at,
  owner_member_id: 'member-2',
  is_active: true,
  is_owner: false,
};

/** The second held household: not active, owned by this person. */
const BEACH: HouseholdSummary = {
  id: 'household-beach',
  name: 'Beach house',
  created_at: AT,
  is_active: false,
  is_owner: true,
  member_count: 1,
};

/** A fresh reachable observation, so no offline notice is rendered. */
const REACHABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: true,
  cause: null,
  observed_at: AT,
};

/** The create control's label (`t('household.create')`). */
const CREATE = 'Create household';

/** One `invoke` call the app made, as the backend would have received it. */
type MockCall = {
  readonly command: string;
  readonly args: Record<string, unknown>;
};

/** One mocked command's answer; a throw is a rejected `invoke`. */
type MockHandler = (args: Record<string, unknown>) => unknown;

/** The normalized payload of one `invoke` call. */
function payloadOf(args: InvokeArgs | undefined): Record<string, unknown> {
  return (args ?? {}) as Record<string, unknown>;
}

/** The household the additional-create flow produces. */
function createdHousehold(args: Record<string, unknown>): Household {
  return {
    id: 'household-beach',
    name: String(args.name),
    created_at: AT,
    owner_member_id: 'member-1',
    is_active: true,
    is_owner: true,
  };
}

/** That household as a held-list row, marked active (HS-4). */
function createdRow(created: Household): HouseholdSummary {
  return {
    id: created.id,
    name: created.name,
    created_at: created.created_at,
    is_active: true,
    is_owner: true,
    member_count: 1,
  };
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
  // Unmount while the mocked bridge is still installed (the screen releases its
  // `listen` subscriptions on unmount), then drop the mock.
  cleanup();
  await Promise.resolve();
  await Promise.resolve();

  clearMocks();
});

/** The identity and display-unit reads the rest of the screen performs. */
function answerOtherReads(): void {
  handlers.set('get_identity', () => ({
    account_user_id: 'account-1',
    display_name: 'Ana',
    resolved: true,
  }));
  handlers.set('get_display_weight_unit', () => 'grams');
  handlers.set('get_service_status', () => REACHABLE);
}

/** Open Settings as its own route, on the app's committed route tree. */
async function openSettings() {
  const client = createTestQueryClient();
  const router = createTestRouter({
    client,
    initialEntries: ['/settings'],
  });

  await router.load();
  renderWithQueryClient(<RouterProvider router={router} />, { client });
  await screen.findByRole('heading', { name: 'Settings & Profile' });

  return router;
}

describeAtBothBaselines('Settings: held households (HS-4)', () => {
  describe('"All households are listed with the active one indicated"', () => {
    it('lists every household with one active marker', async () => {
      handlers.set('list_households', () => [HOME, BEACH]);
      handlers.set('get_active_household', () => HOME_ACTIVE);
      answerOtherReads();

      await openSettings();

      expect(await screen.findByText('Home household')).toBeVisible();
      expect(screen.getByText('Beach house')).toBeVisible();

      const current = document.querySelectorAll('[aria-current="true"]');

      expect(current).toHaveLength(1);

      const activeRow = current[0] as HTMLElement;

      expect(within(activeRow).getByText('Home household')).toBeVisible();
      expect(within(activeRow).getByText('Active')).toBeVisible();
      expect(within(activeRow).queryByText('Owner')).toBeNull();

      // The owner marker follows `is_owner`, not the active flag: the other
      // household is owned by this person and stays switchable.
      const other = screen.getByRole('button', { name: /Beach house/ });

      expect(within(other).getByText('Owner')).toBeVisible();
      expect(within(other).getByText('Make active')).toBeVisible();
      expect(within(other).queryByText('Active')).toBeNull();

      // Exactly one indication of each fact.
      expect(screen.getAllByText('Active')).toHaveLength(1);
      expect(screen.getAllByText('Owner')).toHaveLength(1);
    });
  });

  describe('"Create an additional household without onboarding"', () => {
    it('adds the household in place and stays on Settings', async () => {
      let households: HouseholdSummary[] = [HOME];
      let active: Household | null = HOME_ACTIVE;

      handlers.set('list_households', () => households);
      handlers.set('get_active_household', () => active);
      answerOtherReads();
      handlers.set('create_household', (args) => {
        const created = createdHousehold(args);

        active = created;
        households = [{ ...HOME, is_active: false }, createdRow(created)];

        return created;
      });

      const router = await openSettings();

      await screen.findByText('Home household');

      const user = userEvent.setup();
      const open = screen.getByRole('button', {
        name: 'Create another household',
      });

      await user.click(open);

      const field = screen.getByRole('textbox', { name: /Household name/ });

      await user.type(field, 'Beach house');
      await user.click(screen.getByRole('button', { name: CREATE }));

      await screen.findByText('Beach house');

      // The form closed and the location never moved: the first-run flow is not
      // presented and the dashboard is not visited on the way.
      expect(
        screen.queryByRole('textbox', { name: /Household name/ }),
      ).toBeNull();
      expect(router.state.location.pathname).toBe('/settings');
      expect(
        screen.queryByRole('heading', { name: 'Create your household' }),
      ).toBeNull();

      const creates = createCalls();

      expect(creates).toHaveLength(1);
      expect(creates[0].args.name).toBe('Beach house');

      const current = document.querySelectorAll('[aria-current="true"]');

      expect(current).toHaveLength(1);
      expect(
        within(current[0] as HTMLElement).getByText('Beach house'),
      ).toBeVisible();
    });

    it('re-sends the same request_id when retried', async () => {
      let households: HouseholdSummary[] = [HOME];
      let active: Household | null = HOME_ACTIVE;
      let attempts = 0;

      handlers.set('list_households', () => households);
      handlers.set('get_active_household', () => active);
      answerOtherReads();
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

        const created = createdHousehold(args);

        active = created;
        households = [{ ...HOME, is_active: false }, createdRow(created)];

        return created;
      });

      const router = await openSettings();

      await screen.findByText('Home household');

      const user = userEvent.setup();

      await user.click(
        screen.getByRole('button', { name: 'Create another household' }),
      );

      const field = screen.getByRole('textbox', { name: /Household name/ });

      await user.type(field, 'Beach house');
      await user.click(screen.getByRole('button', { name: CREATE }));

      const notice = await screen.findByRole('alert');

      expect(notice).toHaveTextContent(
        'No connection. Check your internet connection and try again.',
      );

      // A refused create does not close the form or move the route.
      expect(router.state.location.pathname).toBe('/settings');

      await user.click(screen.getByRole('button', { name: 'Try again' }));

      await screen.findByText('Beach house');

      const creates = createCalls();

      expect(creates).toHaveLength(2);
      expect(creates[0].args.name).toBe('Beach house');
      expect(creates[1].args.name).toBe('Beach house');
      expect(creates[1].args.requestId).toBe(creates[0].args.requestId);
    });
  });
});
