/*
 * The weight trend and the weight-only history, as presentation (task 4.11;
 * DASH-7, WT-6, WT-7, D5).
 *
 * **These components hold no read and no range.** `RabbitDashboard` owns the two
 * queries and passes the points down; the range control only *reports* the
 * window it is asked for, so neither view can keep a selection of its own and
 * WT-6's independence stays structural (`../search`).
 *
 * **The chart is the smallest accessible rendering, not a charting library.**
 * No dependency is added: the series is one SVG path plus one circle per point
 * in a fixed `viewBox`, so it scales with the card at both baselines without a
 * measuring pass. Geometry is derived from the points themselves — their span in
 * time and in grams — never from a calendar window recomputed here, because the
 * backend's window (`../queries`) is the only authority on which points this is
 * (R2).
 *
 * **D5 is a constraint on what is absent.** The plot draws the series, a time
 * axis and the drop markers and nothing else: no baseline, no healthy-range
 * band, no target, no legend. There is no `rect` in the SVG and no copy that
 * could name a range the profile does not have, so a later reviewer can see the
 * decision in the markup as well as in the diff (design.md D1.7).
 *
 * **The accessible copy is a list, not a painted label.** The visible SVG is
 * `aria-hidden` because a path is not readable; the figure's caption names the
 * chart and its range, and a visually hidden `<ul>` states every point as a
 * timestamp plus a value in the person's unit, with `weight.dropAlert` for a
 * flagged one. A screen reader therefore hears the range, the number of entries
 * (the list's item count) and the series' extent (its first and last items), and
 * the drop marker is never colour-only. The axis under the plot is decorative:
 * its two dates restate the first and last hidden items.
 *
 * **The drop flag is read, never derived (WT-7, I3).** Both markers call
 * `entryHasDropAlert`; nothing here looks at a value or the threshold, so a
 * point flagged by a predecessor outside the window still carries its marker.
 */
import { Badge } from '../../../components/ui/Badge';
import { FormField } from '../../../components/ui/FormField';
import { List } from '../../../components/ui/List';
import { ListRow } from '../../../components/ui/ListRow';
import {
  SegmentedControl,
  type SegmentedControlOption,
} from '../../../components/ui/SegmentedControl';
import type { MessageKey } from '../../i18n';
import { t } from '../../i18n';
import { entryHasDropAlert } from '../alerts';
import { formatDisplayWeight } from '../api';
import type { WeightTrend } from '../queries';
import { WEIGHT_RANGES, type WeightRange } from '../search';

/**
 * The display unit, derived from the formatter that consumes it rather than
 * imported from the generated bindings: only an `api.ts` may import those
 * (project hard rule 2), and a derived type cannot drift from the conversion it
 * describes.
 */
export type DisplayWeightUnit = Parameters<typeof formatDisplayWeight>[1];

type WeightPoint = WeightTrend['points'][number];

/**
 * The four windows, mapped to their catalogued labels (3.8). The values are the
 * control's source of truth (`WEIGHT_RANGES` fixes the order), so a window
 * added to the contract without copy is a compile error here.
 */
const RANGE_MESSAGE_KEYS = {
  days_7: 'weight.range.days_7',
  days_30: 'weight.range.days_30',
  days_90: 'weight.range.days_90',
  all: 'weight.range.all',
} as const satisfies Record<WeightRange, MessageKey>;

/*
 * Values and timestamps are formatted by the platform, not by hand: the number
 * formatter carries the person's locale and the unit's short symbol ("g", "kg")
 * from the display-unit preference, so no unit string is hard-coded here
 * (NFR-9). The numeric text still comes from `formatDisplayWeight`, the
 * product's one conversion; `Intl` is handed its exact digits and only formats
 * them. The timestamps are instants, so they are shown in the device's zone;
 * the calendar dates on a profile stay UTC (I8) and are a different question.
 */
