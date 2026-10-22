---
name: definition-of-done
description: The exact gate commands, their order, who runs them and what counts as done. Load before claiming a task is done.
---

# Skill: definition-of-done

## Who runs it

The **orchestrator**, once per task, and it never trusts a specialist's claim.
Specialists run only the suite they touched. CI runs the same commands.

## The gate

Cheapest first, stop at the first red stage. Fix, then resume at that stage —
do not restart the whole gate.

| # | Stage | Command | Runs when |
|---|---|---|---|
| 1 | Rust fmt | `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | `src-tauri/**` changed |
| 2 | Rust lint | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | `src-tauri/**` changed |
| 3 | Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml` | `src-tauri/**` changed |
| 4 | iOS compile | `cargo check --manifest-path src-tauri/Cargo.toml --target aarch64-apple-ios` | `src-tauri/**` changed |
| 5 | Bindings | `bun run gen:bindings` then `git diff --exit-code src/bindings.ts` | IPC types, commands or events changed |
| 6 | Router | `git diff --exit-code src/routeTree.gen.ts` after a build | a route was added or moved |
| 7 | Typecheck | `bun run typecheck` | `src/**` changed |
| 8 | Lint | `bun run lint` | `src/**` changed |
| 9 | Unit/component | `bun run test` | `src/**` changed |
| 10 | E2E | `bun run test:e2e` | a user-visible flow changed |

A skipped stage is reported as skipped, with the condition that made it skip.
Skipping a stage whose condition holds is a failed gate, not an optimisation.

## Output

- Passing stages: one line each, `<stage> ok`. No command output.
- A failing stage: the failing lines only (`… 2>&1 | tail -40`), then the fix.
- Never paste a full passing run, `cargo` build log or vite startup banner.

## UI tasks additionally

- Screenshots at 390×844 and 1280×800 in `.artifacts/screens/` — two, not a set.
- Reviewer confirmed touch targets, safe areas and adaptive layout.

## Also required

- The task's spec scenarios are implemented and tested.
- Reviewer verdict `APPROVE`.
- No generated file edited by hand.
- One commit: `feat|fix|test|chore(<change-id>): <task>`.
- Task ticked in `tasks.md`.

## Change level

At the end of a change, not per task: `openspec validate <id> --strict`, then
archive, then `graphify update .` once, then commit.

> Verify once that `cargo check --target aarch64-apple-ios` works in your setup
> (needs Xcode + the rustup target). Keep this file and the `AGENTS.md` table in
> sync.
