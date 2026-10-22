/*
 * Task 3.3's evidence for the dashboard's range state (WT-6, D1.4).
 *
 * The validator is the frontend half of WT-6's three scenarios: it is what
 * turns a typed URL into the two windows the ranged reads are asked with. The
 * assertions below hold the three properties those scenarios depend on — a
 * selected range is carried through unchanged, the full-history range is a
 * value like any other rather than being rewritten to a number of days, and the
 * trend's field and the history's field stay independent of each other.
 *
 * The fallback is asserted as hard as the valid case. A stale or hand-edited
 * URL must not be able to fail the navigation: it opens the default window,
 * which is a readable state, instead of an error page.
 *
 * The validator accepts exactly the values in `WEIGHT_RANGES`, and that list is
 * hand-written. So this file also pins it against the frozen union written out
 * again below: `satisfies` in `search.ts` rejects a value the contract does not
 * have, but only an equality assertion catches a contract value that is *not*
 * in the list. Without it, deleting `days_90` would shrink `it.each` to three
 * cases and make `?trend=days_90` fall back to the default with a green suite.
 */
import { describe, expect, it } from 'vitest';
import {
  DEFAULT_WEIGHT_RANGE,
  validateWeightRangeSearch,
  WEIGHT_RANGES,
  type WeightRange,
} from './search';

/**
 * The frozen `WeightRange` union, written out in `search.test.ts` alone. It is
 * the test's pin, not a second copy the module reads: it exists so a value that
 * leaves `WEIGHT_RANGES` fails here.
 */
const CONTRACT_RANGES = [
  'days_7',
  'days_30',
  'days_90',
  'all',
] as const satisfies readonly WeightRange[];

/** Both fields, so nothing below is exercised on one field only. */
const FIELDS = ['trend', 'history'] as const;

describe('the accepted range list', () => {
  it('is the frozen union in full and in order, so no window can disappear', () => {
    expect(WEIGHT_RANGES).toEqual(CONTRACT_RANGES);
  });
});

describe('the default range', () => {
  it('is one of the four windows', () => {
    expect(WEIGHT_RANGES).toContain(DEFAULT_WEIGHT_RANGE);
  });

  it('fills both fields on a bare visit', () => {
    const search = validateWeightRangeSearch({});

    expect(search.trend).toBe(DEFAULT_WEIGHT_RANGE);
    expect(search.history).toBe(DEFAULT_WEIGHT_RANGE);
  });
});

describe('valid range values', () => {
  it.each(CONTRACT_RANGES)(
    'carries %s through unchanged in each field',
    (range: WeightRange) => {
      for (const field of FIELDS) {
        expect(validateWeightRangeSearch({ [field]: range })[field]).toBe(
          range,
        );
      }
    },
  );

  it('carries the full-history range as itself, not as a period', () => {
    // D1.4: "all" is the absence of a lower bound. A rewrite to a number of
    // days here would silently truncate a rabbit's whole record.
    expect(validateWeightRangeSearch({ trend: 'all', history: 'all' })).toEqual(
      {
        trend: 'all',
        history: 'all',
      },
    );
  });
});

describe('an invalid range value', () => {
  it('falls back to the default rather than failing the navigation', () => {
    for (const field of FIELDS) {
      expect(validateWeightRangeSearch({ [field]: 'weekly' })[field]).toBe(
        DEFAULT_WEIGHT_RANGE,
      );
    }
  });

  it('falls back for a value of the wrong type', () => {
    expect(
      validateWeightRangeSearch({ trend: 30, history: null }).history,
    ).toBe(DEFAULT_WEIGHT_RANGE);
    expect(validateWeightRangeSearch({ trend: 30 }).trend).toBe(
      DEFAULT_WEIGHT_RANGE,
    );
    expect(
      validateWeightRangeSearch({ trend: ['days_7'], history: {} }).history,
    ).toBe(DEFAULT_WEIGHT_RANGE);
  });
});

describe('the two fields are independent', () => {
  it('keeps different windows for the trend and the history', () => {
    // WT-6's independence scenario, at the state layer: this is the pair the
    // two hooks are asked with, and neither value follows the other.
    expect(
      validateWeightRangeSearch({ trend: 'days_30', history: 'days_7' }),
    ).toEqual({ trend: 'days_30', history: 'days_7' });
  });

  it('falls back only for the field that is invalid', () => {
    expect(
      validateWeightRangeSearch({ trend: 'days_90', history: 'nonsense' }),
    ).toEqual({ trend: 'days_90', history: DEFAULT_WEIGHT_RANGE });
  });

  it('does not read the history field as the trend field', () => {
    expect(validateWeightRangeSearch({ history: 'days_7' })).toEqual({
      trend: DEFAULT_WEIGHT_RANGE,
      history: 'days_7',
    });
  });

  it('drops search fields it does not own', () => {
    expect(
      validateWeightRangeSearch({ trend: 'days_7', rabbitId: 'other' }),
    ).toEqual({ trend: 'days_7', history: DEFAULT_WEIGHT_RANGE });
  });
});
