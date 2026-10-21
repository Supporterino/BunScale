/*
 * Settings & Profile's household and display-unit sections at the component
 * layer (tasks 5.12, 7.3 and 7.4; HS-4, SET-4, D0.14, Q44, Q45).
 *
 * Two acceptance scenarios are asserted here:
 *
 *   - "All households are listed with the active one indicated" — every row the
 *     book holds is rendered, exactly one row carries the active marker
 *     (`aria-current="true"` plus the `Active` badge), and the owner badge
 *     follows `is_owner` rather than the active flag;
 *   - "Create an additional household without onboarding" — the inline form
 *     adds the household, marks it active, closes, and leaves the location on
 *     `/settings`: no `/onboarding`, no trip through the dashboard.
 *
 * The create form's retry identity (D0.14) is asserted as well. Q29 records that
 * no earlier task owned a screen-level assertion for it; the Settings form keeps
 * its own handle discipline ("one operation per intent, reused on retry while
 * the name is unchanged"), so the retry is worth its own test here and not only
 * on onboarding.
 *
 * The household read's retry is asserted as well (task 7.4; HS-4, Q45). The
 * active marker comes from the active read, not from the list, so the retry
 * re-reads the held-set pair — the list and the active read — and not the list
 * alone. The two read counts are what make that regression fail: a retry that
 * refreshed only the list would pass every other assertion in this suite.
 *
 * What the retry does *while* it runs is asserted too (task 7.5; HS-4, Q46).
 * Starting the pair's refetch clears the failed read's error, so the section
 * reports `pending`: the failure notice — and the retry control inside it —
 * unmounts and the section shows its loading status until the list read settles
 * (the pair is started together). There is no busy-button state to assert
 * because there is no button during the flight; that absence is the mechanism
 * which makes a second tap impossible. The status's copy is asserted as well —
 * `settings.households.loading`, the label task 7.6 made required — so the
 * screen is shown to *say* it is working, not only to expose a region.
 *
 * The display-unit section's failure is asserted here too (task 7.3; SET-4,
 * Q44). The unit pair reports only `cause: unknown`, which the backend marks
 * `retryable: false`, so the section offers **no retry control at all** — and
 * that absence is the requirement's outcome rather than an omission. The
 * recovery Q44 names is re-selecting the segment (or remounting the screen), so
 * these specs assert the `unknown` copy, the absence of any control named the
 * retry label, and the segments still operable afterwards. Both platforms share
 * this component layer: nothing in this file is per-OS.
 *
 * What is NOT assertable at this layer, and who owns it:
 *   - switching the active household is HS-3 and already covered at the
 *     mutation layer (`src/features/households/switch.test.tsx`); this file only
 *     asserts the marker the list renders;
 *   - "the person is its first member" is a domain fact behind
 *     `create_household`, owned by Rust 5.1;
 *   - geometry (44 pt rows) and the tab-bar-vs-sidebar arrangement need a layout
 *     engine and a real browser; E2E 5.14/5.17 own them. jsdom reports zeroes
 *     for `getBoundingClientRect` and loads no app stylesheet.
 *
 * The suite runs at both component baselines through `describeAtBothBaselines`;
 * nothing here branches on the viewport, and the arranged layout stays E2E's
 * claim.
 *
 * IPC is mocked at `invoke` and the event plugin is mocked because the screen
 * mounts `useAppEvents`.
 */

import { RouterProvider } from '@tanstack/react-router';
import type { InvokeArgs } from '@tauri-apps/api/core';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, cleanup, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type {
  Household,
  HouseholdSummary,
} from '../features/households/queries';
import { t } from '../features/i18n';
import type { ServiceStatus } from '../features/sync/entry-status';
import { SAVE_CAUSE_MESSAGE_KEYS } from '../features/sync/failure';
import {
  createTestQueryClient,
  createTestRouter,
  describeAtBothBaselines,
  renderWithQueryClient,
} from './setup';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

/** The household that is active when Settings opens. */
const HOME: HouseholdSummary = {
  id: 'household-home',
  name: 'Home household',
  created_at: AT,
  is_active: true,
  is_owner: false,
  member_count: 2,
};

/** The same household as the active read answers it (HS-3, DM-3). */
const HOME_ACTIVE: Household = {
  id: HOME.id,
  name: HOME.name,
  created_at: HOME.created_at,
  owner_member_id: 'member-2',
  is_active: true,
  is_owner: false,
};

