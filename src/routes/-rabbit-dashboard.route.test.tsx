/*
 * The rabbit dashboard's route (task 4.16; DASH-6, WT-6, D1.4).
 *
 * D1.4 says the two range windows are a query window, so they live in typed
 * route search parameters rather than in a screen's state. This spec launches
 * the app's committed route tree on the rabbit route and reads the URL through
 * the router, so what is asserted is the real `validateSearch` wiring and not a
 * validator called directly (`src/features/weight/search.test.ts` does that):
 *
 *   - a deep link with `?trend=days_7&history=all` presents each window;
 *   - changing one window rewrites that field and leaves the other's URL value
 *     untouched (WT-6's independence);
 *   - an invalid value falls back to `DEFAULT_WEIGHT_RANGE` instead of failing
 *     the navigation.
 *
 * The file is prefixed `-` because it lives beside the route files: the router
 * plugin's `routeFileIgnorePrefix` is `-`, so it is not treated as a route.
 *
 * IPC is mocked at `invoke` (the `testing-strategy` seam) because the screen the
 * route mounts reads the active household, the dashboard rows, the rabbit, the
 * display unit and the weight series. The event plugin is mocked too because
 * the screen mounts `useAppEvents`.
 */
import { RouterProvider } from '@tanstack/react-router';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { cleanup, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { DashboardRow } from '../features/dashboard/queries';
import type { Household } from '../features/households/queries';
import { t } from '../features/i18n';
import type { Rabbit } from '../features/rabbits/queries';
import type { WeightTrend } from '../features/weight/queries';
import { DEFAULT_WEIGHT_RANGE } from '../features/weight/search';
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

/** Enough entries for the chart's unit read to be exercised, and no more. */
const POINTS: WeightTrend['points'] = [
  {
    id: 'weight-1',
    value_grams: 2000,
    timestamp: '2026-10-01T09:00:00Z',
    has_weight_drop_alert: false,
  },
];

beforeEach(() => {
  mockIPC(
    (command, args) => {
      const payload = (args ?? {}) as Record<string, unknown>;

      switch (command) {
        case 'get_active_household':
          return HOUSEHOLD;
        case 'get_dashboard':
          return [ROW];
        case 'get_rabbit':
          return RABBIT;
        case 'get_display_weight_unit':
          return 'grams';
        case 'get_weight_entries':
          return {
            range: String(payload.range),
            points: POINTS,
            drop_threshold_percent: RABBIT.weight_drop_threshold_percent,
          };
        case 'load_household':
          return {
            household_id: HOUSEHOLD.id,
            items: [],
            source: 'local',
            loaded_at: null,
          };
        default:
          throw new Error(`unexpected mocked command "${command}"`);
      }
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

/** Boot the app's route tree at the rabbit route, as a deep link would. */
async function launch(path: string) {
  const client = createTestQueryClient();
  const router = createTestRouter({ client, initialEntries: [path] });

  await router.load();
  renderWithQueryClient(<RouterProvider router={router} />, { client });

  await screen.findByRole('heading', { name: RABBIT.name });

  return router;
}

function trendRegion(): HTMLElement {
  return screen.getByRole('region', { name: t('weight.trend.title') });
}

function historyRegion(): HTMLElement {
  return screen.getByRole('region', { name: t('weight.history.title') });
}

async function openHistory(): Promise<void> {
  await userEvent
    .setup()
    .click(screen.getByRole('button', { name: t('weight.history.open') }));
}

describe('the rabbit dashboard route (DASH-6, WT-6, D1.4)', () => {
  it('presents the windows a deep link names', async () => {
    const router = await launch(
      '/rabbits/rabbit-hazel?trend=days_7&history=all',
    );

    expect(
      within(trendRegion()).getByRole('radio', {
        name: t('weight.range.days_7'),
      }),
    ).toBeChecked();

    await openHistory();

    expect(
      within(historyRegion()).getByRole('radio', {
        name: t('weight.range.all'),
      }),
    ).toBeChecked();

    expect(router.state.location.pathname).toBe('/rabbits/rabbit-hazel');
  });

  it('changes one window without touching the other in the URL', async () => {
    const router = await launch(
      '/rabbits/rabbit-hazel?trend=days_7&history=all',
    );

    await userEvent.setup().click(
      within(trendRegion()).getByRole('radio', {
        name: t('weight.range.days_30'),
      }),
    );

    await waitFor(() => {
      expect(router.state.location.search).toEqual({
        trend: 'days_30',
        history: 'all',
      });
    });

    // The history window is still the one the URL carried, in the URL and in
    // the control that reads it.
    await openHistory();

    expect(
      within(historyRegion()).getByRole('radio', {
        name: t('weight.range.all'),
      }),
    ).toBeChecked();
  });

  it('falls back to the default window for an invalid value', async () => {
    const router = await launch(
      '/rabbits/rabbit-hazel?trend=fortnight&history=all',
    );

    expect(
      within(trendRegion()).getByRole('radio', {
        name: t(`weight.range.${DEFAULT_WEIGHT_RANGE}`),
      }),
    ).toBeChecked();

    expect(router.state.location.search).toEqual({
      trend: DEFAULT_WEIGHT_RANGE,
      history: 'all',
    });
  });
});
