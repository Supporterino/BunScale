/*
 * The root not-found surface of task 0.21, driven end to end: it renders from
 * the design-system primitives at a path the router does not match, and its one
 * way forward is a real control that returns to the start screen.
 *
 * Why this flow and not only a render check: `NotFoundScreen` is the root's
 * `notFoundComponent`, so it is reachable from anywhere — including a mistyped
 * deep link in the webview — and it is the one screen a user can arrive at with
 * no history behind them. That is why its way forward is a route rather than
 * `router.history.back()` (`tanstack-router-conventions`: an in-UI control, never
 * a reliance on the platform's back gesture), and why the second test below
 * asserts the route change rather than a visual difference.
 *
 * The not-found surface has no `AppShell` and therefore no navigation landmark
 * at all: there is no destination to be current, and a landmark with nothing in
 * it is worse than none. The arrangement assertions in
 * `./adaptive-navigation.spec.ts` therefore do not apply here, which is a fact
 * about this screen rather than a gap in the coverage.
 */
import { expect, test } from './fixtures';

/**
 * A path the router does not match. Hash history is what the app uses (the
 * Tauri webview serves static assets with no server-side fallback), so the
 * unmatched path is the fragment, not the pathname.
 */
const UNMATCHED_PATH = '/#/no-such-route';

const NOT_FOUND = 'Not found';
const NOT_FOUND_BODY = 'This screen does not exist';
const START_SCREEN = 'Design system';
const WAY_FORWARD = 'Go to the start screen';

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

  const button = page.getByRole('button', { name: WAY_FORWARD });

  await button.click();

  const startScreen = page.getByRole('heading', { name: START_SCREEN });

  await expect(startScreen).toBeVisible();
  await expect(page).not.toHaveURL(/no-such-route/);
});
