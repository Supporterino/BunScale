# SETUP.md — local development setup

Machine prerequisites for a clean checkout of BunScale on macOS. Referenced by the
`project-toolchain` requirement TOOL-1 and by task 0.1 of
`openspec/changes/add-shared-household-foundation`.

Phase 1 targets iOS (primary) and macOS. Android, Windows and Linux are later; nothing here
installs them.

## 1. Xcode

1. Install Xcode from the App Store (the full application, not only the Command Line Tools) with at
   least one iOS simulator runtime.
2. Point the toolchain at it and accept the licence:

   ```sh
   sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
   sudo xcodebuild -license accept
   ```

3. Confirm the installation:

   ```sh
   xcodebuild -version
   xcodebuild -showsdks
   ```

The minimum iOS and macOS deployment targets are **not** chosen here: the installed SDKs' minimum is
measured by task 0.7 of the P0 change and recorded in that change's `design.md` (D0.16). The **shipped**
floor is a product decision on top of that measurement — **iOS 16.4 / macOS 13.3**, raised on 2026-10-20
to match Vite's default build target (Q16) — and is set in `src-tauri/tauri.conf.json`.

## 2. Rust and the iOS target

Install Rust with [rustup](https://rustup.rs), then add the iOS device target:

```sh
rustup target add aarch64-apple-ios
rustup target list --installed
```

`aarch64-apple-ios` is required by the gate's iOS compile check
(`cargo check --manifest-path src-tauri/Cargo.toml --target aarch64-apple-ios`) and by
`cargo tauri ios init` / `cargo tauri ios dev`.

## 3. bun

Install [bun](https://bun.sh) as the package manager:

```sh
curl -fsSL https://bun.sh/install | bash
bun --version
```

## 4. Project dependencies

```sh
bun install
```

The dependency set is pinned to exact versions in `package.json`; do not widen it. New dependencies
need a one-line justification in the owning change's `design.md`.

## 5. Tauri CLI

Local runs and CI use the **exactly pinned** `@tauri-apps/cli` dev dependency, which
`bun install` has already put in place:

```sh
bun run tauri --version
```

Task 0.1 of the P0 change is the one exception: it bootstraps `src-tauri/` with the globally
installed official CLI before any `package.json` exists.

```sh
cargo install tauri-cli --version "^2" --locked
```

That global install is a one-off bootstrap step and is not used afterwards.

## 6. Verify the setup

A prepared machine runs the full gate from `definition-of-done` (mirrored in the `AGENTS.md` command
table and in `.github/workflows/ci.yml`):

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml --target aarch64-apple-ios
bun run gen:bindings     # then: git diff --exit-code src/bindings.ts
bun run typecheck
bun run lint
bun run test
bun run test:e2e
```

## 7. Running the app

```sh
bun run tauri dev        # macOS
bun run tauri ios dev    # iOS simulator
```

Signing, notarisation and TestFlight are not part of local setup; they are handled by the
`release-signing` flow when release work starts.
