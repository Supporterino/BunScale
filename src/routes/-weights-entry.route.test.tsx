/*
 * The weight entry detail's route (task 4.19; WT-8, D1.2).
 *
 * WT-8 names one entry as a destination of its own, and D1.2 keeps the detail's
 * context — the rabbit whose cache prefix the entry sits under, the surface Back
 * returns to and that surface's two windows — in typed search parameters rather
 * than in screen state. This spec launches the app's committed route tree on
 * `/weights/$entryId` as a deep link would, so what is asserted is the wiring
 * rather than `WeightEntryDetailScreen` in isolation:
 *
 *   - a deep link renders the placeholder for the entry its parameter names,
 *     reading it with the search's rabbit and offering Back;
 *   - a failed read renders the cause's copy and still offers Back (Q11: a
 *     pushed screen is never a dead end);
 *   - Back returns to the rabbit dashboard with the windows the detail was
 *     opened at (WT-6, D1.4);
 *   - the placeholder is read-only: no Edit, Delete or save control exists
 *     until task 4.21 adds them, so this spec fails if a later change makes the
 *     detail writable without the task that owns it.
 *
 * The file is prefixed `-` because it lives beside the route files: the router
 * plugin's `routeFileIgnorePrefix` is `-`, so it is not treated as a route.
 *
 * The two baselines are not split here: the 4.18 route precedent launches the
 * same committed tree once, and the adaptive arrangement jsdom cannot see is an
 * E2E assertion (see `src/test/setup.ts`). IPC is mocked at `invoke`, the
 * `testing-strategy` seam the other route and feature specs use; the event
 * plugin is mocked because returning to the rabbit dashboard mounts
 * `useAppEvents`.
 */
import type { QueryClient } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { cleanup, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { DashboardRow } from '../features/dashboard/queries';
import type {
  Household,
  HouseholdSummary,
} from '../features/households/queries';
import { t } from '../features/i18n';
import type { Rabbit } from '../features/rabbits/queries';
import type { WeightEntry, WeightTrend } from '../features/weight/queries';
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

/** The note the placeholder draws, and the proof the read reached it. */
const NOTE = 'Morning weigh-in';

/** The entry the detail route reads. */
const ENTRY: WeightEntry = {
  id: 'weight-1',
  rabbit_id: RABBIT.id,
  value_grams: 2000,
  timestamp: '2026-10-01T09:00:00Z',
  created_at: '2026-10-01T09:00:00Z',
  note: NOTE,
  photo_refs: [],
  has_weight_drop_alert: false,
};

/** A connectivity refusal: retryable, so the failure surface offers a refresh. */
function connectivityError() {
  return {
    cause: 'connectivity',
    retryable: true,
    message_key: 'error.connectivity',
    request_id: null,
  };
}

/** The search this spec's deep link carries; `days_7`/`all` prove the pair. */
const ENTRY_PATH =
  '/weights/weight-1?rabbit=rabbit-hazel&from=rabbit&trend=days_7&history=all';

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
    ['get_weight_entry', () => ENTRY],
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

/** The screen title, which the navigation bar's `h1` alone carries. */
function screenTitle(): HTMLElement {
  return screen.getByRole('heading', {
    level: 1,
    name: t('weight.entry.title'),
  });
}

describe('the weight entry detail route (WT-8, D1.2)', () => {
  it('opens the placeholder for the entry its deep link names, with Back', async () => {
    await launch(ENTRY_PATH);

    // The route's parameter is the read's only argument; the search's rabbit
    // scopes the cache key but never travels into the command.
    expect(await screen.findByText(NOTE)).toBeVisible();
    expect(
      calls.some(
        (call) =>
          call.command === 'get_weight_entry' && call.args.entryId === ENTRY.id,
      ),
    ).toBe(true);

    expect(screenTitle()).toBeVisible();
    expect(backControl()).toBeVisible();
  });

  it('returns to the rabbit dashboard with the windows it was opened at', async () => {
    const router = await launch(ENTRY_PATH);

    await screen.findByText(NOTE);
    await userEvent.setup().click(backControl());

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/rabbits/rabbit-hazel');
    });

    // The pair the detail was opened at comes back, so the rabbit dashboard is
    // the one the person left rather than the default window (WT-6, D1.4).
    expect(router.state.location.search).toEqual({
      trend: 'days_7',
      history: 'all',
    });
  });

  it('renders the cause copy and Back when the entry read failed', async () => {
    answers.set('get_weight_entry', () => {
      throw connectivityError();
    });

    await launch(ENTRY_PATH);

    expect(await screen.findByText(t('error.connectivity'))).toBeVisible();
    expect(screenTitle()).toBeVisible();
    expect(backControl()).toBeVisible();
  });

  it('renders nothing weight-writable until 4.21 owns the actions', async () => {
    await launch(ENTRY_PATH);

    await screen.findByText(NOTE);

    // The placeholder stops at the read and Back (its header says so): Edit,
    // Delete and save are 4.21's, so their absence is this route's contract.
    expect(
      screen.queryByRole('button', { name: t('weight.entry.edit') }),
    ).toBeNull();
    expect(
      screen.queryByRole('button', { name: t('weight.entry.delete') }),
    ).toBeNull();
    expect(
      screen.queryByRole('button', { name: t('weight.form.save') }),
    ).toBeNull();
    expect(screen.queryByRole('textbox')).toBeNull();
  });
});
