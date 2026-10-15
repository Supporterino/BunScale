---
name: rust-conventions
description: Rust conventions for the Tauri backend: errors, async, structure, linting, testing. Load for any Rust work.
---
# Rust conventions

## Structure

- `ipc/` = thin command adapters. Domain logic in separate modules with no `tauri` dependency where possible (testable without an app handle).
- Modules by domain, not by type. No `utils.rs` dumping ground.

## Errors

- `thiserror` for typed errors in libraries/domain; `anyhow` only at binary edges, never across IPC.
- No `unwrap()`/`expect()` outside tests, except with an invariant comment.
- Convert to `AppError` at the IPC boundary; log the detail, return a sanitized message.

## Async

- `tokio` runtime from Tauri. No blocking calls in async fns: use `spawn_blocking` for CPU/blocking IO.
- Never hold a `std::sync::Mutex` guard across `.await`.
- Spawned tasks must be cancellable or tied to app lifecycle.

## Style and lint

- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` clean. No blanket `#[allow]`; targeted with reason.
- Prefer `&str`/slices in params, owned types in structs, `Arc` for shared state.
- `tracing` for logs, not `println!`.
- `unsafe` forbidden unless an ADR says otherwise.

## Platforms

- Entry in `lib.rs`: `#[cfg_attr(mobile, tauri::mobile_entry_point)] pub fn run()`. `main.rs` only calls `run()`.
- Gate platform code with `#[cfg(desktop)]` / `#[cfg(mobile)]` (set by `tauri-build`) or `cfg(target_os = "ios"|"macos")`. Desktop-only plugins (menu, window-state, global-shortcut, updater) registered under `cfg(desktop)`.
- Shared code must compile for `aarch64-apple-ios` and host macOS; the gate checks both.
- Assume suspension: no unbounded background tasks; persist state on `RunEvent` pause/exit.

## Persistence and paths

- Use Tauri path resolver (`app_data_dir`, etc.). No hardcoded paths.
- Validate all input from the frontend; the webview is untrusted.

## Tests

- Unit tests next to code, integration tests in `src-tauri/tests/`. Test domain logic without a running app.
