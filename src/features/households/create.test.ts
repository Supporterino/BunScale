/*
 * Household creation at the frontend layer (task 3.5 gap; HS-1, HS-2, HS-4,
 * D0.14).
 *
 * `useCreateHousehold` exists so first-run onboarding (4.1) and Settings'
 * "create an additional household" (4.3, HS-4) share one mutation with one set
 * of cache rules. The properties asserted here are the ones a screen would
 * otherwise have to be trusted about:
 *
 * - the mutation variable is the **operation handle**, so a retry re-sends the
 *   same `request_id` (D0.14);
 * - a success writes the confirmed active household into `active` and rewrites
 *   the cached list's `is_active` flags, then invalidates the held-set pair —
 *   never a collection (HS-3, NFR-10);
 * - a failure writes and invalidates nothing (no optimistic write, D1).
 *
 * IPC is mocked at `invoke`, the seam `testing-strategy` prescribes.
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  createTestQueryClient,
  TestQueryClientProvider,
} from '../../test/setup';
import { createHousehold } from './api';
import { householdKeys } from './keys';
import type {
  Household,
  HouseholdCollection,
  HouseholdSummary,
} from './queries';
import { useCreateHousehold } from './queries';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

const EXISTING: Household = {
  id: 'household-a',
  name: 'Household A',
  created_at: AT,
  owner_member_id: 'member-a',
  is_active: true,
  is_owner: true,
};

/** The creation's confirmed post-state: it is active and the person owns it. */
const CREATED: Household = {
  id: 'household-new',
  name: 'New household',
  created_at: AT,
  owner_member_id: 'member-new',
  is_active: true,
  is_owner: true,
};

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

const COLLECTION: HouseholdCollection = {
  household_id: EXISTING.id,
  items: [],
  source: 'local',
  loaded_at: AT,
};

/** `[id, isActive]` in list order. */
function activeFlags(
  client: ReturnType<typeof createTestQueryClient>,
): Array<[string, boolean]> {
  const list =
    client.getQueryData<HouseholdSummary[]>(householdKeys.list()) ?? [];

  return list.map((row) => [row.id, row.is_active]);
}

describe('creating a household', () => {
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

  it('writes the confirmed household and invalidates only the held-set pair', async () => {
    const client = createTestQueryClient();
    const operation = createHousehold(CREATED.name);

    client.setQueryData(householdKeys.list(), [
      summary(EXISTING, true),
      summary({ ...EXISTING, id: 'household-b' }, false),
    ]);
    client.setQueryData(householdKeys.active(), EXISTING);
    client.setQueryData(householdKeys.collection(EXISTING.id), COLLECTION);

    answers.push(() => CREATED);

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useCreateHousehold(), { wrapper });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(calls).toEqual([
      {
        command: 'create_household',
        args: { name: CREATED.name, requestId: operation.requestId },
      },
    ]);
    expect(client.getQueryData(householdKeys.active())).toEqual(CREATED);

    // The new household is not appended (the command returns no summary), so
    // every cached row is inactive at that instant rather than one stale row
    // still claiming to be active.
    expect(activeFlags(client)).toEqual([
      [EXISTING.id, false],
      ['household-b', false],
    ]);
    expect(client.getQueryState(householdKeys.list())?.isInvalidated).toBe(
      true,
    );
    expect(client.getQueryState(householdKeys.active())?.isInvalidated).toBe(
      true,
    );

    // No collection was created, reloaded or invalidated by the creation.
    expect(client.getQueryData(householdKeys.collection(EXISTING.id))).toEqual(
      COLLECTION,
    );
    expect(
      client.getQueryState(householdKeys.collection(EXISTING.id))
        ?.isInvalidated,
    ).toBe(false);
  });

  it('retries by re-running the same handle, so both attempts share one id', async () => {
    const client = createTestQueryClient();
    const operation = createHousehold(CREATED.name);

    answers.push(
      () => {
        throw {
          cause: 'connectivity',
          retryable: true,
          message_key: 'error.connectivity',
          request_id: operation.requestId,
        };
      },
      () => CREATED,
    );

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useCreateHousehold(), { wrapper });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    // A failed creation writes nothing.
    expect(client.getQueryData(householdKeys.active())).toBeUndefined();

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
      'create_household',
      'create_household',
    ]);
    expect(client.getQueryData(householdKeys.active())).toEqual(CREATED);
  });

  it('leaves the held-set pair untouched when creation fails', async () => {
    const client = createTestQueryClient();

    client.setQueryData(householdKeys.list(), [summary(EXISTING, true)]);
    client.setQueryData(householdKeys.active(), EXISTING);

    answers.push(() => {
      throw {
        cause: 'validation',
        retryable: false,
        message_key: 'error.validation',
        request_id: 'operation-1',
      };
    });

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useCreateHousehold(), { wrapper });

    await act(async () => {
      result.current.mutate(createHousehold('   '));
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(client.getQueryData(householdKeys.active())).toEqual(EXISTING);
    expect(activeFlags(client)).toEqual([[EXISTING.id, true]]);
    expect(client.getQueryState(householdKeys.list())?.isInvalidated).toBe(
      false,
    );
  });
});
