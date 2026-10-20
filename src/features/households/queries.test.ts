/*
 * Task 3.2's evidence for the two properties the collection cache owns:
 * households cannot be served each other's rows (NFR-10), and a refresh
 * replaces the cached collection instead of accumulating items (SYNC-1,
 * SYNC-3).
 *
 * These run against the query layer with IPC mocked — not the `./api` wrapper —
 * because the route tree now imports the household query module eagerly through
 * `src/test/setup.ts`, so a `vi.mock('./api')` would register after that module
 * graph had already been built and the real wrapper would stay bound. Mocking
 * at `invoke` is the seam `testing-strategy` prescribes for specs in this layer,
 * and it has the useful side effect of exercising the real wrappers: what
 * arrives in the cache is what `typedError` produced from the mocked command
 * answer, including a thrown `CommandError`.
 *
 * No React and no IPC beyond the mock for the cache-level specs; the
 * cross-household spec renders the hook through the test harness's provider,
 * because a placeholder carried between keys only appears at an observer, never
 * in the cache itself.
 */
import type { QueryClient } from '@tanstack/react-query';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  createTestQueryClient,
  TestQueryClientProvider,
} from '../../test/setup';
import { householdKeys } from './keys';
import type { HouseholdCollection } from './queries';
import {
  collectionPresence,
  householdCollectionQueryOptions,
  useHouseholdCollection,
} from './queries';

/** Stands in for the contract's `Instant`; the cache never re-parses it. */
const LOADED_AT = '2026-10-20T09:00:00Z';

function collection(
  householdId: string,
  itemIds: readonly string[],
  overrides: Partial<HouseholdCollection> = {},
): HouseholdCollection {
  return {
    household_id: householdId,
    items: itemIds.map((itemId) => ({
      item_id: itemId,
      updated_at: LOADED_AT,
      deleted: false,
    })),
    source: 'local',
    loaded_at: LOADED_AT,
    ...overrides,
  };
}

/** A promise whose settlement the calling test controls. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};

  const promise = new Promise<T>((settle) => {
    resolve = settle;
  });

  return { promise, resolve };
}

describe('the household collection cache', () => {
  let client: QueryClient;
  /** What the next `load_household` call answers: a value, or a throw. */
  let answers: Array<() => unknown>;
  /** The arguments each `load_household` call was given. */
  let calls: unknown[];

  beforeEach(() => {
    client = createTestQueryClient();
    answers = [];
    calls = [];

    mockIPC((command, args) => {
      if (command !== 'load_household') {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      calls.push(args);

      const answer = answers.shift();

      if (answer === undefined) {
        throw new Error('no load_household answer was queued');
      }

      return answer();
    });
  });

  afterEach(() => {
    clearMocks();
  });

  it('keys a collection by its household, so rows cannot cross households', () => {
    const first = collection('household-a', ['item-1']);

    client.setQueryData(householdKeys.collection('household-a'), first);

    expect(householdKeys.collection('household-a')).not.toEqual(
      householdKeys.collection('household-b'),
    );
    expect(
      client.getQueryData(householdKeys.collection('household-b')),
    ).toBeUndefined();
    expect(
      client.getQueryData(householdKeys.collection('household-a')),
    ).toEqual(first);
  });

  it('keeps a pending household from showing another household’s rows', async () => {
    const first = collection('household-a', ['item-1']);
    const next = collection('household-b', ['item-2']);
    const pending = deferred<HouseholdCollection>();

    answers.push(
      () => first,
      () => pending.promise,
    );

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result, rerender } = renderHook(
      ({ householdId }: { householdId: string }) =>
        useHouseholdCollection(householdId),
      { wrapper, initialProps: { householdId: 'household-a' } },
    );

    await waitFor(() => {
      expect(result.current.data).toEqual(first);
    });

    rerender({ householdId: 'household-b' });

    // Nothing may be borrowed from A's entry while B's read is in flight: a
    // cross-key placeholder would render A's rows under B's id (NFR-10).
    expect(result.current.data).toBeUndefined();
    expect(
      client.getQueryData(householdKeys.collection('household-b')),
    ).toBeUndefined();

    pending.resolve(next);

    await waitFor(() => {
      expect(result.current.data).toEqual(next);
    });
    expect(
      client.getQueryData(householdKeys.collection('household-a')),
    ).toEqual(first);
  });

  it('reads the household its key names', async () => {
    answers.push(() => collection('household-b', []));

    await client.fetchQuery(householdCollectionQueryOptions('household-b'));

    expect(calls).toEqual([{ householdId: 'household-b' }]);
  });

  it('replaces a refreshed collection instead of appending to it', async () => {
    answers.push(
      () => collection('household-a', ['item-1', 'item-2']),
      () => collection('household-a', ['item-1']),
    );

    const first = await client.fetchQuery(
      householdCollectionQueryOptions('household-a'),
    );

    expect(first.items.map((item) => item.item_id)).toEqual([
      'item-1',
      'item-2',
    ]);

    const refreshed = await client.fetchQuery(
      householdCollectionQueryOptions('household-a'),
    );

    expect(refreshed.items.map((item) => item.item_id)).toEqual(['item-1']);
    expect(
      client.getQueryData(householdKeys.collection('household-a')),
    ).toEqual(refreshed);
  });

  it('keeps the previously loaded collection when a refresh fails', async () => {
    const loaded = collection('household-a', ['item-1']);

    answers.push(
      () => loaded,
      () => {
        throw {
          cause: 'connectivity',
          retryable: true,
          message_key: 'error.connectivity',
          request_id: null,
        };
      },
    );

    await client.fetchQuery(householdCollectionQueryOptions('household-a'));

    await expect(
      client.fetchQuery(householdCollectionQueryOptions('household-a')),
    ).rejects.toMatchObject({ cause: 'connectivity' });

    expect(
      client.getQueryData(householdKeys.collection('household-a')),
    ).toEqual(loaded);
  });

  it('surfaces the typed failure as the query error, cause intact', async () => {
    answers.push(() => {
      throw {
        cause: 'validation',
        retryable: false,
        message_key: 'error.validation',
        request_id: null,
      };
    });

    await expect(
      client.fetchQuery(householdCollectionQueryOptions('household-a')),
    ).rejects.toMatchObject({
      cause: 'validation',
      retryable: false,
      message_key: 'error.validation',
    });
  });

  it('distinguishes a never-loaded collection from an empty one', () => {
    const neverLoaded = collection('household-a', [], { loaded_at: null });
    const empty = collection('household-a', []);
    const refreshed = collection('household-a', ['item-1'], {
      source: 'service',
    });

    expect(collectionPresence(neverLoaded)).toEqual({ kind: 'never-loaded' });
    expect(collectionPresence(empty)).toEqual({
      kind: 'loaded',
      source: 'local',
      loadedAt: LOADED_AT,
    });
    expect(collectionPresence(refreshed)).toEqual({
      kind: 'loaded',
      source: 'service',
      loadedAt: LOADED_AT,
    });
    expect(neverLoaded.items).toHaveLength(empty.items.length);
  });
});
