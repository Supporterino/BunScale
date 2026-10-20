import './styles/index.css';
import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { queryClient } from './lib/query-client';
import { router } from './router';

/*
 * E2E IPC mock seam (TOOL-5). The E2E layer drives WebKit against the dev
 * server with the IPC layer mocked, and `testing-strategy` fixes both ends of
 * that: the mock lives in `src/test/mock-ipc.ts` and is bootstrapped when
 * `VITE_E2E_MOCK=1`. This is the bootstrap site — the entry module, before
 * anything renders, so `mockIPC` is installed before the first route loader or
 * query can call `invoke`.
 *
 * Why the flag is the only condition: the `test:e2e` script in `package.json` is
 * `VITE_E2E_MOCK=1 playwright test`, and Playwright starts the dev server as a
 * child of that process, so Vite sees the variable and exposes it to the app as
 * `import.meta.env.VITE_E2E_MOCK` — the dev server injects the resolved env
 * object into every module it serves. CI runs that same script, so the local and
 * the CI invocation are byte-identical as far as the mock environment goes: CI
 * supplies no mock variable of its own. The one variable it does set, `CI`,
 * only tightens the harness — `forbidOnly: !!process.env.CI` refuses a committed
 * `.only` and `reuseExistingServer: !process.env.CI` never reuses a server
 * someone else started — and adds nothing the local run lacks (design.md D0.24;
 * the CI workflow is D0.22). Nothing about Tauri, the host or the build mode
 * is consulted: a mock that switched itself on in a normal `tauri dev` run
 * would hide the real backend.
 *
 * Why the glob rather than a plain `import()`: the glob was written while the
 * module did not exist yet — task 0.23 owned it — because a static or literal
 * dynamic import of a missing file fails `bun run build` and `tsc --noEmit`.
 * `import.meta.glob` is resolved by Vite at build time and yields an empty
 * record when the path matches nothing, so the seam stayed inert and the build
 * green until task 0.23 created the file, which fills it without editing this
 * file or the harness config.
 *
 * The flag is read once and narrowed to `string | undefined` rather than used
 * as `any`: `import.meta.env` has an index signature, and this is the one place
 * the value is trusted.
 */
const e2eIpcMockModules = import.meta.glob('./test/mock-ipc.ts');

/*
 * The same path as the glob above, spelled out a second time because it has to
 * be: `import.meta.glob` is not a function but a compile-time macro that reads
 * its argument from the source text, and Vite 8.3.4 accepts only a string or
 * template literal there (in the installed Vite, that guard is
 * `parseImportGlob`'s `validateLiteral`, which throws "Could only use
 * literals" for an identifier), so no form of this constant can be handed to
 * it. The two literals are one path and must be edited together: move the
 * glob alone and the lookup below returns `undefined`, which reports a missing
 * file rather than a stale path.
 */
const E2E_IPC_MOCK_PATH = './test/mock-ipc.ts';

async function installE2eIpcMockIfEnabled(): Promise<void> {
  const e2eMockFlag: string | undefined = import.meta.env.VITE_E2E_MOCK;

  if (e2eMockFlag !== '1') {
    return;
  }

  const loadE2eIpcMock = e2eIpcMockModules[E2E_IPC_MOCK_PATH];

  if (loadE2eIpcMock === undefined) {
    throw new Error(
      `VITE_E2E_MOCK=1 but ${E2E_IPC_MOCK_PATH} does not exist: the E2E IPC mock is task 0.23's file`,
    );
  }

  await loadE2eIpcMock();
}

/*
 * Application entry point. `index.html` references exactly this file
 * (`<script type="module" src="/src/main.tsx">`, task 0.14) and declares the
 * mount node, so the two halves are one contract: this module mounts into
 * `#root` and nothing else in `src/` touches the document. A missing node is
 * therefore a hard failure rather than a silently blank window.
 */
const mountNode = document.getElementById('root');

if (mountNode === null) {
  throw new Error(
    'Missing #root mount node: src/main.tsx mounts into the node index.html declares',
  );
}

/*
 * Provider order: the query cache is the outermost piece of app-wide state, so
 * `QueryClientProvider` sits above the router and every route reads it with
 * `useQueryClient()`. The router carries the same instance as its `context`
 * (see `src/router.tsx`), which a route reads with `Route.useRouteContext()`
 * (or the `useRouteContext` hook) — so both paths reach one cache.
 * `RouterProvider` renders the route tree from `src/routeTree.gen.ts`
 * (generated from `src/routes/`, never edited).
 *
 * `StrictMode` is deliberate and development-only. Its double-invoked effects
 * are what catch a Tauri event subscription that forgets to unlisten on
 * cleanup (`frontend-conventions`, "Events"), and the app's subscriptions are
 * effect-held from task 3.6 on; the cost is a duplicated dev render, and the
 * production build is unaffected. `frontend-conventions` states no preference
 * either way, so the choice is recorded here rather than left implicit.
 */
function renderApp(node: HTMLElement): void {
  createRoot(node).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </StrictMode>,
  );
}

/*
 * The mock is awaited before the first render, which is what makes its install
 * order deterministic (`import.meta.glob` returns a lazy loader, so the module
 * itself is fetched only when the flag is set). A top-level `await` would be the
 * shorter spelling, but it would tie this entry to the build target, and the
 * target is Vite's default rather than a pin: `vite.config.ts` sets no
 * `build.target`, so `baseline-widely-available` applies — Safari 16.4+, which
 * is why the shipped floor is iOS 16.4 / macOS 13.3 (design.md D0.16, Q16).
 * That setting governs the build transpile only: `tauri ios dev` serves
 * untransformed source and `tsconfig.json` is `noEmit`, so the dev loop stays
 * inside the floor by discipline, never by configuration. The `async` function
 * keeps the `await` in a scope both paths handle the same way. `void` therefore
 * discards the value and nothing else — no `catch` is attached on purpose, so a
 * failure surfaces as a loud unhandled rejection instead of a blank window.
 */
async function bootstrap(node: HTMLElement): Promise<void> {
  await installE2eIpcMockIfEnabled();
  renderApp(node);
}

void bootstrap(mountNode);
