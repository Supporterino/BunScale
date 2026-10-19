/*
 * The shared half of the component layer (TOOL-5, `testing-strategy`).
 *
 * `vitest.config.ts` references this file from `setupFiles`, so its two side
 * effects run once per test file, before anything else: the
 * `@testing-library/jest-dom` matchers are registered with Vitest's `expect`,
 * and Testing Library is made to clean up after every test. Every spec imports
 * the exported helpers from this same module (it is already in the module
 * registry by then, so the import is a plain named import and nothing runs
 * twice).
 *
 * What is exported, and who it is for:
 *
 *   - `createTestQueryClient` / `TestQueryClientProvider` /
 *     `renderWithQueryClient` — the `QueryClientProvider` wrapper with
 *     `retry: false` and a client that is fresh per render, so no two tests
 *     share cache state;
 *   - `createTestMemoryHistory` / `createTestRouter` — router tests, which
 *     cannot use the app's `src/router.tsx` instance: that one is bound to
 *     hash history and to the app-wide query cache;
 *   - `setComponentBaseline` / `restoreComponentBaseline` /
 *     `currentComponentViewport` / `describeAtBothBaselines` — the **component
 *     baselines**, 390×844 and 1280×800. This is the piece no config supplies:
 *     `vitest.config.ts` fixes the environment (`jsdom`) and deliberately
 *     declares no viewport, so without this helper a spec would set
 *     `window.innerWidth` itself and the two TOOL-5 baselines would be a
 *     convention rather than something the suite actually runs at.
 *
 * **What the baseline helper makes true, and what it cannot.** These are the
 * limits a spec author has to know before asserting anything:
 *
 *   - it **does** make the viewport real for JavaScript: `window.innerWidth` /
 *     `window.innerHeight` and `document.documentElement.clientWidth` /
 *     `clientHeight` report the baseline, so a component that branches on the
 *     viewport sees it, and a spec can assert the numbers it set;
 *   - it does **not** make the design system's styles apply, and the reason is
 *     *not* that jsdom lacks a CSS pipeline: the pinned jsdom (30.1.3) has one
 *     (`lib/jsdom/living/css/helpers/computed-style.js`, a parsed default
 *     stylesheet and a real `getComputedStyle` on `Window`). Two stronger
 *     reasons hold instead. First, jsdom's media evaluation matches plain media
 *     **types** only — `all` and `screen`, see `evaluateMediaList` in
 *     `MediaList-impl.js` — so every feature query in `src/styles/**`
 *     (`min-width`, `prefers-color-scheme`, `hover`/`pointer`,
 *     `prefers-reduced-motion`) never matches, which is what makes even a
 *     `display` assertion read the base rule at both baselines. Second, the
 *     app's stylesheet is not loaded in a Vitest run at all: `css` is not
 *     enabled in `vitest.config.ts` and `src/styles/index.css` is imported only
 *     by `src/main.tsx`, so no app rule applies and a `getComputedStyle` read
 *     returns jsdom's UA/inline-derived value rather than the design system's.
 *     The upshot is unchanged: the adaptive arrangement the design system
 *     encodes as `display: none` above/below 768 px does not happen here, both
 *     navigation landmarks stay in the DOM and a role query finds two. It is
 *     asserted in E2E, where the viewport is a real browser viewport (see
 *     `./src/components/ui/navigation.ts` and tasks.md 0.25);
 *   - it does **not** give jsdom a layout engine. `getBoundingClientRect()`,
 *     `offsetWidth`/`offsetHeight` and `clientWidth` on ordinary elements are
 *     still zero, so "the touch target is at least 44 pt" and every other
 *     geometric assertion belong to E2E as well;
 *   - it does **not** carry the rest of what a real compact browser implies, so
 *     a spec that branches on density or pointer type is reading jsdom's
 *     defaults rather than the baseline: `window.devicePixelRatio` is `1` where
 *     the E2E `webkit-compact` project sets `deviceScaleFactor: 3`, there is no
 *     touch or coarse-pointer capability at all (`hasTouch: true` and
 *     `pointer: coarse` exist only in a real browser), and
 *     `window.visualViewport` is absent. Density, pointer and visual-viewport
 *     behaviour therefore belong to E2E too;
 *   - it does **not** stub `window.matchMedia`, which the pinned jsdom
 *     (30.1.3) does not implement at all — measured: `typeof window.matchMedia`
 *     is `undefined`. A stub would report a media query as matching while no CSS
 *     was applied, so an assertion resting on it would be asserting the stub. A
 *     spec that needs `matchMedia` should install an explicit fake and say so.
 *
 * What the helper therefore guarantees is one honest thing: a component spec
 * can opt into "this suite runs at 390×844" and "…at 1280×800" uniformly,
 * instead of each spec inventing its own window size.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createMemoryHistory, createRouter } from '@tanstack/react-router';
import {
  cleanup,
  type RenderOptions,
  type RenderResult,
  render,
} from '@testing-library/react';
import { createElement, type ReactNode, useState } from 'react';
import { afterEach, beforeEach, describe } from 'vitest';
import { routeTree } from '../routeTree.gen';
import '@testing-library/jest-dom/vitest';

/*
 * Registering the jest-dom matchers is a side effect of the import above: the
 * `./vitest` entry point (not the default entry) is the one that calls
 * `expect.extend` with Vitest's own `expect`, and its type declaration augments
 * `vitest`'s `Assertion` interface for the whole program — so
 * `expect(node).toBeVisible()` typechecks in every spec without an extra
 * reference. The type augmentation is why this file is part of
 * `tsconfig.json`'s program rather than only of the test run.
 */