const WEIGHT_FORMATS: Record<DisplayWeightUnit, Intl.NumberFormat> = {
  grams: new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 0,
    style: 'unit',
    unit: 'gram',
    unitDisplay: 'short',
  }),
  kilograms: new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 3,
    style: 'unit',
    unit: 'kilogram',
    unitDisplay: 'short',
  }),
};

const POINT_DATE_FORMAT = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'medium',
});

const POINT_TIMESTAMP_FORMAT = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'medium',
  timeStyle: 'short',
});

/** One stored weight as the person's unit, e.g. `1.8 kg` (WT-3, DM-1). */
export function formatWeight(grams: number, unit: DisplayWeightUnit): string {
  return WEIGHT_FORMATS[unit].format(Number(formatDisplayWeight(grams, unit)));
}

export type WeightRangeControlProps = {
  /** The radiogroup's accessible name; the section heading is the visible one. */
  label: string;
  value: WeightRange;
  onChange: (range: WeightRange) => void;
};

/**
 * The 7 / 30 / 90 / full-history control (WT-6), one instance per view.
 *
 * It takes its value and reports the selected window; it never stores one, so
 * the trend's control and the history's control cannot share state. The
 * `FormField` label is clipped by `screens.css` because the card heading above
 * already shows the same words — the same treatment the Settings unit section
 * uses.
 */
export function WeightRangeControl({
  label,
  value,
  onChange,
}: WeightRangeControlProps) {
  const options: Array<SegmentedControlOption<WeightRange>> = WEIGHT_RANGES.map(
    (range) => ({ label: t(RANGE_MESSAGE_KEYS[range]), value: range }),
  );

  return (
    <FormField className="bs-weight-range" label={label}>
      {(field) => (
        <SegmentedControl
          describedBy={field.describedBy}
          id={field.controlId}
          invalid={field.invalid}
          labelId={field.labelId}
          onChange={onChange}
          options={options}
          required={field.required}
          value={value}
        />
      )}
    </FormField>
  );
}

/*
 * The plot's fixed geometry. A `viewBox` rather than measured pixels: the card
 * is the layout owner at every width, and the plot needs no resize observer.
 * Pad keeps the outer markers inside the stroke.
 */
const PLOT_WIDTH = 320;
const PLOT_HEIGHT = 160;
const PLOT_PADDING = 12;
const POINT_RADIUS = 3;
const DROP_RADIUS = 7;

type Coordinate = { x: number; y: number };

/**
 * The points in plot coordinates, oldest first as the payload orders them.
 *
 * A single point, or a series whose values are all equal, would divide by zero;
 * those degenerate spans are centred instead, so one entry still renders as a
 * real point rather than an empty chart with a stray path.
 */
function coordinatesOf(points: readonly WeightPoint[]): Coordinate[] {
  const times = points.map((point) => new Date(point.timestamp).getTime());
  const values = points.map((point) => point.value_grams);
  const firstTime = Math.min(...times);
  const lastTime = Math.max(...times);
  const lowest = Math.min(...values);
  const highest = Math.max(...values);
  const timeSpan = lastTime - firstTime;
  const valueSpan = highest - lowest;
  const innerWidth = PLOT_WIDTH - PLOT_PADDING * 2;
  const innerHeight = PLOT_HEIGHT - PLOT_PADDING * 2;

  return points.map((point, index) => ({
    x:
      PLOT_PADDING +
      (timeSpan === 0
        ? innerWidth / 2
        : ((times[index] - firstTime) / timeSpan) * innerWidth),
    y:
      PLOT_PADDING +
      (valueSpan === 0
        ? innerHeight / 2
        : (1 - (point.value_grams - lowest) / valueSpan) * innerHeight),
  }));
}

export type WeightTrendChartProps = {
  points: readonly WeightPoint[];
  /** The range the payload was read at, echoed by the backend (D1.3). */
  range: WeightRange;
  unit: DisplayWeightUnit;
};

/**
 * The trend plot and its accessible series (DASH-7, WT-6, WT-7, D5).
 *
 * Callers render the empty state themselves: an empty series is stated in
 * words, never as a blank plot, and that decision belongs to the section that
 * also keeps the range control available.
 */