/** The second held household: not active, owned by this person. */
const BEACH: HouseholdSummary = {
  id: 'household-beach',
  name: 'Beach house',
  created_at: AT,
  is_active: false,
  is_owner: true,
  member_count: 1,
};

/** A fresh reachable observation, so no offline notice is rendered. */
const REACHABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: true,
  cause: null,
  observed_at: AT,
};

/** The create control's label (`t('household.create')`). */
const CREATE = 'Create household';

/** The display unit's heading; it is also the radiogroup's accessible name. */
const UNIT = t('settings.displayUnit.heading');

/** The catalog's retry label — the name a control that re-sends would carry. */
const RETRY = t('error.retry');

/** The in-flight label; a refused change must not leave it behind. */
const SAVING = t('settings.displayUnit.saving');

/**
 * The copy the unit pair's only cause produces, read from the map that defines
 * the taxonomy rather than restated here, so a renamed key fails this spec
 * instead of leaving a stale string that happens to match the screen.
 */
const UNIT_FAILURE_COPY = t(SAVE_CAUSE_MESSAGE_KEYS.unknown);

/** One `invoke` call the app made, as the backend would have received it. */
type MockCall = {
  readonly command: string;
  readonly args: Record<string, unknown>;
};

/** One mocked command's answer; a throw is a rejected `invoke`. */
type MockHandler = (args: Record<string, unknown>) => unknown;

/** The normalized payload of one `invoke` call. */
function payloadOf(args: InvokeArgs | undefined): Record<string, unknown> {
  return (args ?? {}) as Record<string, unknown>;
}

/** The household the additional-create flow produces. */
function createdHousehold(args: Record<string, unknown>): Household {
  return {
    id: 'household-beach',
    name: String(args.name),
    created_at: AT,
    owner_member_id: 'member-1',
    is_active: true,
    is_owner: true,
  };
}

/** That household as a held-list row, marked active (HS-4). */
function createdRow(created: Household): HouseholdSummary {
  return {
    id: created.id,
    name: created.name,
    created_at: created.created_at,
    is_active: true,
    is_owner: true,
    member_count: 1,
  };
}

let calls: MockCall[];
let handlers: Map<string, MockHandler>;

/** Every `create_household` call, in order. */
function createCalls(): MockCall[] {
  return calls.filter((call) => call.command === 'create_household');
}

/** Every `list_households` call, in order (the held set). */
function listCalls(): MockCall[] {
  return calls.filter((call) => call.command === 'list_households');
}

/** Every `get_active_household` call, in order (the marker's own read). */
function activeCalls(): MockCall[] {
  return calls.filter((call) => call.command === 'get_active_household');
}

beforeEach(() => {
  calls = [];
  handlers = new Map();

  mockIPC(
    (command, args) => {
      const payload = payloadOf(args);

      calls.push({ command, args: payload });

      const handler = handlers.get(command);

      if (handler === undefined) {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return handler(payload);
    },
    { shouldMockEvents: true },
  );
});

afterEach(async () => {
  // Unmount while the mocked bridge is still installed (the screen releases its
  // `listen` subscriptions on unmount), then drop the mock.
  cleanup();
  await Promise.resolve();
  await Promise.resolve();

  clearMocks();
});

/** The identity and display-unit reads the rest of the screen performs. */
function answerOtherReads(): void {
  handlers.set('get_identity', () => ({
    account_user_id: 'account-1',
    display_name: 'Ana',
    resolved: true,
  }));
  handlers.set('get_display_weight_unit', () => 'grams');
  handlers.set('get_service_status', () => REACHABLE);
}

/**
 * A unit command's only failure, as the backend sends it (Q44): cause
 * `unknown`, which `ipc::failure` marks non-retryable. The backend's own
 * `message_key` is carried for shape completeness; the surface is expected to
 * derive its copy from `cause` (task 3.4's rule).
 */
function unitFailure(requestId: unknown) {
  return {
    cause: 'unknown',
    message_key: 'error.unknown',
    request_id: typeof requestId === 'string' ? requestId : null,
    retryable: false,
  };
}

/** Every `set_display_weight_unit` call, in order. */
function setUnitWrites(): MockCall[] {
  return calls.filter((call) => call.command === 'set_display_weight_unit');
}

/**
 * A **read** failure the backend marks retryable: no HTTP answer arrived, so
 * the same read is promised to be worth re-issuing. Same shape as
 * `unitFailure`, with the cause this regression needs; the copy is derived from
 * `cause` (task 3.4's rule), and `message_key` is carried for shape
 * completeness.
 */
function connectivityFailure(requestId: unknown) {
  return {
    cause: 'connectivity',
    message_key: 'error.connectivity',
    request_id: typeof requestId === 'string' ? requestId : null,
    retryable: true,
  };
}

/**
 * A promise whose settlement the test controls.
 *
 * The retry's second read is held open on one of these instead of on a timer,
 * so "in flight" is a state the test owns rather than a race against the clock.
 */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};
  let reject: (reason: unknown) => void = () => {};

  const promise = new Promise<T>((settle, fail) => {
    resolve = settle;
    reject = fail;
  });

  return { promise, reject, resolve };
}

