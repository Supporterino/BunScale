//! Headless bindings export entry point (D0.9, task 0.11).
//!
//! `bun run gen:bindings` runs this as
//! `cargo run --manifest-path src-tauri/Cargo.toml --example export_bindings`, from the repository
//! root. It does one thing: call `app_lib::ipc::export_bindings`, which owns the output path —
//! resolved against `CARGO_MANIFEST_DIR`, never against the process working directory, so a caller
//! cannot hand it a working-directory-relative path — and writes `src/bindings.ts` alone.
//!
//! It is a cargo example deliberately, not a `src/bin/` target and not a test. The Tauri CLI's
//! bundler does not enumerate examples, so this target is never built as a bundle binary and cannot
//! end up inside or be signed into the app, while `cargo clippy --all-targets` still compiles and
//! lints it. A test would run inside the gate's own `cargo test` step, which precedes the bindings
//! drift check, and would rewrite the file that check is about to verify; for the same reason no
//! `#[test]` under `src-tauri/` may call this entry point or `export_bindings()`.
//!
//! `examples/` is not built by `cargo check --target aarch64-apple-ios` (that check passes no
//! `--all-targets`), so this host-only tooling does not affect the iOS target.

fn main() {
    // Loud on failure: the non-zero exit stops the gate instead of letting it compare a file that
    // was never written (D0.9).
    if let Err(error) = app_lib::ipc::export_bindings() {
        eprintln!("export_bindings failed: {error}");
        std::process::exit(1);
    }
}
