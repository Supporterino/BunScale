/*
 * HS-4 and DM-3 at the frontend layer (task 3.5).
 *
 * The household data is task 3.3's; this module only marks the active row, and
 * these specs pin the two ways that marking could go wrong:
 *
 * - **Two active rows, or a stale one, between an answer and a refetch.** Once
 *   the active read has answered, the marker is derived from it — the state the
 *   switch writes — so after a confirmed switch the row moves immediately, while
 *   the list's invalidation is still in flight, and no second row keeps its old
 *   flag. The list fallback is used only while there is no active-read answer, so
 *   the two sources never produce a marker at the same time.
 * - **Two facts in one row.** The presented row carries `isActive` and not the
 *   contract's `is_active`, so a surface cannot read the list's flag while the
 *   active read says something else.
 *
 * Before the active read answers, the list's own at-most-one flag is used (a
 * first mount has nothing else), and that is reported as `activeMarker: 'list'`
 * rather than passed off as the active read's answer.
 *
 * IPC is mocked at `invoke`: `src/test/setup.ts` imports the committed route
 * tree, whose `/` route loads `src/features/households/entry.ts` and therefore
 * `./queries`, so a `vi.mock` of the household wrapper would register after that
 * module graph had already been built. The switch spec at the end is the
 * "between an answer and a refetch" case: the list's refetch is held open by a
 * deferred answer.
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
import { setActiveHousehold } from '../households/api';
import { householdKeys } from '../households/keys';
import type { Household, HouseholdSummary } from '../households/queries';
import { useSwitchActiveHousehold } from '../households/queries';
import { settingsHouseholds, useSettingsHouseholdList } from './households';
import { settingsKeys } from './keys';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

const HOUSEHOLD_A: Household = {
  id: 'household-a',
  name: 'Household A',
  created_at: AT,
  owner_member_id: 'member-a',
  is_active: true,
  is_owner: true,
};

const HOUSEHOLD_B: Household = {
  id: 'household-b',
  name: 'Household B',
  created_at: AT,
  owner_member_id: 'member-b',
  is_active: true,
  is_owner: false,
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

/** A promise whose settlement the calling test controls. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};

  const promise = new Promise<T>((settle) => {
    resolve = settle;
  });

  return { promise, resolve };
}

/** `[id, isActive]` in list order, for exact one-active assertions. */
function activeFlags(
  households: readonly { id: string; isActive: boolean }[],
): Array<[string, boolean]> {
  return households.map((household) => [household.id, household.isActive]);
}

describe('marking the active household', () => {
  const LIST = [summary(HOUSEHOLD_A, true), summary(HOUSEHOLD_B, false)];

  it('marks exactly the household the active fact names', () => {
    const rows = settingsHouseholds(LIST, HOUSEHOLD_B.id);

    expect(activeFlags(rows)).toEqual([
      [HOUSEHOLD_A.id, false],
      [HOUSEHOLD_B.id, true],
    ]);
    expect(rows.filter((row) => row.isActive)).toHaveLength(1);

    // One active fact per row: the contract's flag is replaced, not carried
    // alongside the derived one.
    expect(rows.every((row) => !('is_active' in row))).toBe(true);
  });

  it('marks none when the backend says none is active', () => {
    expect(activeFlags(settingsHouseholds(LIST, null))).toEqual([
      [HOUSEHOLD_A.id, false],
      [HOUSEHOLD_B.id, false],
    ]);
  });

  it('uses the list’s own flag only while the active read has no answer', () => {
    expect(activeFlags(settingsHouseholds(LIST, undefined))).toEqual([
      [HOUSEHOLD_A.id, true],
      [HOUSEHOLD_B.id, false],
    ]);
  });

  it('does not invent a row for an active household the list does not hold', () => {
    const rows = settingsHouseholds(LIST, 'household-c');

    expect(activeFlags(rows)).toEqual([
      [HOUSEHOLD_A.id, false],
      [HOUSEHOLD_B.id, false],
    ]);
    expect(rows).toHaveLength(LIST.length);
  });
});

