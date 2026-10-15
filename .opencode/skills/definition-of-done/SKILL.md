---
name: definition-of-done
description: The exact gate commands and completion criteria for any task. Load before claiming a task is done.
---
# Definition of done

All must pass, run fresh, in order:

```
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml --target aarch64-apple-ios
bun run gen:bindings            # then: git diff --exit-code src/bindings.ts
bun run typecheck
bun run lint
bun run test
```

Router generation: after a build, `git diff --exit-code src/routeTree.gen.ts`.

`bun run test:e2e` when the task changes a user-visible flow.

## UI tasks additionally

- Screenshots at 390×844 and 1280×800 in `.artifacts/screens/`.
- Reviewer confirmed touch targets, safe areas and adaptive layout.

## Also required

- Task's spec scenarios implemented and tested.
- Reviewer verdict `APPROVE`.
- No generated file edited by hand.
- One commit: `feat|fix|test|chore(<change-id>): <task>`.
- Task ticked in `tasks.md`.

## Reporting

State commands run and outcomes. "Should pass" is not a result.

> Verify once that `cargo check --target aarch64-apple-ios` works in your setup (needs Xcode + the rustup target). Keep this file and the `AGENTS.md` table in sync.
