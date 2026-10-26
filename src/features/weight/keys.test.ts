/*
 * Task 3.3's evidence for the weight key's composition (WT-6, D1.4).
 *
 * The two ranged views are independent only because the cache entry names both
 * the rabbit and the window. The validator specs next door cannot see that: a
 * factory that ignored its `range`, or its `rabbitId`, would leave every one of
 * them green while the trend at 30 days and the history at 7 days shared one
 * entry — and whichever view mounted last would decide what both saw.
 *
 * The shape follows `../households/queries.test.ts`'s cross-household spec: the
 * inequality of the keys is asserted through the cache, so the property proved
 * is that no entry is reachable under the other key. The range pair below is
 * exactly WT-6's independence scenario at the state layer; the cross-view
 * scenario at the surface is the E2E layer's (5.x).
 *
 * The prefix spec is the other half of the key's contract: `weightKeys.series`
 * is the unit task 3.5 invalidates after a write, so every range of one rabbit
 * has to sit under it. Nothing else asserts that composition.
 */
import { describe, expect, it } from 'vitest';
import { createTestQueryClient } from '../../test/setup';
import { weightKeys } from './keys';

describe('the weight key composition', () => {
  it('keys a series by its rabbit, so one rabbit cannot serve another', () => {
    const client = createTestQueryClient();

    client.setQueryData(weightKeys.range('rabbit-a', 'days_7'), 'rabbit-a');

    expect(weightKeys.series('rabbit-a')).not.toEqual(
      weightKeys.series('rabbit-b'),
    );
    expect(
      client.getQueryData(weightKeys.range('rabbit-b', 'days_7')),
    ).toBeUndefined();
  });

  it('keys an entry by its window, so the trend and the history cannot share one', () => {
    const client = createTestQueryClient();

    client.setQueryData(weightKeys.range('rabbit-a', 'days_30'), 'trend');

    // WT-6: trend at 30 days and history at 7 days are two entries.
    expect(weightKeys.range('rabbit-a', 'days_7')).not.toEqual(
      weightKeys.range('rabbit-a', 'days_30'),
    );
    expect(
      client.getQueryData(weightKeys.range('rabbit-a', 'days_7')),
    ).toBeUndefined();
  });

  it('nests every window of a rabbit under that rabbit, so one write invalidates them together', () => {
    const prefix = weightKeys.series('rabbit-a');

    expect(weightKeys.range('rabbit-a', 'all').slice(0, prefix.length)).toEqual(
      prefix,
    );
  });

  it('nests an entry detail under its rabbit, so a threshold edit reaches it', async () => {
    // Task 4.19: the detail key sits under `series(rabbitId)` because a rabbit
    // write invalidates exactly that prefix, and the entry's drop flag is
    // re-derived on read. A detail keyed outside it would keep the flag the
    // previous threshold produced (D1.3, D1.13).
    const client = createTestQueryClient();
    const key = weightKeys.detail('rabbit-a', 'weight-1');

    client.setQueryData(key, 'entry');
    await client.invalidateQueries({ queryKey: weightKeys.series('rabbit-a') });

    expect(client.getQueryState(key)?.isInvalidated).toBe(true);
    expect(key.slice(0, weightKeys.series('rabbit-a').length)).toEqual(
      weightKeys.series('rabbit-a'),
    );
    expect(key).not.toEqual(weightKeys.detail('rabbit-b', 'weight-1'));
  });
});
