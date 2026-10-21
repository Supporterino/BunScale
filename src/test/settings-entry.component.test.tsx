/*
 * SET-1 at the component layer (task 5.12): the dashboard's gear and Settings as
 * its own screen.
 *
 * What is asserted — the DOM's own facts, through the app's committed route
 * tree, so the gear's destination is the real `/settings` route:
 *
 *   - the gear is a labelled control (its accessible name is
 *     `t('dashboard.settings')`, never a glyph with no name);
 *   - activating it lands on `/settings`, the Settings title is the screen's
 *     `h1`, and the dashboard's title is gone: Settings replaces the dashboard
 *     rather than stacking under it (`[mobile]`);
 *   - arriving moves focus into the new `main` region, so a keyboard or
 *     screen-reader user does not restart at `<body>`;
 *   - a control returns to the dashboard (`[mobile]`), and the dashboard
 *     destination is still offered while Settings is open (`[desktop]`).
 *
 * What is NOT assertable at this layer, and who owns it:
 *   - the 44×44 touch target is geometry: jsdom has no layout engine, so
 *     `getBoundingClientRect()` reports zeroes — E2E 5.14 asserts the size;
 *   - *which* navigation arrangement is laid out (tab bar at 390×844, sidebar at
 *     1280×800) is a CSS media-query fact. Both landmarks are in the DOM under
 *     jsdom and the app stylesheet is not loaded in a Vitest run, so the
 *     arrangement is E2E's claim (`src/components/ui/navigation.ts`). This spec
 *     scopes by `data-navigation-arrangement` and lets `describeAtBothBaselines`
 *     pick the landmark the baseline's form factor uses for the interaction,
 *     without pretending to know which one is displayed.
 *
 * The suite runs at both component baselines (390×844 and 1280×800) through
 * `describeAtBothBaselines`; IPC is mocked at `invoke`, and the event plugin is
 * mocked because both screens mount `useAppEvents`. Both platforms share this
 * layer: nothing here is per-OS.
 */

import { RouterProvider } from '@tanstack/react-router';
import type { InvokeArgs } from '@tauri-apps/api/core';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { cleanup, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, it } from 'vitest';
import type { NavigationArrangement } from '../components/ui/navigation';
import type {
  Household,
  HouseholdSummary,
} from '../features/households/queries';
import type { ServiceStatus } from '../features/sync/entry-status';
import {
  createTestQueryClient,
  createTestRouter,
  describeAtBothBaselines,
  renderWithQueryClient,
} from './setup';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

/** The gear's accessible name, `t('dashboard.settings')`. */
const GEAR = 'Open Settings & Profile';

/** The dashboard destination's visible label, `t('dashboard.title')`. */
const DASHBOARD = 'Your household';

/** The attribute each navigation primitive tags its own landmark with. */
const ARRANGEMENT_ATTR = 'data-navigation-arrangement';

/** The held household, which the dashboard is titled with. */
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

/** A fresh reachable observation; no offline notice is rendered with it. */
const REACHABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: true,
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
  // Unmount while the mocked bridge is still installed (both screens release
  // their `listen` subscriptions on unmount), then drop the mock.
  cleanup();
  await Promise.resolve();
  await Promise.resolve();

  clearMocks();
});

/**
 * One arrangement's landmark. Scoped by `data-navigation-arrangement`, which is
 * a constant per primitive — never a class name, and never a claim about which
 * one the stylesheet lays out.
 */
function navigationLandmark(arrangement: NavigationArrangement): HTMLElement {
  const landmarks = screen.getAllByRole('navigation');
  const landmark = landmarks.find(
    (candidate) => candidate.getAttribute(ARRANGEMENT_ATTR) === arrangement,
  );

  if (landmark === undefined) {
    throw new Error(`no ${arrangement} navigation landmark was rendered`);
  }

  return landmark;
}

/** Boot a launch that holds one household, as `src/routes/index.tsx` does. */
async function openDashboard() {
  handlers.set('list_households', () => [HOME]);
  handlers.set('get_active_household', () => HOME_ACTIVE);
  handlers.set('get_identity', () => ({
    account_user_id: 'account-1',
    display_name: 'Ana',
    resolved: true,
  }));
  handlers.set('get_display_weight_unit', () => 'grams');
  handlers.set('get_service_status', () => REACHABLE);

  const client = createTestQueryClient();
  const router = createTestRouter({ client, initialEntries: ['/'] });

  await router.load();
  renderWithQueryClient(<RouterProvider router={router} />, { client });
  await screen.findByRole('heading', { name: HOME.name });

  return router;
}

/** Open Settings the way SET-1 says a person does: by the gear. */
async function openSettingsFromGear() {
  const router = await openDashboard();
  const user = userEvent.setup();

  await user.click(screen.getByRole('button', { name: GEAR }));

  await waitFor(() => {
    expect(router.state.location.pathname).toBe('/settings');
  });
  await screen.findByRole('heading', { name: 'Settings & Profile' });

  return { router, user };
}

describeAtBothBaselines(
  'Settings & Profile entry point (SET-1)',
  (baseline) => {
    it('opens Settings as its own screen from the gear control', async () => {
      const router = await openDashboard();
      const gear = screen.getByRole('button', { name: GEAR });

      expect(gear).toHaveAccessibleName(GEAR);

      const user = userEvent.setup();

      await user.click(gear);

      await waitFor(() => {
        expect(router.state.location.pathname).toBe('/settings');
      });

      expect(
        await screen.findByRole('heading', { name: 'Settings & Profile' }),
      ).toBeVisible();

      // Its own screen: the dashboard's title is gone, not stacked underneath.
      expect(screen.queryByRole('heading', { name: HOME.name })).toBeNull();
    });

    it('moves focus into the Settings screen on arrival', async () => {
      await openSettingsFromGear();

      const main = await waitFor(() => {
        const element = document.getElementById('bs-settings-main');

        if (element === null) {
          throw new Error('Settings rendered no main region');
        }

        return element;
      });

      await waitFor(() => {
        expect(document.activeElement).toBe(main);
      });

      expect(main).toHaveAttribute('tabindex', '-1');
    });

    it('returns to the dashboard from Settings [mobile]', async () => {
      const { router, user } = await openSettingsFromGear();

      await user.click(screen.getByRole('button', { name: 'Back' }));

      await waitFor(() => {
        expect(router.state.location.pathname).toBe('/');
      });

      expect(
        await screen.findByRole('heading', { name: HOME.name }),
      ).toBeVisible();
    });

    it('keeps the dashboard reachable while Settings is open', async () => {
      const { router, user } = await openSettingsFromGear();

      // Both arrangements are in the DOM under jsdom; the stylesheet decides which
      // is laid out, and the baseline names the one this form factor uses.
      const arrangement: NavigationArrangement =
        baseline === 'compact' ? 'tabs' : 'sidebar';
      const tabs = navigationLandmark('tabs');
      const sidebar = navigationLandmark('sidebar');

      for (const landmark of [tabs, sidebar]) {
        const link = within(landmark).getByRole('link', { name: DASHBOARD });

        expect(link).toBeVisible();
      }

      const target = navigationLandmark(arrangement);
      const link = within(target).getByRole('link', { name: DASHBOARD });

      await user.click(link);

      await waitFor(() => {
        expect(router.state.location.pathname).toBe('/');
      });

      expect(
        await screen.findByRole('heading', { name: HOME.name }),
      ).toBeVisible();
    });
  },
);
