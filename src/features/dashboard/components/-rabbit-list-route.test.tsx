/*
 * The dashboard rows' quick-weight control as a route (task 4.7; DASH-9, WT-2,
 * NFR-9, D1.10).
 *
 * `rabbit-list.test.tsx` owns the list's presentation and renders it through
 * `renderWithQueryClient`, which supplies no router — and the reworked list
 * navigates through `useNavigate`. This file therefore launches the app's
 * committed route tree (`createTestRouter`/`RouterProvider`, the harness the
 * `-rabbit-dashboard.route.test.tsx` and `-weight.route.test.tsx` specs use)
 * and asserts the wiring a component render cannot see:
 *
 *   - every row carries a real button whose accessible name is qualified by
 *     that row's rabbit, and the names differ per row (DASH-9, NFR-9);
 *   - activating it lands on `/weight` with the rabbit, the shared quick mode
 *     and the dashboard origin in the typed search (DASH-9, WT-2, D1.10);
 *   - the quick control is a sibling of the row's open-dashboard control, not
 *     a child of it, so both stay operable (NFR-9);
 *   - the second row's control binds the second rabbit — the control is
 *     per-row, not per-list (DASH-9);
 *   - the list no longer mounts the weight form inline: the form belongs to
 *     the `/weight` route, and activating a control replaces the dashboard
 *     instead of swapping a form into it (D1.10's "one form route").
 *
 * Both component baselines are exercised through `describeAtBothBaselines`, as
 * the dashboard screen's own route spec does: the router and cache are built
 * per test, so the baseline only changes the viewport the committed tree is
 * mounted at. Instants are fixed and no timers are used.
 *
 * IPC is mocked at `invoke` (the `testing-strategy` seam); the event plugin is
 * mocked too because the dashboard mounts `useAppEvents`.
 */
import { RouterProvider } from '@tanstack/react-router';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { cleanup, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, it } from 'vitest';
import {
  createTestQueryClient,
  createTestRouter,
  describeAtBothBaselines,
  renderWithQueryClient,
} from '../../../test/setup';
import type { Household, HouseholdSummary } from '../../households/queries';
import { t } from '../../i18n';
import type { Rabbit } from '../../rabbits/queries';
import { DEFAULT_WEIGHT_RANGE } from '../../weight/search';
import type { DashboardRow } from '../queries';

/** Stands in for the contract's `Instant`; every fixture date is fixed. */
const AT = '2026-10-20T09:00:00Z';

const HOUSEHOLD: Household = {
  id: 'household-home',
  name: 'Home household',
  created_at: AT,
  owner_member_id: 'member-1',
  is_active: true,
  is_owner: true,
};

const SUMMARY: HouseholdSummary = {
  id: HOUSEHOLD.id,
  name: HOUSEHOLD.name,
  created_at: HOUSEHOLD.created_at,
  is_active: true,
  is_owner: true,
  member_count: 1,
};

const HAZEL: Rabbit = {
  id: 'rabbit-hazel',
  household_id: HOUSEHOLD.id,
  name: 'Hazel',
  photo_ref: null,
  breed: null,
  dob: null,
  dob_unknown: true,
  sex: 'unknown',
  neuter_status: 'unknown',
  adoption_date: null,
  is_archived: false,
  weight_drop_threshold_percent: 5,
};

const BRAMBLE: Rabbit = {
  ...HAZEL,
  id: 'rabbit-bramble',
  name: 'Bramble',
};

const RABBITS: readonly Rabbit[] = [HAZEL, BRAMBLE];

/** One dashboard row as the read model answers it (DASH-1 … DASH-4). */
function dashboardRow(rabbit: Rabbit): DashboardRow {
  return {
    rabbit: {
      id: rabbit.id,
      name: rabbit.name,
      thumbnail_ref: null,
      is_archived: false,
    },
    last_weighed_at: AT,
    never_weighed: false,
    has_weight_drop_alert: false,
  };
}

const HAZEL_ROW = dashboardRow(HAZEL);
const BRAMBLE_ROW = dashboardRow(BRAMBLE);

type Handler = (args: Record<string, unknown>) => unknown;

let rows: DashboardRow[];
let answers: Map<string, Handler>;

