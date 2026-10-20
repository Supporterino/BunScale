/*
 * House switching at the frontend layer (task 3.3; HS-3, HS-4, DM-3, D0.14).
 *
 * Four properties are asserted, and each is something a later surface would
 * otherwise have to be trusted about:
 *
 * - the mutation variable is the **operation handle** from
 *   `setActiveHousehold`, and a retry re-runs it — so both attempts carry one
 *   `request_id`, which is D0.14's rule applied to the switch;
 * - a failed switch writes nothing: neither the active cache nor the held
 *   list's `is_active` flags change until the backend confirms (no optimistic
 *   write, D1);
 * - a successful switch writes the command's post-state into **both** caches,
 *   so exactly one household is marked active in the same render that receives
 *   the answer (DM-3) rather than after a refetch;
 * - no collection entry is touched: collections are keyed by household, so the
 *   previous household's rows stay its own entry and are not invalidated
 *   (NFR-10).
 *
 * IPC is mocked at `invoke` for the same reason as the other query specs: the
 * route tree loads these modules eagerly through `src/test/setup.ts`.
 */
import type { QueryClient } from '@tanstack/react-query';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  createTestQueryClient,
  TestQueryClientProvider,
} from '../../test/setup';
import { setActiveHousehold } from './api';
import { householdKeys } from './keys';
import type {
  Household,
  HouseholdCollection,
  HouseholdSummary,
} from './queries';
import { useSwitchActiveHousehold } from './queries';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

/** The household that is active before the switch. */
const PREVIOUS: Household = {
  id: 'household-a',
  name: 'Household A',
  created_at: AT,
  owner_member_id: 'member-a',
  is_active: true,
  is_owner: true,
};

/** The household the person switches to, and the post-state the backend sends. */
const SWITCHED: Household = {
  id: 'household-b',
  name: 'Household B',
  created_at: AT,
  owner_member_id: 'member-b',
  is_active: true,
  is_owner: true,
};

/** One row of the held list, as the book reports it (HS-4). */
function summary(household: Household, isActive: boolean): HouseholdSummary {
  return {
    id: household.id,
    name: household.name,
    created_at: household.created_at,
    is_active: isActive,
    is_owner: household.is_owner,
    member_count: 1,
  };
}

/** The previous household's loaded collection, which a switch must not touch. */
const PREVIOUS_COLLECTION: HouseholdCollection = {
  household_id: PREVIOUS.id,
  items: [{ item_id: 'item-1', updated_at: AT, deleted: false }],
  source: 'local',
  loaded_at: AT,
};

/** The held list's active flags, in list order. */
function activeFlags(client: QueryClient): Array<[string, boolean]> {
  const list =
    client.getQueryData<HouseholdSummary[]>(householdKeys.list()) ?? [];

  return list.map((row) => [row.id, row.is_active]);
}

describe('switching the active household', () => {
  let answers: Array<() => unknown>;
  let calls: Array<{ command: string; args: unknown }>;

  beforeEach(() => {
    answers = [];
    calls = [];

    mockIPC((command, args) => {
      calls.push({ command, args });

      const answer = answers.shift();

      if (answer === undefined) {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return answer();
    });
  });

  afterEach(() => {
    clearMocks();
  });

  it('writes the confirmed post-state into both caches and leaves collections alone', async () => {
    const client = createTestQueryClient();
    const operation = setActiveHousehold(SWITCHED.id);
    const previousCollectionKey = householdKeys.collection(PREVIOUS.id);

    client.setQueryData(householdKeys.list(), [
      summary(PREVIOUS, true),
      summary(SWITCHED, false),
    ]);
    client.setQueryData(previousCollectionKey, PREVIOUS_COLLECTION);

    answers.push(() => {
      throw {
        cause: 'connectivity',
        retryable: true,
        message_key: 'error.connectivity',
        request_id: operation.requestId,
      };
    });

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useSwitchActiveHousehold(), {
      wrapper,
    });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    // A failed switch changes nothing (D1): no active entry, the list still
    // marks the previous household active, and the collection is untouched.
    expect(client.getQueryData(householdKeys.active())).toBeUndefined();
    expect(activeFlags(client)).toEqual([
      [PREVIOUS.id, true],
      [SWITCHED.id, false],
    ]);
    expect(client.getQueryData(previousCollectionKey)).toEqual(
      PREVIOUS_COLLECTION,
    );

    // Retrying re-runs the same handle, so both attempts carry one identity.
    answers.push(() => SWITCHED);

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    const requestIds = calls.map(
      (call) => (call.args as { requestId?: string }).requestId,
    );

    expect(requestIds).toEqual([operation.requestId, operation.requestId]);
    expect(calls.map((call) => call.command)).toEqual([
      'set_active_household',
      'set_active_household',
    ]);
    expect(client.getQueryData(householdKeys.active())).toEqual(SWITCHED);

    // Exactly one active household, from the confirmed post-state (DM-3): the
    // list is rewritten in `onSuccess`, not left at its pre-switch flags until
    // the invalidation's refetch lands.
    expect(activeFlags(client)).toEqual([
      [PREVIOUS.id, false],
      [SWITCHED.id, true],
    ]);
    const actives = activeFlags(client).filter(([, isActive]) => isActive);
    expect(actives).toHaveLength(1);

    // The previous household's collection is its own entry: unchanged and not
    // invalidated (NFR-10).
    expect(client.getQueryData(previousCollectionKey)).toEqual(
      PREVIOUS_COLLECTION,
    );
    expect(client.getQueryState(previousCollectionKey)?.isInvalidated).toBe(
      false,
    );

    // The list is still marked stale, so the next read is the book's own.
    expect(client.getQueryState(householdKeys.list())?.isInvalidated).toBe(
      true,
    );
  });
});
