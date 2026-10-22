/*
 * The P1 event policy (task 3.9; RAB-5, RAB-6, RAB-7, RAB-10, WT-5, DASH-4).
 *
 * `rabbit:changed` and `weight:changed` are reports, not data: the listener
 * cannot merge them into the cache, so it refreshes the device's copy through
 * the collection read and only then invalidates the projections that copy
 * feeds. The order is the requirement — invalidating first would re-read the
 * stale copy — and each set is required to be exact, because a broader
 * invalidation would refetch another household's or another rabbit's rows
 * (NFR-10).
 *
 * The spec at the end emits through the real generated event descriptors and
 * the real Tauri bridge, so a wrong event name cannot pass unnoticed; the
 * policy specs before it call the mapping directly, without waiting on a
 * subscription.
 */
import type { QueryClient } from '@tanstack/react-query';
import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { createElement } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { createTestQueryClient, TestQueryClientProvider } from '../test/setup';
import {
  applyRabbitChange,
  applyWeightChange,
  useAppEvents,
} from './app-events';
import { dashboardKeys } from './dashboard/keys';
import { householdKeys } from './households/keys';
import type { Household, HouseholdCollection } from './households/queries';
import { rabbitKeys } from './rabbits/keys';
import { weightKeys } from './weight/keys';

/** Stands in for the contract's `Instant`; the cache never re-parses it. */
const AT = '2026-10-20T09:00:00Z';

const HOUSEHOLD: Household = {
  id: 'household-a',
  name: 'Household A',
  created_at: AT,
  owner_member_id: 'member-a',
  is_active: true,
  is_owner: true,
};

const COLLECTION: HouseholdCollection = {
  household_id: HOUSEHOLD.id,
  items: [],
  source: 'service',
  loaded_at: AT,
};

const RABBIT_EVENT = {
  household_id: HOUSEHOLD.id,
  rabbit_id: 'rabbit-1',
  kind: 'archived',
} as const;

const WEIGHT_EVENT = {
  household_id: HOUSEHOLD.id,
  rabbit_id: 'rabbit-1',
  entry_id: 'entry-1',
  kind: 'created',
} as const;

/** The keys one rabbit event must invalidate, exactly. */
const RABBIT_KEYS = [
  rabbitKeys.list(HOUSEHOLD.id, 'active'),
  rabbitKeys.list(HOUSEHOLD.id, 'archived'),
  rabbitKeys.detail('rabbit-1'),
  dashboardKeys.rows(HOUSEHOLD.id),
  weightKeys.series('rabbit-1'),
] as const;

/** The keys one weight event must invalidate, exactly (design.md's set). */
const WEIGHT_KEYS = [
  weightKeys.series('rabbit-1'),
  rabbitKeys.detail('rabbit-1'),
  dashboardKeys.rows(HOUSEHOLD.id),
] as const;

/** Another household's and another rabbit's entries: never invalidated. */
const OTHER_KEYS = [
  rabbitKeys.list('household-b', 'active'),
  rabbitKeys.list('household-b', 'archived'),
  rabbitKeys.detail('rabbit-2'),
  dashboardKeys.rows('household-b'),
  weightKeys.series('rabbit-2'),
] as const;

/** One household's data plus the neighbouring entries an event must not touch. */
function seed(client: QueryClient): void {
  client.setQueryData(householdKeys.active(), HOUSEHOLD);
  client.setQueryData(rabbitKeys.list(HOUSEHOLD.id, 'active'), []);
  client.setQueryData(rabbitKeys.list(HOUSEHOLD.id, 'archived'), []);
  client.setQueryData(rabbitKeys.detail('rabbit-1'), { id: 'rabbit-1' });
  client.setQueryData(dashboardKeys.rows(HOUSEHOLD.id), []);
  client.setQueryData(weightKeys.series('rabbit-1'), { points: [] });
  client.setQueryData(rabbitKeys.list('household-b', 'active'), []);
  client.setQueryData(rabbitKeys.list('household-b', 'archived'), []);
  client.setQueryData(rabbitKeys.detail('rabbit-2'), { id: 'rabbit-2' });
  client.setQueryData(dashboardKeys.rows('household-b'), []);
  client.setQueryData(weightKeys.series('rabbit-2'), { points: [] });
}

function isInvalidated(client: QueryClient, key: readonly unknown[]): boolean {
  return client.getQueryState(key)?.isInvalidated === true;
}

/**
 * Mock `load_household` and record each call's arguments.
 *
 * `onLoad` runs while the read is in flight, which is what lets a spec see
 * whether an invalidation had already happened when the refresh ran. Every other
 * command is a defect in the listener and fails the test.
 */
function mockLoadHousehold(
  answer: () => HouseholdCollection,
  onLoad?: () => void,
): unknown[] {
  const calls: unknown[] = [];

  mockIPC((command, args) => {
    if (command !== 'load_household') {
      throw new Error(`unexpected mocked command "${command}"`);
    }

    calls.push(args);
    onLoad?.();

    return answer();
  });

  return calls;
}

