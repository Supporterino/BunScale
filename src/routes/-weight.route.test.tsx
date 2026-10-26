/*
 * The weight form's route (task 4.18; WT-2, DASH-9, D1.10).
 *
 * D1.10 says both quick paths open *one* weight form route with the rabbit bound
 * through typed search parameters, and that cancel and a confirmed save return
 * to the surface that opened it. This spec launches the app's committed route
 * tree on that route, as a deep link would, so what is asserted is the wiring
 * rather than `WeightForm` in isolation (`WeightForm.test.tsx` owns the form and
 * `../features/weight/search.test.ts` owns the validator):
 *
 *   - `?rabbit=…&mode=quick&from=dashboard` presents the form bound to that
 *     rabbit, and the save that leaves it names the bound rabbit;
 *   - Cancel returns to `/` with the rabbit id in the history entry that
 *     `HouseholdDashboardScreen` restores row focus from (4.16);
 *   - a confirmed save returns the same way, and a `from=rabbit` save returns to
 *     that rabbit's dashboard with the windows the form was opened at;
 *   - every read-gated state still offers Back, with the catalog's copy;
 *   - focus lands on the value field when a form is drawn, and on Back when the
 *     state drew no form instead (Q11's dead-end reason for the wrapper).
 *
 * The file is prefixed `-` because it lives beside the route files: the router
 * plugin's `routeFileIgnorePrefix` is `-`, so it is not treated as a route.
 *
 * The two baselines are not split here: the 4.16 route precedent launches the
 * same committed tree once, and the adaptive arrangement jsdom cannot see is an
 * E2E assertion (see `src/test/setup.ts`). IPC is mocked at `invoke`, the
 * `testing-strategy` seam the other route and feature specs use; the event
 * plugin is mocked because returning to `/` mounts `useAppEvents`.
 */
