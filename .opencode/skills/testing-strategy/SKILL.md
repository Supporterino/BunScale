---
name: testing-strategy
description: Test pyramid and tooling for Rust, React and E2E in a mobile-first Tauri app on iOS/macOS, including IPC mocking and viewports. Load for any test work.
---
# Testing strategy

| Layer | Tool | Scope |
|---|---|---|
| Rust unit | `cargo test` (`#[cfg(test)]`) | domain logic, no app handle |
| Rust integration | `src-tauri/tests/` | module boundaries, persistence |
| Frontend unit/component | vitest + Testing Library | features, hooks, routes |
| E2E | Playwright (WebKit) + mocked IPC | main flows at two viewports |
| Native smoke | manual on simulator (phase 1) | boot, lifecycle, native plugins |

## Rules

- Each spec `#### Scenario:` maps to ≥ 1 test, named after it.
- Test behavior, not implementation. Never weaken, skip or delete a failing test.
- Deterministic: no real time/network; inject clocks and fakes.

## Viewports

Run component and E2E tests at **390×844** (compact) and **1280×800** (expanded). Assert adaptive navigation (tabs vs sidebar), not just rendering.

## Frontend + IPC

```ts
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
beforeEach(() => mockIPC((cmd, args) => { if (cmd === "load_settings") return {...}; }));
afterEach(() => clearMocks());
```

- Wrap in `QueryClientProvider` (`retry: false`). Routes: `createMemoryHistory`.
- E2E bootstraps `src/test/mock-ipc.ts` when `VITE_E2E_MOCK=1`.

## What cannot be automated yet

- `tauri-driver` does **not** support iOS or macOS (no WKWebView driver). Do not use it.
- Real-device/simulator E2E (Maestro or XCUITest) is phase 2. Until then, native behavior (lifecycle, Keychain, plugins) is covered by Rust/Swift unit tests plus manual simulator checks listed in the change's `tasks.md` under "Manual verification".

## Coverage

No numeric target. Every scenario covered; every bug fix gets a regression test.
