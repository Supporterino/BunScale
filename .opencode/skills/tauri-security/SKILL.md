---
name: tauri-security
description: Tauri v2 security rules: capabilities, permissions, CSP, scopes, updater. Load for any config or permission change.
---
# Tauri security

## Capabilities (`src-tauri/capabilities/*.json`)

- One capability file per window/purpose. Explicit `windows`/`webviews` lists.
- Grant the minimal permissions; prefer specific (`fs:allow-read-text-file`) over broad (`fs:default`).
- Every new plugin = new permission review. Core defaults only when needed.

## Scopes

- FS: allow only app dirs (`$APPDATA/**`, `$APPCONFIG/**`) or user-picked paths via dialog. Explicit deny for sensitive locations.
- Shell/opener: allowlisted commands and args by regex. No arbitrary command execution.
- HTTP: allowlisted URLs.

## CSP (`app.security.csp`)

- `default-src 'self'`; `connect-src` only needed origins, plus `ipc:` / `http://ipc.localhost` as Tauri requires.
- No `unsafe-eval`. No remote scripts. Inline styles only if unavoidable (ADR).
- Never disable CSP.

## Platforms

- Capability files carry `"platforms": [...]` (`"iOS"`, `"macOS"`). Desktop-only permissions never apply to iOS.
- iOS: usage description strings in `Info.plist` for each used API (camera, Face ID, photos, etc.); ATS defaults on. Secrets in Keychain, not files or `localStorage`.
- macOS: App Sandbox entitlements minimal (`com.apple.security.app-sandbox`, plus only needed network/file entitlements); hardened runtime for distribution.
- Deep links / universal links: explicit domains, validate every incoming URL as untrusted input.
- Document each entitlement and usage string in `design.md` with its reason.

## Frontend assumptions

- Treat webview content as untrusted. All commands validate inputs on the Rust side.
- No secrets in frontend code or `tauri.conf.json`.

## Updater

- iOS: no in-app updater (App Store/TestFlight only). macOS: updater only if distributing outside the Mac App Store; public key in config, HTTPS endpoints, private key never in repo (see `release-signing`).

## Change protocol

Every widening: one line in the change's `design.md` stating what, why, and the narrower option rejected.
