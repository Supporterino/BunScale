/*
 * SYNC-3's offline surfaces at the component layer (task 5.13).
 *
 * What is asserted, through the app's committed route tree so the entry guard
 * and the screens are wired as they are in a launch:
 *
 *   - "Nothing loaded and unreachable" — the explicit connectivity-required
 *     screen is what the guard reaches (`data-household-entry="service-required"`),
 *     it names connectivity in its own words, and it carries the observed
 *     status. It is neither "no household" (no empty-household state, no
 *     onboarding) nor empty data (no dashboard content). Its one action asks
 *     the backend for a fresh check — none on arrival, exactly one per press;
 *   - "Previously loaded data is shown while unreachable" — the local-copy
 *     notice appears for a collection the device has loaded (`source: local`
 *     with `loaded_at` present), the household's own content stays on screen
 *     beside it, and the notice is absent for the never-loaded value
 *     (`loaded_at: null`) and for a read that came from the service. The notice
 *     is a polite status with no action: a stale copy is a fact to state, not a
 *     failure to fix.
 *
 * Plus the account half of the same screen, which the copy branches on: a
 * person who is online and whose account the service refused
 * (`account-unusable`), and one whose account was not determined
 * (`account-undetermined`), are both told the service cannot be used right
 * now — never told to connect to the internet.
 *
 * What is NOT assertable at this layer, and who owns it:
 *   - the three-failure offline threshold, the probe timeout and the
 *     "re-evaluate on foreground/restore" timing are backend and OS facts:
 *     Rust 5.1–5.10 and the native lifecycle checks own them. Here the status
 *     is an input, so what the *surface* does with it is the whole claim;
 *   - the arranged layout, the offline badge's position, safe areas and touch
 *     targets are browser facts. jsdom has no layout engine — geometry reads
 *     zeroes — and loads no app stylesheet, so E2E 5.15 owns the visual offline
 *     launch at both viewports.
 *
 * The suite runs at both component baselines (390×844 and 1280×800) through
 * `describeAtBothBaselines`; nothing here branches on the viewport. Both
 * platforms share this layer: nothing here is per-OS.
 *
 * IPC is mocked at `invoke` (`testing-strategy`), and the event plugin is
 * mocked because the dashboard shell mounts `useAppEvents`.
 */

import { RouterProvider } from '@tanstack/react-router';
import type { InvokeArgs } from '@tauri-apps/api/core';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { cleanup, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { householdKeys } from '../features/households/keys';
import type {
  Household,
  HouseholdCollection,
  HouseholdSummary,
} from '../features/households/queries';
import { LocalCopyNotice } from '../features/sync/components/LocalCopyNotice';
import type { ServiceStatus } from '../features/sync/entry-status';
import {
  createTestQueryClient,
  createTestRouter,
  describeAtBothBaselines,
  renderWithQueryClient,
} from './setup';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

/** The connectivity screen's heading and body (`connectivity.required.*`). */
const REQUIRED_TITLE = 'Connect to continue';
const REQUIRED_BODY =
  'Your household data has not been loaded on this device yet. Connect to the internet and try again.';

/**
 * The neutral heading for a person who is online but cannot use the service
 * (`connectivity.account.title`): never "connect".
 */
const ACCOUNT_TITLE = 'This device cannot use the shared service right now.';

/** The three bodies the account half can show. */
const NO_ACCOUNT_BODY =
  'Your account is not available right now. Try again later.';
const TIMEOUT_BODY = 'The service took too long to answer. Try again.';

/**
 * The unclassified cause's body (`error.unknown`).
 *
 * Its wording is operation-neutral on purpose (Q60): the same key is also the
 * body of a failed load in `HouseholdEntryScreens`, so it may not describe a
 * change that no surface here attempted.
 */
const UNKNOWN_BODY = 'That did not work. Nothing was changed.';

/** The previously-loaded-copy notice (`connectivity.localCopy`). */
const LOCAL_COPY =
  'Showing saved data. Recent changes from other members may not be here yet.';

/** The household the device holds; its dashboard proves a household exists. */
const HOME: HouseholdSummary = {
  id: 'household-home',
  name: 'Home household',
  created_at: AT,
  is_active: true,
  is_owner: true,
  member_count: 1,
};

/** The same household as the active read answers it. */
const HOME_ACTIVE: Household = {
  id: HOME.id,
  name: HOME.name,
  created_at: HOME.created_at,
  owner_member_id: 'member-1',
  is_active: true,
  is_owner: true,
};

/** The collection as the device's own copy answers it while unreachable. */
const LOADED_LOCAL: HouseholdCollection = {
  household_id: HOME.id,
  items: [],
  loaded_at: AT,
  source: 'local',
};

/** Nothing was ever loaded here: the explicit never-loaded value (D0.2). */
const NEVER_LOADED: HouseholdCollection = {
  household_id: HOME.id,
  items: [],
  loaded_at: null,
  source: 'local',
};

/** A read whose refresh reached the service: not a local copy. */
const LOADED_FROM_SERVICE: HouseholdCollection = {
  household_id: HOME.id,
  items: [],
  loaded_at: AT,
  source: 'service',
};

/** Three failed checks without a success: the backend's offline state. */
const OFFLINE: ServiceStatus = {
  state: 'unreachable',
  account_valid: null,
  cause: 'connectivity',
  observed_at: AT,
};

/** The service answered and refused the account (SYNC-2, SYNC-3). */
const ACCOUNT_UNUSABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: false,
  cause: 'no_account',
  observed_at: AT,
};

