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
 * **How per-command handlers slot in later, without restructuring this file.**
 * The app calls no command yet — the IPC contract is empty until task 1.3
 * (`src-tauri/src/ipc/`, `collect_commands![]`) — so today the mock's whole job
 * is to make the flag harmless and the app renderable. When commands arrive,
 * each one is registered by name:
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

/**
 * One mocked command: receives the command's arguments (the object
 * `invoke(cmd, args)` was given, i.e. what the generated bindings pass) and
 * returns the command's result, synchronously or as a promise.
 */
export type MockCommandHandler = (args?: InvokeArgs) => unknown;

/*
 * The registry. A plain object keyed by the command name the Rust side exposes
 * (`collect_commands![]`) and the generated bindings call — so adding a command
 * is one entry. The key is a **hand-written literal** today, matched to the
 * generated command name by eye; the intended fix, once task 1.3 lands the first
 * command, is to type it as the generated bindings' command union (e.g.
 * `keyof typeof commands` from `src/bindings.ts`) so a mismatch is a compile
 * error rather than a runtime throw.
 *
 * The object is **null-prototype** on purpose. With `{}`, `commandHandlers[
 * command]` would resolve `Object.prototype`'s members, so a command named
 * `toString`, `constructor`, `valueOf`, `hasOwnProperty` or `__proto__` would be
 * dispatched instead of throwing — exactly the silent hole the throw below
 * exists to close. `Object.create(null)` leaves no prototype to fall back to;
 * `lib` is ES2021, so `Object.hasOwn` is not available to gate the lookup.
 */
const commandHandlers: Record<string, MockCommandHandler> = Object.create(null);

/**
 * Registers `handler` for `command`, replacing any previous registration for the
 * same name. Registration happens **at this module's top level**, which is the
 * only place it can happen today: a component spec never loads this module (it
 * is the Vitest half and mocks IPC itself — see the header), and a Playwright
 * spec's own module graph is Node's, so it cannot import this module and get the
 * page's instance. Varying an answer per E2E spec would therefore need a seam
 * this file does not provide yet — a hook exposed on `window`, or the spec
 * driving the page (e.g. `page.evaluate` dynamically importing the served
 * module, which is how task 0.23 verified the throw, and which is not a
 * per-test handler API). Tasks 1.3 and 3.6 must not assume one exists.
 */
export function registerMockCommand(
  command: string,
  handler: MockCommandHandler,
): void {
  commandHandlers[command] = handler;
}

/** The names currently registered. Read-only view, for diagnostics and specs. */
export function registeredMockCommands(): string[] {
  return Object.keys(commandHandlers);
}

/*
 * The install itself. `mockIPC` replaces `window.__TAURI_INTERNALS__.invoke`
 * with a function that runs this callback for every `invoke` call, so this is
 * the single dispatch point every mocked command goes through — which is also
 * what makes the registry above the only thing a later task has to extend. The
 * lookup is a plain index into the null-prototype registry, so an unregistered
 * name — including one that names an `Object.prototype` member — reads
 * `undefined` and reaches the throw rather than dispatching something inherited.
 *
 * The callback is deliberately not `async`: a handler's promise is returned
 * as-is and awaited by `invoke`, and the throw for an unknown command reaches
 * the caller as a rejected promise. `args` is passed through unchanged
 * (`InvokeArgs` is the type the package's own `invoke` accepts), so a handler
 * body is written against the same shape the real command receives.
 */
mockIPC(
  (command, args) => {
    const handler = commandHandlers[command];

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