afterEach(() => {
  clearMocks();
});

describe('a rabbit:changed event', () => {
  it('refreshes the copy, once, before invalidating anything', async () => {
    const client = createTestQueryClient();
    seed(client);

    let invalidatedAtRefresh: boolean | null = null;
    const calls = mockLoadHousehold(
      () => COLLECTION,
      () => {
        invalidatedAtRefresh = RABBIT_KEYS.some((key) =>
          isInvalidated(client, key),
        );
      },
    );

    await applyRabbitChange(client, RABBIT_EVENT);

    // One read for the one event, for the household the event names.
    expect(calls).toEqual([{ householdId: HOUSEHOLD.id }]);
    // If the invalidation ran first, this would be true.
    expect(invalidatedAtRefresh).toBe(false);
    for (const key of RABBIT_KEYS) {
      expect(isInvalidated(client, key)).toBe(true);
    }
  });

  it('invalidates exactly the household’s lists, the profile, the rows and the series', async () => {
    const client = createTestQueryClient();
    seed(client);
    mockLoadHousehold(() => COLLECTION);

    await applyRabbitChange(client, RABBIT_EVENT);

    for (const key of RABBIT_KEYS) {
      expect(isInvalidated(client, key)).toBe(true);
    }

    for (const key of OTHER_KEYS) {
      expect(isInvalidated(client, key)).toBe(false);
    }

    // The read has just written the copy; naming its key would refresh twice.
    expect(
      client.getQueryState(householdKeys.collection(HOUSEHOLD.id))
        ?.isInvalidated,
    ).toBe(false);
  });

  for (const kind of ['created', 'updated', 'archived', 'restored'] as const) {
    it(`invalidates the whole set for ${kind}`, async () => {
      const client = createTestQueryClient();
      seed(client);
      mockLoadHousehold(() => COLLECTION);

      await applyRabbitChange(client, { ...RABBIT_EVENT, kind });

      for (const key of RABBIT_KEYS) {
        expect(isInvalidated(client, key)).toBe(true);
      }
      // Invalidated, not removed: both entries are still cached.
      expect(client.getQueryState(rabbitKeys.detail('rabbit-1'))).toBeDefined();
      expect(client.getQueryState(weightKeys.series('rabbit-1'))).toBeDefined();
    });
  }

  it('removes the deleted rabbit’s profile and series instead of invalidating them', async () => {
    const client = createTestQueryClient();
    seed(client);
    mockLoadHousehold(() => COLLECTION);

    await applyRabbitChange(client, { ...RABBIT_EVENT, kind: 'deleted' });

    // RAB-10: a refetch would refuse, and TanStack keeps the last data on a
    // failed refetch — the deleted profile and history would stay on screen.
    expect(client.getQueryState(rabbitKeys.detail('rabbit-1'))).toBeUndefined();
    expect(client.getQueryState(weightKeys.series('rabbit-1'))).toBeUndefined();

    for (const key of [
      rabbitKeys.list(HOUSEHOLD.id, 'active'),
      rabbitKeys.list(HOUSEHOLD.id, 'archived'),
      dashboardKeys.rows(HOUSEHOLD.id),
    ]) {
      expect(isInvalidated(client, key)).toBe(true);
    }
    for (const key of OTHER_KEYS) {
      // `removeQueries` would leave `isInvalidated` false too, so "untouched"
      // has to be proven by the entry still existing.
      expect(client.getQueryState(key)).toBeDefined();
      expect(isInvalidated(client, key)).toBe(false);
    }
  });

  it('does nothing for a household this device is not showing', async () => {
    const client = createTestQueryClient();
    seed(client);
    const calls = mockLoadHousehold(() => COLLECTION);

    await applyRabbitChange(client, {
      ...RABBIT_EVENT,
      household_id: 'household-b',
      rabbit_id: 'rabbit-2',
    });

    expect(calls).toHaveLength(0);
    for (const key of [...RABBIT_KEYS, ...OTHER_KEYS]) {
      expect(isInvalidated(client, key)).toBe(false);
    }
  });

  it('does nothing when no household is shown', async () => {
    const client = createTestQueryClient();
    seed(client);
    client.removeQueries({ queryKey: householdKeys.active() });
    const calls = mockLoadHousehold(() => COLLECTION);

    await applyRabbitChange(client, RABBIT_EVENT);

    expect(calls).toHaveLength(0);
    for (const key of RABBIT_KEYS) {
      expect(isInvalidated(client, key)).toBe(false);
    }
  });

  it('treats the content-unloaded refusal as a refresh, not a permanent failure', async () => {
    const client = createTestQueryClient();
    seed(client);

    let loads = 0;
    mockLoadHousehold(() => {
      loads += 1;

      if (loads === 1) {
        throw {
          cause: 'unknown',
          retryable: false,
          message_key: 'error.household_content_unloaded',
          request_id: null,
        };
      }

      return COLLECTION;
    });

    await applyRabbitChange(client, RABBIT_EVENT);

    // The refusal is not terminal: the collection read recorded it, and the
    // invalidated projections are left to answer it.
    expect(
      client.getQueryState(householdKeys.collection(HOUSEHOLD.id))?.error,
    ).toMatchObject({
      cause: 'unknown',
      message_key: 'error.household_content_unloaded',
    });
    expect(isInvalidated(client, rabbitKeys.detail('rabbit-1'))).toBe(true);

    await applyRabbitChange(client, RABBIT_EVENT);

    // The second refresh filled the copy, so the refusal cleared.
    expect(client.getQueryData(householdKeys.collection(HOUSEHOLD.id))).toEqual(
      COLLECTION,
    );
    expect(
      client.getQueryState(householdKeys.collection(HOUSEHOLD.id))?.error,
    ).toBeNull();
  });
});