/*
 * Testing Library's own auto-cleanup does not run in this repository, and this
 * is the first reason the file exists. It registers itself behind a
 * `typeof afterEach === 'function'` check, and `globals` is off in
 * `vitest.config.ts` (deliberately — `tsconfig.json` does not list
 * `vitest/globals` in `types`), so there is no global `afterEach` for it to
 * find and mounted trees would survive from one test to the next in the same
 * file. Importing `afterEach` from `vitest` and registering `cleanup` by hand
 * is what keeps tests isolated without turning the globals on.
 *
 * The module is `.ts`, not `.tsx` — `vitest.config.ts`'s `setupFiles` names this
 * path and tasks 0.23/0.25 consume it by that name — so the wrapper components
 * below are built with `createElement`: `tsconfig.json` enables JSX syntax for
 * `.tsx` files only.
 */
afterEach(() => {
  cleanup();
});

/*
 * React's `act` environment flag. Testing Library sets it only around its own
 * calls and, in its index module, only when it finds the global `beforeAll` /
 * `afterAll` hooks — which do not exist here for the reason above. This restores
 * the environment Testing Library's own setup would have installed, for the
 * whole file rather than per call.
 *
 * Measured rather than assumed, because the line's weight is easy to overstate:
 * `@testing-library/react`'s `act` wrapper sets this flag around every call it
 * makes itself, so a suite that only renders and queries through Testing Library
 * is green and warning-free whether or not this line is here — the temporary
 * verification spec for task 0.23 was run with the flag set `true` and with it
 * explicitly set `false` and passed both times. What the line changes is the
 * flag a spec observes (`globalThis.IS_REACT_ACT_ENVIRONMENT`) and React's
 * treatment of an update that happens *outside* Testing Library's own calls,
 * which the flag unset would report as an unconfigured environment rather than
 * as the unwrapped update it is.
 */
(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

/*
 * The query client every component wrapper hands to `QueryClientProvider`.
 *
 * `retry: false` is the requirement (`testing-strategy`, "Frontend + IPC") and
 * it is not cosmetic: a query that fails would otherwise be retried by the
 * cache with a backoff, so a test asserting the failure surface would wait
 * through attempts the app deliberately does not make in this shape — the
 * app's own client (`src/lib/query-client.ts`) disables retries for the same
 * reason. `refetchOnWindowFocus: false` mirrors that client too; jsdom fires no
 * focus events, so it changes nothing today and keeps the two in step.
 *
 * A client **per call** rather than one shared instance: the cache is the state
 * a suite would otherwise leak between tests, and `renderWithQueryClient`
 * creates one per render, so two renders in one test cannot share a cache
 * either.
 */
export function createTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, refetchOnWindowFocus: false },
      mutations: { retry: false },
    },
  });
}

