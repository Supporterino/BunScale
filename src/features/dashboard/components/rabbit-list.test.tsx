/*
 * The dashboard rabbit list at the component layer (task 5.10; DASH-1, DASH-2,
 * DASH-3).
 *
 * What this layer can decide, and what it cannot:
 *
 *   - **The order is the payload's, not the list's.** DASH-3's sorting is the
 *     read model's (task 2.14, proven in Rust 5.6); the component's claim is the
 *     other half of it — that it presents the rows in the order they arrive and
 *     derives no order of its own. The two order specs prove that by rendering
 *     the same three rabbits in both directions: a client-side sort would pass
 *     the first and fail the second.
 *   - **DASH-1's details are the row's own content**: the name, a thumbnail
 *     placeholder with an accessible name when the payload carries no
 *     reference, and the last-weighed date as a `<time>` carrying the payload's
 *     instant.
 *   - **DASH-2's indication replaces the date**, and a never-weighed row holds
 *     no `<time>` at all — the spec asks for "no date", not for a hidden one.
 *   - **A read in flight is not an empty household.** Until the read answers the
 *     list presents the caller's fallback and states nothing about how many
 *     rabbits exist (DASH-8's empty state is task 4.2's claim).
 *   - **Zero resolved rows is the empty state** (task 4.2; DASH-8, RAB-4): the
 *     P1 title and body, and a directly visible add control that opens the
 *     rabbit-creation form. The same section asserts the two readings that keep
 *     it honest — a household with a rabbit shows the list instead, and a
 *     household whose only rabbits are archived still shows the empty state,
 *     because the state is keyed on the active list (design.md D1.6).
 *   - **A read that fails never becomes the empty state.** A failed attempt that
 *     never succeeded leaves the fallback in place; DASH-8 is a fact about a
 *     successful answer, and 4.15 owns the failure surface.
 *
 * The renders run at both component baselines (390×844 and 1280×800) through
 * `describeAtBothBaselines`. Both platforms share this layer: nothing here is
 * per-OS. IPC is mocked at `invoke` (the same seam `src/test/mock-ipc.ts`
 * registers for E2E) so the real `useDashboardRows` hook, the real query client
 * and the real `get_active_household` guard run unchanged; only the answers are
 * fixed. Touch geometry and the adaptive layout are E2E's (5.15–5.18).
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import {
  cleanup,
  fireEvent,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import {
  describeAtBothBaselines,
  renderWithQueryClient,
} from '../../../test/setup';
import type { Household } from '../../households/queries';
import { t } from '../../i18n';
import { dashboardKeys } from '../keys';
import type { DashboardRow } from '../queries';
import { RabbitList } from './RabbitList';

/** Stands in for the contract's `Instant`. */
const TODAY = '2026-10-01T09:00:00Z';

/** The same instant a week earlier: DASH-3's "another a week ago". */
const LAST_WEEK = '2026-09-24T09:00:00Z';

/** The active household the rows belong to (DASH-1). */
const HOUSEHOLD: Household = {
  id: 'household-home',
  name: 'Home household',
  created_at: TODAY,
  owner_member_id: 'member-1',
  is_active: true,
  is_owner: true,
};

/** The identifying part of a rabbit, which is all a dashboard row carries. */
type FixtureRabbit = { readonly id: string; readonly name: string };

const HAZEL: FixtureRabbit = { id: 'rabbit-hazel', name: 'Hazel' };
const BRAMBLE: FixtureRabbit = { id: 'rabbit-bramble', name: 'Bramble' };
const CLOVER: FixtureRabbit = { id: 'rabbit-clover', name: 'Clover' };

/**
 * The household's only rabbit, archived. It belongs to the archived list's own
 * screen (RAB-6) and never to `get_dashboard`, whose rows are the active list;
 * it is seeded so the empty state's key — the active list — is under test rather
 * than the fixture's silence.
 */
const ARCHIVED_RABBIT = {
  id: 'rabbit-willow',
  name: 'Willow',
  thumbnail_ref: null,
  is_archived: true,
};

const RABBITS: readonly FixtureRabbit[] = [HAZEL, BRAMBLE, CLOVER];