beforeEach(() => {
  rows = [HAZEL_ROW];
  answers = new Map<string, Handler>([
    ['list_households', () => [SUMMARY]],
    ['get_active_household', () => HOUSEHOLD],
    ['get_dashboard', () => rows],
    [
      'get_rabbit',
      (args) =>
        RABBITS.find((rabbit) => rabbit.id === String(args.rabbitId)) ?? null,
    ],
    ['get_display_weight_unit', () => 'grams'],
    [
      'load_household',
      () => ({
        household_id: HOUSEHOLD.id,
        items: [],
        source: 'local',
        loaded_at: null,
      }),
    ],
  ]);

  mockIPC(
    (command, args) => {
      const payload = (args ?? {}) as Record<string, unknown>;
      const answer = answers.get(command);

      if (answer === undefined) {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return answer(payload);
    },
    { shouldMockEvents: true },
  );
});

afterEach(async () => {
  // Unmount before dropping the bridge, so `useAppEvents` releases its
  // listeners through the still-mocked `invoke`.
  cleanup();
  await Promise.resolve();
  clearMocks();
});

/** Boot the app's committed route tree at the dashboard. */
async function launchDashboard() {
  const client = createTestQueryClient();
  const router = createTestRouter({ client, initialEntries: ['/'] });

  await router.load();
  renderWithQueryClient(<RouterProvider router={router} />, { client });

  /*
   * The toolbar's picker is rendered by the same resolved answer as the rows,
   * and it is unique (the shell's navigation lists are not pickers); waiting
   * for it avoids a rabbit name, which the picker repeats as an `<option>`
   * (task 4.20).
   */
  await screen.findByRole('combobox', { name: t('weight.picker.label') });

  return router;
}

/** The quick-weight control of one row, by its rabbit-qualified name. */
function quickWeightControl(rabbit: Rabbit): HTMLElement {
  return screen.getByRole('button', {
    name: `${rabbit.name} ${t('weight.log')}`,
  });
}

/** The dashboard list's row for a rabbit, by the id the row carries. */
function rowOf(rabbit: Rabbit): HTMLElement {
  const row = document.querySelector<HTMLElement>(
    `li[data-rabbit-id="${rabbit.id}"]`,
  );

  if (row === null) {
    throw new Error(`no dashboard row for "${rabbit.name}"`);
  }

  return row;
}

/** Wait for the dashboard read to paint a rabbit's row. */
async function findRow(rabbit: Rabbit): Promise<HTMLElement> {
  await waitFor(() => {
    expect(rowOf(rabbit)).toBeInTheDocument();
  });

  return rowOf(rabbit);
}

describeAtBothBaselines('Dashboard quick-weight rows (DASH-9, WT-2)', () => {
  it('gives each row a real button named for that rabbit', async () => {
    rows = [HAZEL_ROW, BRAMBLE_ROW];

    await launchDashboard();
    await findRow(BRAMBLE);

    // Each row's control is named by that row's rabbit plus the catalog's
    // action, so the literal example the task names holds for Hazel…
    const hazel = within(rowOf(HAZEL)).getByRole('button', {
      name: 'Hazel Log weight',
    });
    // …and the same rule qualifies the other row differently.
    const bramble = within(rowOf(BRAMBLE)).getByRole('button', {
      name: `${BRAMBLE.name} ${t('weight.log')}`,
    });

    for (const control of [hazel, bramble]) {
      // A real, enabled button, not a styled span or a hidden menu item.
      expect(control.tagName).toBe('BUTTON');
      expect(control).toBeEnabled();
      expect(control).toBeVisible();
    }

    expect(hazel).not.toBe(bramble);
  });

  it('opens the weight route bound to the row’s rabbit', async () => {
    rows = [HAZEL_ROW];

    const router = await launchDashboard();
    const control = quickWeightControl(HAZEL);

    expect(control).toBeVisible();

    await userEvent.setup().click(control);

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/weight');
    });

    // The binding travels in typed search, not in component state. The route's
    // validator fills the range pair both views share; none of the three
    // binding fields is optional.
    expect(router.state.location.search).toEqual({
      trend: DEFAULT_WEIGHT_RANGE,
      history: DEFAULT_WEIGHT_RANGE,
      rabbit: HAZEL.id,
      mode: 'quick',
      from: 'dashboard',
    });
  });

  it('keeps the quick control outside the row’s open-dashboard control', async () => {
    rows = [HAZEL_ROW];

    await launchDashboard();

    const row = rowOf(HAZEL);
    const quick = quickWeightControl(HAZEL);
    const buttons = within(row).getAllByRole('button');

    // Both targets are present in one row, and they are the only two.
    expect(buttons).toHaveLength(2);

    const open = buttons.find((button) => button !== quick);

    expect(open).toBeDefined();

    if (open === undefined) {
      return;
    }

    // Siblings in the row, not nested inside each other: nesting interactive
    // controls would make one of them unreachable by touch (NFR-9).
    expect(open.contains(quick)).toBe(false);
    expect(quick.contains(open)).toBe(false);
  });

  it('binds the control to its own row’s rabbit, not the list’s', async () => {
    rows = [HAZEL_ROW, BRAMBLE_ROW];

    const router = await launchDashboard();
    await findRow(BRAMBLE);

    await userEvent.setup().click(quickWeightControl(BRAMBLE));

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/weight');
    });

    expect(router.state.location.search).toMatchObject({
      rabbit: BRAMBLE.id,
      mode: 'quick',
      from: 'dashboard',
    });
    expect(router.state.location.search.rabbit).not.toBe(HAZEL.id);
  });

  it('does not mount the weight form on the dashboard', async () => {
    rows = [HAZEL_ROW];

    const router = await launchDashboard();
    const dashboardMain = screen.getByRole('main');

    expect(dashboardMain).toHaveAttribute('data-household-entry', 'household');

    // With the rows painted, the dashboard holds no form and no weight value
    // field: the form is the `/weight` route's, not the list's (D1.10).
    expect(document.querySelector('form')).toBeNull();
    expect(
      screen.queryByRole('textbox', {
        name: `${t('weight.form.valueLabel')} (${t('settings.displayUnit.grams')})`,
      }),
    ).toBeNull();

    await userEvent.setup().click(quickWeightControl(HAZEL));

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/weight');
    });

    // The route replaced the dashboard rather than growing a form inside it.
    await waitFor(() => {
      expect(dashboardMain.isConnected).toBe(false);
    });
    expect(
      document.querySelector('[data-household-entry="household"]'),
    ).toBeNull();
    expect(
      screen.queryByRole('button', {
        name: `${HAZEL.name} ${t('weight.log')}`,
      }),
    ).toBeNull();

    // The form that is on screen belongs to the weight route's main.
    const value = await screen.findByRole('textbox', {
      name: `${t('weight.form.valueLabel')} (${t('settings.displayUnit.grams')})`,
    });
    const form = value.closest('form');

    expect(form).not.toBeNull();
    // The form that is on screen belongs to the route that replaced the
    // dashboard, not to a dashboard that grew one.
    const weightMain = screen.getByRole('main');

    expect(weightMain).not.toHaveAttribute('data-household-entry');
    expect(weightMain.contains(form)).toBe(true);
  });
});