import type { QueryClient } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import {
  act,
  cleanup,
  fireEvent,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { DashboardRow } from '../features/dashboard/queries';
import type {
  Household,
  HouseholdSummary,
} from '../features/households/queries';
import { t } from '../features/i18n';
import type { Rabbit } from '../features/rabbits/queries';
import type { WeightUnit } from '../features/settings/display-unit';
import { settingsKeys } from '../features/settings/keys';
import type { WeightTrend } from '../features/weight/queries';
import {
  createTestQueryClient,
  createTestRouter,
  renderWithQueryClient,
} from '../test/setup';

/** Stands in for the contract's `Instant`. */
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

const RABBIT: Rabbit = {
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

const ROW: DashboardRow = {
  rabbit: {
    id: RABBIT.id,
    name: RABBIT.name,
    thumbnail_ref: null,
    is_archived: false,
  },
  last_weighed_at: null,
  never_weighed: true,
  has_weight_drop_alert: false,
};

/** One point, enough for the rabbit dashboard's series read to resolve. */
const POINTS: WeightTrend['points'] = [
  {
    id: 'weight-1',
    value_grams: 2000,
    timestamp: '2026-10-01T09:00:00Z',
    has_weight_drop_alert: false,
  },
];

/** The command's confirmed answer: the entry the form just saved. */
const SAVED = {
  id: 'weight-2',
  rabbit_id: RABBIT.id,
  value_grams: 2000,
  timestamp: AT,
  created_at: AT,
  note: null,
  photo_refs: [],
  has_weight_drop_alert: false,
};

/** The two unit names the value field's accessible name uses. */
const UNIT_NAMES = {
  grams: t('settings.displayUnit.grams'),
  kilograms: t('settings.displayUnit.kilograms'),
} as const;

/** A connectivity refusal: retryable, so the failure surfaces offer a refresh. */
function connectivityError() {
  return {
    cause: 'connectivity',
    retryable: true,
    message_key: 'error.connectivity',
    request_id: null,
  };
}

type Handler = (args: Record<string, unknown>) => unknown;
type Call = { command: string; args: Record<string, unknown> };

let answers: Map<string, Handler>;
let calls: Call[];

beforeEach(() => {
  calls = [];
  answers = new Map<string, Handler>([
    ['get_active_household', () => HOUSEHOLD],
    ['list_households', () => [SUMMARY]],
    ['get_dashboard', () => [ROW]],
    ['get_rabbit', () => RABBIT],
    ['get_display_weight_unit', () => 'grams'],
    [
      'get_weight_entries',
      (args) => ({
        range: String(args.range),
        points: POINTS,
        drop_threshold_percent: RABBIT.weight_drop_threshold_percent,
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
    ['create_weight_entry', () => SAVED],
  ]);

  mockIPC(
    (command, args) => {
      const payload = (args ?? {}) as Record<string, unknown>;
      calls.push({ command, args: payload });

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
  // Unmount before dropping the bridge, so the event hook releases its
  // listeners through the still-mocked `invoke`.
  cleanup();
  await Promise.resolve();
  clearMocks();
});

/** Boot the app's route tree at `path`, optionally seeding the query cache. */
async function launch(path: string, seed?: (client: QueryClient) => void) {
  const client = createTestQueryClient();
  seed?.(client);

  const router = createTestRouter({ client, initialEntries: [path] });
  await router.load();
  renderWithQueryClient(<RouterProvider router={router} />, { client });

  return router;
}

/** The navigation bar's leading control: the way out of every state (Q11). */
function backControl(): HTMLElement {
  return screen.getByRole('button', { name: t('settings.back') });
}

async function findValueField(): Promise<HTMLElement> {
  return screen.findByRole('textbox', {
    name: `${t('weight.form.valueLabel')} (${UNIT_NAMES.grams})`,
  });
}

/** Type a value in grams and submit; the unit read is cached or mocked. */
async function enterAndSave(value: string): Promise<void> {
  fireEvent.change(await findValueField(), { target: { value } });
  await userEvent
    .setup()
    .click(screen.getByRole('button', { name: t('weight.form.save') }));
}

/**
 * The bound rabbit's title in the dashboard row a return lands on, found by the
 * row's `data-rabbit-id`. The toolbar's picker repeats every rabbit's name as
 * an `<option>`, so the name is looked up inside the row rather than
 * document-wide (task 4.20).
 */
async function rowTitle(): Promise<HTMLElement> {
  const row = await waitFor(() => {
    const found = document.querySelector<HTMLElement>(
      `li[data-rabbit-id="${RABBIT.id}"]`,
    );

    if (found === null) {
      throw new Error(`no dashboard row for "${RABBIT.name}"`);
    }

    return found;
  });

  return within(row).getByText(RABBIT.name);
}

describe('the weight form route (WT-2, DASH-9, D1.10)', () => {
  it('opens the form bound to the rabbit the search names', async () => {
    await launch('/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard');

    // The profile read is asked with the search's rabbit, and the form it
    // gates names the same rabbit above the fields.
    expect(await screen.findByText(RABBIT.name)).toBeVisible();
    expect(
      calls.some(
        (call) =>
          call.command === 'get_rabbit' && call.args.rabbitId === RABBIT.id,
      ),
    ).toBe(true);

    await findValueField();

    expect(
      screen.getByRole('button', { name: t('common.cancel') }),
    ).toBeVisible();
  });

  it('saves the entry for the bound rabbit and returns to the dashboard row', async () => {
    const router = await launch(
      '/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard',
    );

    await enterAndSave('2000');

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/');
    });

    // The binding travelled into the write, not just into the profile read.
    expect(
      calls.some(
        (call) =>
          call.command === 'create_weight_entry' &&
          call.args.rabbitId === RABBIT.id,
      ),
    ).toBe(true);

    // The return carries the id the dashboard's row-focus consumer reads
    // (4.16); without it the person lands at the top of the list.
    expect(router.state.location.state).toMatchObject({
      focusRabbitId: RABBIT.id,
    });

    expect(await rowTitle()).toBeVisible();

    await waitFor(async () => {
      expect((await rowTitle()).closest('button')).toHaveFocus();
    });
  });

  it('returns to the dashboard from Cancel without writing anything', async () => {
    const router = await launch(
      '/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard',
    );

    await findValueField();
    await userEvent
      .setup()
      .click(screen.getByRole('button', { name: t('common.cancel') }));

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/');
    });

    expect(calls.some((call) => call.command === 'create_weight_entry')).toBe(
      false,
    );
    expect(router.state.location.state).toMatchObject({
      focusRabbitId: RABBIT.id,
    });
  });

  it('returns to the rabbit dashboard with its windows for from=rabbit', async () => {
    const router = await launch(
      '/weight?rabbit=rabbit-hazel&mode=quick&from=rabbit&trend=days_7&history=all',
    );

    await enterAndSave('2000');

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/rabbits/rabbit-hazel');
    });

    // The range pair the form was opened at comes back, so the rabbit dashboard
    // is the one the person left rather than the default window (WT-6, D1.4).
    expect(router.state.location.search).toEqual({
      trend: 'days_7',
      history: 'all',
    });
  });

  it('offers Back while the display unit is still loading', async () => {
    let resolveUnit: (unit: WeightUnit) => void = () => {};
    const pending = new Promise<WeightUnit>((resolve) => {
      resolveUnit = resolve;
    });

    answers.set('get_display_weight_unit', () => pending);

    await launch('/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard');

    expect(
      await screen.findByText(t('settings.displayUnit.loading')),
    ).toBeVisible();
    expect(backControl()).toBeVisible();

    await act(async () => {
      resolveUnit('grams');
    });
  });

  it('offers Back and the unit failure copy when the unit read failed', async () => {
    answers.set('get_display_weight_unit', () => {
      throw connectivityError();
    });

    await launch('/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard');

    expect(
      await screen.findByText(t('settings.displayUnit.heading')),
    ).toBeVisible();
    expect(screen.getByText(t('error.connectivity'))).toBeVisible();
    expect(backControl()).toBeVisible();
  });

  it('offers Back and the profile failure copy when the rabbit read failed', async () => {
    answers.set('get_rabbit', () => {
      throw connectivityError();
    });

    await launch('/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard');

    expect(await screen.findByText(t('rabbit.loadFailed.title'))).toBeVisible();
    expect(screen.getByText(t('error.connectivity'))).toBeVisible();
    expect(backControl()).toBeVisible();
  });

  it('offers Back and the household failure copy when the household read failed', async () => {
    answers.set('get_active_household', () => {
      throw connectivityError();
    });

    await launch('/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard');

    expect(await screen.findByText(t('entry.loadFailed.title'))).toBeVisible();
    expect(screen.getByText(t('error.connectivity'))).toBeVisible();
    expect(backControl()).toBeVisible();
  });

  it('offers Back and no save for an archived rabbit (RAB-8)', async () => {
    answers.set('get_rabbit', () => ({ ...RABBIT, is_archived: true }));

    await launch('/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard');

    expect(
      await screen.findByText(t('rabbit.archived.readOnly')),
    ).toBeVisible();
    expect(
      screen.queryByRole('button', { name: t('weight.form.save') }),
    ).toBeNull();
    expect(backControl()).toBeVisible();
  });

  it('offers Back and refuses a rabbit of another household (NFR-10)', async () => {
    answers.set('get_rabbit', () => ({
      ...RABBIT,
      household_id: 'household-other',
    }));

    await launch('/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard');

    expect(await screen.findByText(t('rabbit.loadFailed.title'))).toBeVisible();
    expect(screen.getByText(t('error.validation'))).toBeVisible();
    expect(
      screen.queryByRole('button', { name: t('weight.form.save') }),
    ).toBeNull();
    expect(backControl()).toBeVisible();
  });

  it('focuses the value field, not Back, when the display unit is cached', async () => {
    await launch(
      '/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard',
      (client) => {
        client.setQueryData(settingsKeys.displayWeightUnit(), 'grams');
      },
    );

    await screen.findByText(RABBIT.name);

    await waitFor(() => {
      expect(
        screen.getByRole('textbox', {
          name: `${t('weight.form.valueLabel')} (${UNIT_NAMES.grams})`,
        }),
      ).toHaveFocus();
    });
    expect(backControl()).not.toHaveFocus();
  });

  it('focuses Back when the state drew no form', async () => {
    answers.set('get_rabbit', () => ({ ...RABBIT, is_archived: true }));

    await launch('/weight?rabbit=rabbit-hazel&mode=quick&from=dashboard');

    expect(
      await screen.findByText(t('rabbit.archived.readOnly')),
    ).toBeVisible();

    await waitFor(() => {
      expect(backControl()).toHaveFocus();
    });
  });
});
