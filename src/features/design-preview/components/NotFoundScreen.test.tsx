/*
 * The component layer's half of TOOL-5 for the root not-found surface of task
 * 0.21, at both baselines through `describeAtBothBaselines`.
 *
 * The surface is reached the way a user reaches it — the app's own route tree at
 * a path no route matches — so `src/routes/__root.tsx`'s `notFoundComponent`
 * wiring is exercised rather than assumed. What belongs here is what the DOM
 * says: the surface composes out of the design-system primitives, it has one
 * `main` and one `h1`, and its single way forward is a real, enabled control.
 *
 * The second test asserts behaviour rather than appearance: the way forward is a
 * route, not `history.back()` (`tanstack-router-conventions`: an in-UI control a
 * cold launch from a bad URL can actually use), so pressing it must leave the
 * not-found surface and land on the start route. Since task 3.3 that route is
 * `/`, the household entry, whose screen depends on the household state — so the
 * assertion is the destination and the surface left behind, never a particular
 * screen's copy, which tasks 4.1/4.2 will own. The household reads are stubbed
 * so the entry resolves deterministically instead of failing on absent IPC: with
 * none mocked, jsdom has no Tauri internals and `/` would render its error
 * boundary, which would make this test pass for the wrong reason.
 *
 * Nothing about the arrangement is asserted here, and this screen is why the
 * distinction is easy to keep: it renders no `AppShell`, so there is no
 * navigation landmark to arrange at all.
 */
import { RouterProvider } from '@tanstack/react-router';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import { createTestRouter, describeAtBothBaselines } from '../../../test/setup';
import type { Household, HouseholdSummary } from '../../households/queries';

vi.mock('../../households/api');

import { getActiveHousehold, listHouseholds } from '../../households/api';

const NOT_FOUND = 'Not found';
const BODY = 'This screen does not exist';
const WAY_FORWARD = 'Go to the start screen';
const SCENARIO =
  'TOOL-5: Component tests exercise the compact baseline — ' +
  'Component tests exercise the expanded baseline';

/** One household, so the entry guard decides "household" without a probe. */
const HOUSEHOLD: Household = {
  id: 'household-a',
  name: 'Household A',
  created_at: '2026-10-20T09:00:00Z',
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

beforeEach(() => {
  vi.mocked(listHouseholds).mockResolvedValue({
    status: 'ok',
    data: [SUMMARY],
  });
  vi.mocked(getActiveHousehold).mockResolvedValue({
    status: 'ok',
    data: HOUSEHOLD,
  });
});

async function renderUnmatchedRoute() {
  const router = createTestRouter({ initialEntries: ['/no-such-route'] });

  await router.load();
  render(<RouterProvider router={router} />);
  await screen.findByRole('heading', { name: NOT_FOUND });

  return router;
}

describeAtBothBaselines(`${SCENARIO} — NotFoundScreen`, () => {
  it('Renders the surface from the design-system primitives', async () => {
    await renderUnmatchedRoute();

    const title = screen.getByRole('heading', { name: NOT_FOUND });
    const body = screen.getByRole('heading', { name: BODY });
    const button = screen.getByRole('button', { name: WAY_FORWARD });
    const main = screen.getByRole('main');

    expect(title).toBeVisible();
    expect(body).toBeVisible();
    expect(button).toBeEnabled();
    expect(main).toBeVisible();
  });

  it('Offers a way forward to the start screen', async () => {
    const router = await renderUnmatchedRoute();

    const user = userEvent.setup();
    const button = screen.getByRole('button', { name: WAY_FORWARD });

    await user.click(button);

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/');
    });
    expect(screen.queryByRole('heading', { name: BODY })).toBeNull();
  });
});
