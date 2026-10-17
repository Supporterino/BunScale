import { defineConfig } from '@playwright/test';

/*
 * The E2E layer (TOOL-5, `testing-strategy`): WebKit, the engine both shipped
 * platforms actually run (WKWebView), driven at the two form-factor baselines
 * the app is built against — 390×844 compact (AGENTS.md baseline 1) and
 * 1280×800 expanded (baseline 2, and the macOS window geometry fixed in
 * design.md D0.15). `tauri-driver` is not used: it has no WKWebView driver, so
 * it can drive neither iOS nor macOS (`testing-strategy`, "What cannot be
 * automated yet"). What runs here is the frontend against the Vite dev server
 * with the IPC layer mocked, which is why the suite needs no native device and
 * no shared service — and why it is headless and secret-free in CI (D0.22).
 *
 * One project per baseline rather than one project looping over sizes: the
 * baseline is what TOOL-5's E2E and adaptive-navigation scenarios assert
 * ("compact" vs "expanded"), so making it a project makes a failing assertion
 * name the baseline it failed in (`webkit-compact › …`), and
 * `--project=webkit-compact` runs exactly one baseline.
 *
 * Specs live in `e2e/` (the path design.md's scaffold-surface table records),
 * and that directory is in `tsconfig.json`'s `include`, so the specs are
 * inside `bun run typecheck`'s program — Playwright still transpiles them
 * itself when it runs them.
 */

/*
 * The dev server is the app's own (`bun run dev` → `vite`), and its address is
 * the same single value stated in three places that have to agree by hand:
 * `server.port`/`strictPort` in `vite.config.ts`, `build.devUrl` in
 * `src-tauri/tauri.conf.json`, and this `baseURL`/`webServer.url`. Task 0.24
 * asserts the first two; this file is the E2E side of the same value.
 *
 * The server is started by Playwright as a child of the `playwright test`
 * process, so it inherits that process's environment — which is how
 * `VITE_E2E_MOCK=1` reaches Vite and, through it, `import.meta.env` in the
 * page (see the bootstrap seam in `src/main.tsx`). Nothing else about the mock
 * is configured here: the mock is app-side and gated on that one variable.
 */
const devServerUrl = 'http://localhost:1420';

export default defineConfig({
  testDir: './e2e',

  // Independent specs run in parallel; this suite is a handful of smoke tests,
  // so Playwright's default worker count is left alone.
  fullyParallel: true,

  // A `.only` committed by accident must not silently shrink the gate on CI.
  forbidOnly: !!process.env.CI,

  // No retries. `testing-strategy` requires deterministic tests, and a retry
  // that hides a flake also hides the flake; a failing spec is reported as
  // failed on the first run.
  retries: 0,

  use: {
    baseURL: devServerUrl,

    // Diagnosis on failure, not on success: both artefacts land in
    // `test-results/`, which task 0.19 excludes from the repository. A trace is
    // kept only when a test fails (`on-first-retry` would never fire with
    // `retries: 0`).
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },

  projects: [
    {
      name: 'webkit-compact',
      use: {
        browserName: 'webkit',

        // Baseline 1: 390×844 CSS pixels. Declared explicitly rather than
        // borrowed from a `devices[…]` preset: the closest preset
        // (`iPhone 12 Pro`) is 390×664 and would have to be overridden anyway,
        // so the numbers stated here are the numbers the suite runs at.
        viewport: { width: 390, height: 844 },

        // iPhone-class pixel density. It is what `devicePixelRatio` reports in
        // the page, so it is what any `-webkit-min-device-pixel-ratio` styling
        // or rasterisation assertion sees; the default (1) would make the
        // compact baseline a desktop-density viewport.
        deviceScaleFactor: 3,

        // Touch is the baseline input on compact (AGENTS.md rule 3: hover and
        // shortcuts are the `pointer: fine` enhancement), so the compact
        // project is a touch device.
        hasTouch: true,
      },
    },
    {
      name: 'webkit-expanded',
      use: {
        browserName: 'webkit',

        // Baseline 2: 1280×800 CSS pixels — the macOS window geometry.
        viewport: { width: 1280, height: 800 },

        // Retina Mac density, for the same reason as above.
        deviceScaleFactor: 2,

        hasTouch: false,
      },
    },
  ],

  webServer: {
    command: 'bun run dev',
    url: devServerUrl,

    // The script sets `VITE_E2E_MOCK=1` for this process and the server
    // inherits it, so the local and CI invocations of `bun run test:e2e` are
    // byte-identical as far as the mock environment goes — CI injects no mock
    // variable of its own, and the `CI` it does set only tightens the harness.
    // Reusing an already-running dev server is a local convenience only — on
    // CI the runner starts its own, so the gate never depends on a server
    // someone else started (and possibly without the mock flag).
    reuseExistingServer: !process.env.CI,

    // Vite binds `strictPort` 1420 and fails loudly if it is taken, so a
    // timeout here means the server did not come up rather than that it moved.
    timeout: 120_000,
  },
});
