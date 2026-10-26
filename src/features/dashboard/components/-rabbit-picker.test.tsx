/*
 * The quick-entry path's rabbit picker as a route (task 4.20; WT-2, NFR-9, Q12).
 *
 * `RabbitPicker` is a native `<select>` in the dashboard toolbar, and what it
 * promises is a route: choosing a rabbit opens the one weight-form route bound
 * to it (WT-2's "The quick-entry control offers a rabbit picker"). That
 * navigation needs a router, so this file launches the app's committed route
 * tree through `createTestRouter`/`RouterProvider`, the harness
 * `-rabbit-list-route.test.tsx` and `-weight.route.test.tsx` use:
 *
 *   - the control exists on the dashboard, labelled by `weight.picker.label`,
 *     with an unset prompt and one option per active rabbit — never-weighed
 *     included, because "no weigh-in yet" is exactly who is being weighed;
 *   - an archived rabbit is not offered: the options are `get_dashboard`'s
 *     active rows, never `list_rabbits`' archived answer (D1.6);
 *   - choosing a rabbit lands on `/weight` with `rabbit`, `mode: 'quick'` and
 *     `from: 'dashboard'` in the typed search (WT-2, D1.10);
 *   - NFR-9's operability is the native control's: a real `<select>` with the
 *     field's `<label>` as its accessible name, so a keyboard and VoiceOver get
 *     the platform's own picker instead of a div or a long press.
 *
 * Both component baselines are exercised through `describeAtBothBaselines`, as
 * the route spec beside it does. IPC is mocked at `invoke` (the
 * `testing-strategy` seam) and the event plugin too, because the dashboard
 * mounts `useAppEvents`.
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
import type { Rabbit, RabbitSummary } from '../../rabbits/queries';
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

/** The identifying part of a rabbit, which is all a row's summary carries. */
type FixtureRabbit = { readonly id: string; readonly name: string };

const HAZEL: FixtureRabbit = { id: 'rabbit-hazel', name: 'Hazel' };
const BRAMBLE: FixtureRabbit = { id: 'rabbit-bramble', name: 'Bramble' };
const CLOVER: FixtureRabbit = { id: 'rabbit-clover', name: 'Clover' };

/** The household's archived rabbit; it belongs to the archived list (RAB-6). */
const ARCHIVED = {
  id: 'rabbit-willow',
  name: 'Willow',
  thumbnail_ref: null,
  is_archived: true,
} satisfies RabbitSummary;

/** One dashboard row as the read model answers it (DASH-1 … DASH-4). */
function dashboardRow(
  rabbit: FixtureRabbit,
  lastWeighedAt: string | null,
): DashboardRow {
  return {
    rabbit: {
      id: rabbit.id,
      name: rabbit.name,
      thumbnail_ref: null,
      is_archived: false,
    },
    last_weighed_at: lastWeighedAt,
    never_weighed: lastWeighedAt === null,
    has_weight_drop_alert: false,
  };
}

/** A full profile, as `get_rabbit` answers the route the picker opens. */
function profile(rabbit: FixtureRabbit): Rabbit {
  return {
    id: rabbit.id,
    household_id: HOUSEHOLD.id,
    name: rabbit.name,
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
}

const HAZEL_ROW = dashboardRow(HAZEL, AT);
const BRAMBLE_ROW = dashboardRow(BRAMBLE, AT);
/** CLOVER has no weigh-in; the picker must still offer her (WT-2, DASH-2). */
const CLOVER_ROW = dashboardRow(CLOVER, null);

const ROWS: DashboardRow[] = [HAZEL_ROW, BRAMBLE_ROW, CLOVER_ROW];
const PROFILES = [profile(HAZEL), profile(BRAMBLE), profile(CLOVER)];

type Handler = (args: Record<string, unknown>) => unknown;

let answers: Map<string, Handler>;

beforeEach(() => {
  answers = new Map<string, Handler>([
    ['list_households', () => [SUMMARY]],
    ['get_active_household', () => HOUSEHOLD],
    ['get_dashboard', () => ROWS],
    // The archived read answers an archived rabbit; the picker must not take it.
    ['list_rabbits', () => [ARCHIVED]],
    [
      'get_rabbit',
      (args) =>
        PROFILES.find((rabbit) => rabbit.id === String(args.rabbitId)) ?? null,
    ],
    ['get_display_weight_unit', () => 'grams'],
    [
      'get_weight_entries',
      (args) => ({
        range: args.range,
        points: [],
        drop_threshold_percent: 5,
      }),
    ],
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

/** Boot the app's committed route tree and wait for the picker's rows. */
async function launchDashboard() {
  const client = createTestQueryClient();
  const router = createTestRouter({ client, initialEntries: ['/'] });

  await router.load();
  renderWithQueryClient(<RouterProvider router={router} />, { client });

  await screen.findByRole('combobox', { name: t('weight.picker.label') });

  return router;
}

/** The toolbar picker, by the one name its field gives it. */
function pickerControl(): HTMLElement {
  return screen.getByRole('combobox', { name: t('weight.picker.label') });
}

describeAtBothBaselines('Dashboard rabbit picker (WT-2, NFR-9)', () => {
  it('offers a labelled picker with an unset prompt and every active rabbit', async () => {
    await launchDashboard();

    const picker = pickerControl();

    // NFR-9: a directly visible control on the dashboard, not a hidden menu.
    expect(picker).toBeVisible();
    expect(screen.getByRole('main')).toContainElement(picker);

    const options = within(picker).getAllByRole('option');

    // The unset prompt first — the control opens on no rabbit — then one option
    // per row in the read model's order. CLOVER is never-weighed and still
    // offered, because a rabbit without an entry is one to weigh (WT-2).
    expect(options.map((option) => option.textContent)).toEqual([
      t('weight.picker.label'),
      HAZEL.name,
      BRAMBLE.name,
      CLOVER.name,
    ]);
    expect(picker).toHaveValue('');
  });

  it('does not offer an archived rabbit', async () => {
    // `list_rabbits` answers Willow for the archived list; the options are
    // `get_dashboard`'s active rows, so she is never offered (D1.6).
    await launchDashboard();

    const picker = pickerControl();

    expect(
      within(picker).queryByRole('option', { name: ARCHIVED.name }),
    ).toBeNull();
    expect(
      within(picker)
        .getAllByRole('option')
        .map((option) => option.textContent),
    ).not.toContain(ARCHIVED.name);
  });

  it('opens the weight form bound to the chosen rabbit', async () => {
    const router = await launchDashboard();

    await userEvent.setup().selectOptions(pickerControl(), HAZEL.id);

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/weight');
    });

    // The binding travels in typed search, not in picker state: the same
    // destination a row's weight control names (WT-2, DASH-9, D1.10). The
    // route's validator fills the range pair both views share.
    expect(router.state.location.search).toEqual({
      trend: DEFAULT_WEIGHT_RANGE,
      history: DEFAULT_WEIGHT_RANGE,
      rabbit: HAZEL.id,
      mode: 'quick',
      from: 'dashboard',
    });
  });

  it('is a native select named by its field label', async () => {
    await launchDashboard();

    const picker = screen.getByRole('combobox', {
      name: t('weight.picker.label'),
    });

    // NFR-9: the platform's own picker, so touch, a keyboard and VoiceOver get
    // the behaviour they already know — not a div with role and key handlers.
    expect(picker.tagName).toBe('SELECT');
    expect(picker).toHaveAccessibleName(t('weight.picker.label'));
    expect(screen.getByLabelText(t('weight.picker.label'))).toBe(picker);
  });
});
