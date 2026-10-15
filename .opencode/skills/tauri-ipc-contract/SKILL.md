---
name: tauri-ipc-contract
description: Rules for Tauri v2 commands, events and typed bindings between Rust and the frontend. Load for any IPC work.
---
# IPC contract

## Typed bindings

- Use `tauri-specta` v2 (still `2.0.0-rc.x`; pin tauri-specta and specta to exact versions together, bump deliberately). Never hand-write or edit `src/bindings.ts`.
- Register with `Builder::<tauri::Wry>::new().commands(collect_commands![...]).events(collect_events![...])`, mount events in `setup`, annotate commands with `#[tauri::command]` + `#[specta::specta]`.
- `bun run gen:bindings` must export headlessly (no running app). Implement once as a Rust test or small bin that calls `builder.export(Typescript::default(), "../src/bindings.ts")`; the architect creates it in the first IPC change. CI fails on a diff after regeneration.
- Alternative to evaluate via ADR: `tauri-typed-ipc` (trait-based, built-in drift check, young 0.1.x).

## Commands

- Naming: `snake_case`, `<verb>_<noun>`: `load_settings`, `start_import`.
- Coarse and domain-level. No CRUD mirror of the database.
- Args: one typed struct for ≥ 3 params. All types derive `Serialize, Deserialize, specta::Type`.
- Async commands for anything touching IO. Never block the main thread.
- State via `tauri::State<T>`; interior mutability with `tokio::sync` primitives for async, `std::sync::Mutex` only when never held across `.await`.
- Return `Result<T, AppError>`.

## Errors

```rust
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(tag = "kind", content = "message")]
pub enum AppError { NotFound(String), Validation(String), Io(String), Internal(String) }
```

- Tagged enum so the frontend can switch on `kind`. No stringly-typed errors. No leaking paths/secrets in messages.

## Events

- Naming: `domain:event` (`import:progress`). Typed payloads via specta events.
- Use for progress, push updates and backend-initiated changes. Not for request/response.
- Long tasks: command returns an id immediately, progress by events, cancel by command.

## Frontend side

- Only `src/features/<f>/api.ts` imports `bindings`. Components and routes use those wrappers / query hooks.
- Handle `AppError.kind` explicitly.

## Changing the contract

Edit `design.md` first, then stubs, regenerate, then both sides. Breaking change = new spec delta.
