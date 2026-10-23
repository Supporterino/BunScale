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
import { act, cleanup, screen, waitFor } from '@testing-library/react';
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
import type { Rabbit, RabbitSummary } from '../../rabbits/queries';
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
let archivedRows: RabbitSummary[];

function payloadOf(args: InvokeArgs | undefined): Record<string, unknown> {
  return (args ?? {}) as Record<string, unknown>;
}

beforeEach(() => {
  calls = [];
  handlers = new Map();
  rows = [];
  archivedRows = [];
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
  handlers.set('list_rabbits', (args) =>
    args.includeArchived === true ? [...archivedRows] : [],
  );
  handlers.set('get_rabbit', (args) => ({
    ...CREATED,
    id: String(args.rabbitId),
    name: String(args.rabbitId) === 'rabbit-hazel-2' ? 'Hazel 2' : CREATED.name,
  }));
  /*
   * The rabbit dashboard's two reads (task 4.11). Opening a row now mounts the
   * trend and the History section, so a screen spec that opens one must answer
   * them; the answers are empty and the requested window is echoed, which keeps
   * the composition assertions about the surface rather than about a series.
   */
  handlers.set('get_display_weight_unit', () => 'grams');
  handlers.set('get_weight_entries', (args) => ({
    range: args.range,
    points: [],
    drop_threshold_percent: CREATED.weight_drop_threshold_percent,
  }));
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

  return { client, router };
}

/** The Add control currently on the dashboard, by its visible label. */
function addControl(): HTMLElement {
  return screen.getByRole('button', { name: t('rabbit.add') });
}

/** The dashboard's always-present polite region. */
function dashboardAnnouncement(): string {
  return (
    document.querySelector('[data-announcement="dashboard"]')?.textContent ?? ''
  );
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

  it("opens the selected rabbit's weight dashboard (DASH-6, DASH-7)", async () => {
    rows = [CREATED_ROW];
    answerDashboard();

    await launchDashboard();

    const user = userEvent.setup();

    await user.click(await screen.findByText(CREATED.name));

    // The row's control now leads to the rabbit dashboard: the profile plus the
    // trend and the History section, not the profile alone (task 4.11).
    expect(
      await screen.findByRole('region', { name: t('weight.trend.title') }),
    ).toBeVisible();
    expect(
      screen.getByRole('region', { name: t('weight.history.title') }),
    ).toBeVisible();
    expect(
      screen.getByRole('button', { name: t('weight.history.open') }),
    ).toBeVisible();

    // The trend asked the read for this rabbit at the route's default window.
    await waitFor(() => {
      expect(
        calls.some(
          (call) =>
            call.command === 'get_weight_entries' &&
            call.args.rabbitId === CREATED.id &&
            call.args.range === 'days_30',
        ),
      ).toBe(true);
    });
  });

  it('returns focus to the row that opened the detail when two names share a prefix', async () => {
    rows = [
      CREATED_ROW,
      {
        ...CREATED_ROW,
        rabbit: {
          ...CREATED_ROW.rabbit,
          id: 'rabbit-hazel-2',
          name: 'Hazel 2',
        },
      },
    ];
    answerDashboard();

    await launchDashboard();

    const user = userEvent.setup();
    const second = await screen.findByText('Hazel 2');

    await user.click(second);

    expect(
      await screen.findByRole('heading', { name: 'Hazel 2' }),
    ).toBeVisible();

    await user.click(screen.getByRole('button', { name: t('settings.back') }));

    // Focus is restored by the rabbit's id, not by a name prefix: "Hazel" is a
    // prefix of "Hazel 2" and must not capture the return.
    await waitFor(() => {
      expect(screen.getByText('Hazel 2').closest('button')).toHaveFocus();
    });
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

    // `onSaved` closes the form and the invalidated dashboard read answers
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

  it('reaches the archived list from the empty dashboard', async () => {
    // RAB-6: an all-archived household has zero active rows and presents the
    // empty state, so the way into the archived list cannot live behind rows.
    archivedRows = [
      {
        id: 'rabbit-willow',
        name: 'Willow',
        thumbnail_ref: null,
        is_archived: true,
      },
    ];
    answerDashboard();

    await launchDashboard();

    expect(
      await screen.findByRole('heading', { name: t('dashboard.emptyTitle') }),
    ).toBeVisible();

    // A first arrival on an empty dashboard is not a replacement: the polite
    // region exists but says nothing, and being visually hidden on itself it is
    // not a flex item adding a gap (finding 1).
    expect(dashboardAnnouncement()).toBe('');
    expect(
      document.querySelector('[data-announcement="dashboard"]'),
    ).toHaveClass('bs-visually-hidden');

    const user = userEvent.setup();

    await user.click(
      screen.getByRole('button', { name: t('rabbit.archivedList.title') }),
    );

    expect(await screen.findByText('Willow')).toBeVisible();
    expect(
      screen.getByRole('heading', { name: t('rabbit.archivedList.title') }),
    ).toBeVisible();
    expect(
      calls.some(
        (call) =>
          call.command === 'list_rabbits' && call.args.includeArchived === true,
      ),
    ).toBe(true);
  });

  it('keeps the archived entry point while the dashboard read is pending', async () => {
    answerDashboard();
    handlers.set('get_dashboard', () => new Promise<DashboardRow[]>(() => {}));

    await launchDashboard();

    expect(
      screen.getByRole('button', { name: t('rabbit.archivedList.title') }),
    ).toBeVisible();
    // The pending read has not claimed there are no rabbits.
    expect(
      screen.queryByRole('heading', { name: t('dashboard.emptyTitle') }),
    ).toBeNull();
  });

  it('keeps the archived entry point reachable from the create form and preserves its draft', async () => {
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

    await user.type(screen.getByRole('textbox', { name: /^Name/ }), 'Bramble');

    const entry = screen.getByRole('button', {
      name: t('rabbit.archivedList.title'),
    });

    // Present in the form state, and opening it must not unmount the form.
    await user.click(entry);

    expect(
      await screen.findByRole('heading', {
        name: t('rabbit.archivedList.title'),
      }),
    ).toBeVisible();
    expect(
      screen.queryByRole('heading', { name: t('rabbit.form.createTitle') }),
    ).toBeNull();

    await user.click(screen.getByRole('button', { name: t('settings.back') }));

    // The same form is back, with its unsaved draft rather than a blank one.
    expect(
      await screen.findByRole('heading', {
        name: t('rabbit.form.createTitle'),
      }),
    ).toBeVisible();
    expect(screen.getByRole('textbox', { name: /^Name/ })).toHaveValue(
      'Bramble',
    );
    expect(entry).toHaveFocus();
  });

  it('keeps the archived entry point reachable from the rabbit detail', async () => {
    rows = [CREATED_ROW];
    answerDashboard();

    await launchDashboard();

    const user = userEvent.setup();

    await user.click(await screen.findByText(CREATED.name));

    expect(
      await screen.findByRole('heading', { name: CREATED.name }),
    ).toBeVisible();

    const entry = screen.getByRole('button', {
      name: t('rabbit.archivedList.title'),
    });

    await user.click(entry);

    expect(
      await screen.findByRole('heading', {
        name: t('rabbit.archivedList.title'),
      }),
    ).toBeVisible();

    // The detail is hidden, not unmounted.
    expect(screen.queryByRole('heading', { name: CREATED.name })).toBeNull();

    await user.click(screen.getByRole('button', { name: t('settings.back') }));

    expect(
      await screen.findByRole('heading', { name: CREATED.name }),
    ).toBeVisible();
    expect(entry).toHaveFocus();
  });

  it('does not announce the active list emptying while the archived list is presented', async () => {
    rows = [CREATED_ROW];
    archivedRows = [
      {
        id: 'rabbit-willow',
        name: 'Willow',
        thumbnail_ref: null,
        is_archived: true,
      },
    ];
    answerDashboard();

    const { client } = await launchDashboard();
    const user = userEvent.setup();

    await user.click(
      screen.getByRole('button', { name: t('rabbit.archivedList.title') }),
    );

    expect(await screen.findByText('Willow')).toBeVisible();
    expect(dashboardAnnouncement()).toBe('');

    /*
     * The active rows now reach zero while the archived list is on screen —
     * the shape a `rabbit:changed` refresh has. The dashboard's region must
     * stay silent instead of reading its empty state over the archived screen
     * (finding 3).
     */
    rows = [];

    await act(async () => {
      await client.invalidateQueries();
    });

    expect(dashboardAnnouncement()).toBe('');
    expect(screen.getByText('Willow')).toBeVisible();
  });

  it('archives the last rabbit back to an announced empty state', async () => {
    rows = [CREATED_ROW];
    answerDashboard();
    handlers.set('archive_rabbit', () => {
      rows = [];
      archivedRows = [
        {
          id: CREATED.id,
          name: CREATED.name,
          thumbnail_ref: null,
          is_archived: true,
        },
      ];

      return { ...CREATED, is_archived: true };
    });

    await launchDashboard();

    const user = userEvent.setup();

    await user.click(await screen.findByText(CREATED.name));

    expect(
      await screen.findByRole('heading', { name: CREATED.name }),
    ).toBeVisible();

    // RAB-6, D1.8: one press, no confirmation.
    await user.click(screen.getByRole('button', { name: t('rabbit.archive') }));

    expect(
      await screen.findByRole('heading', { name: t('dashboard.emptyTitle') }),
    ).toBeVisible();
    expect(screen.queryByText(CREATED.name)).toBeNull();

    // The empty state replaced content after an action, and its copy was
    // written into the region that was mounted before the change.
    await waitFor(() => {
      expect(dashboardAnnouncement()).toContain(t('dashboard.emptyTitle'));
    });
    expect(dashboardAnnouncement()).toContain(t('dashboard.emptyBody'));

    // The entry point survives the transition, and the rabbit is reachable.
    const entry = screen.getByRole('button', {
      name: t('rabbit.archivedList.title'),
    });

    expect(entry).toBeVisible();

    await user.click(entry);

    expect(await screen.findByText(CREATED.name)).toBeVisible();
  });
});