/** Open Settings over one held household, with the ordinary reads answered. */
function seedSettings(): void {
  handlers.set('list_households', () => [HOME]);
  handlers.set('get_active_household', () => HOME_ACTIVE);
  answerOtherReads();
}

/** The segments' `radiogroup`, named by the field's label. */
function unitGroup(): HTMLElement {
  return screen.getByRole('radiogroup', { name: UNIT });
}

/**
 * The display-unit section, located from the heading that names it rather than
 * from a class name: `ListSection` labels its `<section>` with that heading, so
 * the absence assertions below are scoped to a real named group.
 */
function unitSection(): HTMLElement {
  const heading = screen.getByRole('heading', { name: UNIT });
  const section = heading.closest('section');

  if (section === null) {
    throw new Error('the Weight unit heading is not inside a section');
  }

  return section;
}

/**
 * The Households section, located the same way: `ListSection` labels its
 * `<section>` with the heading that names it, so "the section reports the read"
 * is scoped to a real named group rather than to the whole screen — which also
 * carries the profile and display-unit sections.
 */
function householdsSection(): HTMLElement {
  const heading = screen.getByRole('heading', {
    name: t('settings.households.heading'),
  });
  const section = heading.closest('section');

  if (section === null) {
    throw new Error('the Households heading is not inside a section');
  }

  return section;
}

/** Open Settings as its own route, on the app's committed route tree. */
async function openSettings() {
  const client = createTestQueryClient();
  const router = createTestRouter({
    client,
    initialEntries: ['/settings'],
  });

  await router.load();
  renderWithQueryClient(<RouterProvider router={router} />, { client });
  await screen.findByRole('heading', { name: 'Settings & Profile' });

  return router;
}

