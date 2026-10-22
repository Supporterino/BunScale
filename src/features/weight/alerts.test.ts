/*
 * Task 3.7's evidence for the frontend's half of WT-7 and DASH-4.
 *
 * The derivation itself belongs to the backend (2.13, 2.14; D1.12/D1.13).
 * These specs therefore assert one thing about it: the selector presents the
 * payload's flag unchanged, whatever the values and the threshold say. That is
 * not a tautology to skip. A "helpful" client-side re-derivation would pass a
 * value-only test and fail these, because a point can legitimately be flagged on
 * the basis of a predecessor outside the returned window (I3), and a threshold
 * change reaches the surface only as a refreshed payload.
 *
 * The payloads are the contract's `WeightTrend` and `DashboardRow` shapes, with
 * the flags the backend would produce for the stated values: 2000 g → 1800 g
 * exceeds 5%, 2000 g → 1950 g does not, and so on. Each spec names the scenario
 * it reflects; nothing here recomputes the drop.
 */
import { describe, expect, it } from 'vitest';
import type { DashboardRow } from '../dashboard/queries';
import { dashboardRowHasDropAlert, entryHasDropAlert } from './alerts';
import type { WeightTrend } from './queries';

/** One series point, as `WeightTrend['points']` carries it. */
type Point = WeightTrend['points'][number];

/** A series point. The value and timestamp only make the fixture plausible. */
function point(
  id: string,
  valueGrams: number,
  hasWeightDropAlert: boolean,
  timestamp = '2026-10-01T09:00:00Z',
): Point {
  return {
    id,
    value_grams: valueGrams,
    timestamp,
    has_weight_drop_alert: hasWeightDropAlert,
  };
}

/** One read's payload: the points beside the threshold they were judged by. */
function series(
  dropThresholdPercent: number | null,
  points: Point[],
): WeightTrend {
  return {
    range: 'all',
    points,
    drop_threshold_percent: dropThresholdPercent,
  };
}

/** One dashboard row carrying the backend's derived flag (DASH-4). */
function row(hasWeightDropAlert: boolean): DashboardRow {
  return {
    rabbit: {
      id: 'rabbit-a',
      name: 'Hazel',
      thumbnail_ref: null,
      is_archived: false,
    },
    last_weighed_at: '2026-10-01T09:00:00Z',
    never_weighed: false,
    has_weight_drop_alert: hasWeightDropAlert,
  };
}

