/*
 * Task 3.4's evidence for the two properties the rabbit writes own: each
 * mutation invalidates exactly the queries its confirmed answer supersedes
 * (RAB-1, RAB-5, RAB-6, RAB-7, RAB-10), and a failed write invalidates nothing
 * while reaching P0's one failure surface (D1, SYNC-2).
 *
 * IPC is mocked at `invoke`, the seam `testing-strategy` prescribes for this
 * layer and the one `households/create.test.ts` uses: the real wrappers and
 * `typedError` run, so the `request_id` the retry re-sends is the one the
 * wrapper minted rather than one the spec wrote.
 *
 * **The cache is the default assertion.** Each spec seeds every key a rabbit
 * write could plausibly reach — both list modes of two households, two weight
 * series, two dashboards — and reads back which of them the cache marked
 * `isInvalidated`. A key absent from the result was not touched, which is the
 * half of "only the affected queries" that a spy on the invalidation call could
 * not show.
 *
 * **One exception, for the key the cache cannot show.** `create` seeds no
 * profile, because the id is minted by the command and no profile of it can
 * already be cached; an invalidation of that key would therefore flip nothing
 * and would go unnoticed. That one spec also reads the keys the mutation names
 * from a spy, so re-adding the detail call fails it. Every other spec asserts
 * from the cache alone.
 */
import type { QueryClient } from '@tanstack/react-query';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  createTestQueryClient,
  TestQueryClientProvider,
} from '../../test/setup';
import { dashboardKeys } from '../dashboard/keys';
import {
  getSaveRetryState,
  resetSaveRetry,
  retryFailedSave,
} from '../sync/save-retry';
import { weightKeys } from '../weight/keys';
import {
  archiveRabbit,
  createRabbit,
  deleteRabbit,
  restoreRabbit,
  updateRabbit,
} from './api';
import { rabbitKeys } from './keys';
import {
  useArchiveRabbit,
  useCreateRabbit,
  useDeleteRabbit,
  useRestoreRabbit,
  useUpdateRabbit,
} from './mutations';
import type { Rabbit } from './queries';

/** The active household every fixture belongs to. */
const HOUSEHOLD = 'household-a';

/** A household whose keys no rabbit write may touch (NFR-10). */
const OTHER_HOUSEHOLD = 'household-b';

/** The rabbit a spec creates, updates, archives, restores or deletes. */
const RABBIT_ID = 'rabbit-a';

/** A second rabbit, whose keys must survive every one of those writes. */
const OTHER_RABBIT_ID = 'rabbit-b';

/** A row already in the archived list, so both modes are cached. */
const ARCHIVED_ID = 'rabbit-archived';

/** One rabbit, with the contract's absent optionals as `null`. */
function rabbit(overrides: Partial<Rabbit> = {}): Rabbit {
  return {
    id: RABBIT_ID,
    household_id: HOUSEHOLD,
    name: 'Pip',
    photo_ref: null,
    breed: null,
    dob: null,
    dob_unknown: false,
    sex: 'female',
    neuter_status: 'spayed',
    adoption_date: null,
    is_archived: false,
    weight_drop_threshold_percent: 5,
    ...overrides,
  };
}

/** `RabbitProfile` derived from the wrapper that takes it (hard rule 2). */
type RabbitProfile = Parameters<typeof createRabbit>[1];

/** The whole editable set, as a form would submit it. */
const PROFILE: RabbitProfile = {
  name: 'Pip',
  photo_ref: null,
  breed: null,
  dob: null,
  dob_unknown: false,
  sex: 'female',
  neuter_status: 'spayed',
  adoption_date: null,
  weight_drop_threshold_percent: 5,
};

/** One row of a rabbit list (DASH-1, RAB-6). */
function summary(id: string, isArchived: boolean) {
  return {
    id,
    name: id,
    thumbnail_ref: null,
    is_archived: isArchived,
  };
}

/** One cached weight window, of the shape `weight_keys::range` names. */
function trend(range: 'days_30' | 'all') {
  return { range, points: [], drop_threshold_percent: 5 };
}

/**
 * Every key a rabbit write could plausibly reach, named as the spec reports
 * them. `seededKeys` and `seed` are two views of the same list, kept together
 * so a key cannot be seeded without being asserted.
 */
function seededKeys(): Array<{ name: string; key: readonly unknown[] }> {
  return [
    { name: 'active list', key: rabbitKeys.list(HOUSEHOLD, 'active') },
    { name: 'archived list', key: rabbitKeys.list(HOUSEHOLD, 'archived') },
    { name: 'profile', key: rabbitKeys.detail(RABBIT_ID) },
    {
      name: 'weight series (30 days)',
      key: weightKeys.range(RABBIT_ID, 'days_30'),
    },
    { name: 'weight series (all)', key: weightKeys.range(RABBIT_ID, 'all') },
    { name: 'dashboard rows', key: dashboardKeys.rows(HOUSEHOLD) },
    {
      name: 'another household’s active list',
      key: rabbitKeys.list(OTHER_HOUSEHOLD, 'active'),
    },
    {
      name: 'another household’s archived list',
      key: rabbitKeys.list(OTHER_HOUSEHOLD, 'archived'),
    },
    {
      name: 'another rabbit’s profile',
      key: rabbitKeys.detail(OTHER_RABBIT_ID),
    },
    {
      name: 'another rabbit’s weight series',
      key: weightKeys.range(OTHER_RABBIT_ID, 'all'),
    },
    {
      name: 'another household’s dashboard rows',
      key: dashboardKeys.rows(OTHER_HOUSEHOLD),
    },
  ];
}