/** The probe answered without determining the account. */
const ACCOUNT_UNDETERMINED: ServiceStatus = {
  state: 'reachable',
  account_valid: null,
  cause: 'timeout',
  observed_at: AT,
};

/** An answered probe that classified no cause at all. */
const ACCOUNT_UNCLASSIFIED: ServiceStatus = {
  state: 'reachable',
  account_valid: null,
  cause: null,
  observed_at: AT,
};

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

let calls: MockCall[];
let handlers: Map<string, MockHandler>;

/** Every `check_service_reachability` call, in order. */
function probeCalls(): MockCall[] {
  return calls.filter((call) => call.command === 'check_service_reachability');
}

/** Every `load_household` call, in order. */
function collectionCalls(): MockCall[] {
  return calls.filter((call) => call.command === 'load_household');
}

/** The guard's answers for "no household, and here is the status". */
function answerNoHousehold(status: ServiceStatus): void {
  handlers.set('list_households', () => []);
  handlers.set('get_active_household', () => null);
  handlers.set('get_service_status', () => status);
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
  /*
   * Unmount first: the dashboard releases its `listen` subscriptions on
   * unmount, and those calls need the mocked bridge. The awaits let the
   * released promises settle before `clearMocks` removes `invoke`.
   */
  cleanup();
  await Promise.resolve();
  await Promise.resolve();

  clearMocks();
});

/** Boot the app's real route tree at `/`, as a launch would. */
async function launch() {
  const client = createTestQueryClient();
  const router = createTestRouter({ client });

  await router.load();
  renderWithQueryClient(<RouterProvider router={router} />, { client });

  return router;
}

