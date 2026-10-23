/*
 * The dashboard↔rabbit-form composition at the component layer (task 4.3;
 * RAB-4, DASH-8).
 *
 * The list's spec proves the add control calls its handler. This file proves
 * the next link: the dashboard mounts `RabbitForm` from that control in both the
 * empty and non-empty states, returns to the list on cancel, returns after a
 * confirmed create, and moves focus into the form and back to the control it
 * was opened from. That composition lives in `HouseholdDashboardScreen`, so the
 * spec launches the app's real route tree (`createTestRouter`) instead of
 * rendering the screen without the router context `AppShell` needs.
 *
 * What is not asserted here: the form's own field behaviour (its spec owns it)
 * and the arranged tab-bar/sidebar layout, which needs a browser (E2E).
 *
 * IPC is mocked at `invoke`; the event plugin is mocked too because the screen
 * mounts `useAppEvents`.
 */
import { RouterProvider } from '@tanstack/react-router';
import type { InvokeArgs } from '@tauri-apps/api/core';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { cleanup, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, it } from 'vitest';
import {
  createTestQueryClient,
  createTestRouter,
  describeAtBothBaselines,
  renderWithQueryClient,
} from '../../../test/setup';
import type { DashboardRow } from '../../dashboard/queries';
import { t } from '../../i18n';
import type { Rabbit } from '../../rabbits/queries';
import type { ServiceStatus } from '../../sync/entry-status';
import { resetSaveRetry } from '../../sync/save-retry';
import type { Household } from '../queries';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

/** The active household the dashboard presents. */
const HOUSEHOLD: Household = {
  id: 'household-home',
  name: 'Home household',
  created_at: AT,
  owner_member_id: 'member-1',
  is_active: true,
  is_owner: true,
};

/** A fresh reachable observation, so the entry guard reaches the dashboard. */
const REACHABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: true,
  cause: null,
  observed_at: AT,
};

/** The rabbit a successful create answers. */
const CREATED: Rabbit = {
  id: 'rabbit-hazel',
  household_id: HOUSEHOLD.id,
  name: 'Hazel',
  photo_ref: null,
  breed: null,
  dob: null,
  dob_unknown: false,
  sex: 'unknown',
  neuter_status: 'unknown',
  adoption_date: null,
  is_archived: false,
  weight_drop_threshold_percent: 5,
};

/** The dashboard row the read answers once the rabbit exists. */
const CREATED_ROW: DashboardRow = {
  rabbit: {
    id: CREATED.id,
    name: CREATED.name,
    thumbnail_ref: null,
    is_archived: false,
  },
  last_weighed_at: null,
  never_weighed: true,
  has_weight_drop_alert: false,
};

type MockCall = {
  readonly command: string;
  readonly args: Record<string, unknown>;
};

type MockHandler = (args: Record<string, unknown>) => unknown;

let calls: MockCall[];
let handlers: Map<string, MockHandler>;
let rows: DashboardRow[];

function payloadOf(args: InvokeArgs | undefined): Record<string, unknown> {
  return (args ?? {}) as Record<string, unknown>;
}

beforeEach(() => {
  calls = [];
  handlers = new Map();
  rows = [];
  resetSaveRetry();

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
  // Unmount before dropping the bridge, so `useAppEvents` can release its
  // listeners through the still-mocked `invoke`.
  cleanup();
  await Promise.resolve();
  await Promise.resolve();
  clearMocks();
  resetSaveRetry();
});

/** The entry guard's answers for "a household exists, service reachable". */
function answerDashboard(): void {
  handlers.set('list_households', () => [
    {
      id: HOUSEHOLD.id,
      name: HOUSEHOLD.name,
      created_at: HOUSEHOLD.created_at,
      is_active: true,
      is_owner: true,
      member_count: 1,
    },
  ]);
  handlers.set('get_active_household', () => HOUSEHOLD);
  handlers.set('get_service_status', () => REACHABLE);
  handlers.set('get_dashboard', () => rows);
  handlers.set('load_household', () => ({
    household_id: HOUSEHOLD.id,
    items: [],
    source: 'local',
    loaded_at: null,
  }));
}

/** Boot the app's real route tree at the dashboard. */
async function launchDashboard() {
  const client = createTestQueryClient();
  const router = createTestRouter({ client, initialEntries: ['/'] });

  await router.load();
  renderWithQueryClient(<RouterProvider router={router} />, { client });

  await screen.findByRole('main');

  return router;
}

/** The Add control currently on the dashboard, by its visible label. */
function addControl(): HTMLElement {
  return screen.getByRole('button', { name: t('rabbit.add') });
}

describeAtBothBaselines('Dashboard rabbit creation (RAB-4)', () => {
  it('opens the form from the empty state and returns focus on cancel', async () => {
    answerDashboard();

    await launchDashboard();

    expect(
      await screen.findByRole('heading', { name: t('dashboard.emptyTitle') }),
    ).toBeVisible();

    const user = userEvent.setup();

    await user.click(addControl());

    expect(
      await screen.findByRole('heading', {
        name: t('rabbit.form.createTitle'),
      }),
    ).toBeVisible();

    // The form owns focus once it replaces the control (finding 7).
    expect(screen.getByRole('textbox', { name: /^Name/ })).toHaveFocus();

    await user.click(screen.getByRole('button', { name: t('common.cancel') }));

    expect(
      await screen.findByRole('heading', { name: t('dashboard.emptyTitle') }),
    ).toBeVisible();
    expect(
      screen.queryByRole('heading', { name: t('rabbit.form.createTitle') }),
    ).toBeNull();
    expect(addControl()).toHaveFocus();
  });

  it('opens the form from the toolbar on a non-empty dashboard', async () => {
    rows = [CREATED_ROW];
    answerDashboard();

    await launchDashboard();

    expect(await screen.findByText(CREATED.name)).toBeVisible();

    const user = userEvent.setup();

    await user.click(addControl());

    expect(
      await screen.findByRole('heading', {
        name: t('rabbit.form.createTitle'),
      }),
    ).toBeVisible();

    await user.click(screen.getByRole('button', { name: t('common.cancel') }));

    expect(await screen.findByText(CREATED.name)).toBeVisible();
    expect(
      screen.queryByRole('heading', { name: t('rabbit.form.createTitle') }),
    ).toBeNull();
    expect(addControl()).toHaveFocus();
  });

  it('returns to the list after a confirmed create', async () => {
    answerDashboard();
    handlers.set('create_rabbit', () => {
      // The backend's post-state: the dashboard read now carries the rabbit.
      rows = [CREATED_ROW];

      return CREATED;
    });

    await launchDashboard();

    expect(
      await screen.findByRole('heading', { name: t('dashboard.emptyTitle') }),
    ).toBeVisible();

    const user = userEvent.setup();

    await user.click(addControl());
    await user.type(
      screen.getByRole('textbox', { name: /^Name/ }),
      CREATED.name,
    );
    await user.click(
      screen.getByRole('button', { name: t('rabbit.form.save') }),
    );

    // `onCreated` closes the form and the invalidated dashboard read answers
    // with the new row.
    expect(await screen.findByText(CREATED.name)).toBeVisible();
    expect(
      screen.queryByRole('heading', { name: t('rabbit.form.createTitle') }),
    ).toBeNull();
    expect(
      screen.queryByRole('heading', { name: t('dashboard.emptyTitle') }),
    ).toBeNull();
    expect(addControl()).toHaveFocus();

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_rabbit'),
      ).toHaveLength(1);
    });
  });
});