export type TestQueryClientProviderProps = {
  children: ReactNode;
  /** Supply a client when a test needs a handle on the same one it renders. */
  client?: QueryClient;
};

/*
 * The wrapper itself, for a spec that renders through Testing Library's own
 * `render(ui, { wrapper })` or `renderHook(callback, { wrapper })`.
 *
 * The fallback client is created **inside** the component with `useState`'s
 * lazy initialiser, so it is one client per mount: this wrapper cannot be
 * hoisted to module scope by accident, because it does not need to be given
 * one.
 */
export function TestQueryClientProvider({
  children,
  client,
}: TestQueryClientProviderProps) {
  const [fallbackClient] = useState(createTestQueryClient);

  return createElement(
    QueryClientProvider,
    { client: client ?? fallbackClient },
    children,
  );
}

export type RenderWithQueryClientOptions = Omit<RenderOptions, 'wrapper'> & {
  /** Reuse a client a test already created, e.g. to prefill the cache. */
  client?: QueryClient;
};

export type RenderWithQueryClientResult = RenderResult & {
  /** The client the tree was rendered with. */
  client: QueryClient;
};

/*
 * `render` with the provider already applied, returning the client it used so
 * a test can inspect or seed the cache through the same instance the tree
 * reads. `wrapper` is excluded from the options on purpose: passing one would
 * silently replace the provider.
 */
export function renderWithQueryClient(
  ui: ReactNode,
  options: RenderWithQueryClientOptions = {},
): RenderWithQueryClientResult {
  const { client = createTestQueryClient(), ...renderOptions } = options;
  const result = render(ui, {
    ...renderOptions,
    wrapper: ({ children }: { children: ReactNode }) =>
      createElement(QueryClientProvider, { client }, children),
  });

  return { ...result, client };
}

export type TestMemoryHistoryOptions = {
  /** Initial history entries. Defaults to `['/']`. */
  initialEntries?: string[];
};

/*
 * Memory history for router tests. The app's router (`src/router.tsx`) uses
 * hash history — that is what keeps a deep link reachable inside `index.html`
 * in a Tauri build — and a test wants neither that nor a shared singleton, so a
 * router test builds its own router on this history.
 */
export function createTestMemoryHistory(
  options: TestMemoryHistoryOptions = {},
) {
  const { initialEntries = ['/'] } = options;

  return createMemoryHistory({ initialEntries });
}

export type TestRouterOptions = TestMemoryHistoryOptions & {
  /** Context value for `Route.useRouteContext()`. Defaults to a fresh client. */
  client?: QueryClient;
};

/*
 * A router over the **committed** route tree (`src/routeTree.gen.ts`, imported
 * above — never regenerated by a test run, because `vitest.config.ts` leaves the
 * router plugin out) with memory history and a query client in its context, the
 * same context key `src/router.tsx` uses. A route loader that prefetches
 * through `Route.useRouteContext()` therefore works unchanged under test.
 */
export function createTestRouter(options: TestRouterOptions = {}) {
  const { client = createTestQueryClient(), initialEntries } = options;

  return createRouter({
    routeTree,
    history: createTestMemoryHistory({ initialEntries }),
    context: { queryClient: client },
  });
}

/** The two form-factor baselines the app is built against (AGENTS.md). */
export const COMPONENT_BASELINES = {
  compact: { width: 390, height: 844 },
  expanded: { width: 1280, height: 800 },
} as const;

export type ComponentBaselineName = keyof typeof COMPONENT_BASELINES;

/** Fixed order, so `describeAtBothBaselines` is deterministic. */
export const COMPONENT_BASELINE_NAMES: readonly ComponentBaselineName[] = [
  'compact',
  'expanded',
];

export type ComponentViewport = { width: number; height: number };

type SavedProperty = {
  target: object;
  key: string;
  descriptor: PropertyDescriptor | undefined;
};

/*
 * The pre-baseline descriptors, captured the first time the helper touches a
 * property rather than at module load, so `restoreComponentBaseline` puts back
 * exactly what jsdom had. `document.documentElement.clientWidth` is a getter on
 * `Element.prototype` and has no own descriptor, which is why the saved value
 * may be `undefined`: restoring then means deleting the own property so the
 * prototype getter applies again, which is the honest "back to jsdom" state
 * (that getter returns 0 — jsdom has no layout).
 */
