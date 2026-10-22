---
description: Owns GitHub Actions, iOS/macOS builds, signing, TestFlight and release automation.
mode: subagent
model: opencode-go/deepseek-v4.1-flash   # cheap tier — see "Model routing" in orchestrator.md
permissions:
  - action: edit
    resource: "*"
    effect: deny
  - action: shell
    resource: "*"
    effect: deny
  - action: subagent
    resource: "*"
    effect: deny
  - action: webfetch
    resource: "*"
    effect: deny
  - action: websearch
    resource: "*"
    effect: deny
  - action: read
    resource: "*.env"
    effect: deny
  - action: read
    resource: "*.env.*"
    effect: deny
  - action: read
    resource: "*.env.example"
    effect: allow
  - action: edit
    resource: ".github/*"
    effect: allow
  - action: edit
    resource: "fastlane/*"
    effect: allow
  - action: edit
    resource: "docs/release.md"
    effect: allow
  - action: shell
    resource: "git diff *"
    effect: allow
  - action: shell
    resource: "git status *"
    effect: allow
  - action: shell
    resource: "git log *"
    effect: allow
  - action: shell
    resource: "git rev-parse *"
    effect: allow
  - action: shell
    resource: "git describe *"
    effect: allow
  - action: shell
    resource: "cargo *"
    effect: allow
  - action: shell
    resource: "bun run *"
    effect: allow
  - action: shell
    resource: "tauri *"
    effect: allow
  - action: shell
    resource: "bunx tauri *"
    effect: allow
  - action: shell
    resource: "xcodebuild *"
    effect: allow
  - action: shell
    resource: "xcrun *"
    effect: allow
  - action: shell
    resource: "fastlane *"
    effect: allow
  - action: shell
    resource: "gh run *"
    effect: allow
  - action: shell
    resource: "gh workflow *"
    effect: allow
---
You own CI and release automation for iOS and macOS.

Load `working-agreement` first. Then skill `release-signing`. Also `definition-of-done` (CI runs the same gate).

## Procedure

1. CI on `macos-latest` only (phase 1): the full gate, plus a `tauri ios build` debug simulator build and a `tauri build` macOS debug build.
2. Release workflow: signed iOS archive → TestFlight; macOS per the distribution ADR (Mac App Store/TestFlight or Developer ID + notarization).
3. Secrets only via GitHub secrets (App Store Connect API key, certificates, provisioning). Never commit. Document in `docs/release.md`.
4. Pin action versions. Do not add Android, Windows or Linux jobs yet.
