/*
 * The weight entry detail's route (tasks 4.19 and 4.21; WT-8, D1.2, NFR-9).
 *
 * WT-8 names one entry as a destination of its own, and D1.2 keeps the detail's
 * context — the rabbit whose cache prefix the entry sits under, the surface Back
 * returns to and that surface's two windows — in typed search parameters rather
 * than in screen state. This spec launches the app's committed route tree on
 * `/weights/$entryId` as a deep link would, so what is asserted is the wiring
 * rather than `WeightEntryDetailScreen` in isolation. Task 4.21 replaced the
 * placeholder with the real screen; the contract asserted here is therefore its
 * positive one:
 *
 *   - a deep link renders the entry its parameter names, reading it with the
 *     search's rabbit and offering Back;
 *   - a loaded entry shows the value in the display unit, the timestamp, the
 *     whole note, the derived drop marker only when it is flagged, and its
 *     photos as a strip when it has any;
 *   - Edit opens the pre-filled form and Cancel returns without writing; a
 *     confirmed save sends one identified update with the new value in canonical
 *     grams (WT-5, WT-8, DM-1); Delete is confirmed, naming the consequence, and
 *     sends one identified removal (WT-5, WT-8);
 *   - the payload is the household's, so an entry another member reads is
 *     presented with the same value, note and capabilities — the detail invents
 *     no per-member view (HS-8; DM-4's retained data);
 *   - a failed read renders the cause's copy and still offers Back (Q11: a
 *     pushed screen is never a dead end); a failed *profile* read names its
 *     cause where the writes would be, with its own refresh, rather than
 *     silently withholding Edit and Delete (NFR-8, D1.16);
 *   - Back returns to the rabbit dashboard with the windows the detail was
 *     opened at (WT-6, D1.4);
 *   - an archived rabbit's entry is presented read-only (RAB-8, D1.5).
 *
 * The file is prefixed `-` because it lives beside the route files: the router
 * plugin's `routeFileIgnorePrefix` is `-`, so it is not treated as a route.
 *
 * The two baselines are not split here: the 4.18 route precedent launches the
 * same committed tree once, and the adaptive arrangement jsdom cannot see is an
 * E2E assertion (see `src/test/setup.ts`). IPC is mocked at `invoke`, the
 * `testing-strategy` seam the other route and feature specs use; the event
 * plugin is mocked because returning to the rabbit dashboard mounts
 * `useAppEvents`.
 */
