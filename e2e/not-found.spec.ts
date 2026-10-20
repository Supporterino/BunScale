/*
 * The root not-found surface of task 0.21, driven end to end: it renders from
 * the design-system primitives at a path the router does not match, and its one
 * way forward is a real control that returns to the start route.
 *
 * Why this flow and not only a render check: `NotFoundScreen` is the root's
 * `notFoundComponent`, so it is reachable from anywhere — including a mistyped
 * deep link in the webview — and it is the one screen a user can arrive at with
 * no history behind them. That is why its way forward is a route rather than
 * `router.history.back()` (`tanstack-router-conventions`: an in-UI control, never
 * a reliance on the platform's back gesture), and why the second test below
 * asserts the route change rather than a visual difference.
 *
 * Since task 3.3 that route is `/`, the household entry guard, so what the
 * destination renders depends on the household and service state. The test
 * stubs a household in the book — which is what a person who has completed
 * onboarding sees — so the guard keeps them on the entry route; with the mock's
 * default (no household, fresh reachable account) `/` would correctly redirect
 * to `/onboarding`, and asserting either screen's copy would put tasks 4.1/4.2
 * in charge of this test. What is asserted is the destination and the surface
 * left behind, which is the behaviour the screen owns.
 *
 * The not-found surface has no `AppShell` and therefore no navigation landmark
 * at all: there is no destination to be current, and a landmark with nothing in
 * it is worse than none. The arrangement assertions in
 * `./adaptive-navigation.spec.ts` therefore do not apply here, which is a fact
 * about this screen rather than a gap in the coverage.
 */
import { expect, stubCommandResult, test } from './fixtures';

/**
 * A path the router does not match. Hash history is what the app uses (the
 * Tauri webview serves static assets with no server-side fallback), so the
 * unmatched path is the fragment, not the pathname.
 */
const UNMATCHED_PATH = '/#/no-such-route';

const NOT_FOUND = 'Not found';
const NOT_FOUND_BODY = 'This screen does not exist';
const WAY_FORWARD = 'Go to the start screen';

/** Stands in for the contract's `Instant`, as the component layer does. */
const AT = '2026-10-20T09:00:00Z';

/** One household, so the entry guard resolves "household" without a probe. */
const HOUSEHOLD = {
  id: 'household-a',
  name: 'Household A',
  created_at: AT,
  owner_member_id: 'member-a',
  is_active: true,
  is_owner: true,
};

const HOUSEHOLD_SUMMARY = {
  id: HOUSEHOLD.id,
  name: HOUSEHOLD.name,
  created_at: HOUSEHOLD.created_at,
  is_active: HOUSEHOLD.is_active,
  is_owner: HOUSEHOLD.is_owner,
  member_count: 1,
};

test('The not-found surface renders from the primitives', async ({ page }) => {
  await page.goto(UNMATCHED_PATH);

  const title = page.getByRole('heading', { level: 1, name: NOT_FOUND });
  const body = page.getByRole('heading', { name: NOT_FOUND_BODY });
  const button = page.getByRole('button', { name: WAY_FORWARD });

  /*
   * The `h1` is the navigation bar's title — the screen's outline starts there
   * — and the body is a real heading rather than a styled paragraph, so the
   * surface is reachable by a heading rotor in the webview.
   */
  await expect(title).toBeVisible();
  await expect(body).toBeVisible();

  /*
   * Exactly one control: the way forward. Nothing else is offered — no second
   * button and no link, which is what makes this the screen's single exit
   * rather than merely one enabled control among several.
   */
  await expect(page.getByRole('button')).toHaveCount(1);
  await expect(page.getByRole('link')).toHaveCount(0);
  await expect(button).toBeEnabled();
});

test('The way forward returns to the start screen', async ({ page }) => {
  await page.goto(UNMATCHED_PATH);

  const title = page.getByRole('heading', { level: 1, name: NOT_FOUND });
  const body = page.getByRole('heading', { name: NOT_FOUND_BODY });

  /*
   * Wait for the surface before touching the mock seam: this `goto` is a fresh
   * document, and the not-found heading proves its bootstrap (and therefore the
   * mock install) has completed.
   */
  await expect(title).toBeVisible();

  await stubCommandResult(page, 'list_households', [HOUSEHOLD_SUMMARY]);
  await stubCommandResult(page, 'get_active_household', HOUSEHOLD);

  const button = page.getByRole('button', { name: WAY_FORWARD });

  await button.click();

  /*
   * The destination is the household entry route (`/`, task 3.3), and with a
   * household in the book the guard renders its entry surface there instead of
   * redirecting. `data-household-entry` is the stable name of that decision; the
   * screen's copy belongs to tasks 4.1/4.2.
   */
  await expect(
    page.locator('[data-household-entry="household"]'),
  ).toBeVisible();

  const destination = new URL(page.url());

  expect(destination.pathname).toBe('/');
  expect(destination.hash).toBe('#/');

  // The not-found surface is left behind, not merely covered.
  await expect(title).toHaveCount(0);
  await expect(body).toHaveCount(0);
});