describe('a weight:changed event', () => {
  it('refreshes the copy, once, before invalidating anything', async () => {
    const client = createTestQueryClient();
    seed(client);

    let invalidatedAtRefresh: boolean | null = null;
    const calls = mockLoadHousehold(
      () => COLLECTION,
      () => {
        invalidatedAtRefresh = WEIGHT_KEYS.some((key) =>
          isInvalidated(client, key),
        );
      },
    );

    await applyWeightChange(client, WEIGHT_EVENT);

    expect(calls).toEqual([{ householdId: HOUSEHOLD.id }]);
    expect(invalidatedAtRefresh).toBe(false);
  });

  it('invalidates exactly the rabbit’s series, profile and the household’s rows', async () => {
    const client = createTestQueryClient();
    seed(client);
    mockLoadHousehold(() => COLLECTION);

    await applyWeightChange(client, WEIGHT_EVENT);

    for (const key of WEIGHT_KEYS) {
      expect(isInvalidated(client, key)).toBe(true);
    }

    // A weight write changes no list row. The profile is still named: the
    // event is the cross-device path and `design.md`'s set includes it.
    expect(isInvalidated(client, rabbitKeys.list(HOUSEHOLD.id, 'active'))).toBe(
      false,
    );
    expect(
      isInvalidated(client, rabbitKeys.list(HOUSEHOLD.id, 'archived')),
    ).toBe(false);
    for (const key of OTHER_KEYS) {
      expect(isInvalidated(client, key)).toBe(false);
    }
  });

  it('does nothing for a household this device is not showing', async () => {
    const client = createTestQueryClient();
    seed(client);
    const calls = mockLoadHousehold(() => COLLECTION);

    await applyWeightChange(client, {
      ...WEIGHT_EVENT,
      household_id: 'household-b',
      rabbit_id: 'rabbit-2',
    });

    expect(calls).toHaveLength(0);
    for (const key of [...WEIGHT_KEYS, ...OTHER_KEYS]) {
      expect(isInvalidated(client, key)).toBe(false);
    }
  });
});

/** Let the resolved `listen` promises reach their cleanup handlers. */
async function flush(): Promise<void> {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  });
}

describe('useAppEvents', () => {
  it('subscribes to both P1 events through the real descriptors and releases them', async () => {
    const client = createTestQueryClient();
    seed(client);
    client.setQueryData(weightKeys.series('rabbit-3'), { points: [] });

    const eventCalls: unknown[] = [];
    mockIPC(
      (command, args) => {
        if (command !== 'load_household') {
          throw new Error(`unexpected mocked command "${command}"`);
        }

        eventCalls.push(args);

        return COLLECTION;
      },
      { shouldMockEvents: true },
    );

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { unmount } = renderHook(() => useAppEvents(), { wrapper });

    await flush();

    await act(async () => {
      await emit('rabbit:changed', RABBIT_EVENT);
    });

    await waitFor(() => {
      expect(isInvalidated(client, rabbitKeys.detail('rabbit-1'))).toBe(true);
    });

    await act(async () => {
      await emit('weight:changed', { ...WEIGHT_EVENT, rabbit_id: 'rabbit-3' });
    });

    await waitFor(() => {
      expect(isInvalidated(client, weightKeys.series('rabbit-3'))).toBe(true);
    });
    expect(eventCalls).toHaveLength(2);

    unmount();
    await flush();

    // Each event reaches nobody after unmount: a missing release for either
    // listener would run the refresh and raise `eventCalls`.
    await act(async () => {
      await emit('rabbit:changed', { ...RABBIT_EVENT, rabbit_id: 'rabbit-2' });
      await emit('weight:changed', { ...WEIGHT_EVENT, rabbit_id: 'rabbit-3' });
    });
    await flush();

    expect(isInvalidated(client, rabbitKeys.detail('rabbit-2'))).toBe(false);
    expect(eventCalls).toHaveLength(2);
  });
});