import type { QueryClient } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import {
  cleanup,
  fireEvent,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { DashboardRow } from '../features/dashboard/queries';
import type {
  Household,
  HouseholdSummary,
} from '../features/households/queries';
import { t } from '../features/i18n';
import type { Rabbit } from '../features/rabbits/queries';
import type { WeightEntry, WeightTrend } from '../features/weight/queries';
import {
  createTestQueryClient,
  createTestRouter,
  renderWithQueryClient,
} from '../test/setup';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

const HOUSEHOLD: Household = {
  id: 'household-home',
  name: 'Home household',
  created_at: AT,
  owner_member_id: 'member-1',
  is_active: true,
  is_owner: true,
};

const SUMMARY: HouseholdSummary = {
  id: HOUSEHOLD.id,
  name: HOUSEHOLD.name,
  created_at: HOUSEHOLD.created_at,
  is_active: true,
  is_owner: true,
  member_count: 1,
};

const RABBIT: Rabbit = {
  id: 'rabbit-hazel',
  household_id: HOUSEHOLD.id,
  name: 'Hazel',
  photo_ref: null,
  breed: null,
  dob: null,
  dob_unknown: true,
  sex: 'unknown',
  neuter_status: 'unknown',
  adoption_date: null,
  is_archived: false,
  weight_drop_threshold_percent: 5,
};

const ROW: DashboardRow = {
  rabbit: {
    id: RABBIT.id,
    name: RABBIT.name,
    thumbnail_ref: null,
    is_archived: false,
  },
  last_weighed_at: null,
  never_weighed: true,
  has_weight_drop_alert: false,
};

/** One point, enough for the rabbit dashboard's series read to resolve. */
const POINTS: WeightTrend['points'] = [
  {
    id: 'weight-1',
    value_grams: 2000,
    timestamp: '2026-10-01T09:00:00Z',
    has_weight_drop_alert: false,
  },
];

/** The note the entry read answers with, and the proof the read reached it. */
const NOTE = 'Morning weigh-in';

/**
 * The note a second member reads back: the payload the household share
 * answers with, not a note invented for the member asking (HS-8, DM-4).
 */
const SHARED_NOTE = 'Weighed while the other member was away';

/**
 * A note longer than a card would show if it truncated: the whole text is the
 * assertion, so the last sentence has to be reachable (WT-8, NFR-9).
 */
const LONG_NOTE =
  'Hazel was restless overnight and would not settle until the early hours, so this reading was taken once she was calm and eating again. She had already finished her hay and was drinking normally. The note continues past the point where a card would cut it off, and ends with the sentence that proves the whole note is still there: every word of the entry is the entry.';

/** The references an entry with photos carries (WT-8). */
const PHOTOS = [
  'https://photos.example/hazel-1.jpg',
  'https://photos.example/hazel-2.jpg',
] as const;

/** The entry the detail route reads. */
const ENTRY: WeightEntry = {
  id: 'weight-1',
  rabbit_id: RABBIT.id,
  value_grams: 2000,
  timestamp: '2026-10-01T09:00:00Z',
  created_at: '2026-10-01T09:00:00Z',
  note: NOTE,
  photo_refs: [],
  has_weight_drop_alert: false,
};

/** A connectivity refusal: retryable, so the failure surface offers a refresh. */
function connectivityError() {
  return {
    cause: 'connectivity',
    retryable: true,
    message_key: 'error.connectivity',
    request_id: null,
  };
}

/** The search this spec's deep link carries; `days_7`/`all` prove the pair. */
const ENTRY_PATH =
  '/weights/weight-1?rabbit=rabbit-hazel&from=rabbit&trend=days_7&history=all';

type Handler = (args: Record<string, unknown>) => unknown;
type Call = { command: string; args: Record<string, unknown> };

let answers: Map<string, Handler>;
let calls: Call[];

beforeEach(() => {
  calls = [];
  answers = new Map<string, Handler>([
    ['get_active_household', () => HOUSEHOLD],
    ['list_households', () => [SUMMARY]],
    ['get_dashboard', () => [ROW]],
    ['get_rabbit', () => RABBIT],
    ['get_display_weight_unit', () => 'grams'],
    [
      'get_weight_entries',
      (args) => ({
        range: String(args.range),
        points: POINTS,
        drop_threshold_percent: RABBIT.weight_drop_threshold_percent,
      }),
    ],
    [
      'load_household',
      () => ({
        household_id: HOUSEHOLD.id,
        items: [],
        source: 'local',
        loaded_at: null,
      }),
    ],
    ['get_weight_entry', () => ENTRY],
    ['delete_weight_entry', () => null],
  ]);

  mockIPC(
    (command, args) => {
      const payload = (args ?? {}) as Record<string, unknown>;
      calls.push({ command, args: payload });

      const answer = answers.get(command);

      if (answer === undefined) {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return answer(payload);
    },
    { shouldMockEvents: true },
  );
});

afterEach(async () => {
  // Unmount before dropping the bridge, so the event hook releases its
  // listeners through the still-mocked `invoke`.
  cleanup();
  await Promise.resolve();
  clearMocks();
});

/** Boot the app's route tree at `path`, optionally seeding the query cache. */
async function launch(path: string, seed?: (client: QueryClient) => void) {
  const client = createTestQueryClient();
  seed?.(client);

  const router = createTestRouter({ client, initialEntries: [path] });
  await router.load();
  const view = renderWithQueryClient(<RouterProvider router={router} />, {
    client,
  });

  return { ...view, router };
}

/** The moment a `datetime-local` control shows for an instant, in local time. */
function localDateTimeValue(date: Date): string {
  const pad = (value: number) => String(value).padStart(2, '0');

  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(
    date.getDate(),
  )}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

/** The navigation bar's leading control: the way out of every state (Q11). */
function backControl(): HTMLElement {
  return screen.getByRole('button', { name: t('settings.back') });
}

/** The screen title, which the navigation bar's `h1` alone carries. */
function screenTitle(): HTMLElement {
  return screen.getByRole('heading', {
    level: 1,
    name: t('weight.entry.title'),
  });
}

describe('the weight entry detail route (WT-8, D1.2)', () => {
  it('opens the entry its deep link names, with Back', async () => {
    await launch(ENTRY_PATH);

    // The route's parameter is the read's only argument; the search's rabbit
    // scopes the cache key but never travels into the command.
    expect(await screen.findByText(NOTE)).toBeVisible();
    expect(
      calls.some(
        (call) =>
          call.command === 'get_weight_entry' && call.args.entryId === ENTRY.id,
      ),
    ).toBe(true);

    expect(screenTitle()).toBeVisible();
    expect(backControl()).toBeVisible();
  });

  it('returns to the rabbit dashboard with the windows it was opened at', async () => {
    const { router } = await launch(ENTRY_PATH);

    await screen.findByText(NOTE);
    await userEvent.setup().click(backControl());

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/rabbits/rabbit-hazel');
    });

    // The pair the detail was opened at comes back, so the rabbit dashboard is
    // the one the person left rather than the default window (WT-6, D1.4).
    expect(router.state.location.search).toEqual({
      trend: 'days_7',
      history: 'all',
    });
  });

  it('renders the cause copy and Back when the entry read failed', async () => {
    answers.set('get_weight_entry', () => {
      throw connectivityError();
    });

    await launch(ENTRY_PATH);

    expect(await screen.findByText(t('error.connectivity'))).toBeVisible();
    expect(screenTitle()).toBeVisible();
    expect(backControl()).toBeVisible();
  });

  it('shows the value in the display unit, the timestamp and the whole note', async () => {
    answers.set('get_display_weight_unit', () => 'kilograms');
    answers.set('get_weight_entry', () => ({ ...ENTRY, note: LONG_NOTE }));

    const { container } = await launch(ENTRY_PATH);

    // WT-8: 2000 stored grams read as the person's unit, not as grams.
    expect(await screen.findByText(/^2\s*kg$/)).toBeVisible();
    expect(screen.getByText(LONG_NOTE)).toBeVisible();
    expect(
      container.querySelector(`time[datetime="${ENTRY.timestamp}"]`),
    ).not.toBeNull();

    // The flag is the backend's and this fixture carries none.
    expect(screen.queryByText(t('weight.dropAlert'))).toBeNull();
  });

  it('shows the drop marker when the entry is flagged', async () => {
    answers.set('get_weight_entry', () => ({
      ...ENTRY,
      has_weight_drop_alert: true,
    }));

    await launch(ENTRY_PATH);

    expect(await screen.findByText(t('weight.dropAlert'))).toBeVisible();
  });

  it('opens the pre-filled form from Edit and returns without writing on Cancel', async () => {
    answers.set('get_display_weight_unit', () => 'kilograms');

    await launch(ENTRY_PATH);

    fireEvent.click(
      await screen.findByRole('button', { name: t('weight.entry.edit') }),
    );

    // WT-8: the form starts on the entry's own fields — the value in the
    // display unit, the entry's instant, and its note.
    expect(
      await screen.findByRole('textbox', {
        name: `${t('weight.form.valueLabel')} (${t('settings.displayUnit.kilograms')})`,
      }),
    ).toHaveValue('2');
    expect(screen.getByLabelText(t('weight.form.timestampLabel'))).toHaveValue(
      localDateTimeValue(new Date(ENTRY.timestamp)),
    );
    expect(
      screen.getByRole('textbox', { name: t('weight.form.noteLabel') }),
    ).toHaveValue(NOTE);

    fireEvent.click(screen.getByRole('button', { name: t('common.cancel') }));

    // Cancel returns to the detail and writes nothing.
    expect(await screen.findByText(NOTE)).toBeVisible();
    expect(
      screen.getByRole('button', { name: t('weight.entry.edit') }),
    ).toBeVisible();
    expect(
      calls.filter((call) => call.command === 'update_weight_entry'),
    ).toHaveLength(0);
  });

  it('saves one identified edit with the new value in canonical grams', async () => {
    answers.set('get_display_weight_unit', () => 'kilograms');

    // The confirming read answers with the corrected entry, so the detail the
    // person returns to is the stored value rather than a local guess (D1).
    let stored: WeightEntry = ENTRY;

    answers.set('get_weight_entry', () => stored);
    answers.set('update_weight_entry', () => {
      stored = { ...ENTRY, value_grams: 1800 };

      return stored;
    });

    await launch(ENTRY_PATH);

    fireEvent.click(
      await screen.findByRole('button', { name: t('weight.entry.edit') }),
    );

    fireEvent.change(
      await screen.findByRole('textbox', {
        name: `${t('weight.form.valueLabel')} (${t('settings.displayUnit.kilograms')})`,
      }),
      { target: { value: '1.8' } },
    );
    fireEvent.click(
      screen.getByRole('button', { name: t('weight.form.save') }),
    );

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'update_weight_entry'),
      ).toHaveLength(1);
    });

    // WT-5, WT-8, DM-1: the correction is a whole-entry replacement, the value
    // crosses the display-unit boundary as whole grams, the untouched timestamp
    // and note are re-sent, and the write carries its operation identity.
    const update = calls.find((call) => call.command === 'update_weight_entry');

    expect(update?.args).toEqual({
      entryId: ENTRY.id,
      entry: {
        value_grams: 1800,
        timestamp: ENTRY.timestamp,
        note: NOTE,
        outlier_confirmed: false,
      },
      requestId: expect.any(String),
    });
    expect(typeof update?.args.requestId).toBe('string');

    // The confirming read reaches the mounted detail under the entry's rabbit
    // prefix, so the surface the person returns to shows the corrected value.
    expect(await screen.findByText(/^1\.8\s*kg$/)).toBeVisible();
  });

  it('presents a second member the shared entry payload, with no reduced view', async () => {
    // The active household says the person is a member who did not create it:
    // the entry the first member recorded is the one they reach (HS-8).
    answers.set('get_active_household', () => ({
      ...HOUSEHOLD,
      owner_member_id: 'member-1',
      is_owner: false,
    }));
    answers.set('get_display_weight_unit', () => 'grams');
    answers.set('get_weight_entry', () => ({
      ...ENTRY,
      value_grams: 845,
      note: SHARED_NOTE,
    }));

    await launch(ENTRY_PATH);

    // DM-4 keeps the entry readable, and its presentation is the payload's:
    // the other member's value, timestamp and note, not a per-member copy.
    expect(await screen.findByText(SHARED_NOTE)).toBeVisible();
    expect(screen.getByText(/^845\s*g$/)).toBeVisible();
    expect(
      document.querySelector(`time[datetime="${ENTRY.timestamp}"]`),
    ).not.toBeNull();

    // The read names no member: the answer is the household's shared entry.
    const entryRead = calls.find((call) => call.command === 'get_weight_entry');

    expect(entryRead?.args).toEqual({ entryId: ENTRY.id });

    // HS-8: no capability is withheld because of the member's role.
    expect(
      screen.getByRole('button', { name: t('weight.entry.edit') }),
    ).toBeVisible();
    expect(
      screen.getByRole('button', { name: t('weight.entry.delete') }),
    ).toBeVisible();
  });

  it('names a failed profile read and offers its refresh in place of the writes', async () => {
    answers.set('get_display_weight_unit', () => 'kilograms');
    answers.set('get_rabbit', () => {
      throw connectivityError();
    });

    await launch(ENTRY_PATH);

    // The entry read succeeded, so its facts stay readable.
    expect(await screen.findByText(NOTE)).toBeVisible();
    expect(screen.getByText(/^2\s*kg$/)).toBeVisible();

    // The read the writes depend on failed; its cause is named where the
    // actions would be, with the retry a read has (NFR-8, D1.16).
    expect(await screen.findByText(t('error.connectivity'))).toBeVisible();
    expect(
      screen.getByRole('heading', {
        level: 2,
        name: t('rabbit.loadFailed.title'),
      }),
    ).toBeVisible();

    const refresh = screen.getByRole('button', { name: t('error.retry') });

    expect(refresh).toBeVisible();

    // While the archived state is unknown neither write is offered — and not
    // silently: the cause and its refresh stand in their place.
    expect(
      screen.queryByRole('button', { name: t('weight.entry.edit') }),
    ).toBeNull();
    expect(
      screen.queryByRole('button', { name: t('weight.entry.delete') }),
    ).toBeNull();

    // The refresh re-reads the profile; once it answers, both actions appear.
    answers.set('get_rabbit', () => RABBIT);
    fireEvent.click(refresh);

    expect(
      await screen.findByRole('button', { name: t('weight.entry.edit') }),
    ).toBeVisible();
    expect(
      screen.getByRole('button', { name: t('weight.entry.delete') }),
    ).toBeVisible();
    expect(screen.queryByText(t('error.connectivity'))).toBeNull();
  });

  it('confirms the consequence before deleting once with the entry identity', async () => {
    const { router } = await launch(ENTRY_PATH);

    await screen.findByText(NOTE);

    fireEvent.click(
      screen.getByRole('button', { name: t('weight.entry.delete') }),
    );

    const dialog = screen.getByRole('alertdialog');

    // NFR-9: the confirmation names what cannot be undone, before any write.
    expect(dialog).toHaveAccessibleName(t('weight.entry.deleteConfirm.title'));
    expect(dialog).toHaveAccessibleDescription(
      new RegExp(t('weight.entry.deleteConfirm.body')),
    );
    expect(
      within(dialog).getByText(t('weight.entry.deleteConfirm.body')),
    ).toBeVisible();
    expect(
      calls.filter((call) => call.command === 'delete_weight_entry'),
    ).toHaveLength(0);

    fireEvent.click(
      within(dialog).getByRole('button', {
        name: t('weight.entry.deleteConfirm.cancel'),
      }),
    );

    expect(screen.queryByRole('alertdialog')).toBeNull();
    expect(
      calls.filter((call) => call.command === 'delete_weight_entry'),
    ).toHaveLength(0);

    // Confirming removes the entry the detail presented, so the surface closes.
    fireEvent.click(
      screen.getByRole('button', { name: t('weight.entry.delete') }),
    );
    fireEvent.click(
      within(screen.getByRole('alertdialog')).getByRole('button', {
        name: t('weight.entry.deleteConfirm.confirm'),
      }),
    );

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'delete_weight_entry'),
      ).toHaveLength(1);
    });

    const deletion = calls.find(
      (call) => call.command === 'delete_weight_entry',
    );

    expect(deletion?.args).toEqual({
      entryId: ENTRY.id,
      requestId: expect.any(String),
    });
    expect(typeof deletion?.args.requestId).toBe('string');

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/rabbits/rabbit-hazel');
    });
  });

  it("presents an archived rabbit's entry read-only", async () => {
    answers.set('get_rabbit', () => ({ ...RABBIT, is_archived: true }));
    answers.set('get_display_weight_unit', () => 'kilograms');

    const { container } = await launch(ENTRY_PATH);

    // RAB-8, D1.5: the facts stay readable…
    expect(await screen.findByText(NOTE)).toBeVisible();
    expect(screen.getByText(/^2\s*kg$/)).toBeVisible();
    expect(container.querySelector('time')).not.toBeNull();

    // …and neither write is offered.
    expect(
      screen.queryByRole('button', { name: t('weight.entry.edit') }),
    ).toBeNull();
    expect(
      screen.queryByRole('button', { name: t('weight.entry.delete') }),
    ).toBeNull();
    expect(
      screen.queryByRole('button', { name: t('weight.form.save') }),
    ).toBeNull();
  });

  it('presents the photos as a strip when the entry has any', async () => {
    answers.set('get_weight_entry', () => ({
      ...ENTRY,
      photo_refs: [...PHOTOS],
    }));

    await launch(ENTRY_PATH);

    await screen.findByText(NOTE);

    // WT-8: the strip is one labelled group, and each reference is a photo.
    const heading = screen.getByRole('heading', {
      level: 2,
      name: t('weight.detail.photos'),
    });
    const section = heading.closest('section');

    expect(heading).toBeVisible();
    expect(section).not.toBeNull();
    expect(
      Array.from(section?.querySelectorAll('img') ?? []).map((image) =>
        image.getAttribute('src'),
      ),
    ).toEqual([...PHOTOS]);
  });

  it('shows no photo strip when the entry has none', async () => {
    const { container } = await launch(ENTRY_PATH);

    await screen.findByText(NOTE);

    // WT-8: no pager, and no empty frame that could stand in for one.
    expect(
      screen.queryByRole('heading', {
        level: 2,
        name: t('weight.detail.photos'),
      }),
    ).toBeNull();
    expect(container.querySelector('img')).toBeNull();
  });
});