describeAtBothBaselines('Connectivity surfaces (SYNC-3)', () => {
  describe('"Nothing loaded and unreachable"', () => {
    it('shows the explicit required state, not empty data', async () => {
      answerNoHousehold(OFFLINE);

      await launch();

      const main = await screen.findByRole('main');

      expect(main).toHaveAttribute('data-household-entry', 'service-required');
      expect(main).toHaveAttribute('data-service-state', 'unreachable');
      expect(main).toHaveAttribute('data-service-cause', 'connectivity');

      expect(
        await screen.findByRole('heading', { name: REQUIRED_TITLE }),
      ).toBeVisible();
      expect(screen.getByText(REQUIRED_BODY)).toBeVisible();
      expect(main).toHaveTextContent('Offline');

      // Not "no household": neither the empty-household copy nor onboarding.
      expect(screen.queryByText('No household on this device')).toBeNull();
      expect(
        screen.queryByRole('heading', { name: 'Create your household' }),
      ).toBeNull();

      // Not empty data either: the dashboard and its content are not rendered.
      expect(
        screen.queryByRole('heading', { name: 'Your household' }),
      ).toBeNull();
      expect(main).not.toHaveTextContent(/shared space/);
    });

    it('offers exactly one check per press', async () => {
      answerNoHousehold(OFFLINE);
      handlers.set('check_service_reachability', () => OFFLINE);

      const router = await launch();

      await screen.findByRole('heading', { name: REQUIRED_TITLE });

      // Arriving does not probe: the guard already had an observation, and the
      // screen's action is the person's to press.
      expect(probeCalls()).toHaveLength(0);

      const user = userEvent.setup();

      await user.click(screen.getByRole('button', { name: 'Check again' }));

      await waitFor(() => {
        expect(probeCalls()).toHaveLength(1);
      });

      // The outcome is announced politely, and the screen stays put: a check
      // that changes nothing must not look like nothing happened.
      await waitFor(() => {
        expect(screen.getByRole('status')).toHaveTextContent(REQUIRED_BODY);
      });

      expect(router.state.location.pathname).toBe('/');

      await user.click(
        await screen.findByRole('button', { name: 'Check again' }),
      );

      await waitFor(() => {
        expect(probeCalls()).toHaveLength(2);
      });

      expect(screen.getByRole('main')).toHaveAttribute(
        'data-household-entry',
        'service-required',
      );
    });
  });

  describe('"Previously loaded data is shown while unreachable"', () => {
    it('shows the local-copy notice beside the content', async () => {
      handlers.set('list_households', () => [HOME]);
      handlers.set('get_active_household', () => HOME_ACTIVE);
      handlers.set('get_service_status', () => OFFLINE);
      handlers.set('load_household', () => LOADED_LOCAL);

      await launch();

      expect(
        await screen.findByRole('heading', { name: HOME.name }),
      ).toBeVisible();

      /*
       * Located by its own copy, not by a bare `role="status"`: the dashboard
       * mounts a persistent, empty polite region for its empty-state
       * announcement (a live region must exist before its content changes), so
       * the first `status` is that announcer, not this notice. The copy is
       * unique on the screen and the announcer is always empty here, so the
       * text match cannot land on the wrong region — and the enclosing status
       * region is asserted, so the notice cannot stop being a polite status.
       */
      const copy = await screen.findByText(LOCAL_COPY);
      const notice = copy.closest<HTMLElement>('[role="status"]');

      if (notice === null) {
        throw new Error('the local-copy notice is not inside a status region');
      }

      expect(notice).toHaveTextContent(LOCAL_COPY);

      // Beside, never instead: the screen's own content is still presented.
      expect(screen.getByRole('main')).toHaveTextContent(/shared space/);

      // Non-blocking: a stale copy is stated, not something to act on here.
      expect(within(notice).queryByRole('button')).toBeNull();
    });

    it('omits the notice when nothing was ever loaded', async () => {
      const client = createTestQueryClient();

      client.setQueryData(householdKeys.collection(HOME.id), NEVER_LOADED);

      handlers.set('load_household', () => NEVER_LOADED);

      const { container } = renderWithQueryClient(
        <LocalCopyNotice householdId={HOME.id} />,
        { client },
      );

      await waitFor(() => {
        expect(collectionCalls()).toHaveLength(1);
      });

      // The never-loaded read is the connectivity screen's job, not a copy.
      expect(container).toBeEmptyDOMElement();
    });

    it('omits the notice when the read came from the service', async () => {
      const client = createTestQueryClient();

      client.setQueryData(
        householdKeys.collection(HOME.id),
        LOADED_FROM_SERVICE,
      );

      handlers.set('load_household', () => LOADED_FROM_SERVICE);

      const { container } = renderWithQueryClient(
        <LocalCopyNotice householdId={HOME.id} />,
        { client },
      );

      await waitFor(() => {
        expect(collectionCalls()).toHaveLength(1);
      });

      expect(container).toBeEmptyDOMElement();
    });
  });

  describe('Account branches', () => {
    it('names a refused account instead of connectivity', async () => {
      answerNoHousehold(ACCOUNT_UNUSABLE);

      await launch();

      expect(
        await screen.findByRole('heading', { name: ACCOUNT_TITLE }),
      ).toBeVisible();
      expect(screen.getByText(NO_ACCOUNT_BODY)).toBeVisible();
      expect(screen.getByRole('main')).toHaveAttribute(
        'data-service-cause',
        'no_account',
      );

      // The person is online; asking them to connect would be false.
      expect(screen.queryByText(REQUIRED_TITLE)).toBeNull();
      expect(screen.queryByText(REQUIRED_BODY)).toBeNull();
      expect(screen.queryByText('Offline')).toBeNull();
    });

    it('names an undetermined account instead of connectivity', async () => {
      answerNoHousehold(ACCOUNT_UNDETERMINED);

      await launch();

      expect(
        await screen.findByRole('heading', { name: ACCOUNT_TITLE }),
      ).toBeVisible();
      expect(screen.getByText(TIMEOUT_BODY)).toBeVisible();

      expect(screen.queryByText(REQUIRED_BODY)).toBeNull();
      expect(screen.queryByText('Offline')).toBeNull();
    });

    it('falls back to the unknown cause when none was named', async () => {
      answerNoHousehold(ACCOUNT_UNCLASSIFIED);

      await launch();

      expect(
        await screen.findByRole('heading', { name: ACCOUNT_TITLE }),
      ).toBeVisible();

      /*
       * No cause was classified, so the neutral wording is the honest one —
       * honest on a read/status surface too, because nothing was attempted
       * here that could have changed and the sentence does not claim one.
       */
      expect(screen.getByText(UNKNOWN_BODY)).toBeVisible();

      expect(screen.queryByText(REQUIRED_BODY)).toBeNull();
    });
  });
});
