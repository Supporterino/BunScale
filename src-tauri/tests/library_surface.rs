//! Integration tests for the library crate's public surface (task 0.12, TOOL-1).
//!
//! `src-tauri/tests/` is where tests that link `app_lib` as a library live; unit tests stay beside
//! their code (`rust-conventions`). This is the harness's first target, so it covers what the
//! scaffold's public surface promises rather than one domain behaviour — the scaffold has no domain
//! module yet — and task 0.24 adds the scaffold's smoke tests alongside it.
//!
//! Three limits shape what can be asserted here, and each one is a rule rather than a gap to be
//! filled later. `app_lib::run()` starts the Tauri event loop, so it is never called: what is
//! checkable without launching an app is the context `run` hands to `tauri::Builder::run` and the
//! shape of the builder it mounts. `app_lib::ipc::export_bindings()` is never called, and nothing
//! here emits bindings, by the rule D0.9 records: `cargo test` precedes the two generated-artefact
//! drift checks in the gate, so a test that exported would rewrite the very files those checks are
//! about to verify. And `tauri::ipc::Invoke` cannot be constructed outside tauri, so the mounted
//! handler cannot be called — what proves the empty command and event set is the exported bindings
//! file, not this harness.
//!
//! Both platforms (TOOL-1).

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::thread;

use app_lib::ipc;

/// Holds `Send + Sync` for a value's type at compile time.
///
/// The call compiles only while `T` satisfies both bounds and checks nothing at run time. It is what
/// pins the `Send + Sync` half of the `Fn(Invoke<R>) -> bool + Send + Sync + 'static` that
/// `tauri_specta::Builder::invoke_handler` declares: the function is returned as an opaque
/// `impl Trait`, so a bound dropped from that declaration would otherwise stop being enforced here
/// without failing to build.
fn assert_send_sync<T: Send + Sync>(_: &T) {}

/// Every entry of the repository's `src/` (the crate root's parent), rendered so that two captures
/// are equal exactly when the tree is unchanged, and sorted so the comparison does not depend on
/// directory order.
///
/// A regular file contributes its contents, not just its name: the violation D0.9 forbids — a
/// `#[test]` calling `export_bindings()` — rewrites the `src/bindings.ts` that already exists, which
/// leaves a name-only comparison identical. Anything that is not a regular file contributes its
/// kind, which is everything it can vary by without someone writing to it.
fn repository_src_entries() -> Vec<(String, Vec<u8>)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src");

    let read_dir = match fs::read_dir(&src) {
        Ok(read_dir) => read_dir,
        // On a clean checkout `src/` does not exist at all: its only file today is the untracked
        // `src/bindings.ts` that task 0.16 commits, and git cannot carry an empty directory, so the
        // directory appears only when that file does. A missing directory is no entries rather than
        // a failure, and the comparison stays meaningful in that state too — anything that wrote a
        // file there would show up in the second capture. Every other kind of error stays loud.
        Err(error) if error.kind() == ErrorKind::NotFound => return Vec::new(),
        Err(error) => panic!("cannot read {}: {error}", src.display()),
    };

    let mut entries: Vec<(String, Vec<u8>)> = read_dir
        .map(|entry| {
            let entry = entry.unwrap_or_else(|error| {
                panic!("cannot read an entry of {}: {error}", src.display())
            });
            let name = entry.file_name().to_string_lossy().into_owned();
            let file_type = entry
                .file_type()
                .unwrap_or_else(|error| panic!("cannot read the type of {name}: {error}"));

            let content = if file_type.is_file() {
                fs::read(entry.path()).unwrap_or_else(|error| panic!("cannot read {name}: {error}"))
            } else if file_type.is_dir() {
                b"dir".to_vec()
            } else if file_type.is_symlink() {
                b"symlink".to_vec()
            } else {
                // Sockets, fifos and devices: none expected under `src/`, but a change of kind is
                // still a change, so it is described rather than ignored.
                b"other".to_vec()
            };

            (name, content)
        })
        .collect();

    entries.sort();
    entries
}

#[test]
fn the_context_the_binary_edge_runs_carries_the_scaffold_identity() {
    // The same macro `app_lib::run()` passes to `tauri::Builder::run`, so this is the crate's real
    // compile-time input, built here without starting anything (D0.15). The runtime is named because
    // this is a `let`: the macro's `default_runtime` only fills the parameter in where the type is
    // inferred.
    let context: tauri::Context<tauri::Wry> = tauri::generate_context!();

    assert_eq!(context.config().identifier, "com.bunscale.app");
    assert_eq!(context.config().product_name.as_deref(), Some("BunScale"));
}

/// What this test is: two compile-time bounds and one move.
///
/// `assert_send_sync` holds the `Send + Sync` that `tauri::Builder::invoke_handler` requires of the
/// handler it is given, and the move holds the `Send + 'static` that `tauri::Builder::setup` requires
/// of the closure it takes. The thread body is infallible, so there is no run-time condition left to
/// assert, and this test does not pretend there is one.
#[test]
fn the_ipc_builder_hands_out_a_mountable_handler_and_survives_being_moved() {
    let builder = ipc::builder();

    // `run()` takes the handler through a shared borrow and only afterwards moves the same value
    // into the `'static` `setup` closure, so two handlers have to come off one builder — the reading
    // of "one builder, mounted and exported" D0.9 records.
    let invoke_handler = builder.invoke_handler();
    let second_handler = builder.invoke_handler();

    assert_send_sync(&invoke_handler);
    assert_send_sync(&second_handler);

    // The move is the other half: it proves the builder and both handlers can cross into `setup`'s
    // `'static` closure. The body has no panic path, so the join below is not a check — it is how the
    // moved values stay alive to the end of the test instead of a detached thread outliving it.
    let moved = thread::spawn(move || (builder, invoke_handler, second_handler));
    let _values = moved.join();
}

#[test]
fn building_the_ipc_builder_creates_removes_or_rewrites_no_source_file() {
    let before = repository_src_entries();

    let builder = ipc::builder();
    let _ = builder.invoke_handler();

    // Only the explicit `cargo run --example export_bindings` writes bindings, and it writes exactly
    // `src/bindings.ts` (D0.9). Constructing and mounting the builder must therefore leave the
    // repository's `src/` untouched, contents included: if it did not, the gate's own `cargo test`
    // step could rewrite the file the bindings drift check verifies, and a rewrite in place is
    // exactly what comparing names alone would miss.
    assert_eq!(before, repository_src_entries());
}