export function WeightTrendChart({
  points,
  range,
  unit,
}: WeightTrendChartProps) {
  const coordinates = coordinatesOf(points);
  const first = points[0];
  const last = points[points.length - 1];
  const line = coordinates
    .map(
      (coordinate, index) =>
        `${index === 0 ? 'M' : 'L'}${coordinate.x.toFixed(1)} ${coordinate.y.toFixed(1)}`,
    )
    .join(' ');

  return (
    <figure className="bs-weight-chart">
      <figcaption className="bs-visually-hidden">
        {`${t('weight.trend.chartLabel')} (${t(RANGE_MESSAGE_KEYS[range])})`}
      </figcaption>
      <svg
        aria-hidden="true"
        className="bs-weight-chart__plot"
        focusable="false"
        viewBox={`0 0 ${PLOT_WIDTH} ${PLOT_HEIGHT}`}
      >
        {coordinates.length > 1 ? (
          <path className="bs-weight-chart__line" d={line} />
        ) : null}
        {coordinates.map((coordinate, index) => (
          <g key={points[index].id}>
            {entryHasDropAlert(points[index]) ? (
              <circle
                className="bs-weight-chart__drop"
                cx={coordinate.x}
                cy={coordinate.y}
                r={DROP_RADIUS}
              />
            ) : null}
            <circle
              className="bs-weight-chart__point"
              cx={coordinate.x}
              cy={coordinate.y}
              r={POINT_RADIUS}
            />
          </g>
        ))}
      </svg>
      <div aria-hidden="true" className="bs-weight-chart__axis">
        <time dateTime={first.timestamp}>
          {POINT_DATE_FORMAT.format(new Date(first.timestamp))}
        </time>
        {points.length > 1 ? (
          <time dateTime={last.timestamp}>
            {POINT_DATE_FORMAT.format(new Date(last.timestamp))}
          </time>
        ) : null}
      </div>
      <ul className="bs-visually-hidden">
        {points.map((point) => (
          <li key={point.id}>
            <time dateTime={point.timestamp}>
              {POINT_TIMESTAMP_FORMAT.format(new Date(point.timestamp))}
            </time>
            {`, ${formatWeight(point.value_grams, unit)}`}
            {entryHasDropAlert(point) ? `, ${t('weight.dropAlert')}` : null}
          </li>
        ))}
      </ul>
    </figure>
  );
}

export type WeightHistoryListProps = {
  /** Oldest first, as `WeightTrend.points` arrives. */
  points: readonly WeightPoint[];
  unit: DisplayWeightUnit;
};

/**
 * The weight-only history listing (DASH-7, WT-6, WT-7).
 *
 * Newest first, because a history is read from the most recent entry — the
 * reversal is a presentation decision on an ordered payload, not a re-sort.
 * Rows are the frozen `List`/`ListRow` pair in its static mode: no control, no
 * chevron, because opening one entry is task 4.10's detail and this task adds no
 * action the route tree cannot honour. The drop marker is a `Badge` with the
 * catalog's words, so the flag is never a colour-only dot (DASH-4, WT-7).
 *
 * `ListRow.detail` is a string, so the timestamp here is text rather than a
 * `<time>` element; the chart's hidden series keeps the element because it
 * builds its own markup, and no meaning is lost — the value shown is the same
 * instant.
 */
export function WeightHistoryList({ points, unit }: WeightHistoryListProps) {
  const newestFirst = [...points].reverse();

  return (
    <List>
      {newestFirst.map((point) => (
        <ListRow
          control={
            entryHasDropAlert(point) ? (
              <Badge tone="warning">{t('weight.dropAlert')}</Badge>
            ) : undefined
          }
          detail={POINT_TIMESTAMP_FORMAT.format(new Date(point.timestamp))}
          key={point.id}
          title={formatWeight(point.value_grams, unit)}
        />
      ))}
    </List>
  );
}
