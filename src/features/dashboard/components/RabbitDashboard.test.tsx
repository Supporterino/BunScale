/*
 * The rabbit dashboard at the component layer (task 4.11; DASH-6, DASH-7, WT-6,
 * WT-7, D5, D1.16).
 *
 * What this layer owns: the destination a dashboard row opens (the profile
 * detail plus the weight trend and the History section), the trend's range
 * control limiting the series, the history's range being independent of the
 * trend's, the drop marker being the payload's flag rather than a second
 * derivation, the chart's accessible series, and every read outcome (pending,
 * data, empty, failed) being told apart.
 *
 * What it does not own: the row → dashboard swap (the screen's own spec proves
 * it), the profile's edit/lifecycle controls (4.4's spec) and the archived
 * read-only presentation (4.6's). The archived case here asserts only that this
 * task contributed no weight write control and that both sections are still
 * presented.
 *
 * IPC is mocked at `invoke`, the seam the other feature specs use, so the real
 * query hooks, range keys and `formatDisplayWeight` run. The harness holds the
 * two windows the way `HouseholdDashboardScreen` does — one state, one key per
 * update — because that is the shape the rabbit route's `validateSearch` will
 * supply.
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, cleanup, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { afterEach, beforeEach, expect, it } from 'vitest';
import {
  describeAtBothBaselines,
  renderWithQueryClient,
} from '../../../test/setup';
import { t } from '../../i18n';
import type { Rabbit } from '../../rabbits/queries';
import type { WeightTrend } from '../../weight/queries';
import {
  DEFAULT_WEIGHT_RANGE,
  type WeightRangeSearch,
} from '../../weight/search';
import { RabbitDashboard } from './RabbitDashboard';

type WeightPoint = WeightTrend['points'][number];

const RABBIT: Rabbit = {
  id: 'rabbit-hazel',
  household_id: 'household-home',
  name: 'Hazel',
  photo_ref: null,
  breed: 'Holland Lop',
  dob: null,
  dob_unknown: true,
  sex: 'female',
  neuter_status: 'neutered',
  adoption_date: null,
  is_archived: false,
  weight_drop_threshold_percent: 5,
};

/*
 * Three entries over six weeks. `FLAGGED` is the middle one: its 10% drop is
 * beyond the rabbit's 5% threshold, so the backend flags it — the fixture never
 * decides that itself.
 */
const OLDEST: WeightPoint = {
  id: 'weight-1',
  value_grams: 2000,
  timestamp: '2026-09-01T09:00:00Z',
  has_weight_drop_alert: false,
};
const FLAGGED: WeightPoint = {
  id: 'weight-2',
  value_grams: 1800,
  timestamp: '2026-09-20T09:00:00Z',
  has_weight_drop_alert: true,
};
const NEWEST: WeightPoint = {
  id: 'weight-3',
  value_grams: 1810,
  timestamp: '2026-10-19T09:00:00Z',
  has_weight_drop_alert: false,
};

const THIRTY_DAY_POINTS = [OLDEST, FLAGGED, NEWEST];
const SEVEN_DAY_POINTS = [NEWEST];

/*
 * One entry older than the 30-day window, so "Full history" is distinguishable
 * from the bounded ranges: selecting it must present one entry more, not the
 * same three (WT-6's "the full-history range presents every entry").
 */
const ARCHIVED: WeightPoint = {
  id: 'weight-0',
  value_grams: 2100,
  timestamp: '2026-05-01T09:00:00Z',
  has_weight_drop_alert: false,
};

const FULL_HISTORY_POINTS = [ARCHIVED, ...THIRTY_DAY_POINTS];

type Call = { command: string; args: Record<string, unknown> };
type Failure = {
  cause: string;
  retryable: boolean;
  message_key: string;
  request_id: string | null;
};

/** A read that stays pending until the test resolves it. */
type Deferred = {
  promise: Promise<WeightTrend>;
  resolve: (trend: WeightTrend) => void;
};

function deferSeries(): Deferred {
  let resolve: (trend: WeightTrend) => void = () => {};
  const promise = new Promise<WeightTrend>((settle) => {
    resolve = settle;
  });

  return { promise, resolve };
}

function payload(
  range: WeightTrend['range'],
  entries: WeightPoint[],
): WeightTrend {
  return {
    range,
    points: entries,
    drop_threshold_percent: RABBIT.weight_drop_threshold_percent,
  };
}

let calls: Call[];
let profile: Rabbit;
let failure: Failure | null;
let deferred: Deferred | null;
let points: (range: string) => WeightPoint[];

