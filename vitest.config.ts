import { configDefaults, defineConfig } from 'vitest/config';

/*
 * The unit/component layer of the test harness (TOOL-5, `testing-strategy`).
 *
 * This file is deliberately **standalone** rather than a `mergeConfig` of
 * `vite.config.ts`, and that choice needs to be read rather than assumed:
 *
 * - Vitest resolves config files in the order `vitest.config.*` then
 *   `vite.config.*`, so this file *replaces* the app's Vite config for the test
 *   pipeline instead of extending it. Nothing is lost by that: the test layer
 *   renders components through esbuild/tsconfig (`jsx: "react-jsx"`), never
 *   through the dev server, so `server.port`, `server.host`, `clearScreen` and
 *   the `build` block — everything `vite.config.ts` carries for `tauri dev` —
 *   has no meaning here.
 * - The one plugin that *would* change behaviour is `tanstackRouter()`, and it
 *   is deliberately **not** merged in: it renders `src/routeTree.gen.ts` as a
 *   side effect of transforming route files, and that file is a tracked,
 *   generated artefact whose drift the gate checks (TOOL-4, design.md D0.9).
 *   A test run must never rewrite a generated file; tests exercise the
 *   committed route tree (`src/routeTree.gen.ts`) that the app itself imports
 *   through `src/router.tsx`.
 * - Dropping the React plugin costs nothing here either: it adds Fast Refresh
 *   and Babel's automatic JSX runtime to the dev server, while Vitest compiles
 *   `.tsx` with esbuild, which reads `jsx: "react-jsx"` from `tsconfig.json`.
 *
 * `globals` is left at its default (`false`) on purpose: `tsconfig.json` does
 * not list `vitest/globals` in `types`, so global `describe`/`it`/`expect`
 * would typecheck only if the app's typecheck program were widened for it.
 * Specs import them from `vitest` explicitly instead.
 *
 * `setupFiles` is **absent** by design: task 0.23 adds `src/test/setup.ts` (the
 * `@testing-library/jest-dom` matchers and the `QueryClientProvider`
 * (`retry: false`) / `createMemoryHistory` helpers) and references it here in
 * one line. It is not pre-wired, because nothing in this task may point at a
 * file that does not exist yet (a dangling `setupFiles` entry would also be a
 * second, silent failure mode for `bun run test`).
 */
export default defineConfig({
  test: {
    // The component layer runs in a DOM (TOOL-5). jsdom is the pinned
    // environment (design.md D0.8) and is what Testing Library drives.
    environment: 'jsdom',

    // Specs live beside the code they cover (`frontend-conventions`:
    // `src/features/<name>/*.test.tsx`). Scoping the include to `src/` is what
    // keeps the Playwright specs in `e2e/` out of Vitest's reach: both runners
    // use the same `*.spec.ts` / `*.test.tsx` naming, so an unscoped default
    // would make `bun run test` try to execute E2E specs.
    include: ['src/**/*.{test,spec}.{ts,tsx}'],

    // `configDefaults.exclude` is spread rather than replaced so Vitest's own
    // defaults (`node_modules`, `.git`, …) survive. The additions are the
    // directories this repository writes that are not test sources: the E2E
    // specs (`e2e/`), the build output (`dist/`), the Rust crate
    // (`src-tauri/`) and the Playwright runner's output (`test-results/`, plus
    // `playwright-report/`, which task 0.19 ignores even though this config
    // enables no `html` reporter yet).
    exclude: [
      ...configDefaults.exclude,
      'e2e/**',
      'dist/**',
      'src-tauri/**',
      'test-results/**',
      'playwright-report/**',
    ],
  },
});
