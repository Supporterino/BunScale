/*
 * Task 3.5's evidence for the two properties the weight writes own: the typed
 * value crosses the display-unit boundary exactly once (WT-3, DM-1), and a
 * confirmed write invalidates exactly the reads it superseded (WT-1, WT-5).
 *
 * IPC is mocked at `invoke`, the same seam task 3.4's rabbit-write spec uses:
 * the real wrappers and `typedError` run, so the `request_id` a retry re-sends
 * is the one the wrapper minted rather than one the spec wrote, and the grams
 * this spec reads are the ones the command would receive.
 *
 * **The cache is the default assertion.** Every spec seeds the keys a weight
 * write could plausibly reach — four windows of the rabbit's own series, two
 * dashboards, another rabbit's series, the rabbit's profile and its active-list
 * row — and reads back which of them the cache marked `isInvalidated`. A key
 * absent from the result was not touched, which is the half of "only the
 * affected queries" that a spy on the invalidation call could not show.
 *
 * There is no weight-entry detail key in `./keys.ts` and no query reads
 * `get_weight_entry`, so this spec seeds none: naming a key the app never
 * writes would assert a cache entry that cannot exist.
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { CommandData } from '../../lib/command-result';
import {
  createTestQueryClient,
  TestQueryClientProvider,
} from '../../test/setup';
import { dashboardKeys } from '../dashboard/keys';
import { rabbitKeys } from '../rabbits/keys';
import {
  getSaveRetryState,
  resetSaveRetry,
  retryFailedSave,
} from '../sync/save-retry';
import type { getWeightEntry } from './api';
import { weightKeys } from './keys';
import type { UpdateWeightEntryVariables } from './mutations';
import {
  useCreateWeightEntry,
  useDeleteWeightEntry,
  useUpdateWeightEntry,
} from './mutations';

/** The active household every fixture belongs to. */
const HOUSEHOLD = 'household-a';

/** A household whose keys no weight write may touch (NFR-10). */
const OTHER_HOUSEHOLD = 'household-b';

/** The rabbit whose series the write changes. */
const RABBIT_ID = 'rabbit-a';

/** A second rabbit, whose series must survive every write. */
const OTHER_RABBIT_ID = 'rabbit-b';

/** The entry a spec corrects or removes. */
const ENTRY_ID = 'entry-a';

/** `WeightEntry` derived from the wrapper that returns it (hard rule 2). */
type WeightEntry = CommandData<Awaited<ReturnType<typeof getWeightEntry>>>;

/** One saved entry, with the contract's absent optionals as `null`. */
function entry(overrides: Partial<WeightEntry> = {}): WeightEntry {
  return {
    id: ENTRY_ID,
    rabbit_id: RABBIT_ID,
    value_grams: 1200,
    timestamp: '2026-10-01T09:00:00Z',
    note: null,
    photo_refs: [],
    has_weight_drop_alert: false,
    ...overrides,
  };
}

/** One cached weight window, of the shape `weight_keys::range` names. */
function trend(range: 'days_7' | 'days_30' | 'days_90' | 'all') {
  return { range, points: [], drop_threshold_percent: 5 };
}

/**
 * Every key a weight write could plausibly reach, named as the spec reports
 * them. `seededKeys` and `seed` are two views of the same list, kept together
 * so a key cannot be seeded without being asserted.
 */
function seededKeys(): Array<{ name: string; key: readonly unknown[] }> {
  return [
    {
      name: 'weight series (7 days)',
      key: weightKeys.range(RABBIT_ID, 'days_7'),
    },
    {
      name: 'weight series (30 days)',
      key: weightKeys.range(RABBIT_ID, 'days_30'),
    },
    {
      name: 'weight series (90 days)',
      key: weightKeys.range(RABBIT_ID, 'days_90'),
    },
    { name: 'weight series (all)', key: weightKeys.range(RABBIT_ID, 'all') },
    {
      name: 'another rabbit’s weight series',
      key: weightKeys.range(OTHER_RABBIT_ID, 'all'),
    },
    { name: 'dashboard rows', key: dashboardKeys.rows(HOUSEHOLD) },
    {
      name: 'another household’s dashboard rows',
      key: dashboardKeys.rows(OTHER_HOUSEHOLD),
    },
    { name: 'profile', key: rabbitKeys.detail(RABBIT_ID) },
    { name: 'active list', key: rabbitKeys.list(HOUSEHOLD, 'active') },
  ];
}

