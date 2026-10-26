/*
 * One rabbit's dashboard (task 4.11; DASH-6, DASH-7, WT-6, WT-7, D5).
 *
 * The destination DASH-6 names: it extends task 4.4's profile detail rather
 * than replacing it, because selecting a rabbit has opened that detail since
 * 4.4 and the profile is part of "that rabbit's dashboard". The trend and the
 * History section mount below it in the same scroll region and in the same
 * measure, so the row's control now leads to a dashboard rather than a profile.
 * The detail keeps its own actions and focus behaviour untouched; on close it
 * still returns to the row it was opened from.
 *
 * **Two independent windows, one read each (WT-6, D1.4).** `search` is the
 * route's typed range state (`../../weight/search`); this component never
 * stores a selection and never derives one field from the other. Each section
 * receives its own window and a callback that reports only that window, so the
 * trend's control cannot move the history and vice versa. The parent applies
 * one key at a time. Until the rabbit route exists, `HouseholdDashboardScreen`
 * holds the same shape in component state and passes it in; moving it to
 * `validateSearch: validateWeightRangeSearch` is then a prop swap, not a
 * rewrite here.
 *
 * **A history row opens its entry** (task 4.22; WT-8). Both windows and the
 * rabbit travel into the list, because 4.19's detail route validates them as its
 * typed search and Back has to return to this dashboard at the windows it was
 * left at (WT-6, D1.4). The trend window is therefore read here even though the
 * link is drawn in the History card; it is passed through, never displayed
 * below it.
 *
 * **D1.16's read outcomes are three, not two.** A pending read is a labelled
 * spinner; a failed read is an `ErrorState` with the cause's copy and a refresh
 * when a fresh read can change the answer; only a resolved, empty series says
 * "No weight entries yet." A failed read is never rendered as empty. Task 4.15
 * owns the shared read-failure surface and will fold this one into it; until
 * then each section names itself as the failed surface and uses the shared
 * mapper rather than inventing copy.
 *
 * **The unit read gates the values, not the geometry.** Grams are canonical and
 * the display unit is the device's preference (WT-3, DM-1), so a value is only
 * rendered once the unit is known: while that local read is pending the section
 * shows its label, and a failed unit read gets the same failure surface instead
 * of a silent guess at a unit. The range control never waits on a read, because
 * changing a window is how a person recovers from an empty one.
 *
 * **Archived means read-only, and 4.6 owns that presentation (DASH-7, D1.5).**
 * This task contributes no weight write control on either rabbit: the trend, the
 * history and their markers are display-only, so an archived rabbit's dashboard
 * is read-only by construction. The read-only banner and the archived profile's
 * own surface are task 4.6's and are deliberately not duplicated here.
 *
 * **The History section is an entry point, not a second listing on screen
 * (D1.6, DASH-7).** It contributes exactly P1's weight-only entry, shaped as a
 * grouped row like RAB-6's archived entry point: activating it opens the
 * weight-only history in the section, with its own range control and the drop
 * markers, and Back returns focus to the row. The entry row stays visible while
 * the history is closed, so the section says what it offers without claiming a
 * history screen this route tree does not have.
 */
import type { ReactNode } from 'react';
import { useEffect, useRef, useState } from 'react';
import { Button } from '../../../components/ui/Button';
import { Card } from '../../../components/ui/Card';
import { ErrorState } from '../../../components/ui/ErrorState';
import { List } from '../../../components/ui/List';
import { ListRow } from '../../../components/ui/ListRow';
import { LoadingIndicator } from '../../../components/ui/LoadingIndicator';
import { Text } from '../../../components/ui/Text';
import { t } from '../../i18n';
import { RabbitDetail } from '../../rabbits/components/RabbitDetail';
import {
  type DisplayWeightUnitFailure,
  useDisplayWeightUnit,
  type WeightUnit,
} from '../../settings/display-unit';
import { describeReadFailure } from '../../sync/read-failure';
import {
  WeightHistoryList,
  WeightRangeControl,
  WeightTrendChart,
} from '../../weight/components/WeightSeries';
import {
  useWeightHistory,
  useWeightTrend,
  type WeightSeriesFailure,
  type WeightTrend,
} from '../../weight/queries';
import type { WeightRange, WeightRangeSearch } from '../../weight/search';