beforeEach(() => {
  calls = [];
  profile = RABBIT;
  failure = null;
  deferred = null;
  points = (range) => {
    if (range === 'days_7') {
      return SEVEN_DAY_POINTS;
    }

    return range === 'all' ? FULL_HISTORY_POINTS : THIRTY_DAY_POINTS;
  };

  mockIPC((command, args) => {
    const argsObject = (args ?? {}) as Record<string, unknown>;

    calls.push({ command, args: argsObject });

    switch (command) {
      case 'get_rabbit':
        return profile;
      case 'get_display_weight_unit':
        return 'grams';
      case 'get_weight_entries': {
        if (deferred !== null) {
          return deferred.promise;
        }

        if (failure !== null) {
          throw failure;
        }

        const range = String(argsObject.range) as WeightTrend['range'];

        return payload(range, points(range));
      }
      default:
        throw new Error(`unexpected mocked command "${command}"`);
    }
  });
});

afterEach(async () => {
  cleanup();
  await Promise.resolve();
  clearMocks();
});

const DEFAULT_SEARCH: WeightRangeSearch = {
  trend: DEFAULT_WEIGHT_RANGE,
  history: DEFAULT_WEIGHT_RANGE,
};

/** The route's two windows, applied one key at a time (D1.4). */
function Harness({
  initialSearch = DEFAULT_SEARCH,
}: {
  initialSearch?: WeightRangeSearch;
}) {
  const [search, setSearch] = useState<WeightRangeSearch>(initialSearch);

  return (
    <RabbitDashboard
      onClose={() => {}}
      onHistoryRangeChange={(history) => {
        setSearch((current) => ({ ...current, history }));
      }}
      onTrendRangeChange={(trend) => {
        setSearch((current) => ({ ...current, trend }));
      }}
      rabbitId={RABBIT.id}
      search={search}
    />
  );
}

function renderDashboard() {
  return renderWithQueryClient(<Harness />);
}

function trendRegion(): HTMLElement {
  return screen.getByRole('region', { name: t('weight.trend.title') });
}

function historyRegion(): HTMLElement {
  return screen.getByRole('region', { name: t('weight.history.title') });
}

/**
 * The chart's accessible series: one list item per entry, each starting with
 * its `<time>`. The plot's axis is `aria-hidden`, so it is not counted. The
 * history listing is a `ListRow` list and has no `<time>` (its detail is a
 * string), so it is counted with `historyRows` instead.
 */
function seriesTimestamps(region: HTMLElement): string[] {
  return within(region)
    .getAllByRole('listitem')
    .map((item) => item.querySelector('time')?.getAttribute('datetime') ?? '');
}

/** The weight-only history's static rows, newest first. */
function historyRows(region: HTMLElement): HTMLElement[] {
  return within(region).getAllByRole('listitem');
}

function rangeCalls(): string[] {
  return calls
    .filter((call) => call.command === 'get_weight_entries')
    .map((call) => String(call.args.range));
}

async function waitForTrend(count: number): Promise<void> {
  await waitFor(() => {
    expect(seriesTimestamps(trendRegion())).toHaveLength(count);
  });
}

async function openHistory(): Promise<void> {
  const user = userEvent.setup();

  await user.click(
    screen.getByRole('button', { name: t('weight.history.open') }),
  );
}