/**
 * Cache a value under every key above, so "not invalidated" is observable.
 *
 * `profile: false` leaves the rabbit's own profile unseeded, which is the only
 * state a create can be in: its id is minted by the command and learned from
 * the confirming answer, so no profile of it can already be cached. The other
 * keys stay seeded in that spec so "create touches nothing else" is a real
 * assertion rather than a consequence of an empty cache.
 */
function seed(client: QueryClient, options: { profile?: boolean } = {}): void {
  client.setQueryData(rabbitKeys.list(HOUSEHOLD, 'active'), [
    summary(RABBIT_ID, false),
  ]);
  client.setQueryData(rabbitKeys.list(HOUSEHOLD, 'archived'), [
    summary(ARCHIVED_ID, true),
  ]);
  client.setQueryData(rabbitKeys.list(OTHER_HOUSEHOLD, 'active'), [
    summary('rabbit-other', false),
  ]);
  client.setQueryData(rabbitKeys.list(OTHER_HOUSEHOLD, 'archived'), [
    summary('rabbit-other-archived', true),
  ]);

  if (options.profile !== false) {
    client.setQueryData(rabbitKeys.detail(RABBIT_ID), rabbit());
  }

  client.setQueryData(
    rabbitKeys.detail(OTHER_RABBIT_ID),
    rabbit({ id: OTHER_RABBIT_ID }),
  );
  client.setQueryData(weightKeys.range(RABBIT_ID, 'days_30'), trend('days_30'));
  client.setQueryData(weightKeys.range(RABBIT_ID, 'all'), trend('all'));
  client.setQueryData(weightKeys.range(OTHER_RABBIT_ID, 'all'), trend('all'));
  client.setQueryData(dashboardKeys.rows(HOUSEHOLD), []);
  client.setQueryData(dashboardKeys.rows(OTHER_HOUSEHOLD), []);
}

/** The names of the seeded keys the cache has marked invalidated. */
function invalidated(client: QueryClient): string[] {
  return seededKeys()
    .filter(({ key }) => client.getQueryState(key)?.isInvalidated === true)
    .map(({ name }) => name);
}

/** A `CommandError` the write path really reports (D1.1). */
function connectivity(requestId: string) {
  return {
    cause: 'connectivity',
    retryable: true,
    message_key: 'error.connectivity',
    request_id: requestId,
  };
}

/** The validation refusal, which belongs to the form field rather than the shell. */
function validation(requestId: string) {
  return {
    cause: 'validation',
    retryable: false,
    message_key: 'error.validation',
    request_id: requestId,
  };
}

function wrapperFor(client: QueryClient) {
  return ({ children }: { children: ReactNode }) =>
    createElement(TestQueryClientProvider, { client, children });
}

