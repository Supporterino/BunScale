---
name: tauri-apple-plugins
description: Writing custom Tauri v2 plugins with Swift for iOS and macOS. Load for any native Apple code.
---
# Apple native plugins

## Prefer existing plugins

Check official `tauri-plugin-*` (biometric, haptics, notification, deep-link, etc.) first. Verify iOS **and** macOS support before adopting.

## Structure

```
src-tauri/plugins/<name>/
  Cargo.toml, build.rs, src/lib.rs   Rust API + commands
  ios/Package.swift, Sources/        Swift package (Tauri plugin API)
  permissions/                       generated permission files
```

- Swift plugin class subclasses `Plugin`; commands take `Invoke` and resolve/reject with typed payloads (`Decodable` args).
- Rust side registers via `register_ios_plugin` under `cfg(target_os = "ios")`; macOS native code uses Rust bindings (`objc2`) or the same Swift package if supported by your Tauri version. Verify against current docs.

## Rules

- Errors map to `AppError` kinds; never throw raw strings across the bridge.
- Main-thread UI work in Swift only (`DispatchQueue.main` / `@MainActor`).
- No force-unwrap in Swift. Handle missing permission as a typed error, not a crash.
- Permission usage strings (`NSFaceIDUsageDescription`, etc.) and entitlements are `tauri-security` responsibility.
- Handle app lifecycle (background/foreground) for anything holding resources.
