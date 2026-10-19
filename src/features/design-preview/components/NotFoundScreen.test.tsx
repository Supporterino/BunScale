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
 * cold launch from a bad URL can actually use), so pressing it must land on the
 * start screen's own `h1` and leave the not-found surface behind.
 *
 * Nothing about the arrangement is asserted here, and this screen is why the
 * distinction is easy to keep: it renders no `AppShell`, so there is no
 * navigation landmark to arrange at all.
 */
import { RouterProvider } from '@tanstack/react-router';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it } from 'vitest';
import { createTestRouter, describeAtBothBaselines } from '../../../test/setup';

const NOT_FOUND = 'Not found';
const BODY = 'This screen does not exist';
const START_TITLE = 'Design system';
const WAY_FORWARD = 'Go to the start screen';
const SCENARIO =
  'TOOL-5: Component tests exercise the compact baseline — ' +
  'Component tests exercise the expanded baseline';

async function renderUnmatchedRoute(): Promise<void> {
  const router = createTestRouter({ initialEntries: ['/no-such-route'] });

  await router.load();
  render(<RouterProvider router={router} />);
  await screen.findByRole('heading', { name: NOT_FOUND });
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
    await renderUnmatchedRoute();

    const user = userEvent.setup();
    const button = screen.getByRole('button', { name: WAY_FORWARD });

    await user.click(button);

    const startScreen = await screen.findByRole('heading', {
      name: START_TITLE,
    });

    expect(startScreen).toBeVisible();
    expect(screen.queryByRole('heading', { name: BODY })).toBeNull();
  });
});
