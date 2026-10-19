//! The application crate: the Tauri entry point, and the IPC contract surface it mounts.
//!
//! [`ipc`] owns the single builder constructor (D0.9); [`run`] mounts it, and the bindings export
//! example calls it. Nothing here holds domain logic.

pub mod diagnostics;
pub mod domain;
pub mod ipc;
pub mod settings;
pub mod state;
pub mod storage;
pub mod sync;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // One builder, from the one constructor (D0.9): commands are registered through its
    // `invoke_handler`, events through `mount_events` below. Constructed here and moved into the
    // `setup` closure, which is `'static` and so cannot borrow a local; `Builder<Wry>` is cloned
    // nowhere, because both calls take `&self`.
    let ipc_builder = ipc::builder();

    tauri::Builder::default()
        .invoke_handler(ipc_builder.invoke_handler())
        // The crate's only `setup` call: `tauri::Builder::setup` assigns rather than composes, so
        // a helper installing its own would clobber this one.
        .setup(move |app| {
            ipc_builder.mount_events(app);
            // The one app-wide state: the shared service, the local copies, the status tracker and
            // the household book the section-2 commands read (`state::AppState`). It is installed
            // here rather than by a helper for the reason above. `production` reports a client that
            // could not be built instead of hiding it; P0 has no account, so every write command
            // still refuses honestly with `no_account` (Q25).
            app.manage(crate::state::AppState::production(app.handle())?);
            Ok(())
        })
        .run(tauri::generate_context!())
        // Unrecoverable, so a panic is the honest report — the invariant `rust-conventions` requires
        // an `expect` outside tests to state. `run` is the process's last frame: `src/main.rs` calls
        // it and returns, and on mobile tauri's `mobile_entry_point` wrapper calls it inside
        // `stop_unwind`, which discards a returned value, so neither caller could act on a `Result`.
        // The only failure reachable here is `Builder::build` failing before the event loop starts
        // (`Builder::run` is `self.build(context)?.run(..)`), after which there is no app to carry on
        // with and nothing to report through. The message is the whole failure surface a person
        // sees: stderr on macOS; on iOS the mobile entry-point wrapper's `stop_unwind` catches the
        // panic, logs `attempt to unwind out of \`rust\` with err: …` and calls
        // `std::process::abort()`, so the panic never unwinds out to the app edge — and
        // `tauri::log_stdout` only redirected stdout and stderr into the device log before the
        // wrapper body ran.
        .expect("error while building tauri application");
}