describe('the Settings household list', () => {
  let answers: Map<string, Array<() => unknown>>;
  let invoked: string[];

  /** Queue the next answer for one command. */
  function queue(command: string, answer: () => unknown): void {
    const pending = answers.get(command) ?? [];

    pending.push(answer);
    answers.set(command, pending);
  }

  beforeEach(() => {
    answers = new Map();
    invoked = [];

    mockIPC((command) => {
      invoked.push(command);

      const answer = answers.get(command)?.shift();

      if (answer === undefined) {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return answer();
    });
  });

  afterEach(() => {
    clearMocks();
  });

  it('lets the active read win when the cached list flags disagree', async () => {
    queue('list_households', () => [
      summary(HOUSEHOLD_A, true),
      summary(HOUSEHOLD_B, false),
    ]);
    queue('get_active_household', () => HOUSEHOLD_B);

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, {
        client: createTestQueryClient(),
        children,
      });
    const { result } = renderHook(() => useSettingsHouseholdList(), {
      wrapper,
    });

    await waitFor(() => {
      expect(result.current.kind).toBe('ready');
    });

    if (result.current.kind !== 'ready') {
      throw new Error('the list did not become ready');
    }

    expect(activeFlags(result.current.households)).toEqual([
      [HOUSEHOLD_A.id, false],
      [HOUSEHOLD_B.id, true],
    ]);
    expect(result.current.activeHouseholdId).toBe(HOUSEHOLD_B.id);
    expect(result.current.activeMarker).toBe('active-read');

    // A read-only hook asks for the two household reads and nothing else.
    expect([...invoked].sort()).toEqual([
      'get_active_household',
      'list_households',
    ]);
    expect(invoked).not.toContain('set_active_household');
  });

  it('uses the list’s flag until the active read answers, then its answer', async () => {
    const active = deferred<Household | null>();

    queue('list_households', () => [
      summary(HOUSEHOLD_A, true),
      summary(HOUSEHOLD_B, false),
    ]);
    queue('get_active_household', () => active.promise);

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, {
        client: createTestQueryClient(),
        children,
      });
    const { result } = renderHook(() => useSettingsHouseholdList(), {
      wrapper,
    });

    await waitFor(() => {
      expect(result.current.kind).toBe('ready');
    });

    if (result.current.kind !== 'ready') {
      throw new Error('the list did not become ready');
    }

    expect(result.current.activeMarker).toBe('list');
    expect(activeFlags(result.current.households)).toEqual([
      [HOUSEHOLD_A.id, true],
      [HOUSEHOLD_B.id, false],
    ]);

    active.resolve(HOUSEHOLD_B);

    await waitFor(() => {
      expect(result.current.kind).toBe('ready');
      expect(
        result.current.kind === 'ready'
          ? result.current.activeMarker
          : 'pending',
      ).toBe('active-read');
    });

    if (result.current.kind !== 'ready') {
      throw new Error('the list stopped being ready');
    }

    expect(activeFlags(result.current.households)).toEqual([
      [HOUSEHOLD_A.id, false],
      [HOUSEHOLD_B.id, true],
    ]);
    expect(
      result.current.households.filter((row) => row.isActive),
    ).toHaveLength(1);
  });

  it('moves the mark with a confirmed switch before the list refetch lands', async () => {
    const client = createTestQueryClient();
    const refetch = deferred<HouseholdSummary[]>();

    queue('list_households', () => [
      summary(HOUSEHOLD_A, true),
      summary(HOUSEHOLD_B, false),
    ]);
    queue('get_active_household', () => HOUSEHOLD_A);

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(
      () => ({
        list: useSettingsHouseholdList(),
        switchTo: useSwitchActiveHousehold(),
      }),
      { wrapper },
    );

    await waitFor(() => {
      expect(result.current.list.kind).toBe('ready');
    });

    queue('set_active_household', () => HOUSEHOLD_B);

    // The list's post-switch refetch is held open, so the assertions below run
    // in exactly the window between the answer and the refetch.
    queue('list_households', () => refetch.promise);

    await act(async () => {
      result.current.switchTo.mutate(setActiveHousehold(HOUSEHOLD_B.id));
    });

    await waitFor(() => {
      expect(result.current.switchTo.isSuccess).toBe(true);
    });

    await waitFor(() => {
      expect(client.getQueryState(householdKeys.list())?.fetchStatus).toBe(
        'fetching',
      );
    });

    if (result.current.list.kind !== 'ready') {
      throw new Error('the list stopped being ready after the switch');
    }

    expect(activeFlags(result.current.list.households)).toEqual([
      [HOUSEHOLD_A.id, false],
      [HOUSEHOLD_B.id, true],
    ]);
    expect(result.current.list.activeHouseholdId).toBe(HOUSEHOLD_B.id);
    expect(
      result.current.list.households.filter((row) => row.isActive),
    ).toHaveLength(1);
    expect(
      invoked.filter((command) => command === 'set_active_household'),
    ).toHaveLength(1);

    refetch.resolve([summary(HOUSEHOLD_A, false), summary(HOUSEHOLD_B, true)]);

    await waitFor(() => {
      expect(client.getQueryState(householdKeys.list())?.fetchStatus).toBe(
        'idle',
      );
    });

    if (result.current.list.kind !== 'ready') {
      throw new Error('the list stopped being ready after the refetch');
    }

    expect(activeFlags(result.current.list.households)).toEqual([
      [HOUSEHOLD_A.id, false],
      [HOUSEHOLD_B.id, true],
    ]);
  });

  it('reports a failed list read instead of an empty held set', async () => {
    queue('list_households', () => {
      throw {
        cause: 'unknown',
        retryable: false,
        message_key: 'error.unknown',
        request_id: null,
      };
    });
    queue('get_active_household', () => null);

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, {
        client: createTestQueryClient(),
        children,
      });
    const { result } = renderHook(() => useSettingsHouseholdList(), {
      wrapper,
    });

    await waitFor(() => {
      expect(result.current.kind).toBe('failed');
    });

    if (result.current.kind === 'failed') {
      expect(result.current.error).toMatchObject({ cause: 'unknown' });
    }
  });

  it('leaves the display unit alone when the household list changes', async () => {
    const client = createTestQueryClient();

    client.setQueryData(settingsKeys.displayWeightUnit(), 'kilograms');

    queue('list_households', () => [
      summary(HOUSEHOLD_A, true),
      summary(HOUSEHOLD_B, false),
    ]);
    queue('get_active_household', () => HOUSEHOLD_A);

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useSettingsHouseholdList(), {
      wrapper,
    });

    await waitFor(() => {
      expect(result.current.kind).toBe('ready');
    });

    queue('list_households', () => [
      summary(HOUSEHOLD_A, false),
      summary(HOUSEHOLD_B, true),
    ]);
    queue('get_active_household', () => HOUSEHOLD_B);

    await act(async () => {
      await client.invalidateQueries({ queryKey: householdKeys.list() });
      await client.invalidateQueries({ queryKey: householdKeys.active() });
    });

    // A household change moves the list/active pair and nothing else: the unit
    // is per device and is not part of a household collection (D0.2, R7).
    expect(client.getQueryData(settingsKeys.displayWeightUnit())).toBe(
      'kilograms',
    );
    expect(
      client.getQueryState(settingsKeys.displayWeightUnit())?.isInvalidated,
    ).toBe(false);
  });
});
