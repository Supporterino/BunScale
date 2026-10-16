//! The typed IPC contract surface.
//!
//! Owns the single `tauri-specta` builder that both the running app and the headless bindings
//! export are built from (D0.9), and the export function the export example calls. It holds no
//! domain logic, and in section 0 no command, event or shared type either: both `collect_*!` lists
//! are empty until task 1.3 fills them.
//!
//! Section 0 ships it this way deliberately: the scaffold's own gate must regenerate
//! `src/bindings.ts` with no diff before any command exists (TOOL-3). Task 1.3 extends the two
//! lists below and the types they reference; it does not change this pipeline.

use std::path::Path;

use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands, collect_events};

/// The generated frontend bindings file, relative to this crate's root (`src-tauri/`).
///
/// It is never used as a bare path: [`export_bindings`] joins it onto `CARGO_MANIFEST_DIR`,
/// because `bun run gen:bindings` runs from the repository root, where a bare `../src/bindings.ts`
/// would point outside the repository instead of failing.
const BINDINGS_PATH: &str = "../src/bindings.ts";

/// The one IPC builder constructor.
///
/// Both consumers share it, so what the running app registers and what the export writes cannot
/// disagree: the crate mounts it (task 0.10 — commands through `Builder::invoke_handler`, events
/// through `Builder::mount_events` inside the app's `setup`), and the export example calls
/// [`export_bindings`] (task 0.11).
///
/// The empty command and event sets are what section 0 ships; task 1.3 adds the P0 entries here.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![])
        .events(collect_events![])
}

/// Export the typed bindings headlessly: no window, no app, no running process.
///
/// The output path is resolved here, against `CARGO_MANIFEST_DIR`, rather than being taken from
/// the caller, so no caller can pass a path that resolves against the process working directory.
/// It writes exactly one file, [`BINDINGS_PATH`], and nothing else, so a bindings diff is
/// attributable to that file alone (D0.9).
///
/// Reached only through `cargo run --manifest-path src-tauri/Cargo.toml --example export_bindings`:
/// no test may call it, because `cargo test` precedes the bindings drift check in the gate and
/// would rewrite the file that check is about to verify (D0.9).
pub fn export_bindings() -> Result<(), specta_typescript::Error> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(BINDINGS_PATH);

    builder().export(Typescript::default(), path)
}
