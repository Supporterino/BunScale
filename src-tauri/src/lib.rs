//! The application crate: the Tauri entry point, and the IPC contract surface it mounts.
//!
//! [`ipc`] owns the single builder constructor (D0.9); [`run`] mounts it, and the bindings export
//! example calls it. Nothing here holds domain logic.

pub mod ipc;

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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
