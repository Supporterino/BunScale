/*
 * The dashboard rabbit list at the component layer (task 5.10; DASH-1, DASH-2,
 * DASH-3, DASH-4, DASH-8, DASH-9).
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
 *   - **DASH-4's flag is the payload's flag, in the catalog's words.** A row
 *     whose `has_weight_drop_alert` is set prints the warning phrase as text;
 *     the unflagged row beside it prints nothing; and because the list keeps no
 *     alert state of its own, a confirming read that answers an unflagged row
 *     is all it takes for the marker to go — which is what raising the
 *     threshold produces (3.4/3.9's refresh, WT-7's re-derivation).
 *   - **DASH-9 adds here only what the route spec cannot see.**
 *     `-rabbit-list-route.test.tsx` owns the control's destination, its typed
 *     search binding and its sibling relation to the row's open control. This
 *     file asserts the one row-wide part of "every dashboard row": a
 *     never-weighed row carries the control too, so the affordance is not
 *     conditional on a weigh-in existing.
 *   - **A read in flight is not an empty household.** Until the read answers the
 *     list presents the caller's fallback and states nothing about how many
 *     rabbits exist (DASH-8's empty state is task 4.2's claim).
 *   - **Zero resolved rows is the empty state** (task 4.2; DASH-8, RAB-4): the
 *     P1 title and body, and a directly visible add control that opens the
 *     rabbit-creation form. The same section asserts the two readings that keep
 *     it honest — a household with a rabbit shows the list instead, and a
 *     household whose only rabbits are archived still shows the empty state,
 *     because the state is keyed on the active list (design.md D1.6).
 *   - **A read that fails is its own state, never "no rabbits" and never
 *     "nothing loaded yet"** (task 4.15; NFR-8, D1.16). The surface is
 *     `describeReadFailure`'s cause copy — D1.16's surface key when the backend
 *     named one — with a refresh that re-reads when the mapper says a fresh read
 *     can change the answer. DASH-8's empty state stays a fact about a
 *     successful answer, so no failed read draws it.
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
  act,
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
import type { SaveCommandError } from '../../sync/failure';
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
 * PHOTO-2 in P2 (design.md D1.11) — and `has_weight_drop_alert` is the derived
 * flag of 2.14/WT-7, so it is a parameter: DASH-4's two states are this one row
 * with the flag either way, and the fixture never decides what should be
 * flagged.
 */
function dashboardRow(
  rabbit: FixtureRabbit,
  lastWeighedAt: string | null,
  hasWeightDropAlert = false,
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
    has_weight_drop_alert: hasWeightDropAlert,
  };
}

const HAZEL_ROW = dashboardRow(HAZEL, TODAY);
const BRAMBLE_ROW = dashboardRow(BRAMBLE, LAST_WEEK);
const CLOVER_ROW = dashboardRow(CLOVER, null);

/** Hazel's row once an entry's drop exceeds her threshold (DASH-4, WT-7). */
const HAZEL_FLAGGED_ROW = dashboardRow(HAZEL, TODAY, true);

/**
 * D1.16's unknown-content refusal, as a dashboard read answers it: the read
 * derives a last-weighed instant and an alert flag from rows, so a row this
 * version cannot describe refuses the read instead of answering a partial list
 * (task 3.8's `error.household_content_unloaded`).
 *
 * `retryable: false` is the failure's real value — a read carries no
 * `request_id` for a write-style retry — and it is deliberately not what decides
 * the refresh: 3.10's read mapper keys that on the cause, so a cause a fresh read
 * can change still offers one.
 */
const CONTENT_UNLOADED_ERROR: SaveCommandError = {
  cause: 'unknown',
  retryable: false,
  message_key: 'error.household_content_unloaded',
  request_id: null,
};

/**
 * The one cause whose copy asks for corrected input rather than another read,
 * which is why 3.10 withholds the refresh for it: a fresh read cannot change the
 * answer.
 */
const VALIDATION_ERROR: SaveCommandError = {
  cause: 'validation',
  retryable: false,
  message_key: 'error.validation',
  request_id: null,
};

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
/** The failure `get_dashboard` answers with, or `null` for a successful read. */
let readFailure: SaveCommandError | null;
let calls: string[];