describe('the entry drop alert', () => {
  it('presents a flagged drop that exceeds the threshold', () => {
    // WT-7: threshold 5%, previous 2000 g, new entry 1800 g (10% drop).
    const payload = series(5, [
      point('previous', 2000, false),
      point('current', 1800, true),
    ]);

    expect(entryHasDropAlert(payload.points[1])).toBe(true);
  });

  it('presents no warning for a drop within the threshold', () => {
    // WT-7: threshold 5%, previous 2000 g, new entry 1950 g (2.5% drop).
    const payload = series(5, [
      point('previous', 2000, false),
      point('current', 1950, false),
    ]);

    expect(entryHasDropAlert(payload.points[1])).toBe(false);
  });

  it('clears a flag from the refreshed series when the threshold is raised', () => {
    // WT-7 "Raising the threshold clears an existing flag": 10% drop flagged at
    // 5%, unflagged at 15%. That the refreshed payload actually replaces the
    // cached series is 3.4's proof, in `rabbits/mutations.test.ts`'s "update
    // invalidates the row, the profile, both weight windows and the dashboard
    // row"; this spec only proves the selector follows the new payload's
    // threshold and flag together, with no stored alert to clear.
    const before = series(5, [
      point('previous', 2000, false),
      point('current', 1800, true),
    ]);
    const after = series(15, [
      point('previous', 2000, false),
      point('current', 1800, false),
    ]);

    expect(entryHasDropAlert(before.points[1])).toBe(true);
    expect(after.drop_threshold_percent).toBe(15);
    expect(entryHasDropAlert(after.points[1])).toBe(false);
  });

  it('raises a flag from the refreshed series when the threshold is lowered', () => {
    // WT-7 "Lowering the threshold raises a flag": 2.5% drop unflagged at 5%,
    // flagged at 2%.
    const before = series(5, [
      point('previous', 2000, false),
      point('current', 1950, false),
    ]);
    const after = series(2, [
      point('previous', 2000, false),
      point('current', 1950, true),
    ]);

    expect(entryHasDropAlert(before.points[1])).toBe(false);
    expect(after.drop_threshold_percent).toBe(2);
    expect(entryHasDropAlert(after.points[1])).toBe(true);
  });

  it('presents a point flagged from a predecessor outside the returned window', () => {
    // I3: the window limits the points, never the derivation. This single
    // returned point still carries the flag the backend derived against the
    // entry before it. A client-side derivation over `points` has no
    // predecessor here and would report "not flagged", which is the second
    // derivation these accessors exist to prevent.
    const payload = series(5, [point('current', 1800, true)]);

    expect(entryHasDropAlert(payload.points[0])).toBe(true);
  });

  it('presents the flag the payload carries for points that share a timestamp', () => {
    // D1.12's tie ordering (the earlier-created entry is the basis) is the
    // backend's, derived in 2.13 and pinned by the Rust drop-alert spec (5.5).
    // This selector takes one point at a time and cannot order anything; the
    // spec asserts only what it owns: each tied point's flag comes through
    // unchanged, including the flagged later-created one.
    const sameInstant = '2026-10-01T09:00:00Z';
    const payload = series(5, [
      point('created-first', 2000, false, sameInstant),
      point('created-second', 1800, true, sameInstant),
    ]);

    expect(entryHasDropAlert(payload.points[0])).toBe(false);
    expect(entryHasDropAlert(payload.points[1])).toBe(true);
  });

  it('follows the re-derived flag of the following entry after a deletion', () => {
    // WT-7/D1.13: A 2000 g, B 1500 g, C 1600 g. Before the deletion, C follows
    // B and is not flagged (its weight rose). With B deleted, C follows A and
    // its 20% drop is flagged. The payload is the re-read; no stored entry is
    // repaired.
    const before = series(5, [
      point('a', 2000, false),
      point('b', 1500, true),
      point('c', 1600, false),
    ]);
    const after = series(5, [point('a', 2000, false), point('c', 1600, true)]);

    expect(before.points.map(entryHasDropAlert)).toEqual([false, true, false]);
    expect(after.points.map(entryHasDropAlert)).toEqual([false, true]);
  });

  it('follows the re-derived flags around an edited entry', () => {
    // WT-7/D1.13: A 2000 g, B 1820 g, C 1600 g at a 10% threshold. Before the
    // edit B is unflagged (9%) and C is flagged (12.1%). After B is corrected
    // to 1700 g, B is flagged (15%) and C is unflagged (5.9%). Both flags are
    // the refreshed payload's, including C's although C was never edited.
    const before = series(10, [
      point('a', 2000, false),
      point('b', 1820, false),
      point('c', 1600, true),
    ]);
    const after = series(10, [
      point('a', 2000, false),
      point('b', 1700, true),
      point('c', 1600, false),
    ]);

    expect(before.points.map(entryHasDropAlert)).toEqual([false, false, true]);
    expect(after.points.map(entryHasDropAlert)).toEqual([false, true, false]);
  });
});

describe('the dashboard drop flag', () => {
  it('presents the row flag a rabbit with a flagged entry carries', () => {
    // DASH-4: a rabbit carrying an entry flagged by a drop beyond its threshold
    // is flagged on the dashboard. The row carries the backend's derivation.
    expect(dashboardRowHasDropAlert(row(true))).toBe(true);
  });

  it('clears the row flag when the raised threshold clears every entry', () => {
    // DASH-4: the flag disappears once no entry is flagged. The threshold edit
    // invalidates the dashboard rows (3.4) and the refreshed row carries the
    // cleared flag; it is never folded from cached points here.
    const before = row(true);
    const after = row(false);

    expect(dashboardRowHasDropAlert(before)).toBe(true);
    expect(dashboardRowHasDropAlert(after)).toBe(false);
  });
});
