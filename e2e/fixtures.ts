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
 *   1. the harness's start surface is on screen. This proves only that the app
 *      **booted**: a server without the flag renders the same surface, which is
 *      the Q23 shape, so the surface's presence is not evidence that the flag
 *      was read. It is also the only wait that is honest on a cold dev server,
 *      where the first load transpiles the app;
 *   2. `window.__TAURI_INTERNALS__.invoke` is a function. This is the assertion
 *      the fixture exists for, and it is what fails for a reused server that
 *      carries no flag — the failure names the cause rather than leaving a
 *      later assertion to fail for the wrong reason. It is also where the
 *      flag's effect is evidenced: with the flag set, reaching this point means
 *      `src/test/mock-ipc.ts` resolved (and, if it had not, the seam in
 *      `src/main.tsx` would have thrown under D0.24 instead of booting).
 *
 * The start surface is the design-system preview at `/#/design-preview`, not
 * `/`. Since task 3.3, `/` is the household entry guard, so what it renders —
 * onboarding, a dashboard, or the connectivity-required slot — depends on the
 * household and service state, and this check is about neither: it is about the
 * app booting with a mocked IPC layer. The preview is the one surface in the
 * app that holds no IPC and no router logic, so it can prove that on its own.
 * A spec that drives another route navigates to it after this fixture has run;
 * the mock assertion stays the single check every spec consumes.
 *
 * The opposite failure mode is loud as well: with the flag set and
 * `src/test/mock-ipc.ts` missing, the seam in `src/main.tsx` throws instead of
 * booting unmocked (D0.24), so nothing renders and step 1 fails.
 *
 * Nothing here is a `globalSetup`, which has no page: the check is about the
 * page's own `window`, so it has to run in the browser context.
 */
import type { Page } from '@playwright/test';
import { test as base, expect } from '@playwright/test';

/**
 * The route of the surface the fixture boots: the design-system preview, which
 * is the app's only IPC-free screen (see the header). Hash history is what the
 * app uses, so the route is the fragment.
 */
const START_PATH = '/#/design-preview';

/** The preview's `h1` — `NavigationBar`'s title on that route. */
const START_SURFACE_TITLE = 'Design system';

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
   * Automatic: boots the app at the harness's start surface
   * (`/#/design-preview`) and asserts the IPC mock is installed, before the test
   * body runs. Never requested by a spec; the `e2e-fixture-consumption` guard is
   * what requires every E2E spec to import `test` and `expect` from this module
   * rather than from `@playwright/test`. It resolves to `true` once that has
   * held, which is the only value it ever produces.
   */
  mockedIpc: boolean;
};

export const test = base.extend<E2EFixtures>({
  mockedIpc: [
    async ({ page }, use) => {
      await page.goto(START_PATH);

      const startSurface = page.getByRole('heading', {
        level: 1,
        name: START_SURFACE_TITLE,
      });

      await expect(startSurface).toBeVisible();

      const mockInstalled = await page.evaluate(invokeIsMocked);

      expect(mockInstalled, MOCK_MISSING_MESSAGE).toBe(true);

      await use(true);
    },
    { auto: true },
  ],
});

/*
 * Overriding an answer for one spec (tasks 5.14–5.17).
 *
 * The mock's defaults are the entry guard's "no household, fresh reachable
 * account" state (`src/test/mock-ipc.ts`). A spec that drives another state —
 * offline, a populated book, a save that fails — changes one command for the
 * current document through the page-side seam the mock installs:
 *
 * ```ts
 * await stubCommandResult(page, 'get_service_status', {
 *   state: 'unreachable', account_valid: null, cause: 'connectivity',
 *   observed_at: '2026-10-20T09:00:00Z',
 * });
 * ```
 *
 * It hands over a value, not a handler, because this function runs in Node: the
 * value is serialised into the page and registered there. Overrides last until
 * the next document load, and a fresh page starts from the defaults again.
 */
export async function stubCommandResult(
  page: Page,
  command: string,
  result: unknown,
): Promise<void> {
  /*
   * The mock is loaded by an awaited dynamic import in `src/main.tsx` before the
   * first render, so on a cold document the seam can appear after the `load`
   * event `page.goto` waited for. Waiting for it here is what keeps a spec from
   * racing the bootstrap; it is the same fact the fixture's own assertion
   * checks, not a second copy of it.
   */
  await page.waitForFunction(
    () =>
      (window as unknown as { __BUNSCALE_E2E_MOCK__?: unknown })
        .__BUNSCALE_E2E_MOCK__ !== undefined,
  );

  await page.evaluate(
    ({ command, result }) => {
      const seam = (
        window as unknown as {
          __BUNSCALE_E2E_MOCK__?: {
            setResult: (command: string, result: unknown) => void;
          };
        }
      ).__BUNSCALE_E2E_MOCK__;

      if (seam === undefined) {
        throw new Error(
          'VITE_E2E_MOCK=1 is not in effect: the page exposes no __BUNSCALE_E2E_MOCK__ seam, so a command cannot be overridden (Q23).',
        );
      }

      seam.setResult(command, result);
    },
    { command, result },
  );
}

/*
 * Re-exported so a spec has one import source for the harness it runs under.
 * The import is the consumption, and `src/test/e2e-fixture-consumption.spec.ts`
 * makes importing from anywhere else a suite failure rather than a convention.
 */
export { expect };