beforeEach(() => {
  answer = [];
  readFailure = null;
  calls = [];

  mockIPC((command) => {
    calls.push(command);

    switch (command) {
      case 'get_active_household':
        return HOUSEHOLD;
      case 'get_dashboard':
        if (readFailure !== null) {
          // Thrown at call time, not stored as a rejected promise, so the test
          // owns exactly one failure and nothing leaks an unhandled rejection.
          // A non-`Error` throw is the backend's `CommandError` arriving
          // unchanged, which is the value 3.10's mapper reads.
          throw readFailure;
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

/**
 * The rows' own list. The toolbar's picker repeats every rabbit's name as an
 * `<option>`, so a rabbit name is looked up here rather than document-wide
 * (task 4.20).
 */
function rowsList(): HTMLElement {
  return screen.getByRole('list');
}

/** The row an accessible name resolves to: the `<li>` that contains it. */
function rowOf(name: string): HTMLElement {
  const title = within(rowsList()).getByText(name);
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
  await within(await screen.findByRole('list')).findByText(firstName);
}

describeAtBothBaselines(
  'Dashboard rabbit list (DASH-1, DASH-2, DASH-3, DASH-4, DASH-8, DASH-9)',
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

    it('does not list an archived rabbit', async () => {
      // DASH-1's second scenario: one archived rabbit beside one non-archived
      // one. The mock's `list_rabbits` answer is the archived read and would
      // present Willow; the dashboard's rows are the active list, so she never
      // reaches this list while Hazel does.
      answer = [HAZEL_ROW];

      await showRows(HAZEL.name);

      expect(renderedNames()).toEqual([HAZEL.name]);
      expect(screen.queryByText(ARCHIVED_RABBIT.name)).toBeNull();
    });

    it('sorts the most recently weighed rabbit first', async () => {
      answer = SORTED_ROWS;

      await showRows(HAZEL.name);

      const names = renderedNames();

      // DASH-3: weighed today above weighed a week ago.
      expect(names.indexOf(HAZEL.name)).toBeLessThan(
        names.indexOf(BRAMBLE.name),
      );
      expect(names).toEqual([HAZEL.name, BRAMBLE.name, CLOVER.name]);
    });

    it('sorts a never-weighed rabbit last', async () => {
      /*
       * A `DashboardRow` carries no creation order, so "created first" cannot be
       * stated at this layer: the claim the list can make is that the
       * never-weighed row arrives last and is presented below every weighed row,
       * because it neither derives nor re-groups the payload's order.
       */
      answer = SORTED_ROWS;

      await showRows(HAZEL.name);

      const names = renderedNames();
      const neverWeighed = names.indexOf(CLOVER.name);

      expect(neverWeighed).toBe(names.length - 1);

      for (const rabbit of [HAZEL, BRAMBLE]) {
        expect(names.indexOf(rabbit.name)).toBeLessThan(neverWeighed);
      }
    });

    it('reversing the payload reverses the rows', async () => {
      answer = REVERSED_ROWS;

      const first = renderWithQueryClient(<RabbitList />);
      await within(await screen.findByRole('list')).findByText(CLOVER.name);

      expect(renderedNames()).toEqual([CLOVER.name, BRAMBLE.name, HAZEL.name]);

      first.unmount();

      answer = SORTED_ROWS;

      renderWithQueryClient(<RabbitList />);
      await within(await screen.findByRole('list')).findByText(HAZEL.name);

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

    it('flags a rabbit with a flagged drop', async () => {
      answer = [HAZEL_FLAGGED_ROW, BRAMBLE_ROW];

      await showRows(HAZEL.name);

      const flagged = rowOf(HAZEL.name);
      const unflagged = rowOf(BRAMBLE.name);

      // DASH-4: the row presents the warning, and it presents it as the
      // catalog's phrase rather than as a colour-only dot.
      expect(within(flagged).getByText(t('weight.dropAlert'))).toBeVisible();

      // The row beside it is unflagged, so the marker is the row's own state and
      // not something the list adds once any rabbit is flagged.
      expect(within(unflagged).queryByText(t('weight.dropAlert'))).toBeNull();
    });

    it('clears the flag when the threshold is raised', async () => {
      answer = [HAZEL_FLAGGED_ROW];

      const { client } = renderWithQueryClient(<RabbitList />);

      expect(await screen.findByText(t('weight.dropAlert'))).toBeVisible();

      /*
       * Raising the rabbit's threshold re-derives the flag on the backend and
       * the rabbit write refreshes the dashboard rows (3.4, 3.9; D1.13), so the
       * confirming read is the whole of what the list needs: it holds no alert
       * state that could outlive the payload.
       */
      answer = [HAZEL_ROW];

      await act(async () => {
        await client.invalidateQueries({
          queryKey: dashboardKeys.rows(HOUSEHOLD.id),
        });
      });

      await waitFor(() => {
        expect(screen.queryByText(t('weight.dropAlert'))).toBeNull();
      });

      // Only the flag cleared: the row is still present, with its date.
      expect(rowOf(HAZEL.name).querySelector('time')).toHaveAttribute(
        'dateTime',
        TODAY,
      );
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

    it('gives a never-weighed row its quick-weight control too', async () => {
      answer = SORTED_ROWS;

      renderWithQueryClient(<RabbitList />);

      await within(await screen.findByRole('list')).findByText(CLOVER.name);

      /*
       * DASH-9 says every row. The route spec proves the control's destination
       * and typed-search binding; what this layer adds is that a row with no
       * weigh-in still carries it, so the affordance is the row's rather than a
       * consequence of there being an entry to compare against.
       */
      const control = within(rowOf(CLOVER.name)).getByRole('button', {
        name: `${CLOVER.name} ${t('weight.log')}`,
      });

      expect(control).toBeVisible();
      expect(control).toBeEnabled();
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

      await within(await screen.findByRole('list')).findByText(HAZEL.name);

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
      // A failed read is its own surface (task 4.15), not "no rabbits" and not
      // "nothing loaded yet". The cause is D1.16's unknown-content refusal, so
      // the copy has to come from 3.10's read mapper rather than from a guess.
      readFailure = CONTENT_UNLOADED_ERROR;

      renderWithQueryClient(<RabbitList fallback={<p>{NO_ROWS_YET}</p>} />);

      const failure = await screen.findByRole('alert');

      expect(
        within(failure).getByText(t('error.household_content_unloaded')),
      ).toBeVisible();

      /*
       * A fresh read can fill the missing content — `describeReadFailure` decides
       * that from the cause, not from the write-side `retryable` flag this
       * failure carries as `false` — so the failure offers exactly that action.
       */
      const refresh = within(failure).getByRole('button', {
        name: t('error.retry'),
      });

      expect(refresh).toBeEnabled();

      // No empty-state prompt, no rows and no pending intro: DASH-8 is a fact
      // about a successful answer, and the fallback is the pending state.
      expect(screen.queryByText(t('dashboard.emptyTitle'))).toBeNull();
      expect(screen.queryByText(t('dashboard.emptyBody'))).toBeNull();
      expect(screen.queryByText(NO_ROWS_YET)).toBeNull();
      expect(screen.queryByRole('list')).toBeNull();
      expect(screen.queryAllByRole('listitem')).toHaveLength(0);

      // The refresh is a fresh read, and the next answer has the rows.
      readFailure = null;
      answer = SORTED_ROWS;

      fireEvent.click(refresh);

      await within(await screen.findByRole('list')).findByText(HAZEL.name);

      expect(renderedNames()).toEqual([HAZEL.name, BRAMBLE.name, CLOVER.name]);
      expect(screen.queryByRole('alert')).toBeNull();
      expect(screen.queryByText(t('dashboard.emptyTitle'))).toBeNull();
    });

    it('offers no refresh for a failed read a fresh read cannot change', async () => {
      // 3.10's only non-refreshable cause: the copy asks for corrected input, so
      // another identical read cannot answer differently and no control promises
      // one. The failure surface itself is still the failure surface.
      readFailure = VALIDATION_ERROR;

      renderWithQueryClient(<RabbitList fallback={<p>{NO_ROWS_YET}</p>} />);

      expect(await screen.findByText(t('error.validation'))).toBeVisible();
      expect(
        screen.queryByRole('button', { name: t('error.retry') }),
      ).toBeNull();

      expect(screen.queryByText(t('dashboard.emptyTitle'))).toBeNull();
      expect(screen.queryByText(t('dashboard.emptyBody'))).toBeNull();
      expect(screen.queryByRole('list')).toBeNull();
    });
  },
);
