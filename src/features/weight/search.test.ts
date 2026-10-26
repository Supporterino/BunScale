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
  validateWeightEntrySearch,
  validateWeightFormSearch,
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

/*
 * Task 4.18's typed search (WT-2, DASH-9, D1.10).
 *
 * The weight form is one route reached by both quick paths, and its binding,
 * entry kind and origin are search parameters rather than screen state. The
 * validator is the same "never fail the navigation" shape as the range pair
 * above: a deep link is a preference, so a missing or misspelled value becomes
 * the ordinary default instead of an error page.
 *
 * What the fallbacks *are*, asserted rather than guessed: a missing `rabbit`
 * becomes the empty string (the profile read is the surface that refuses an
 * unknown id), `mode` is always `quick` because only quick entry exists, and
 * `from` is `dashboard` unless it says `rabbit` *and* a rabbit is actually
 * bound — without an id there is no rabbit dashboard to return to.
 */
describe('the weight form search', () => {
  /** The rabbit a dashboard row's quick-weight control binds (DASH-9). */
  const RABBIT = 'rabbit-hazel';

  it('round-trips a complete binding, ranges included', () => {
    expect(
      validateWeightFormSearch({
        rabbit: RABBIT,
        mode: 'quick',
        from: 'dashboard',
        trend: 'days_7',
        history: 'all',
      }),
    ).toEqual({
      trend: 'days_7',
      history: 'all',
      rabbit: RABBIT,
      mode: 'quick',
      from: 'dashboard',
    });
  });

  it('defaults a missing rabbit to an empty binding instead of throwing', () => {
    expect(validateWeightFormSearch({})).toEqual({
      trend: DEFAULT_WEIGHT_RANGE,
      history: DEFAULT_WEIGHT_RANGE,
      rabbit: '',
      mode: 'quick',
      from: 'dashboard',
    });
  });

  it('keeps a malformed search on the dashboard when no rabbit is bound', () => {
    // `from: 'rabbit'` without a rabbit would be a return to a route with no
    // id, so it falls back to the dashboard origin.
    expect(validateWeightFormSearch({ from: 'rabbit' })).toMatchObject({
      rabbit: '',
      from: 'dashboard',
    });
  });

  it('honours the rabbit origin only when a rabbit is bound', () => {
    expect(
      validateWeightFormSearch({ rabbit: RABBIT, from: 'rabbit' }),
    ).toMatchObject({ rabbit: RABBIT, from: 'rabbit' });
  });

  it('keeps the dashboard origin when the search omits it', () => {
    expect(validateWeightFormSearch({ rabbit: RABBIT })).toMatchObject({
      rabbit: RABBIT,
      from: 'dashboard',
    });
  });

  it('defaults the origin to the dashboard when the value is unknown', () => {
    expect(
      validateWeightFormSearch({ rabbit: RABBIT, from: 'archived-list' }),
    ).toMatchObject({ rabbit: RABBIT, from: 'dashboard' });
  });

  it('defaults the mode to quick for any unknown value', () => {
    // Only the quick-entry kind exists (WT-2, D1.10); the route still names it
    // so a second kind could be bound without changing the contract.
    expect(
      validateWeightFormSearch({ rabbit: RABBIT, mode: 'picker' }),
    ).toMatchObject({ mode: 'quick' });
  });

  it('ignores a rabbit that is not a string', () => {
    expect(
      validateWeightFormSearch({ rabbit: 42, from: 'rabbit' }),
    ).toMatchObject({ rabbit: '', from: 'dashboard' });
  });

  it('validates the range pair through the range schema', () => {
    const search = { rabbit: RABBIT, trend: 'weekly', history: 'all' };

    // The pair is the dashboard's contract, not a second copy of its windows.
    expect(validateWeightFormSearch(search)).toMatchObject(
      validateWeightRangeSearch(search),
    );
    expect(validateWeightFormSearch(search)).toMatchObject({
      trend: DEFAULT_WEIGHT_RANGE,
      history: 'all',
    });
  });

  it('drops search fields it does not own', () => {
    expect(
      validateWeightFormSearch({ rabbit: RABBIT, householdId: 'other' }),
    ).toEqual({
      trend: DEFAULT_WEIGHT_RANGE,
      history: DEFAULT_WEIGHT_RANGE,
      rabbit: RABBIT,
      mode: 'quick',
      from: 'dashboard',
    });
  });
});