/**
 * One row as the read model answers it (DASH-1 … DASH-4).
 *
 * `never_weighed` is a field of its own, not "the date happens to be null", so
 * the fixture sets both as the backend would: a never-weighed row carries no
 * instant. `thumbnail_ref: null` is P1's normal state — choosing a photo is
 * PHOTO-2 in P2 (design.md D1.11) — and the drop flag is 4.12's, so it is false
 * here and nothing asserts it.
 */
function dashboardRow(
  rabbit: FixtureRabbit,
  lastWeighedAt: string | null,
): DashboardRow {
  return {
    rabbit: {
      id: rabbit.id,
      name: rabbit.name,
      thumbnail_ref: null,
      is_archived: false,
    },
    last_weighed_at: lastWeighedAt,
    never_weighed: lastWeighedAt === null,
    has_weight_drop_alert: false,
  };
}

const HAZEL_ROW = dashboardRow(HAZEL, TODAY);
const BRAMBLE_ROW = dashboardRow(BRAMBLE, LAST_WEEK);
const CLOVER_ROW = dashboardRow(CLOVER, null);

/** The order the read model produces: newest weigh-in first, never weighed last. */
const SORTED_ROWS: DashboardRow[] = [HAZEL_ROW, BRAMBLE_ROW, CLOVER_ROW];

/** The same three rows in the opposite order, which only a payload can cause. */
const REVERSED_ROWS: DashboardRow[] = [CLOVER_ROW, BRAMBLE_ROW, HAZEL_ROW];

/**
 * The last-weighed date as the app presents it: the device's locale at medium
 * length. The spec asks for "its last-weighed date", not for one written
 * pattern, so a hard-coded "Oct 1, 2026" would assert the runner's locale
 * instead of the app. The payload's instant is pinned independently by the
 * `<time>` element's `dateTime`, so this mirror cannot hide a wrong instant.
 */
const DATE = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' });

/** What the list is told to show while it has no loaded row. */
const NO_ROWS_YET = t('dashboard.body');

let answer: DashboardRow[] | Promise<DashboardRow[]>;
let failRead: boolean;
let calls: string[];

beforeEach(() => {
  answer = [];
  failRead = false;
  calls = [];

  mockIPC((command) => {
    calls.push(command);

    switch (command) {
      case 'get_active_household':
        return HOUSEHOLD;
      case 'get_dashboard':
        if (failRead) {
          // Thrown at call time, not stored as a rejected promise, so the test
          // owns exactly one failure and nothing leaks an unhandled rejection.
          throw new Error('dashboard unavailable');
        }

        return answer;
      case 'list_rabbits':
        // The archived read would answer with the household's archived rabbit;
        // the active dashboard read is what the empty state is keyed on.
        return [ARCHIVED_RABBIT];
      default:
        throw new Error(`unexpected mocked command "${command}"`);
    }
  });
});

afterEach(async () => {
  // Unmount before dropping the bridge: the pending-read spec leaves a query in
  // flight, and a resolver that survives the tree would keep it alive.
  cleanup();
  await Promise.resolve();
  clearMocks();
});

/** The row an accessible name resolves to: the `<li>` that contains it. */
function rowOf(name: string): HTMLElement {
  const title = screen.getByText(name);
  const row = title.closest('li');

  if (row === null) {
    throw new Error(`"${name}" is not inside a list row`);
  }

  return row;
}

/**
 * The rendered rows' rabbit names, in DOM order. Read through the visible text
 * rather than a class name, so the assertion is about what a person sees.
 */
function renderedNames(): string[] {
  return screen.getAllByRole('listitem').map((row) => {
    const match = RABBITS.find(
      (rabbit) => within(row).queryByText(rabbit.name) !== null,
    );

    if (match === undefined) {
      throw new Error('a rendered row names no fixture rabbit');
    }

    return match.name;
  });
}

/** Mount the list and wait for the first row it presents. */
async function showRows(firstName: string) {
  renderWithQueryClient(<RabbitList />);
  await screen.findByText(firstName);
}

