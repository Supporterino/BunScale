/*
 * The E2E IPC mock (TOOL-5, `testing-strategy`).
 *
 * This module is **installed as a side effect of being imported**, and that is
 * the whole of its contract with the app: `src/main.tsx` reads
 * `import.meta.glob('./test/mock-ipc.ts')` and awaits the matching loader when
 * `import.meta.env.VITE_E2E_MOCK === '1'`, before the first render, so that
 * `invoke` is mocked before any route loader or query can call it. There is no
 * `install()` to call and nothing for the app to pass in: creating this file
 * is what fills the seam, with no edit to `src/main.tsx` (design.md D0.24).
 *
 * The flag is set by the `test:e2e` script itself (`VITE_E2E_MOCK=1 playwright
 * test`), which Playwright starts the Vite dev server as a child of, so the
 * dev server passes it to the page as `import.meta.env.VITE_E2E_MOCK`. A normal
 * `tauri dev` run or a shipped build never sets it, and then this module is
 * never loaded — a mock that switched itself on by itself would hide the real
 * backend.
 *
 * Why `mockIPC` from `@tauri-apps/api/mocks`: it is already a dependency
 * (`@tauri-apps/api` 2.12.4, D0.8) and it installs the fake
 * `window.__TAURI_INTERNALS__` that the rest of the package talks to — the
 * `invoke` function, `transformCallback`, the callback registry and the event
 * plugin's internals — so the app can boot headlessly with no native device and
 * no shared service (`testing-strategy`, "What cannot be automated yet"). No
 * new dependency, and nothing is hand-rolled that the pinned package already
 * provides.
 *
 * **How commands are answered.** Each command is registered by name:
 *
 * ```ts
 * registerMockCommand('load_household', (args) => ({ … }));
 * ```
 *
 * and `src/bindings.ts`'s generated wrapper for that command then resolves
 * against the registry with no change here, no change to `src/main.tsx` and no
 * change to the seam. Handlers live in one place, keyed by a command-name
 * string that has to match the one the generated bindings use
 * (`src/features/<name>/api.ts` is the only caller, per the project rules), and
 * a handler may return a promise — the registry's dispatch is awaited by
 * `invoke` itself.
 *
 * The handler returns the command's **value**, not a `Result` envelope: Tauri
 * rejects an `Err` and resolves an `Ok(T)` to the bare `T`, and the generated
 * `typedError` wrapper is what turns that into the frontend's
 * `{ status: 'ok', data }` union (`src/bindings.ts`). A handler that returned
 * the envelope would be wrapped a second time and every `CommandData<…>`
 * consumer would read the envelope as the value.
 *
 * **Defaults and per-test overrides.** The module registers the answers the
 * household entry guard (task 3.3) needs, so `/` can resolve without a spec
 * registering anything: a device that holds no household, with a fresh
 * `reachable` status whose account is valid. That is the state that lets the
 * guard reach a *conclusion* — "no household" only counts with a fresh
 * reachable-and-identified status (SYNC-3; design.md, "No household is a
 * fresh-outcome conclusion") — so the default is not an empty value that could
 * mean "nothing was loaded". A spec that needs another state overrides one
 * command for the rest of the page's document through the
 * `window.__BUNSCALE_E2E_MOCK__` seam below (tasks 5.14–5.17), and an override
 * is consulted before the registered default. Each fresh document starts from
 * the defaults again, so nothing leaks between tests.
 *
 * **One way this module silently does not run, measured while task 0.23 was
 * verified.** The flag reaches the page only through the dev server's own
 * environment, and `playwright.config.ts` sets
 * `reuseExistingServer: !process.env.CI`, so a dev server already listening on
 * 1420 that was started **without** the flag is reused as-is: the page then sees
 * an `import.meta.env` without `VITE_E2E_MOCK`, the seam takes its early return,
 * the app renders and no mock is installed. The symptom is exactly that shape:
 * the app renders while `window.__TAURI_INTERNALS__` stays absent. CI is not
 * exposed to it (it never reuses a server, D0.24), a local run is — so a spec
 * that depends on the mock should assert it is installed rather than assume it,
 * which is what makes a run *mocked* instead of merely green.
 *
 * Two decisions worth stating rather than leaving implicit:
 *
 *   - **An unregistered command throws instead of resolving to `undefined`.**
 *     The app's reads resolve against a locally kept copy (design.md D0.2), so
 *     a command that silently returned `undefined` would look like "nothing was
 *     ever loaded" — the state that has its own surface and is never an empty
 *     list — and a spec would pass while testing nothing. Throwing names the
 *     command, which is the same posture as the seam's own throw for an absent
 *     mock module (D0.24);
 *   - **`shouldMockEvents: true`,** so `plugin:event|*` calls are consumed by
 *     the package's event mock rather than reaching this registry. Events are a
 *     subscription mechanism, not a data read: with it on, `emit` from the app
 *     or from a spec reaches the listeners the app registered, which is what a
 *     flow like "a `household:changed` event invalidates the read" (task 3.6)
 *     needs. The registry below therefore carries app commands only.
 *
 * What this file is **not**: it is not `src/test/setup.ts`. This module is the
 * E2E half (a real WebKit page against the dev server) and is loaded only under
 * the flag; `src/test/setup.ts` is the Vitest half, loaded for every component
 * spec, and a component spec that wants IPC mocked calls `mockIPC` itself from
 * that spec (`testing-strategy`, "Frontend + IPC"), so the two never coexist in
 * one run.
 */