/** Cache a value under every key above, so "not invalidated" is observable. */
function seed(client: ReturnType<typeof createTestQueryClient>): void {
  client.setQueryData(weightKeys.range(RABBIT_ID, 'days_7'), trend('days_7'));
  client.setQueryData(weightKeys.range(RABBIT_ID, 'days_30'), trend('days_30'));
  client.setQueryData(weightKeys.range(RABBIT_ID, 'days_90'), trend('days_90'));
  client.setQueryData(weightKeys.range(RABBIT_ID, 'all'), trend('all'));
  client.setQueryData(weightKeys.range(OTHER_RABBIT_ID, 'all'), trend('all'));
  client.setQueryData(dashboardKeys.rows(HOUSEHOLD), []);
  client.setQueryData(dashboardKeys.rows(OTHER_HOUSEHOLD), []);
  client.setQueryData(rabbitKeys.detail(RABBIT_ID), { id: RABBIT_ID });
  client.setQueryData(rabbitKeys.list(HOUSEHOLD, 'active'), []);
}

/** The names of the seeded keys the cache has marked invalidated. */
function invalidated(
  client: ReturnType<typeof createTestQueryClient>,
): string[] {
  return seededKeys()
    .filter(({ key }) => client.getQueryState(key)?.isInvalidated === true)
    .map(({ name }) => name);
}

/** The set every weight write supersedes: one rabbit's windows, one dashboard. */
const AFFECTED = [
  'weight series (7 days)',
  'weight series (30 days)',
  'weight series (90 days)',
  'weight series (all)',
  'dashboard rows',
];

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

function wrapperFor(client: ReturnType<typeof createTestQueryClient>) {
  return ({ children }: { children: ReactNode }) =>
    createElement(TestQueryClientProvider, { client, children });
}