describeAtBothBaselines('Settings: held households (HS-4)', () => {
  describe('"All households are listed with the active one indicated"', () => {
    it('lists every household with one active marker', async () => {
      handlers.set('list_households', () => [HOME, BEACH]);
      handlers.set('get_active_household', () => HOME_ACTIVE);
      answerOtherReads();

      await openSettings();

      expect(await screen.findByText('Home household')).toBeVisible();
      expect(screen.getByText('Beach house')).toBeVisible();

      const current = document.querySelectorAll('[aria-current="true"]');

      expect(current).toHaveLength(1);

      const activeRow = current[0] as HTMLElement;

      expect(within(activeRow).getByText('Home household')).toBeVisible();
      expect(within(activeRow).getByText('Active')).toBeVisible();
      expect(within(activeRow).queryByText('Owner')).toBeNull();

      // The owner marker follows `is_owner`, not the active flag: the other
      // household is owned by this person and stays switchable.
      const other = screen.getByRole('button', { name: /Beach house/ });

      expect(within(other).getByText('Owner')).toBeVisible();
      expect(within(other).getByText('Make active')).toBeVisible();
      expect(within(other).queryByText('Active')).toBeNull();

      // Exactly one indication of each fact.
      expect(screen.getAllByText('Active')).toHaveLength(1);
      expect(screen.getAllByText('Owner')).toHaveLength(1);
    });
  });

  describe('"Create an additional household without onboarding"', () => {
    it('adds the household in place and stays on Settings', async () => {
      let households: HouseholdSummary[] = [HOME];
      let active: Household | null = HOME_ACTIVE;

      handlers.set('list_households', () => households);
      handlers.set('get_active_household', () => active);
      answerOtherReads();
      handlers.set('create_household', (args) => {
        const created = createdHousehold(args);

        active = created;
        households = [{ ...HOME, is_active: false }, createdRow(created)];

        return created;
      });

      const router = await openSettings();

      await screen.findByText('Home household');

      const user = userEvent.setup();
      const open = screen.getByRole('button', {
        name: 'Create another household',
      });

      await user.click(open);

      const field = screen.getByRole('textbox', { name: /Household name/ });

      await user.type(field, 'Beach house');
      await user.click(screen.getByRole('button', { name: CREATE }));

      await screen.findByText('Beach house');

      // The form closed and the location never moved: the first-run flow is not
      // presented and the dashboard is not visited on the way.
      expect(
        screen.queryByRole('textbox', { name: /Household name/ }),
      ).toBeNull();
      expect(router.state.location.pathname).toBe('/settings');
      expect(
        screen.queryByRole('heading', { name: 'Create your household' }),
      ).toBeNull();

      const creates = createCalls();

      expect(creates).toHaveLength(1);
      expect(creates[0].args.name).toBe('Beach house');

      const current = document.querySelectorAll('[aria-current="true"]');

      expect(current).toHaveLength(1);
      expect(
        within(current[0] as HTMLElement).getByText('Beach house'),
      ).toBeVisible();
    });

    it('re-sends the same request_id when retried', async () => {
      let households: HouseholdSummary[] = [HOME];
      let active: Household | null = HOME_ACTIVE;
      let attempts = 0;

      handlers.set('list_households', () => households);
      handlers.set('get_active_household', () => active);
      answerOtherReads();
      handlers.set('create_household', (args) => {
        attempts += 1;

        if (attempts === 1) {
          throw {
            cause: 'connectivity',
            retryable: true,
            message_key: 'error.connectivity',
            request_id: args.requestId,
          };
        }

        const created = createdHousehold(args);

        active = created;
        households = [{ ...HOME, is_active: false }, createdRow(created)];

        return created;
      });

      const router = await openSettings();

      await screen.findByText('Home household');

      const user = userEvent.setup();

      await user.click(
        screen.getByRole('button', { name: 'Create another household' }),
      );

      const field = screen.getByRole('textbox', { name: /Household name/ });

      await user.type(field, 'Beach house');
      await user.click(screen.getByRole('button', { name: CREATE }));

      const notice = await screen.findByRole('alert');

      expect(notice).toHaveTextContent(
        'No connection. Check your internet connection and try again.',
      );

      // A refused create does not close the form or move the route.
      expect(router.state.location.pathname).toBe('/settings');

      await user.click(screen.getByRole('button', { name: 'Try again' }));

      await screen.findByText('Beach house');

      const creates = createCalls();

      expect(creates).toHaveLength(2);
      expect(creates[0].args.name).toBe('Beach house');
      expect(creates[1].args.name).toBe('Beach house');
      expect(creates[1].args.requestId).toBe(creates[0].args.requestId);
    });
  });

  /*
   * Task 7.4 / Q45: HS-4's "All households are listed with the active one
   * indicated" survives a retry of a failed read. The retry refetches the
   * held-set pair — the list and the active read its marker comes from — not
   * the list alone. The fix's entire content is *which two reads* the retry
   * performs, so both counts are asserted; a regression that refreshed the
   * list only would pass every other assertion in this suite.
   *
   * The active read answers normally (`HOME_ACTIVE`) on mount, so the cached
   * marker exists when the list fails: the read failure is the list's alone,
   * which is the state Q45 describes. The retried list carries both held
   * households, so the marker assertion is made over a real set rather than a
   * one-row list.
   */
  describe('HS-4: retrying a failed read', () => {
    it('re-reads the held-set pair and keeps one active marker', async () => {
      let listAttempts = 0;

      seedSettings();
      handlers.set('list_households', () => {
        listAttempts += 1;

        if (listAttempts === 1) {
          throw connectivityFailure(null);
        }

        return [HOME, BEACH];
      });

      await openSettings();

      const notice = await screen.findByRole('alert');

      // The read's cause, under the section, with no "Not saved" header: the
      // failure is a read, not a write.
      expect(notice).toHaveTextContent(t('error.connectivity'));
      expect(within(notice).getByRole('button', { name: RETRY })).toBeVisible();

      // One read of each on mount: the list's attempt failed, the active read
      // answered and supplies the marker's data.
      const listsBefore = listCalls().length;
      const activesBefore = activeCalls().length;

      expect(listsBefore).toBe(1);
      expect(activesBefore).toBe(1);

      const user = userEvent.setup();

      /*
       * The click happens before any refetch starts, so it needs no enabled
       * button during flight. Nothing below reads the control's enabled or
       * disabled state: task 7.5 measured that the control is unmounted for the
       * whole flight (the case below asserts that), so this regression holds
       * without depending on any busy state.
       */
      await user.click(screen.getByRole('button', { name: RETRY }));

      /*
       * The pair, not the list. Each count is asserted to have grown by exactly
       * one so a retry that refetched only the list fails on the second line.
       */
      await waitFor(() => {
        expect(listCalls()).toHaveLength(listsBefore + 1);
        expect(activeCalls()).toHaveLength(activesBefore + 1);
      });

      // The re-read list settled and the failure is gone.
      expect(await screen.findByText('Beach house')).toBeVisible();
      expect(screen.queryByRole('alert')).toBeNull();

      // HS-4's outcome after the pair settles: exactly one active marker, on
      // the active household's row, and one Active badge in the section.
      const current = document.querySelectorAll('[aria-current="true"]');

      expect(current).toHaveLength(1);

      const activeRow = current[0] as HTMLElement;

      expect(within(activeRow).getByText('Home household')).toBeVisible();
      expect(within(activeRow).getByText('Active')).toBeVisible();
      expect(screen.getAllByText('Active')).toHaveLength(1);
    });

    /*
     * Task 7.5 / Q46: what the retry looks like *while* it runs, and what it
     * leaves behind when the re-read fails again.
     *
     * There is deliberately **no busy-button assertion**, and none is possible:
     * starting the pair's refetch clears the failed read's error, so
     * `useSettingsHouseholdList` reports `pending`, the notice — the `Try again`
     * control inside it — unmounts, and the section shows its `LoadingIndicator`
     * until the list read settles. The control is not rendered while the list is
     * in flight, which is exactly the mechanism that stops a second tap from
     * starting a second pair; a disabled or `aria-busy` retry would be a state
     * this screen never reaches (`Button`'s `loading` form is not used here).
     *
     * The status is asserted with its copy, not only its role: task 7.6 made a
     * label required and the section announces `settings.households.loading`,
     * so "the screen says it is working" is a fact here rather than an inference
     * from a live region's presence.
     *
     * The second read is held on a deferred promise rather than a timer, so the
     * flight is a state the test enters and leaves deliberately: the same
     * `connectivity` failure the first attempt got is rejected into it, and the
     * notice must come back with its copy and an idle, enabled control.
     */
    it('removes the retry while the re-read is in flight', async () => {
      const refetch = deferred<HouseholdSummary[]>();

      let listAttempts = 0;

      seedSettings();
      handlers.set('list_households', () => {
        listAttempts += 1;

        if (listAttempts === 1) {
          throw connectivityFailure(null);
        }

        // The retry's answer is this test's to hold and settle.
        return refetch.promise;
      });

      await openSettings();

      const notice = await screen.findByRole('alert');

      expect(notice).toHaveTextContent(t('error.connectivity'));
      expect(within(notice).getByRole('button', { name: RETRY })).toBeVisible();

      const user = userEvent.setup();

      await user.click(screen.getByRole('button', { name: RETRY }));

      // The tap issued the second read and it has not answered: the list is
      // held on the deferred promise. Awaiting the call itself is what makes
      // "in flight" a fact here rather than a race against a settled read.
      await waitFor(() => {
        expect(listCalls()).toHaveLength(2);
      });

      // The failure notice, and with it the retry control, is unmounted. The
      // failure is the only source of the section's `alert`, and the list is
      // held, so this also means the section settled into `pending` rather than
      // into rows.
      await waitFor(() => {
        expect(screen.queryByRole('alert')).toBeNull();
      });

      /*
       * Nothing named the retry label exists anywhere on the screen — not a
       * button and not a link, so there is no second path to the same re-read
       * either. This is the double-tap guard Q46 asked for, stated as the
       * absence it is.
       */
      expect(screen.queryAllByRole('button', { name: RETRY })).toHaveLength(0);
      expect(screen.queryAllByRole('link', { name: RETRY })).toHaveLength(0);

      /*
       * The Households section reports the read as running, through the
       * primitive's polite status. Since 7.6 the label is required, so the copy
       * is part of the contract: the screen has to *say* it is working, not
       * only expose a live region.
       */
      const status = within(householdsSection()).getByRole('status');

      expect(status).toBeVisible();
      expect(status).toHaveTextContent(t('settings.households.loading'));

      /*
       * The re-read fails the same way the first attempt did. The flight ends
       * where it started: the notice returns with its cause and an idle retry,
       * so the failure the person is looking at is still actionable.
       */
      await act(async () => {
        refetch.reject(connectivityFailure(null));
      });

      const again = await screen.findByRole('alert');
      const retry = within(again).getByRole('button', { name: RETRY });

      expect(again).toHaveTextContent(t('error.connectivity'));

      // The pending state ended: the section's only read report is the notice.
      expect(within(householdsSection()).queryByRole('status')).toBeNull();

      expect(retry).toBeEnabled();
      expect(retry).not.toHaveAttribute('aria-busy');
    });
  });
});