const savedProperties: SavedProperty[] = [];

function rememberProperty(target: object, key: string): void {
  const alreadySaved = savedProperties.some(
    (saved) => saved.target === target && saved.key === key,
  );

  if (alreadySaved) {
    return;
  }

  savedProperties.push({
    target,
    key,
    descriptor: Object.getOwnPropertyDescriptor(target, key),
  });
}

/*
 * `Object.defineProperty` rather than assignment, and the reason is measured
 * against the pinned jsdom (30.1.3): `window.innerWidth`/`innerHeight` are
 * configurable own properties of the window and answer 1024/768 by default,
 * while `document.documentElement.clientWidth`/`clientHeight` have **no** own
 * descriptor at all — they are inherited accessors with no setter that answer
 * 0, so assigning one would throw in a module. One mechanism serves both, and
 * the descriptor is written back on restore.
 */
function setViewportProperty(target: object, key: string, value: number): void {
  rememberProperty(target, key);
  Object.defineProperty(target, key, {
    configurable: true,
    writable: true,
    value,
  });
}

/**
 * Sets the component baseline: 390×844 (`compact`) or 1280×800 (`expanded`).
 * Returns the viewport it applied, so a spec can assert against the numbers
 * rather than restating them. See the file header for what this does and does
 * not make true under jsdom.
 */
export function setComponentBaseline(
  baseline: ComponentBaselineName,
): ComponentViewport {
  const viewport = COMPONENT_BASELINES[baseline];
  const root = document.documentElement;

  setViewportProperty(window, 'innerWidth', viewport.width);
  setViewportProperty(window, 'innerHeight', viewport.height);
  setViewportProperty(root, 'clientWidth', viewport.width);
  setViewportProperty(root, 'clientHeight', viewport.height);

  return viewport;
}

/** The viewport `setComponentBaseline` applied, read back from the window. */
export function currentComponentViewport(): ComponentViewport {
  return { width: window.innerWidth, height: window.innerHeight };
}

/**
 * Puts every property `setComponentBaseline` touched back the way jsdom had it.
 * Called automatically after each test inside `describeAtBothBaselines`; a spec
 * that sets a baseline by hand in a `beforeEach` must call this in its own
 * `afterEach`, or the size leaks into the next test in the file.
 */
export function restoreComponentBaseline(): void {
  for (const { target, key, descriptor } of savedProperties) {
    if (descriptor === undefined) {
      Reflect.deleteProperty(target, key);
      continue;
    }

    Object.defineProperty(target, key, descriptor);
  }

  savedProperties.length = 0;
}

/**
 * The uniform baseline entry point: runs `body` **twice**, once at each
 * baseline, so TOOL-5's first two scenarios ("the component suite exercises a
 * 390×844 viewport", "…a 1280×800 viewport") are satisfied by construction for
 * every spec that uses it — the viewport is applied before each test and
 * restored after it, and the generated `describe` names the baseline so a
 * failure reads as the baseline it failed in:
 *
 * ```ts
 * import { describeAtBothBaselines } from '../test/setup';
 *
 * describeAtBothBaselines('DesignSystemPreview', () => {
 *   it('renders the skip link as the first focusable element', () => { … });
 * });
 * ```
 *
 * The `name` argument becomes `"<name> — compact baseline (390×844)"` and
 * `"<name> — expanded baseline (1280×800)"`; `body` receives the baseline name
 * for a spec that needs to branch on it. A spec that only cares about one
 * baseline uses `setComponentBaseline` in `beforeEach` and
 * `restoreComponentBaseline` in `afterEach` instead.
 */
export function describeAtBothBaselines(
  name: string,
  body: (baseline: ComponentBaselineName) => void,
): void {
  for (const baseline of COMPONENT_BASELINE_NAMES) {
    const { width, height } = COMPONENT_BASELINES[baseline];

    describe(`${name} — ${baseline} baseline (${width}×${height})`, () => {
      beforeEach(() => {
        setComponentBaseline(baseline);
      });

      afterEach(() => {
        restoreComponentBaseline();
      });

      body(baseline);
    });
  }
}
