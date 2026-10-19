/*
 * TOOL-5's "E2E runs in WebKit with mocked IPC" scenario, stated as a
 * measurement of the harness rather than as prose: the suite drives **WebKit**
 * — the engine both shipped platforms run (WKWebView) — at the two form-factor
 * baselines `playwright.config.ts` declares, and the IPC layer is mocked.
 *
 * The mocked half is the shared fixture in `./fixtures`, which is `auto` and
 * therefore runs before every test in this file too: it boots the app at the
 * start screen and asserts `window.__TAURI_INTERNALS__.invoke` is a function.
 * A spec cannot assert the mock by importing a helper — it would be asserting
 * its own call — so the run itself is the evidence, and the fixture is what
 * makes an unmocked local run fail instead of passing quietly (D0.24, Q23).
 *
 * The engine half is asserted here because the configuration alone does not
 * prove it: `testInfo.project.use.browserName` is a declaration, while
 * `browser.browserType().name()` is the browser this run actually launched. A
 * suite that silently fell back to Chromium would keep a mocked, both-baselines
 * green run and stop being evidence for the platforms the app ships on, which
 * is exactly the failure TOOL-5 exists to prevent.
 *
 * No native device and no shared service are involved anywhere in that: the
 * frontend runs against the app's own dev server started by Playwright as a
 * child of `playwright test` (`tauri-driver` has no WKWebView driver for either
 * shipped platform — `testing-strategy`, "What cannot be automated yet").
 */
import { expect, test } from './fixtures';

/*
 * The two projects, by name. This map is not where the baselines are declared
 * (`playwright.config.ts` is) — it is where the run is checked against them, so
 * a project whose viewport drifted fails here rather than in an arrangement
 * assertion that would blame the stylesheet for it.
 */
const PROJECT_VIEWPORTS: Record<string, { width: number; height: number }> = {
  'webkit-compact': { width: 390, height: 844 },
  'webkit-expanded': { width: 1280, height: 800 },
};

const SCENARIO = 'TOOL-5: E2E runs in WebKit with mocked IPC';
const BASELINE_TEST = `${SCENARIO} — the project runs at its baseline`;
const WEBKIT_TEST = `${SCENARIO} — the suite drives WebKit`;

test(BASELINE_TEST, async ({ page }, testInfo) => {
  const declared = PROJECT_VIEWPORTS[testInfo.project.name];

  expect(page.viewportSize()).toEqual(declared);
});

test(WEBKIT_TEST, async ({ browser, browserName }) => {
  expect(browserName).toBe('webkit');
  expect(browser.browserType().name()).toBe('webkit');
});