/*
 * Task 7.3 (SET-4, Q44). The unit pair's failure has no retry to offer, so the
 * recovery is the segment itself. These tests assert the outcome as a person
 * meets it: the cause's copy, no control named the retry label, and the
 * segments still there to be chosen again — the last one being what "the
 * recovery is not a retry button" means in the UI.
 */
describeAtBothBaselines('Settings: display unit (SET-4, Q44)', () => {
  // SET-4: "Display weight unit is per-device and applied at the boundary".
  describe('Display unit is per-device (SET-4)', () => {
    it('shows the unknown copy and never looks saved', async () => {
      seedSettings();
      handlers.set('set_display_weight_unit', (args) => {
        throw unitFailure(args.requestId);
      });

      await openSettings();

      // The unit read settles after the screen: await its control.
      await screen.findByRole('radio', { name: 'Grams' });

      const user = userEvent.setup();

      // The backend's effective unit before the change.
      expect(
        within(unitGroup()).getByRole('radio', { name: 'Grams' }),
      ).toBeChecked();

      await user.click(
        within(unitGroup()).getByRole('radio', { name: 'Kilograms' }),
      );

      const notice = await screen.findByRole('alert');

      // The cause's own copy under the "not saved" header: the backend's raw
      // key is not what the surface renders (task 3.4's rule).
      expect(
        within(notice).getByRole('heading', { name: t('error.notSaved') }),
      ).toBeVisible();
      expect(notice).toHaveTextContent(UNIT_FAILURE_COPY);

      // A refused write writes nothing: the unit the backend confirmed is still
      // the checked segment, and the requested one is not shown as stored.
      expect(
        within(unitGroup()).getByRole('radio', { name: 'Grams' }),
      ).toBeChecked();
      expect(
        within(unitGroup()).getByRole('radio', { name: 'Kilograms' }),
      ).not.toBeChecked();

      // Nor does the section still claim the change is in flight.
      expect(screen.queryByText(SAVING)).toBeNull();

      const writes = setUnitWrites();

      expect(writes).toHaveLength(1);
      expect(writes[0].args.unit).toBe('kilograms');
    });

    it('offers no retry control for the refused change', async () => {
      seedSettings();
      handlers.set('set_display_weight_unit', (args) => {
        throw unitFailure(args.requestId);
      });

      await openSettings();

      // The unit read settles after the screen: await its control.
      await screen.findByRole('radio', { name: 'Grams' });

      const user = userEvent.setup();

      await user.click(
        within(unitGroup()).getByRole('radio', { name: 'Kilograms' }),
      );

      const notice = await screen.findByRole('alert');
      const section = unitSection();
      const inNotice = within(notice);
      const inSection = within(section);

      /*
       * The failure surface carries no action at all: `unknown` is not
       * retryable, and the unit section passes no dismissal either. A control
       * that re-sent the same request would have to be here.
       */
      expect(inNotice.queryByRole('button', { name: RETRY })).toBeNull();
      expect(inNotice.queryAllByRole('button')).toHaveLength(0);
      expect(inNotice.queryAllByRole('link')).toHaveLength(0);

      /*
       * The section's only controls are the two segments. Nothing is named the
       * retry label, and there is no button or link at all — no second path
       * that could re-send the refused attempt.
       */
      expect(inSection.queryByRole('button', { name: RETRY })).toBeNull();
      expect(inSection.queryByRole('link', { name: RETRY })).toBeNull();
      expect(inSection.queryAllByRole('button')).toHaveLength(0);
      expect(inSection.queryAllByRole('link')).toHaveLength(0);
      expect(inSection.getAllByRole('radio')).toHaveLength(2);

      /*
       * And the screen as a whole offers none: the unit failure is the only
       * failure on it, so a "Try again" anywhere would be a retry for it.
       */
      expect(screen.queryAllByRole('button', { name: RETRY })).toHaveLength(0);
      expect(screen.queryAllByRole('link', { name: RETRY })).toHaveLength(0);

      // Nothing re-sent the request on its own, either.
      expect(setUnitWrites()).toHaveLength(1);
    });

    it('keeps the segments operable for re-selecting', async () => {
      let attempts = 0;

      seedSettings();
      handlers.set('set_display_weight_unit', (args) => {
        attempts += 1;

        if (attempts === 1) {
          throw unitFailure(args.requestId);
        }

        // The backend stored the choice and read the effective unit back.
        return 'kilograms';
      });

      await openSettings();

      // The unit read settles after the screen: await its control.
      await screen.findByRole('radio', { name: 'Grams' });

      const user = userEvent.setup();

      await user.click(
        within(unitGroup()).getByRole('radio', { name: 'Kilograms' }),
      );

      await screen.findByRole('alert');

      /*
       * The copy says "try again"; the control that does it is the segment
       * itself. A disabled group would leave the person with no recovery at
       * all, which is the failure mode the absence of a retry must not create.
       */
      for (const radio of within(unitGroup()).getAllByRole('radio')) {
        expect(radio).toBeEnabled();
      }

      await user.click(
        within(unitGroup()).getByRole('radio', { name: 'Kilograms' }),
      );

      // One wait on the outcome itself: the mutation clears `error` on the next
      // render, before `onSuccess` resolves, so "the alert is gone" is not
      // evidence the write landed.
      await waitFor(() => {
        expect(
          within(unitGroup()).getByRole('radio', { name: 'Kilograms' }),
        ).toBeChecked();
      });

      // The refusal notice is gone with the confirmed write.
      expect(screen.queryByRole('alert')).toBeNull();

      const writes = setUnitWrites();

      expect(writes).toHaveLength(2);
      expect(writes.map((call) => call.args.unit)).toEqual([
        'kilograms',
        'kilograms',
      ]);

      /*
       * A re-selection is a new choice, so it mints its own operation identity
       * (D0.14). Nothing on this section replays the refused attempt under its
       * `request_id` — that would be the retry Q44 says is not offered.
       */
      expect(writes[1].args.requestId).not.toBe(writes[0].args.requestId);
    });

    it('shows the unknown copy and no retry on a failed read', async () => {
      seedSettings();
      handlers.set('get_display_weight_unit', () => {
        throw unitFailure(null);
      });

      await openSettings();

      // This arrangement fails the read, so the settled element is the notice:
      // await it before scoping the absence assertions to its section.
      const notice = await screen.findByRole('alert');
      const section = unitSection();
      const inNotice = within(notice);

      expect(section).toContainElement(notice);

      /*
       * No "Not saved" header here: nothing was written. The read-failure
       * branch renders `Notice` without a title and names the read's cause in
       * its body. A read that never wrote must not claim a save outcome, so
       * the header is asserted absent rather than merely unmentioned.
       */
      expect(notice).toHaveTextContent(UNIT_FAILURE_COPY);
      expect(within(notice).queryByRole('heading')).toBeNull();

      /*
       * A read whose cause the backend marked non-retryable gets no action: no
       * re-read is promised to fix it, and the recovery is the next mount.
       */
      expect(inNotice.queryByRole('button', { name: RETRY })).toBeNull();
      expect(inNotice.queryAllByRole('button')).toHaveLength(0);
      expect(within(section).queryAllByRole('button')).toHaveLength(0);
      expect(within(section).queryAllByRole('link')).toHaveLength(0);
      expect(screen.queryAllByRole('button', { name: RETRY })).toHaveLength(0);
    });
  });
});
