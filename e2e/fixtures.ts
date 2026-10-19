/*
 * The shared E2E fixture: the one place the IPC mock is verified (TOOL-5,
 * design.md D0.24, Q23).
 *
 * Why it exists at all. `playwright.config.ts` sets
 * `reuseExistingServer: !process.env.CI`, so a local run reuses a dev server
 * already listening on 1420 — and a server started **without**
 * `VITE_E2E_MOCK=1` renders the app with no mock installed (Q23 records the two
 * such servers found and stopped while task 0.23 was verified). The symptom is
 * silent: the app renders, the surface under test looks right, and a spec that
 * asserted an unmocked read would fail for the wrong reason while one that
 * asserted "the never-loaded surface is not shown" (D0.2) could pass *because*
 * nothing was ever mocked. CI never reuses a server, so only a local run is
 * exposed.
 *
 * That is why the check is a fixture rather than a preamble copied into each
 * spec, and why it is `auto`: a spec author who imports `test` from this module
 * pays it without having to remember. `auto` alone is still only a convention,
 * though: a spec that imports `test` from `@playwright/test` typechecks, lints
 * and runs with this fixture silently absent. What makes the convention
 * mechanical is `src/test/e2e-fixture-consumption.spec.ts`, which reads every
 * `e2e/*.spec.ts` and fails the suite when one does not value-import `test` and
 * `expect` from this module, or value-imports anything from `@playwright/test`.
 * It is the same shape as `describeAtBothBaselines` in `src/test/setup.ts` —
 * the arrangement helper for the component layer, which no spec may replace
 * with its own viewport — and for the same reason task 0.23 exists: a per-spec
 * convention is what the harness is meant to remove.
 *
 * The order of the two steps matters:
 *
 *   1. the start screen is on screen. This proves only that the app **booted**:
 *      a server without the flag renders the same start screen, which is the
 *      Q23 shape, so the screen's presence is not evidence that the flag was
 *      read. It is also the only wait that is honest on a cold dev server,
 *      where the first load transpiles the app;
 *   2. `window.__TAURI_INTERNALS__.invoke` is a function. This is the assertion
 *      the fixture exists for, and it is what fails for a reused server that
 *      carries no flag — the failure names the cause rather than leaving a
 *      later assertion to fail for the wrong reason. It is also where the
 *      flag's effect is evidenced: with the flag set, reaching this point means
 *      `src/test/mock-ipc.ts` resolved (and, if it had not, the seam in
 *      `src/main.tsx` would have thrown under D0.24 instead of booting).
 *
 * The opposite failure mode is loud as well: with the flag set and
 * `src/test/mock-ipc.ts` missing, the seam in `src/main.tsx` throws instead of
 * booting unmocked (D0.24), so nothing renders and step 1 fails.
 *
 * Nothing here is a `globalSetup`, which has no page: the check is about the
 * page's own `window`, so it has to run in the browser context.
 */
import { test as base, expect } from '@playwright/test';

/** The start screen's `h1` — `NavigationBar`'s title on the index route. */
const START_SCREEN_TITLE = 'Design system';

const MOCK_MISSING_MESSAGE =
  'VITE_E2E_MOCK=1 is not in effect: the page rendered with no IPC mock, so window.__TAURI_INTERNALS__.invoke is not a function (design.md D0.24; Q23 records the reused dev server that carries no flag).';

/*
 * A module-scope type, used only by the function below — which is serialised
 * and shipped to WebKit, so it may reference nothing from this module at
 * runtime. Types are erased, so the function evaluates in the page; the cast is
 * what lets the untyped Tauri global be read without `any`.
 */
type TauriInternals = { __TAURI_INTERNALS__?: { invoke?: unknown } };

function invokeIsMocked(): boolean {
  const internals = (window as unknown as TauriInternals).__TAURI_INTERNALS__;

  return typeof internals?.invoke === 'function';
}

export type E2EFixtures = {
  /**
   * Automatic: boots the app at the start screen and asserts the IPC mock is
   * installed, before the test body runs. Never requested by a spec; the
   * `e2e-fixture-consumption` guard is what requires every E2E spec to import
   * `test` and `expect` from this module rather than from `@playwright/test`.
   * It resolves to `true` once that has held, which is the only value it ever
   * produces.
   */
  mockedIpc: boolean;
};

export const test = base.extend<E2EFixtures>({
  mockedIpc: [
    async ({ page }, use) => {
      await page.goto('/');

      const startScreen = page.getByRole('heading', {
        level: 1,
        name: START_SCREEN_TITLE,
      });

      await expect(startScreen).toBeVisible();

      const mockInstalled = await page.evaluate(invokeIsMocked);

      expect(mockInstalled, MOCK_MISSING_MESSAGE).toBe(true);

      await use(true);
    },
    { auto: true },
  ],
});

/*
 * Re-exported so a spec has one import source for the harness it runs under.
 * The import is the consumption, and `src/test/e2e-fixture-consumption.spec.ts`
 * makes importing from anywhere else a suite failure rather than a convention.
 */
export { expect };
