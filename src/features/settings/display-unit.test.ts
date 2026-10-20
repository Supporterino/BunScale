/*
 * SET-4 at the frontend layer (task 3.5).
 *
 * Five properties are asserted, each one a thing a later Settings screen would
 * otherwise have to be trusted about:
 *
 * - the read surfaces the **backend's effective unit**; the frontend computes
 *   no regional default (it cannot — Q26 records that iOS reaches no locale
 *   source, and the rule lives in Rust, tested by task 5.11);
 * - the write stores the unit the backend **read back**, not the argument it
 *   was given, so a write that did not land cannot appear selected;
 * - a read that started before the write cannot overwrite the confirmed unit
 *   when it answers afterwards;
 * - a retry re-submits the **handle**, so both attempts carry one
 *   `request_id` (D0.14);
 * - a unit change touches the unit read and **no household read** (D0.2, R7),
 *   and a failed change writes nothing.
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
import { householdKeys } from '../households/keys';
import { setDisplayWeightUnit } from './api';
import type { WeightUnit } from './display-unit';
import {
  displayWeightUnitQueryOptions,
  useDisplayWeightUnit,
  useSetDisplayWeightUnit,
} from './display-unit';
import { settingsKeys } from './keys';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

/** A promise whose settlement the calling test controls. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};

  const promise = new Promise<T>((settle) => {
    resolve = settle;
  });

  return { promise, resolve };
}

describe('the display-unit preference', () => {
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

  it('surfaces the effective unit the backend resolved', async () => {
    answers.push(() => 'kilograms');

    const client = createTestQueryClient();
    const unit = await client.fetchQuery(displayWeightUnitQueryOptions());

    expect(unit).toBe('kilograms');
    expect(calls.map((call) => call.command)).toEqual([
      'get_display_weight_unit',
    ]);
  });

  it('writes the backend’s read-back unit, not the argument', async () => {
    const client = createTestQueryClient();
    const operation = setDisplayWeightUnit('grams');

    // The backend stored the choice and read the effective unit back; here the
    // two differ, which is the case a naive "echo the argument" would hide.
    answers.push(() => 'kilograms');

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useSetDisplayWeightUnit(), {
      wrapper,
    });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toBe('kilograms');
    expect(
      client.getQueryData<WeightUnit>(settingsKeys.displayWeightUnit()),
    ).toBe('kilograms');
    expect(calls).toEqual([
      {
        command: 'set_display_weight_unit',
        args: { unit: 'grams', requestId: operation.requestId },
      },
    ]);
  });

  it('keeps the confirmed unit when an older read resolves after the change', async () => {
    const client = createTestQueryClient();
    const operation = setDisplayWeightUnit('kilograms');
    const read = deferred<WeightUnit>();

    answers.push(
      () => read.promise,
      () => 'kilograms',
    );

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(
      () => ({
        unit: useDisplayWeightUnit(),
        setUnit: useSetDisplayWeightUnit(),
      }),
      { wrapper },
    );

    // The read is in flight when the write lands, so it predates the change.
    await waitFor(() => {
      expect(
        client.getQueryState(settingsKeys.displayWeightUnit())?.fetchStatus,
      ).toBe('fetching');
    });

    await act(async () => {
      result.current.setUnit.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.setUnit.isSuccess).toBe(true);
    });

    expect(
      client.getQueryData<WeightUnit>(settingsKeys.displayWeightUnit()),
    ).toBe('kilograms');

    // The older read now answers with the pre-change unit. It must not
    // overwrite the unit the setter read back from the device.
    await act(async () => {
      read.resolve('grams');
      await Promise.resolve();
    });

    await waitFor(() => {
      expect(
        client.getQueryState(settingsKeys.displayWeightUnit())?.fetchStatus,
      ).toBe('idle');
    });

    expect(
      client.getQueryData<WeightUnit>(settingsKeys.displayWeightUnit()),
    ).toBe('kilograms');
    expect(result.current.unit.data).toBe('kilograms');
  });

  it('retries with the same operation identity and then stores the answer', async () => {
    const client = createTestQueryClient();
    const operation = setDisplayWeightUnit('grams');

    answers.push(
      () => {
        throw {
          cause: 'unknown',
          retryable: false,
          message_key: 'error.unknown',
          request_id: operation.requestId,
        };
      },
      () => 'kilograms',
    );

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useSetDisplayWeightUnit(), {
      wrapper,
    });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    // A failed change writes nothing: there is no confirmed unit to show.
    expect(
      client.getQueryData(settingsKeys.displayWeightUnit()),
    ).toBeUndefined();

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
      'set_display_weight_unit',
      'set_display_weight_unit',
    ]);
    expect(
      client.getQueryData<WeightUnit>(settingsKeys.displayWeightUnit()),
    ).toBe('kilograms');
  });

  it('leaves every household read untouched', async () => {
    const client = createTestQueryClient();
    const operation = setDisplayWeightUnit('kilograms');
    const list = [
      {
        id: 'household-a',
        name: 'Household A',
        created_at: AT,
        is_active: true,
        is_owner: true,
        member_count: 1,
      },
    ];
    const active = {
      id: 'household-a',
      name: 'Household A',
      created_at: AT,
      owner_member_id: 'member-1',
      is_active: true,
      is_owner: true,
    };

    client.setQueryData(householdKeys.list(), list);
    client.setQueryData(householdKeys.active(), active);

    answers.push(() => 'kilograms');

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useSetDisplayWeightUnit(), {
      wrapper,
    });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    // D0.2/R7: the unit is per device and never household-scoped, so neither
    // household entry is rewritten or marked stale.
    expect(client.getQueryData(householdKeys.list())).toEqual(list);
    expect(client.getQueryData(householdKeys.active())).toEqual(active);
    expect(client.getQueryState(householdKeys.list())?.isInvalidated).toBe(
      false,
    );
    expect(client.getQueryState(householdKeys.active())?.isInvalidated).toBe(
      false,
    );
  });

  it('keeps the last confirmed unit when a change fails', async () => {
    const client = createTestQueryClient();

    client.setQueryData<WeightUnit>(settingsKeys.displayWeightUnit(), 'grams');

    answers.push(() => {
      throw {
        cause: 'unknown',
        retryable: false,
        message_key: 'error.unknown',
        request_id: 'operation-1',
      };
    });

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useSetDisplayWeightUnit(), {
      wrapper,
    });

    await act(async () => {
      result.current.mutate(setDisplayWeightUnit('kilograms'));
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(
      client.getQueryData<WeightUnit>(settingsKeys.displayWeightUnit()),
    ).toBe('grams');
  });

  it('reports a failed read with its cause rather than a default unit', async () => {
    answers.push(() => {
      throw {
        cause: 'unknown',
        retryable: false,
        message_key: 'error.unknown',
        request_id: null,
      };
    });

    const client = createTestQueryClient();
    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useDisplayWeightUnit(), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    // There is no unit to display: a default here would be the frontend
    // re-deriving SET-4's rule that the backend owns.
    expect(result.current.data).toBeUndefined();
    expect(result.current.error).toMatchObject({ cause: 'unknown' });
  });
});
