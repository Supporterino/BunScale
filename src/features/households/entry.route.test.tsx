/*
 * The entry guard at the route layer (task 3.3): HS-2's two scenarios, plus
 * SYNC-3's "cannot tell yet" case, asserted as navigation rather than as copy.
 *
 * The router is the app's own committed tree over memory history, so what is
 * exercised is the real `beforeLoad` wiring in `src/routes/index.tsx` and
 * `src/routes/onboarding.tsx` — including the redirect — and not a decision
 * function called directly (`entry.test.ts` does that). The assertions read
 * `data-household-entry`, the attribute the placeholder screens carry to name
 * the state the guard reached, so they survive the copy and styling that tasks
 * 4.1/4.2/4.4 will bring.
 *
 * No baseline loop: nothing here is viewport-sensitive (no layout, density,
 * geometry or arrangement), so `describeAtBothBaselines` would only run the same
 * navigation twice.
 *
 * IPC is mocked at `invoke` because the route tree loads the household query
 * module eagerly through `src/test/setup.ts`; a wrapper-level mock would arrive
 * after that graph was built (`testing-strategy` prescribes this seam).
 */

import { RouterProvider } from '@tanstack/react-router';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  createTestQueryClient,
  createTestRouter,
  renderWithQueryClient,
} from '../../test/setup';
import type { ServiceStatus } from '../sync/entry-status';
import type { Household, HouseholdSummary } from './queries';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

const HOUSEHOLD: Household = {
  id: 'household-a',
  name: 'Household A',
  created_at: AT,
  owner_member_id: 'member-a',
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

const REACHABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: true,
  cause: null,
  observed_at: AT,
};

const UNREACHABLE: ServiceStatus = {
  state: 'unreachable',
  account_valid: null,
  cause: 'connectivity',
  observed_at: AT,
};

describe('the household entry guard', () => {
  let answers: Map<string, () => unknown>;

  beforeEach(() => {
    answers = new Map();

    mockIPC((command) => {
      const answer = answers.get(command);

      if (answer === undefined) {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return answer();
    });
  });

  afterEach(() => {
    clearMocks();
  });

  /** Boot the app's real route tree at `path`, as a launch would. */
  async function launch(path: string) {
    const client = createTestQueryClient();
    const router = createTestRouter({
      client,
      initialEntries: [path],
    });

    await router.load();
    renderWithQueryClient(<RouterProvider router={router} />, { client });

    return router;
  }

  it('presents the active household’s dashboard and skips onboarding', async () => {
    answers.set('list_households', () => [SUMMARY]);
    answers.set('get_active_household', () => HOUSEHOLD);

    const router = await launch('/');

    await waitFor(() => {
      expect(screen.getByRole('main')).toHaveAttribute(
        'data-household-entry',
        'household',
      );
    });
    expect(router.state.location.pathname).toBe('/');
    expect(answers.has('check_service_reachability')).toBe(false);
  });

  it('presents onboarding when no household exists and the service confirms it', async () => {
    answers.set('list_households', () => []);
    answers.set('get_active_household', () => null);
    answers.set('get_service_status', () => ({
      state: 'unknown',
      account_valid: null,
      cause: null,
      observed_at: null,
    }));
    answers.set('check_service_reachability', () => REACHABLE);

    const router = await launch('/');

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/onboarding');
    });
    await waitFor(() => {
      expect(screen.getByRole('main')).toHaveAttribute(
        'data-household-entry',
        'onboarding',
      );
    });
  });

  it('shows the connectivity slot, not onboarding, when the service cannot confirm', async () => {
    answers.set('list_households', () => []);
    answers.set('get_active_household', () => null);
    answers.set('get_service_status', () => UNREACHABLE);

    const router = await launch('/');

    await waitFor(() => {
      expect(screen.getByRole('main')).toHaveAttribute(
        'data-household-entry',
        'service-required',
      );
    });
    expect(router.state.location.pathname).toBe('/');
    expect(router.state.location.pathname).not.toBe('/onboarding');
  });

  it('does not present onboarding again once a household exists', async () => {
    answers.set('list_households', () => [SUMMARY]);
    answers.set('get_active_household', () => HOUSEHOLD);

    const router = await launch('/onboarding');

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/');
    });
    await waitFor(() => {
      expect(screen.getByRole('main')).toHaveAttribute(
        'data-household-entry',
        'household',
      );
    });
  });
});