describeAtBothBaselines('Rabbit dashboard (DASH-6, DASH-7)', () => {
  it('presents the rabbit, the weight trend and the History entry point', async () => {
    renderDashboard();

    expect(
      await screen.findByRole('heading', { name: RABBIT.name }),
    ).toBeVisible();

    // DASH-7: the trend is a named region with its two controls and the series.
    const trend = trendRegion();

    expect(
      within(trend).getByRole('radio', { name: t('weight.range.days_30') }),
    ).toBeChecked();
    await waitForTrend(THIRTY_DAY_POINTS.length);
    expect(seriesTimestamps(trend)).toEqual([
      OLDEST.timestamp,
      FLAGGED.timestamp,
      NEWEST.timestamp,
    ]);

    // The History section contributes P1's weight-only entry point (D1.6).
    expect(historyRegion()).toBeVisible();
    expect(
      screen.getByRole('button', { name: t('weight.history.open') }),
    ).toBeVisible();

    // D5: the plot is the series, its axis and the markers — no band, baseline
    // or legend shape exists in it. A band or baseline would have to be a
    // `rect`, `line`, `polyline` or `polygon`; the only nodes are the series
    // path, the point circles and their groups.
    const plot = trend.querySelector('.bs-weight-chart__plot');
    const shapes =
      plot === null
        ? []
        : Array.from(plot.querySelectorAll('*')).map((node) =>
            node.tagName.toLowerCase(),
          );

    expect(shapes.length).toBeGreaterThan(0);
    expect(
      shapes.every((tag) => tag === 'path' || tag === 'circle' || tag === 'g'),
    ).toBe(true);
    expect(
      trend.querySelectorAll(
        '.bs-weight-chart__plot rect, .bs-weight-chart__plot line, .bs-weight-chart__plot polyline, .bs-weight-chart__plot polygon',
      ),
    ).toHaveLength(0);
  });

  it('presents every entry for the full-history range (WT-6)', async () => {
    renderDashboard();

    await waitForTrend(THIRTY_DAY_POINTS.length);

    const user = userEvent.setup();

    await user.click(
      within(trendRegion()).getByRole('radio', {
        name: t('weight.range.all'),
      }),
    );

    await waitForTrend(FULL_HISTORY_POINTS.length);
    expect(seriesTimestamps(trendRegion())).toEqual(
      FULL_HISTORY_POINTS.map((point) => point.timestamp),
    );
    expect(
      within(trendRegion()).getByRole('radio', {
        name: t('weight.range.all'),
      }),
    ).toBeChecked();
    expect(rangeCalls()).toContain('all');
  });

  it('limits the trend to the selected window (WT-6)', async () => {
    renderDashboard();

    await waitForTrend(THIRTY_DAY_POINTS.length);

    const user = userEvent.setup();

    await user.click(
      within(trendRegion()).getByRole('radio', {
        name: t('weight.range.days_7'),
      }),
    );

    await waitForTrend(SEVEN_DAY_POINTS.length);
    expect(seriesTimestamps(trendRegion())).toEqual([NEWEST.timestamp]);
    expect(
      within(trendRegion()).getByRole('radio', {
        name: t('weight.range.days_7'),
      }),
    ).toBeChecked();
    expect(rangeCalls()).toContain('days_7');
    expect(
      calls.some(
        (call) =>
          call.command === 'get_weight_entries' &&
          call.args.rabbitId === RABBIT.id,
      ),
    ).toBe(true);
  });

  it('keeps the history range independent of the trend range (WT-6)', async () => {
    renderDashboard();

    await waitForTrend(THIRTY_DAY_POINTS.length);
    await openHistory();

    await waitFor(() => {
      expect(historyRows(historyRegion())).toHaveLength(
        THIRTY_DAY_POINTS.length,
      );
    });

    const user = userEvent.setup();

    // The history alone moves to the last 7 days.
    await user.click(
      within(historyRegion()).getByRole('radio', {
        name: t('weight.range.days_7'),
      }),
    );

    await waitFor(() => {
      expect(historyRows(historyRegion())).toHaveLength(
        SEVEN_DAY_POINTS.length,
      );
    });

    // The trend is still the 30-day window and still presents its three points.
    expect(
      within(trendRegion()).getByRole('radio', {
        name: t('weight.range.days_30'),
      }),
    ).toBeChecked();
    expect(seriesTimestamps(trendRegion())).toEqual([
      OLDEST.timestamp,
      FLAGGED.timestamp,
      NEWEST.timestamp,
    ]);

    // Moving the trend does not move the history either.
    await user.click(
      within(trendRegion()).getByRole('radio', {
        name: t('weight.range.days_90'),
      }),
    );

    await waitFor(() => {
      expect(rangeCalls()).toContain('days_90');
    });

    expect(
      within(historyRegion()).getByRole('radio', {
        name: t('weight.range.days_7'),
      }),
    ).toBeChecked();
    expect(historyRows(historyRegion())).toHaveLength(SEVEN_DAY_POINTS.length);
    expect(rangeCalls()).toEqual(expect.arrayContaining(['days_30', 'days_7']));
  });

  it('marks a flagged entry with the drop warning (WT-7)', async () => {
    renderDashboard();

    await waitForTrend(THIRTY_DAY_POINTS.length);
    await openHistory();

    const history = historyRegion();

    await waitFor(() => {
      expect(historyRows(history)).toHaveLength(THIRTY_DAY_POINTS.length);
    });

    // Exactly the flagged entry carries the marker, and it is text, not a dot.
    const markers = within(history).getAllByText(t('weight.dropAlert'));

    expect(markers).toHaveLength(1);

    const flaggedRow = historyRows(history).find(
      (row) => within(row).queryByText(t('weight.dropAlert')) !== null,
    );

    expect(flaggedRow).toBeDefined();
    expect(
      within(flaggedRow as HTMLElement).getByText(t('weight.dropAlert')),
    ).toBeVisible();

    // The chart's accessible series carries the same marker (D1.7).
    expect(trendRegion().textContent).toContain(t('weight.dropAlert'));
  });

  it('presents the marker from the payload flag, never from the values (WT-7)', async () => {
    // The same 10% fall from 2000 g to 1800 g, but the backend has not flagged
    // it. A client-side derivation would paint a warning here; reading the flag
    // does not.
    points = () =>
      THIRTY_DAY_POINTS.map((point) => ({
        ...point,
        has_weight_drop_alert: false,
      }));

    renderDashboard();

    await waitForTrend(THIRTY_DAY_POINTS.length);
    await openHistory();

    await waitFor(() => {
      expect(historyRows(historyRegion())).toHaveLength(
        THIRTY_DAY_POINTS.length,
      );
    });

    expect(
      within(historyRegion()).queryByText(t('weight.dropAlert')),
    ).toBeNull();
    expect(trendRegion().textContent).not.toContain(t('weight.dropAlert'));
  });

  it('gives the chart an accessible summary of the series (DASH-7)', async () => {
    renderDashboard();

    await waitForTrend(THIRTY_DAY_POINTS.length);

    const trend = trendRegion();

    // The range is part of the figure's caption, the count is the list's item
    // count and the extent is its first and last item. The painting is hidden
    // from assistive technology because a path is not readable.
    expect(
      within(trend).getByText(
        `${t('weight.trend.chartLabel')} (${t('weight.range.days_30')})`,
      ),
    ).toBeInTheDocument();
    expect(trend.querySelector('svg')).toHaveAttribute('aria-hidden', 'true');

    const timestamps = seriesTimestamps(trend);

    expect(timestamps).toHaveLength(THIRTY_DAY_POINTS.length);
    expect(timestamps[0]).toBe(OLDEST.timestamp);
    expect(timestamps[timestamps.length - 1]).toBe(NEWEST.timestamp);
  });

  it('announces a pending series as loading, not as empty or failed (D1.16)', async () => {
    deferred = deferSeries();

    renderDashboard();

    const trend = trendRegion();
    const status = await within(trend).findByRole('status');

    // The pending state is its own outcome: it announces the wait and neither
    // claims there are no entries nor reports a failure.
    expect(status).toBeVisible();
    expect(within(trend).queryByText(t('weight.empty'))).toBeNull();
    expect(within(trend).queryByRole('alert')).toBeNull();
    expect(trend.querySelector('.bs-weight-chart')).toBeNull();

    await act(async () => {
      deferred?.resolve(payload('days_30', THIRTY_DAY_POINTS));
    });

    await waitForTrend(THIRTY_DAY_POINTS.length);
  });

  it('states an empty series in words, not as a blank chart (DASH-7)', async () => {
    points = () => [];

    renderDashboard();

    await waitFor(() => {
      expect(within(trendRegion()).getByText(t('weight.empty'))).toBeVisible();
    });

    // The window can still be widened, so the range control is not emptied.
    expect(
      within(trendRegion()).getByRole('radio', {
        name: t('weight.range.all'),
      }),
    ).toBeEnabled();
    expect(trendRegion().querySelector('.bs-weight-chart')).toBeNull();

    await openHistory();

    expect(within(historyRegion()).getByText(t('weight.empty'))).toBeVisible();
  });

  it('renders a failed read as its own state, not as an empty series (D1.16)', async () => {
    failure = {
      cause: 'validation',
      retryable: false,
      message_key: 'error.validation',
      request_id: null,
    };

    renderDashboard();

    expect(
      await within(trendRegion()).findByText(t('error.validation')),
    ).toBeVisible();
    expect(within(trendRegion()).queryByText(t('weight.empty'))).toBeNull();
  });

  it("presents an archived rabbit's dashboard read-only (DASH-7)", async () => {
    profile = { ...RABBIT, is_archived: true };

    renderDashboard();

    await waitForTrend(THIRTY_DAY_POINTS.length);

    // DASH-7: the trend and the history are presented for an archived rabbit.
    expect(trendRegion()).toBeVisible();
    await openHistory();
    await waitFor(() => {
      expect(historyRows(historyRegion())).toHaveLength(
        THIRTY_DAY_POINTS.length,
      );
    });

    // This task contributes no weight write control on any rabbit, so the
    // archived surface offers none. The read-only banner itself is task 4.6's
    // and is not duplicated here.
    for (const label of [
      t('weight.log'),
      t('weight.entry.edit'),
      t('weight.entry.delete'),
    ]) {
      expect(screen.queryByRole('button', { name: label })).toBeNull();
    }
  });
});