import type { InvokeArgs } from '@tauri-apps/api/core';
import { mockIPC } from '@tauri-apps/api/mocks';
import type { Household, HouseholdSummary, ServiceStatus } from '../bindings';

/**
 * One mocked command: receives the command's arguments (the object
 * `invoke(cmd, args)` was given, i.e. what the generated bindings pass) and
 * returns the command's value — never a `Result` envelope, which the generated
 * `typedError` adds (see the header) — synchronously or as a promise.
 */
export type MockCommandHandler = (args?: InvokeArgs) => unknown;

/*
 * The registry. A plain object keyed by the command name the Rust side exposes
 * (`collect_commands![]`) and the generated bindings call — so adding a command
 * is one entry. The key is a **hand-written literal**, matched to the generated
 * command name by eye; typing it as the generated bindings' command union (e.g.
 * `keyof typeof commands` from `src/bindings.ts`) would turn a mismatch into a
 * compile error instead of a runtime throw, and is the follow-up this shape
 * leaves open.
 *
 * The object is **null-prototype** on purpose. With `{}`, `commandHandlers[
 * command]` would resolve `Object.prototype`'s members, so a command named
 * `toString`, `constructor`, `valueOf`, `hasOwnProperty` or `__proto__` would be
 * dispatched instead of throwing — exactly the silent hole the throw below
 * exists to close. `Object.hasOwn` is available at `lib: ES2022` (Q16) and could
 * gate that read, but it does not make `{}` safe to write to: assigning a
 * handler under the name `__proto__` would invoke `Object.prototype`'s setter
 * and replace the registry's prototype rather than store a command.
 * `Object.create(null)` removes the prototype both the read and the write would
 * fall back to, so the lookup stays a plain index with no per-read guard.
 */
const commandHandlers: Record<string, MockCommandHandler> = Object.create(null);

/*
 * The per-document overrides (tasks 5.14–5.17). Separate from the registry so
 * `reset()` can drop an override and fall back to the default that was
 * registered above it, rather than editing the default itself. Null-prototype
 * for the same reason as the registry: a name such as `__proto__` must be a key,
 * not a prototype write.
 */
const overrideHandlers: Record<string, MockCommandHandler | undefined> =
  Object.create(null);

/**
 * Registers `handler` for `command`, replacing any previous registration for the
 * same name. Registration happens **at this module's top level** for the
 * defaults, which is the only durable place it can happen: a component spec
 * never loads this module (it is the Vitest half and mocks IPC itself — see the
 * header), and a Playwright spec's own module graph is Node's, so it cannot
 * import this module and get the page's instance. Varying an answer per E2E
 * spec therefore goes through the page-side seam below
 * (`window.__BUNSCALE_E2E_MOCK__`), which layers an override on top of these
 * defaults and is what tasks 5.14–5.17 drive.
 */
export function registerMockCommand(
  command: string,
  handler: MockCommandHandler,
): void {
  commandHandlers[command] = handler;
}

/** The names currently registered, defaults and overrides alike. */
export function registeredMockCommands(): string[] {
  const names = new Set(Object.keys(commandHandlers));

  for (const command of Object.keys(overrideHandlers)) {
    names.add(command);
  }

  return [...names];
}

