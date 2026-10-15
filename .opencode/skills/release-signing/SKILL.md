---
name: release-signing
description: iOS and macOS build, signing, TestFlight and distribution flow for the Tauri app. Load for CI and release work.
---
# Release and signing (iOS + macOS)

## CI

- `macos-latest` runners. Rust targets: `aarch64-apple-ios`, `aarch64-apple-ios-sim`, host macOS.
- Gate from `definition-of-done`, then debug builds: `tauri ios build` (simulator) and `tauri build`.
- Cache `~/.cargo`, `src-tauri/target`, bun cache.

## iOS

| Item | Notes |
|---|---|
| Apple Developer Program | required |
| Signing | distribution certificate + provisioning profile, or automatic signing with App Store Connect API key |
| Config | `bundle.iOS.developmentTeam` in `tauri.conf.json`; bundle id fixed early |
| Delivery | `tauri ios build` → `.ipa` → TestFlight via App Store Connect API (fastlane `pilot` or `xcrun altool`/`notarytool` equivalents) |
| Updates | App Store/TestFlight only. No in-app updater. |

## macOS: decide via ADR before release work

| Option | Pros | Cons |
|---|---|---|
| Mac App Store (TestFlight) | same pipeline as iOS, store updates | sandbox required, review |
| Developer ID + notarization + updater | no sandbox constraint, own update channel | notarization, updater key management |

- Developer ID path: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, app-specific `APPLE_PASSWORD`, `APPLE_TEAM_ID`; `tauri signer generate` for updater key (`TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]`), public key in config, `tauri-plugin-updater` under `cfg(desktop)`.

## Rules

- No secrets in repo or logs; all via GitHub secrets. List them in `docs/release.md`.
- Version in `tauri.conf.json`, `Cargo.toml`, `package.json` synced by one script; build number auto-increments for TestFlight.
- Pin third-party actions.
- Android/Windows/Linux jobs are out of scope until the platform is added.
