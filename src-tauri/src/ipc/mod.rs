//! The typed IPC contract surface.
//!
//! Owns the single `tauri-specta` builder that both the running app and the headless bindings
//! export are built from (D0.9), and the export function the export example calls. It holds no
//! domain logic: [`types`] fixes the shared shapes, [`events`] fixes the two P0 event names,
//! [`commands`] holds typed stubs whose bodies section 2 replaces, and [`failure`] is the one place
//! a [`types::CommandError`] is built from (task 2.11).
//!
//! The P0 contract was frozen by task 1.3. `src/bindings.ts` is generated from exactly this
//! builder, so the frontend reaches every command only through `src/features/*/api.ts` wrappers
//! over those bindings (project hard rule 2): no route, component or hook imports the bindings or
//! writes an `invoke` call itself.
//!
//! Section 0 shipped this module with an empty command and event set, deliberately, so the
//! scaffold's own gate could regenerate `src/bindings.ts` with no diff before any command existed
//! (TOOL-3). Task 1.3 filled the two `collect_*!` lists below, added the types they reference and
//! registered `Member` explicitly (no P0 command returns it yet); it does not change the pipeline,
//! so what the running app registers and what the export writes stay one and the same set.

use std::path::Path;

use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands, collect_events};

pub mod commands;
pub mod events;
pub mod failure;
pub mod types;

pub use events::{HouseholdChangeKind, HouseholdChanged};
pub use types::*;

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
/// The lists below are the frozen P0 contract (task 1.3): eleven commands and two events, all
/// declared in [`commands`] and [`events`]. `Member` is registered explicitly with
/// [`Builder::typ`] because P0 freezes the member model (D0.3) but no P0 command returns it yet —
/// P3's member list is its first consumer, and exporting it now is what keeps that shape one
/// frozen definition rather than two.
///
/// Each command is named by its **defining** area module — `commands::household::create_household`
/// and so on — not by [`commands`]'s re-export of it. `collect_commands!` expands to tauri's
/// `generate_handler!` and specta's `collect_functions!`, and both resolve a hidden wrapper macro at
/// the path they are handed (`__cmd__<name>` and `__specta__fn__<name>` respectively, in place of
/// the function's own name), while a `pub use` of the function re-exports the function alone. The
/// names the frontend sees are unaffected: both macros take the command's name from the function's
/// own identifier, so `src/bindings.ts` is the same file either way.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::household::create_household,
            commands::household::list_households,
            commands::household::get_active_household,
            commands::household::set_active_household,
            commands::sync::load_household,
            commands::sync::get_service_status,
            commands::sync::check_service_reachability,
            commands::identity::get_identity,
            commands::settings::get_display_weight_unit,
            commands::settings::set_display_weight_unit,
            commands::diagnostics::get_diagnostics_summary,
        ])
        .events(collect_events![
            events::HouseholdChanged,
            types::ServiceStatus
        ])
        .typ::<types::Member>()
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