/*
 * The page-side override seam (tasks 5.14–5.17).
 *
 * Why it exists: a Playwright spec runs in Node and this module runs in the
 * page, so a spec cannot hand a handler to `registerMockCommand` — the function
 * would have to cross the browser boundary. This object is installed on the
 * page's `window` and is the one stable handle a spec can reach through
 * `page.evaluate` (the same technique with which task 0.23 verified the
 * unknown-command throw). It only ever exists in a document where the flag is
 * set, because this module is only loaded then.
 *
 * `setResult` is the common case: a fixed answer for one command, for example
 * `get_service_status` as `unreachable` so the guard shows the
 * connectivity-required state. `setHandler` takes a function the spec builds
 * **inside** the page context, which is what a sequence needs (a first attempt
 * failing, a retry succeeding — task 5.16). `reset` restores the defaults:
 * `reset()` for the whole document, `reset(command)` for one command.
 *
 * All three write the overrides map, never the registry, so a spec's changes
 * cannot outlive the document: the next `page.goto` loads this module again and
 * starts from the defaults.
 */
export type MockCommandSeam = {
  /** Answer `command` with a fixed value for the rest of this document. */
  setResult(command: string, result: unknown): void;
  /** Answer `command` with a function, built in the page context. */
  setHandler(command: string, handler: MockCommandHandler): void;
  /** Drop one override, or every override when `command` is omitted. */
  reset(command?: string): void;
  /** The names currently answerable, defaults and overrides alike. */
  registered(): string[];
};

declare global {
  interface Window {
    __BUNSCALE_E2E_MOCK__?: MockCommandSeam;
  }
}

const mockSeam: MockCommandSeam = {
  setResult(command, result) {
    overrideHandlers[command] = () => result;
  },
  setHandler(command, handler) {
    overrideHandlers[command] = handler;
  },
  reset(command) {
    const names =
      command === undefined ? Object.keys(overrideHandlers) : [command];

    for (const name of names) {
      delete overrideHandlers[name];
    }
  },
  registered: registeredMockCommands,
};

window.__BUNSCALE_E2E_MOCK__ = mockSeam;

/*
 * The defaults the household entry guard (task 3.3) calls, shaped exactly as the
 * frozen contract states them (`src/bindings.ts` is generated from it, so these
 * literals are type-checked against it): `HouseholdSummary[]` with
 * `{ id, name, created_at, is_active, is_owner, member_count }`, `Household` with
 * `{ id, name, created_at, owner_member_id, is_active, is_owner }`, and
 * `ServiceStatus` with `{ state, account_valid, cause, observed_at }`.
 *
 * No household plus a fresh `reachable`/`account_valid: true` status is the one
 * state in which "the person holds no household" is a conclusion rather than an
 * unloaded device (SYNC-3), so it is the honest default: the guard may start
 * onboarding from it, and a spec that needs an unreachable or populated device
 * overrides the one command whose answer differs.
 *
 * `observed_at` is a fixed instant rather than `new Date()`: the entry guard
 * treats a status without an observation as "not looked yet" and probes, so a
 * null here would silently exercise a different branch, and a clock read would
 * make the default non-deterministic (`testing-strategy`).
 */
const OBSERVED_AT = '2026-10-20T09:00:00Z';

const DEFAULT_SERVICE_STATUS: ServiceStatus = {
  state: 'reachable',
  account_valid: true,
  cause: null,
  observed_at: OBSERVED_AT,
};

registerMockCommand('list_households', (): HouseholdSummary[] => []);

registerMockCommand('get_active_household', (): Household | null => null);

registerMockCommand('get_service_status', () => DEFAULT_SERVICE_STATUS);

registerMockCommand('check_service_reachability', () => DEFAULT_SERVICE_STATUS);

/*
 * The install itself. `mockIPC` replaces `window.__TAURI_INTERNALS__.invoke`
 * with a function that runs this callback for every `invoke` call, so this is
 * the single dispatch point every mocked command goes through — which is also
 * what makes the registry above the one place every answer lives. The lookup
 * consults the per-document overrides first and the registered defaults second,
 * so an override shadows a default without rewriting it. Both are plain indexes
 * into null-prototype objects, so an unregistered name — including one that
 * names an `Object.prototype` member — reads `undefined` and reaches the throw
 * rather than dispatching something inherited.
 *
 * The callback is deliberately not `async`: a handler's promise is returned
 * as-is and awaited by `invoke`, and the throw for an unknown command reaches
 * the caller as a rejected promise. `args` is passed through unchanged
 * (`InvokeArgs` is the type the package's own `invoke` accepts), so a handler
 * body is written against the same shape the real command receives.
 */
mockIPC(
  (command, args) => {
    const handler: MockCommandHandler | undefined =
      overrideHandlers[command] ?? commandHandlers[command];

    if (handler === undefined) {
      throw new Error(
        `VITE_E2E_MOCK=1 but no mock handler is registered for "${command}": ` +
          'register it with registerMockCommand in src/test/mock-ipc.ts',
      );
    }

    return handler(args);
  },
  { shouldMockEvents: true },
);