export type RabbitDashboardProps = {
  /** The rabbit the dashboard row opened (DASH-6). */
  rabbitId: string;
  /** Returns to the dashboard list without changing anything. */
  onClose: () => void;
  /** The route's two windows, one per view (WT-6, D1.4). */
  search: WeightRangeSearch;
  /** Reports the trend's window only; the history's value is not touched. */
  onTrendRangeChange: (range: WeightRange) => void;
  /** Reports the history's window only; the trend's value is not touched. */
  onHistoryRangeChange: (range: WeightRange) => void;
};

export function RabbitDashboard({
  rabbitId,
  onClose,
  search,
  onTrendRangeChange,
  onHistoryRangeChange,
}: RabbitDashboardProps) {
  return (
    <div className="bs-rabbit-dashboard">
      <RabbitDetail onClose={onClose} rabbitId={rabbitId} />
      <WeightTrendSection
        onRangeChange={onTrendRangeChange}
        rabbitId={rabbitId}
        range={search.trend}
      />
      <WeightHistorySection
        onRangeChange={onHistoryRangeChange}
        rabbitId={rabbitId}
        range={search.history}
        trend={search.trend}
      />
    </div>
  );
}

/*
 * The two reads share a result shape (`useWeightTrend` and `useWeightHistory`
 * are the same read with different arguments), so one gate serves both. Keeping
 * the branches here — pending, failed, empty, then content — is what stops the
 * two sections from drifting into different answers to "no data yet".
 */
type SeriesRead = {
  data: WeightTrend | undefined;
  error: WeightSeriesFailure | null;
  refetch: () => Promise<unknown>;
};

type UnitRead = {
  data: WeightUnit | undefined;
  error: DisplayWeightUnitFailure | null;
  refetch: () => Promise<unknown>;
};

function WeightTrendSection({
  rabbitId,
  range,
  onRangeChange,
}: {
  rabbitId: string;
  range: WeightRange;
  onRangeChange: (range: WeightRange) => void;
}) {
  const series = useWeightTrend(rabbitId, range);
  const unit = useDisplayWeightUnit();

  return (
    <Card className="bs-weight-trend" heading={t('weight.trend.title')}>
      <WeightRangeControl
        label={t('weight.trend.title')}
        onChange={onRangeChange}
        value={range}
      />
      <SeriesContent
        series={series}
        title={t('weight.trend.title')}
        unit={unit}
      >
        {(trend, displayUnit) => (
          <WeightTrendChart
            points={trend.points}
            range={trend.range}
            unit={displayUnit}
          />
        )}
      </SeriesContent>
    </Card>
  );
}

function WeightHistorySection({
  rabbitId,
  range,
  trend,
  onRangeChange,
}: {
  rabbitId: string;
  /** The history's own window, shown by its control (WT-6). */
  range: WeightRange;
  /** The trend's window, carried into a row's detail and not shown here (WT-6). */
  trend: WeightRange;
  onRangeChange: (range: WeightRange) => void;
}) {
  const [open, setOpen] = useState(false);
  const entryRef = useRef<HTMLDivElement>(null);
  const wasOpen = useRef(false);

  // Back returns focus to the row it replaced, so a keyboard or screen-reader
  // user does not restart at the skip link after closing the history.
  useEffect(() => {
    if (open) {
      wasOpen.current = true;
      return;
    }

    if (!wasOpen.current) {
      return;
    }

    wasOpen.current = false;
    entryRef.current?.querySelector<HTMLButtonElement>('button')?.focus();
  }, [open]);

  return (
    <Card className="bs-weight-history" heading={t('weight.history.title')}>
      {open ? (
        <WeightHistoryView
          onClose={() => {
            setOpen(false);
          }}
          onRangeChange={onRangeChange}
          rabbitId={rabbitId}
          range={range}
          trend={trend}
        />
      ) : (
        <div ref={entryRef}>
          <List>
            <ListRow
              accessory="chevron"
              onClick={() => {
                setOpen(true);
              }}
              title={t('weight.history.open')}
            />
          </List>
        </div>
      )}
    </Card>
  );
}