/*
 * Task 4.19's typed search (WT-8, D1.2, D1.4).
 *
 * The entry detail is one route reached from a rabbit's history or the list, and
 * its binding, origin and the dashboard's two windows are search parameters
 * rather than screen state: Back returns to the surface the person left, at the
 * windows it was left at (WT-6, D1.4). The validator has the same "never fail
 * the navigation" shape as the form's above, and both windows are required in
 * its result even though either may be absent from the URL, because Back cannot
 * invent a window.
 *
 * A missing or blank `rabbit` is deliberately not defaulted away, and that
 * matters beyond this file: `rabbit` scopes the detail's cache key through
 * `weightKeys.detail(rabbitId, entryId)`, so an absent binding keys the entry as
 * `['weights', 'series', '', 'detail', entryId]` — outside every
 * `series(rabbitId)` prefix, which no threshold edit's invalidation reaches. The
 * entry still reads and renders, but its derived drop flag can keep the value a
 * previous threshold produced; task 4.21 must read the rabbit from the entry it
 * loaded before trusting that flag (`./keys` records the same constraint).
 */
describe('the weight entry search', () => {
  /** The rabbit whose series the detail's cache key sits under (WT-8). */
  const RABBIT = 'rabbit-hazel';

  it('round-trips a complete search, ranges included', () => {
    expect(
      validateWeightEntrySearch({
        rabbit: RABBIT,
        from: 'rabbit',
        trend: 'days_7',
        history: 'all',
      }),
    ).toEqual({
      trend: 'days_7',
      history: 'all',
      rabbit: RABBIT,
      from: 'rabbit',
    });
  });

  it('defaults both windows when the deep link names neither', () => {
    expect(validateWeightEntrySearch({ rabbit: RABBIT })).toMatchObject({
      trend: DEFAULT_WEIGHT_RANGE,
      history: DEFAULT_WEIGHT_RANGE,
    });
  });

  it('defaults the origin to the dashboard when the search omits it', () => {
    expect(validateWeightEntrySearch({ rabbit: RABBIT })).toMatchObject({
      from: 'dashboard',
    });
  });

  it('defaults the origin to the dashboard when the value is unknown', () => {
    expect(
      validateWeightEntrySearch({ rabbit: RABBIT, from: 'archived-list' }),
    ).toMatchObject({ from: 'dashboard' });
  });

  it('keeps a missing or blank rabbit as the empty binding', () => {
    // The read is the surface that refuses an unknown id; inventing one here
    // would bind the detail to a rabbit the person did not open. Without a
    // rabbit there is also no rabbit dashboard to return to, so the origin
    // falls back to the list.
    const missing = validateWeightEntrySearch({});
    const blank = validateWeightEntrySearch({ rabbit: '', from: 'rabbit' });

    expect(missing).toMatchObject({ rabbit: '', from: 'dashboard' });
    expect(blank).toMatchObject({ rabbit: '', from: 'dashboard' });

    // The key consequence: `weightKeys.detail('', entryId)` composes to
    // `['weights', 'series', '', 'detail', entryId]`, which no
    // `series(rabbitId)` invalidation reaches — `./keys.test.ts` pins that
    // shape.
  });

  it('validates the range pair through the range schema', () => {
    const search = { rabbit: RABBIT, trend: 'weekly', history: 'all' };

    // The pair is the dashboard's contract, not a second copy of its windows.
    expect(validateWeightEntrySearch(search)).toMatchObject(
      validateWeightRangeSearch(search),
    );
    expect(validateWeightEntrySearch(search)).toMatchObject({
      trend: DEFAULT_WEIGHT_RANGE,
      history: 'all',
    });
  });

  it('drops search fields it does not own', () => {
    expect(
      validateWeightEntrySearch({ rabbit: RABBIT, entryId: 'other' }),
    ).toEqual({
      trend: DEFAULT_WEIGHT_RANGE,
      history: DEFAULT_WEIGHT_RANGE,
      rabbit: RABBIT,
      from: 'dashboard',
    });
  });
});
