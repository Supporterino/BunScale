/*
 * TOOL-5's two adaptive-navigation scenarios — "adaptive navigation is asserted
 * at the compact baseline" and "…at the expanded baseline" — and the reason they
 * live here rather than in the component layer.
 *
 * `AppShell` renders **both** arrangements from one destinations array at every
 * width: `Sidebar` (`data-navigation-arrangement="sidebar"`) and `TabBar`
 * (`…="tabs"`). Which one is laid out is decided by one rule in
 * `src/styles/ui.css`, `@media (min-width: 768px)`, and by nothing in
 * JavaScript. A Vitest spec cannot assert it: jsdom's media evaluation matches
 * plain media **types** only, so no feature query in that stylesheet ever
 * applies, and the app's stylesheet is not loaded in a Vitest run at all — both
 * landmarks stay in the DOM, a role query finds two, and a `display` read
 * returns the base rule at either baseline. Reading it there would be a green
 * assertion proving nothing (`src/components/ui/navigation.ts`,
 * `src/test/setup.ts`, design.md D0.24).
 *
 * What is asserted here is therefore the thing the component layer cannot see:
 * the browser's own computed value, at 390x844 and at 1280x800, scoped by
 * `data-navigation-arrangement` so the assertion does not depend on a class
 * name — and, in the same breath, that a role query sees exactly one
 * `navigation` landmark named "Primary", which is what `display: none` is for:
 * the hidden arrangement leaves the accessibility tree as well as the layout.
 *
 * One project per baseline (not one project looping over sizes) is what makes a
 * failure name its baseline: a failure of either test below reports as
 * `webkit-compact` or `webkit-expanded` (`playwright.config.ts`).
 */
import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';

/** The only breakpoint the arrangement uses: `@media (min-width: 768px)`. */
const MEDIUM_BREAKPOINT_PX = 768;

/*
 * Both scenarios in one name: the two projects are the two baselines, so the
 * project suffix on a failure (`webkit-compact` / `webkit-expanded`) carries
 * the band while this name states the requirement being asserted.
 */
const SCENARIO =
  'TOOL-5: Adaptive navigation is asserted at the compact baseline — ' +
  'Adaptive navigation is asserted at the expanded baseline';
const ARRANGEMENT_TEST = `${SCENARIO} — the arrangement matches the viewport`;
const DESTINATION_TEST = `${SCENARIO} — the destination is current`;

/**
 * The band the running project is in, read from the real viewport. The
 * baselines themselves are declared in `playwright.config.ts` and checked
 * against the run in `./harness.spec.ts`, so this only has to classify a width.
 */
function isExpandedViewport(page: Page): boolean {
  const viewport = page.viewportSize();

  return viewport !== null && viewport.width >= MEDIUM_BREAKPOINT_PX;
}

test(ARRANGEMENT_TEST, async ({ page }) => {
  const isExpanded = isExpandedViewport(page);
  const expected = isExpanded ? 'sidebar' : 'tabs';
  const suppressed = isExpanded ? 'tabs' : 'sidebar';
  const expectedSelector = `[data-navigation-arrangement="${expected}"]`;
  const suppressedSelector = `[data-navigation-arrangement="${suppressed}"]`;
  const laidOut = page.locator(expectedSelector);
  const hidden = page.locator(suppressedSelector);

  /*
   * Exactly one arrangement is exposed: `display: none` removes the other one
   * from the accessibility tree, so a role query matches one landmark rather
   * than two. This is the assertion the DOM alone cannot make.
   */
  await expect(page.getByRole('navigation', { name: 'Primary' })).toHaveCount(
    1,
  );

  // The band's arrangement is laid out…
  await expect(laidOut).toBeVisible();
  await expect(laidOut).not.toHaveCSS('display', 'none');

  // …and the other one is out of the layout, not merely off-screen.
  await expect(hidden).toBeHidden();
  await expect(hidden).toHaveCSS('display', 'none');
});

test(DESTINATION_TEST, async ({ page }) => {
  const arrangement = isExpandedViewport(page) ? 'sidebar' : 'tabs';
  const selector = `[data-navigation-arrangement="${arrangement}"]`;
  const destination = page
    .locator(selector)
    .getByRole('link', { name: 'Design system' });

  /*
   * The same destinations array feeds both primitives, and the active state is
   * the router's own view of the location (`activeProps`), so the laid-out
   * arrangement offers exactly one current destination at either width — the
   * half of "the arrangement changed" that would be cosmetic-only otherwise.
   */
  await expect(destination).toBeVisible();
  await expect(destination).toHaveAttribute('aria-current', 'page');
});