describeAtBothBaselines(
  'Dashboard rabbit list (DASH-1, DASH-2, DASH-3)',
  () => {
    it('lists both active rabbits with their details', async () => {
      answer = SORTED_ROWS;

      await showRows(HAZEL.name);

      // Both non-archived rabbits, and only them: one row each, no duplicate
      // presentation of the same rabbit.
      expect(screen.getAllByRole('listitem')).toHaveLength(3);

      // Each row shows the rabbit's name and its thumbnail — the placeholder,
      // because P1 carries no photo reference.
      for (const rabbit of RABBITS) {
        const row = rowOf(rabbit.name);

        expect(within(row).getByText(rabbit.name)).toBeVisible();
        expect(row.querySelector('img, [role="img"]')).not.toBeNull();
      }

      // A weighed rabbit's date is presented against the payload's instant.
      for (const { rabbit, at } of [
        { rabbit: HAZEL, at: TODAY },
        { rabbit: BRAMBLE, at: LAST_WEEK },
      ]) {
        const time = rowOf(rabbit.name).querySelector('time');

        expect(time).toHaveAttribute('dateTime', at);
        expect(time).toHaveTextContent(DATE.format(new Date(at)));
      }
    });

    it('presents newest weigh-in first and never weighed last', async () => {
      answer = SORTED_ROWS;

      await showRows(HAZEL.name);

      expect(renderedNames()).toEqual([HAZEL.name, BRAMBLE.name, CLOVER.name]);
    });

    it('reversing the payload reverses the rows', async () => {
      answer = REVERSED_ROWS;

      const first = renderWithQueryClient(<RabbitList />);
      await screen.findByText(CLOVER.name);

      expect(renderedNames()).toEqual([CLOVER.name, BRAMBLE.name, HAZEL.name]);

      first.unmount();

      answer = SORTED_ROWS;

      renderWithQueryClient(<RabbitList />);
      await screen.findByText(HAZEL.name);

      // The same three rows, rendered the other way round because the answer is
      // the other way round (DASH-3 is 2.14's derivation; this list only
      // presents it).
      expect(renderedNames()).toEqual([HAZEL.name, BRAMBLE.name, CLOVER.name]);
    });

    it('marks a rabbit with no entries as never weighed', async () => {
      answer = SORTED_ROWS;

      await showRows(HAZEL.name);

      const row = rowOf(CLOVER.name);

      expect(within(row).getByText(t('dashboard.neverWeighed'))).toBeVisible();
      expect(row.querySelector('time')).toBeNull();
      expect(within(row).queryByText(t('dashboard.lastWeighed'))).toBeNull();

      // The contrast that makes the absence the flag's doing, not the fixture's:
      // the weighed row beside it carries a date.
      expect(rowOf(HAZEL.name).querySelector('time')).not.toBeNull();
    });

    it('gives the thumbnail placeholder an accessible name', async () => {
      answer = SORTED_ROWS;

      await showRows(HAZEL.name);

      for (const rabbit of RABBITS) {
        expect(
          within(rowOf(rabbit.name)).getByRole('img', {
            name: t('rabbit.thumbnailPlaceholder'),
          }),
        ).toBeVisible();
      }
    });

    it('prompts for the first rabbit with a visible add control', async () => {
      const onAddRabbit = vi.fn();

      renderWithQueryClient(<RabbitList onAddRabbit={onAddRabbit} />);

      // The P1 copy (3.8), not a message invented here.
      expect(
        await screen.findByRole('heading', {
          name: t('dashboard.emptyTitle'),
        }),
      ).toBeVisible();
      expect(screen.getByText(t('dashboard.emptyBody'))).toBeVisible();

      // RAB-4: the control that opens the creation form is directly visible,
      // and it is a real, enabled button because this caller supplies the form.
      const add = screen.getByRole('button', { name: t('rabbit.add') });

      expect(add).toBeVisible();
      expect(add).toBeEnabled();

      fireEvent.click(add);

      expect(onAddRabbit).toHaveBeenCalledTimes(1);

      // The empty state replaces the list; it does not sit above an empty one.
      expect(screen.queryByRole('list')).toBeNull();
    });

    it('opens the form from the add control on a non-empty dashboard', async () => {
      // RAB-4's non-empty half: the control is not confined to the zero-row
      // branch, so a household that already holds rabbits can add another.
      answer = SORTED_ROWS;

      const onAddRabbit = vi.fn();

      renderWithQueryClient(<RabbitList onAddRabbit={onAddRabbit} />);

      await screen.findByText(HAZEL.name);

      const add = screen.getByRole('button', { name: t('rabbit.add') });

      expect(add).toBeEnabled();
      fireEvent.click(add);

      expect(onAddRabbit).toHaveBeenCalledTimes(1);
    });

    it('opens the form while the read is still pending', async () => {
      // RAB-4 says "whenever the dashboard is shown", which includes the
      // moment before the first answer resolves.
      answer = new Promise<DashboardRow[]>(() => {});

      const onAddRabbit = vi.fn();

      renderWithQueryClient(
        <RabbitList
          fallback={<p>{NO_ROWS_YET}</p>}
          onAddRabbit={onAddRabbit}
        />,
      );

      await waitFor(() => {
        expect(calls).toContain('get_dashboard');
      });

      // The fallback still says nothing about how many rabbits exist…
      expect(screen.getByText(NO_ROWS_YET)).toBeVisible();
      expect(screen.queryByText(t('dashboard.emptyTitle'))).toBeNull();

      // …while the form stays reachable.
      const add = screen.getByRole('button', { name: t('rabbit.add') });

      expect(add).toBeEnabled();
      fireEvent.click(add);

      expect(onAddRabbit).toHaveBeenCalledTimes(1);
    });

    it('presents the list instead of the empty state once a rabbit exists', async () => {
      answer = [HAZEL_ROW];

      await showRows(HAZEL.name);

      expect(screen.getAllByRole('listitem')).toHaveLength(1);
      expect(screen.queryByText(t('dashboard.emptyTitle'))).toBeNull();
      expect(screen.queryByText(t('dashboard.emptyBody'))).toBeNull();
    });

    it('presents the empty state for a household whose rabbits are all archived', async () => {
      // `get_dashboard` answers the active list only; the household's sole
      // rabbit is archived, so the resolved answer is zero rows. The archived
      // read would return it, and the list must not consult it (design.md D1.6).
      answer = [];

      renderWithQueryClient(<RabbitList />);

      expect(
        await screen.findByRole('heading', {
          name: t('dashboard.emptyTitle'),
        }),
      ).toBeVisible();
      expect(screen.getByText(t('dashboard.emptyBody'))).toBeVisible();

      expect(calls).toContain('get_dashboard');
      expect(calls).not.toContain('list_rabbits');
      expect(screen.queryByText(ARCHIVED_RABBIT.name)).toBeNull();
    });

    it('claims nothing until the read answers', async () => {
      // A read that never answers: the state before the first response, which a
      // person sits in on a cold start.
      answer = new Promise<DashboardRow[]>(() => {});

      renderWithQueryClient(<RabbitList fallback={<p>{NO_ROWS_YET}</p>} />);

      // The read was actually asked for, so "nothing loaded yet" is the state
      // under test and not a query that never ran.
      await waitFor(() => {
        expect(calls).toContain('get_dashboard');
      });

      expect(screen.getByText(NO_ROWS_YET)).toBeVisible();

      // No list and, above all, no empty state: "no rabbits" is a fact only a
      // loaded answer can state (DASH-8).
      expect(screen.queryByRole('list')).toBeNull();
      expect(screen.queryByText(t('dashboard.emptyTitle'))).toBeNull();
      expect(screen.queryByText(t('dashboard.emptyBody'))).toBeNull();
    });

    it('never presents the empty state for a read that failed', async () => {
      // A failure that never resolved is not "no rabbits"; 4.15 owns the
      // failure surface, so this list keeps the nothing-loaded fallback.
      failRead = true;

      const { client } = renderWithQueryClient(
        <RabbitList fallback={<p>{NO_ROWS_YET}</p>} />,
      );

      await waitFor(() => {
        expect(
          client.getQueryState(dashboardKeys.rows(HOUSEHOLD.id))?.status,
        ).toBe('error');
      });

      expect(screen.getByText(NO_ROWS_YET)).toBeVisible();
      expect(screen.queryByText(t('dashboard.emptyTitle'))).toBeNull();
      expect(screen.queryByText(t('dashboard.emptyBody'))).toBeNull();
    });
  },
);