describe('the rabbit mutations', () => {
  let answers: Array<() => unknown>;
  let calls: Array<{ command: string; args: unknown }>;

  beforeEach(() => {
    resetSaveRetry();
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

  it('create invalidates the active row and the dashboard row, and nothing else', async () => {
    const client = createTestQueryClient();
    const operation = createRabbit(HOUSEHOLD, PROFILE);

    /*
     * No profile is seeded: the id only exists once the command answers, so a
     * cached profile for it is a state the app cannot reach. That is also why
     * this spec asserts the keys the mutation names rather than only the ones
     * the cache flipped — an invalidation of `detail(newId)` would flip no
     * cached entry, so the cache alone could not catch it coming back.
     */
    seed(client, { profile: false });
    answers.push(() => rabbit());

    const invalidation = vi.spyOn(client, 'invalidateQueries');
    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateRabbit(), { wrapper });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(calls).toEqual([
      {
        command: 'create_rabbit',
        args: {
          householdId: HOUSEHOLD,
          profile: PROFILE,
          requestId: operation.requestId,
        },
      },
    ]);
    expect(
      invalidation.mock.calls.map(([filters]) => filters?.queryKey),
    ).toEqual([
      rabbitKeys.list(HOUSEHOLD, 'active'),
      dashboardKeys.rows(HOUSEHOLD),
    ]);
    expect(invalidated(client)).toEqual(['active list', 'dashboard rows']);
  });

  it('update invalidates the row, the profile, both weight windows and the dashboard row', async () => {
    const client = createTestQueryClient();
    const operation = updateRabbit(RABBIT_ID, { ...PROFILE, name: 'Pip II' });

    seed(client);
    answers.push(() => rabbit({ name: 'Pip II' }));

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useUpdateRabbit(), { wrapper });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(invalidated(client)).toEqual([
      'active list',
      'profile',
      'weight series (30 days)',
      'weight series (all)',
      'dashboard rows',
    ]);
  });

  it('archive invalidates both list modes, the profile and the dashboard rows', async () => {
    const client = createTestQueryClient();
    const operation = archiveRabbit(RABBIT_ID);

    seed(client);
    answers.push(() => rabbit({ is_archived: true }));

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useArchiveRabbit(), { wrapper });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(invalidated(client)).toEqual([
      'active list',
      'archived list',
      'profile',
      'dashboard rows',
    ]);
  });

  it('restore invalidates both list modes, the profile and the dashboard rows', async () => {
    const client = createTestQueryClient();
    const operation = restoreRabbit(RABBIT_ID);

    seed(client);
    answers.push(() => rabbit());

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useRestoreRabbit(), { wrapper });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(invalidated(client)).toEqual([
      'active list',
      'archived list',
      'profile',
      'dashboard rows',
    ]);
  });

  it('delete invalidates the archived list, the profile and every weight window', async () => {
    const client = createTestQueryClient();
    const operation = deleteRabbit(RABBIT_ID);

    seed(client);
    answers.push(() => null);

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useDeleteRabbit(), { wrapper });

    await act(async () => {
      result.current.mutate({
        householdId: HOUSEHOLD,
        rabbitId: RABBIT_ID,
        operation,
      });
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(calls).toEqual([
      {
        command: 'delete_rabbit',
        args: { rabbitId: RABBIT_ID, requestId: operation.requestId },
      },
    ]);
    expect(invalidated(client)).toEqual([
      'archived list',
      'profile',
      'weight series (30 days)',
      'weight series (all)',
    ]);
  });

  it('records a failed write instead of invalidating anything', async () => {
    const client = createTestQueryClient();
    const operation = createRabbit(HOUSEHOLD, PROFILE);

    seed(client);
    answers.push(() => {
      throw connectivity(operation.requestId);
    });

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateRabbit(), { wrapper });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    // Nothing was written and nothing was invalidated (D1, no optimistic write).
    expect(invalidated(client)).toEqual([]);

    const failure = getSaveRetryState().failure;

    expect(failure?.cause).toBe('connectivity');
    expect(failure?.messageKey).toBe('error.connectivity');
    expect(failure?.retryable).toBe(true);
    expect(failure?.operation.requestId).toBe(operation.requestId);
  });

  it('re-issues a recorded failure with the same request id and invalidates on the retry', async () => {
    const client = createTestQueryClient();
    const operation = archiveRabbit(RABBIT_ID);

    seed(client);
    answers.push(() => {
      throw connectivity(operation.requestId);
    });

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useArchiveRabbit(), { wrapper });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(getSaveRetryState().failure?.operation.requestId).toBe(
      operation.requestId,
    );

    answers.push(() => rabbit({ is_archived: true }));

    await act(async () => {
      await retryFailedSave();
    });

    expect(
      calls.map((call) => (call.args as { requestId?: string }).requestId),
    ).toEqual([operation.requestId, operation.requestId]);
    expect(calls.map((call) => call.command)).toEqual([
      'archive_rabbit',
      'archive_rabbit',
    ]);

    // The retry went through the mutation, so the confirmed answer invalidated
    // exactly what a first-attempt success would have (D0.14 + task 3.4).
    expect(getSaveRetryState().failure).toBeNull();
    expect(invalidated(client)).toEqual([
      'active list',
      'archived list',
      'profile',
      'dashboard rows',
    ]);
  });

  it('keeps a retried validation refusal in the mutation’s error', async () => {
    const client = createTestQueryClient();
    const operation = createRabbit(HOUSEHOLD, PROFILE);

    seed(client);
    answers.push(() => {
      throw connectivity(operation.requestId);
    });

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateRabbit(), { wrapper });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    answers.push(() => {
      throw validation(operation.requestId);
    });

    await act(async () => {
      await retryFailedSave();
    });

    /*
     * `settleRetryFailure` clears the slot for a `validation` refusal because
     * the recorder's form shows it; routing the retry through `mutateAsync` is
     * what puts it in that form's error state rather than nowhere.
     */
    expect(getSaveRetryState().failure).toBeNull();
    expect(calls.map((call) => call.command)).toEqual([
      'create_rabbit',
      'create_rabbit',
    ]);
    await waitFor(() => {
      expect(result.current.error).toMatchObject({ cause: 'validation' });
    });
    expect(invalidated(client)).toEqual([]);
  });

  it('leaves a validation refusal to the mutation’s own error', async () => {
    const client = createTestQueryClient();
    const operation = createRabbit(HOUSEHOLD, PROFILE);

    seed(client);
    answers.push(() => {
      throw validation(operation.requestId);
    });

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateRabbit(), { wrapper });

    await act(async () => {
      result.current.mutate(operation);
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    // The field owns this refusal, so it never enters the shell's record and
    // the cache is untouched (`SaveFailureNotice`'s rule, `settleRetryFailure`).
    expect(getSaveRetryState().failure).toBeNull();
    expect(result.current.error).toMatchObject({ cause: 'validation' });
    expect(invalidated(client)).toEqual([]);
  });
});