describe('the weight mutations', () => {
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

  it('create sends the typed kilograms as grams, with no unit, and invalidates the series and the dashboard rows', async () => {
    const client = createTestQueryClient();
    seed(client);
    answers.push(() => entry());

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateWeightEntry(), { wrapper });

    await act(async () => {
      result.current.mutate({
        householdId: HOUSEHOLD,
        rabbitId: RABBIT_ID,
        input: {
          text: '1.2',
          unit: 'kilograms',
          timestamp: null,
          note: null,
          outlierConfirmed: false,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    /*
     * The entry is the whole payload assertion: `value_grams` is the parse's
     * exact integer, and the absence of any unit key is the DM-1 half — a unit
     * cannot be stored because it is never sent.
     */
    expect(calls).toEqual([
      {
        command: 'create_weight_entry',
        args: {
          rabbitId: RABBIT_ID,
          entry: {
            value_grams: 1200,
            timestamp: null,
            note: null,
            outlier_confirmed: false,
          },
          requestId: expect.any(String),
        },
      },
    ]);
    expect(invalidated(client)).toEqual(AFFECTED);
  });

  it('create carries a confirmed outlier to the draft as outlier_confirmed', async () => {
    const client = createTestQueryClient();
    seed(client);
    answers.push(() => entry({ value_grams: 30_000 }));

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateWeightEntry(), { wrapper });

    await act(async () => {
      result.current.mutate({
        householdId: HOUSEHOLD,
        rabbitId: RABBIT_ID,
        input: {
          text: '30',
          unit: 'kilograms',
          timestamp: null,
          note: null,
          outlierConfirmed: true,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    /*
     * WT-4's "A confirmed outlier is saved", end to end: the form's flag is
     * copied to the wire field and the whole payload is asserted, so a dropped
     * or inverted flag cannot pass on the strength of the other fields.
     */
    expect(calls).toEqual([
      {
        command: 'create_weight_entry',
        args: {
          rabbitId: RABBIT_ID,
          entry: {
            value_grams: 30_000,
            timestamp: null,
            note: null,
            outlier_confirmed: true,
          },
          requestId: expect.any(String),
        },
      },
    ]);
  });

  it('create records the instant the person chose, not the moment of the save', async () => {
    const client = createTestQueryClient();
    seed(client);

    const chosen = '2026-09-01T08:00:00Z';

    answers.push(() => entry({ timestamp: chosen }));

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateWeightEntry(), { wrapper });

    await act(async () => {
      result.current.mutate({
        householdId: HOUSEHOLD,
        rabbitId: RABBIT_ID,
        input: {
          text: '1200',
          unit: 'grams',
          timestamp: chosen,
          note: null,
          outlierConfirmed: false,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    /*
     * The whole payload, not the field: a `draftOf` that dropped a supplied
     * instant — recording "now" instead of the back-dated moment the person
     * chose — fails here where the `timestamp: null` cases cannot see it (WT-1).
     */
    expect(calls).toEqual([
      {
        command: 'create_weight_entry',
        args: {
          rabbitId: RABBIT_ID,
          entry: {
            value_grams: 1200,
            timestamp: chosen,
            note: null,
            outlier_confirmed: false,
          },
          requestId: expect.any(String),
        },
      },
    ]);
  });

  it('update moves the timestamp when the draft names one', async () => {
    const client = createTestQueryClient();
    seed(client);

    const moved = '2026-08-15T07:30:00Z';

    answers.push(() => entry({ timestamp: moved }));

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useUpdateWeightEntry(), { wrapper });

    await act(async () => {
      result.current.mutate({
        householdId: HOUSEHOLD,
        rabbitId: RABBIT_ID,
        entryId: ENTRY_ID,
        input: {
          text: '1.2',
          unit: 'kilograms',
          timestamp: moved,
          note: null,
          outlierConfirmed: false,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    // An explicit instant moves the entry; the whole payload is asserted (WT-1, WT-5).
    expect(calls).toEqual([
      {
        command: 'update_weight_entry',
        args: {
          entryId: ENTRY_ID,
          entry: {
            value_grams: 1200,
            timestamp: moved,
            note: null,
            outlier_confirmed: false,
          },
          requestId: expect.any(String),
        },
      },
    ]);
  });

  it('update keeps the timestamp when the draft has none and invalidates the same set', async () => {
    const client = createTestQueryClient();
    seed(client);
    answers.push(() => entry({ value_grams: 520 }));

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useUpdateWeightEntry(), { wrapper });

    await act(async () => {
      result.current.mutate({
        householdId: HOUSEHOLD,
        rabbitId: RABBIT_ID,
        entryId: ENTRY_ID,
        input: {
          text: '520',
          unit: 'grams',
          timestamp: null,
          note: 'after the vet',
          outlierConfirmed: false,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    // `timestamp: null` is "keep the entry's instant" on update (WT-5, I1).
    expect(calls).toEqual([
      {
        command: 'update_weight_entry',
        args: {
          entryId: ENTRY_ID,
          entry: {
            value_grams: 520,
            timestamp: null,
            note: 'after the vet',
            outlier_confirmed: false,
          },
          requestId: expect.any(String),
        },
      },
    ]);
    expect(invalidated(client)).toEqual(AFFECTED);
  });

  it('delete invalidates the same set from the id its answer cannot carry', async () => {
    const client = createTestQueryClient();
    seed(client);
    answers.push(() => null);

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useDeleteWeightEntry(), { wrapper });

    await act(async () => {
      result.current.mutate({
        householdId: HOUSEHOLD,
        rabbitId: RABBIT_ID,
        entryId: ENTRY_ID,
      });
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(calls).toEqual([
      {
        command: 'delete_weight_entry',
        args: { entryId: ENTRY_ID, requestId: expect.any(String) },
      },
    ]);
    expect(invalidated(client)).toEqual(AFFECTED);
  });

  it('refuses unparseable text before the command and leaves it in the mutation’s error', async () => {
    const client = createTestQueryClient();
    seed(client);

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateWeightEntry(), { wrapper });

    await act(async () => {
      result.current.mutate({
        householdId: HOUSEHOLD,
        rabbitId: RABBIT_ID,
        input: {
          // Sub-gram precision: 1234.5 g, which grams cannot carry (WT-3).
          text: '1.2345',
          unit: 'kilograms',
          timestamp: null,
          note: null,
          outlierConfirmed: false,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    // No command was reached, nothing was invalidated, nothing was recorded.
    expect(calls).toEqual([]);
    expect(invalidated(client)).toEqual([]);
    expect(getSaveRetryState().failure).toBeNull();
    expect(result.current.error).toMatchObject({
      cause: 'validation',
      retryable: false,
      request_id: null,
    });
  });

  it('records a failed write instead of invalidating anything, and the retry re-sends the same id', async () => {
    const client = createTestQueryClient();
    seed(client);

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useUpdateWeightEntry(), { wrapper });

    const variables: UpdateWeightEntryVariables = {
      householdId: HOUSEHOLD,
      rabbitId: RABBIT_ID,
      entryId: ENTRY_ID,
      input: {
        text: '1.25',
        unit: 'kilograms',
        timestamp: null,
        note: null,
        outlierConfirmed: false,
      },
    };

    /*
     * The failure needs the id the wrapper minted, which the spec reads back from
     * the recorded call rather than reproducing.
     */
    answers.push(() => {
      const args = calls[calls.length - 1]?.args as { requestId: string };
      throw connectivity(args.requestId);
    });

    await act(async () => {
      result.current.mutate(variables);
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(invalidated(client)).toEqual([]);

    const failure = getSaveRetryState().failure;

    expect(failure?.cause).toBe('connectivity');
    expect(failure?.retryable).toBe(true);

    answers.push(() => entry({ value_grams: 1250 }));

    await act(async () => {
      await retryFailedSave();
    });

    /*
     * One id across both attempts: the retry through `mutateAsync` re-sends the
     * draft the first attempt bound, and mints nothing.
     */
    expect(
      calls.map((call) => (call.args as { requestId?: string }).requestId),
    ).toEqual([failure?.operation.requestId, failure?.operation.requestId]);
    expect(calls.map((call) => call.command)).toEqual([
      'update_weight_entry',
      'update_weight_entry',
    ]);
    // Both attempts send the same grams: the retry re-sends, it does not re-read.
    expect(
      calls.map((call) => (call.args as { entry?: unknown }).entry),
    ).toEqual([
      {
        value_grams: 1250,
        timestamp: null,
        note: null,
        outlier_confirmed: false,
      },
      {
        value_grams: 1250,
        timestamp: null,
        note: null,
        outlier_confirmed: false,
      },
    ]);
    expect(getSaveRetryState().failure).toBeNull();
    expect(invalidated(client)).toEqual(AFFECTED);
  });

  it('gives a re-submitted intent a new id once the previous write landed', async () => {
    const client = createTestQueryClient();
    seed(client);
    answers.push(() => entry());
    answers.push(() => entry({ id: 'entry-b' }));

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateWeightEntry(), { wrapper });

    const variables = {
      householdId: HOUSEHOLD,
      rabbitId: RABBIT_ID,
      input: {
        text: '1.2',
        unit: 'kilograms' as const,
        timestamp: null,
        note: null,
        outlierConfirmed: false,
      },
    };

    await act(async () => {
      result.current.mutate(variables);
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    await act(async () => {
      result.current.mutate(variables);
    });

    await waitFor(() => {
      expect(calls).toHaveLength(2);
    });

    /*
     * The confirmed intent's handle is dropped, so re-submitting the same object
     * is a second entry with its own identity — not a re-issue the shared
     * service would deduplicate away.
     */
    const [first, second] = calls.map(
      (call) => (call.args as { requestId: string }).requestId,
    );

    expect(first).not.toBe(second);
  });

  it('records the id the failure echoes rather than one minted after the fact', async () => {
    const client = createTestQueryClient();
    seed(client);

    /*
     * The backend echoes the id it was sent, so the spec makes the two differ to
     * make the priority observable: `onError` must name the operation the command
     * ran under, not re-derive one (D0.14).
     */
    const echoed = 'request-id-echoed-by-the-failure';

    answers.push(() => {
      throw connectivity(echoed);
    });

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateWeightEntry(), { wrapper });

    await act(async () => {
      result.current.mutate({
        householdId: HOUSEHOLD,
        rabbitId: RABBIT_ID,
        input: {
          text: '1.2',
          unit: 'kilograms',
          timestamp: null,
          note: null,
          outlierConfirmed: false,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(getSaveRetryState().failure?.operation.requestId).toBe(echoed);
    expect(getSaveRetryState().failure?.cause).toBe('connectivity');
  });

  it('leaves a validation refusal from the command to the mutation’s own error', async () => {
    const client = createTestQueryClient();
    seed(client);

    const wrapper = wrapperFor(client);
    const { result } = renderHook(() => useCreateWeightEntry(), { wrapper });

    answers.push(() => {
      const args = calls[calls.length - 1]?.args as { requestId: string };
      throw validation(args.requestId);
    });

    await act(async () => {
      result.current.mutate({
        householdId: HOUSEHOLD,
        rabbitId: RABBIT_ID,
        input: {
          text: '-5',
          unit: 'grams',
          timestamp: null,
          note: null,
          outlierConfirmed: false,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    // A non-positive value is the command's refusal, not the parse's (WT-4).
    expect(calls).toHaveLength(1);
    expect(getSaveRetryState().failure).toBeNull();
    expect(result.current.error).toMatchObject({ cause: 'validation' });
    expect(invalidated(client)).toEqual([]);
  });
});