function WeightHistoryView({
  rabbitId,
  range,
  trend,
  onRangeChange,
  onClose,
}: {
  rabbitId: string;
  range: WeightRange;
  trend: WeightRange;
  onRangeChange: (range: WeightRange) => void;
  onClose: () => void;
}) {
  const series = useWeightHistory(rabbitId, range);
  const unit = useDisplayWeightUnit();
  const containerRef = useRef<HTMLDivElement>(null);

  // The row that opened this view has unmounted; the Back control is the first
  // control, so it takes focus the way the form and detail swaps do.
  useEffect(() => {
    containerRef.current?.querySelector<HTMLButtonElement>('button')?.focus();
  }, []);

  return (
    <div className="bs-weight-history__view" ref={containerRef}>
      <div className="bs-weight-history__actions">
        <Button onClick={onClose} variant="plain">
          {t('settings.back')}
        </Button>
      </div>
      <WeightRangeControl
        label={t('weight.history.open')}
        onChange={onRangeChange}
        value={range}
      />
      <SeriesContent
        series={series}
        title={t('weight.history.title')}
        unit={unit}
      >
        {(trendData, displayUnit) => (
          <WeightHistoryList
            history={range}
            points={trendData.points}
            rabbitId={rabbitId}
            trend={trend}
            unit={displayUnit}
          />
        )}
      </SeriesContent>
    </div>
  );
}

/**
 * The read gate both sections share (D1.16).
 *
 * The order is the contract: an answer first, then emptiness, then the unit
 * needed to say a value. An empty series is stated in `weight.empty` and the
 * range control above stays usable; a failure is never relabelled as that
 * state.
 */
function SeriesContent({
  series,
  unit,
  title,
  children,
}: {
  series: SeriesRead;
  unit: UnitRead;
  /** Names the surface in a read failure's heading. */
  title: string;
  children: (trend: WeightTrend, unit: WeightUnit) => ReactNode;
}) {
  if (series.data === undefined) {
    return series.error === null ? (
      <LoadingIndicator label={t('weight.loading')} />
    ) : (
      <ReadFailure
        error={series.error}
        onRefresh={() => {
          void series.refetch();
        }}
        title={title}
      />
    );
  }

  if (series.data.points.length === 0) {
    return <Text tone="secondary">{t('weight.empty')}</Text>;
  }

  if (unit.data === undefined) {
    return unit.error === null ? (
      <LoadingIndicator label={t('settings.displayUnit.loading')} />
    ) : (
      <ReadFailure
        error={unit.error}
        onRefresh={() => {
          void unit.refetch();
        }}
        title={title}
      />
    );
  }

  return <>{children(series.data, unit.data)}</>;
}

/**
 * A failed read, with the cause's copy (NFR-8, D1.16).
 *
 * `describeReadFailure` decides whether a fresh read can change the answer, so
 * a `validation` refusal offers no refresh; every other cause's copy promises
 * another attempt and gets the control. Task 4.15 will replace this with the
 * shared surface; it already speaks the same mapper rather than its own copy.
 */
function ReadFailure({
  error,
  title,
  onRefresh,
}: {
  error: unknown;
  title: string;
  onRefresh: () => void;
}) {
  const failure = describeReadFailure(error);

  return (
    <ErrorState
      {...(failure.refreshable ? { onRetry: onRefresh } : {})}
      retryLabel={t('error.retry')}
      title={title}
    >
      {t(failure.messageKey)}
    </ErrorState>
  );
}
