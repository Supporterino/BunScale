# project-toolchain Specification

## Purpose
The toolchain establishes the workspace scaffold every other capability builds on: a clean checkout on macOS builds with no manual step beyond the documented setup and passes the complete `definition-of-done` gate, including the iOS target compile check and headless tests, while the app boots on both the iOS simulator and macOS. The generated typed bindings and route tree are produced reproducibly, committed and never hand-edited, and a build fails naming the file when either drifts from its source. The harness runs unit/component tests and a WebKit E2E suite with IPC mocked at both the 390×844 and 1280×800 baselines, the design system's tokens and shared primitives are recorded as defined before dependent UI work starts, CI on a macOS runner runs the same gate without signing or distributing, and every third-party dependency is pinned to an exact version and usable on both iOS and macOS.

## Requirements

### Requirement: Workspace builds from a clean checkout and the full gate passes

**TOOL-1.** A clean checkout on macOS SHALL build with no manual step beyond the documented setup,
and the complete `definition-of-done` gate SHALL pass on it: Rust formatting, Rust linting, Rust
tests, the iOS target compile check, bindings regeneration with no diff, route-tree regeneration
with no diff, TypeScript typechecking, linting, unit/component tests and the E2E suite. A gate
command that fails SHALL prevent any task from being reported as done.

#### Scenario: A fresh checkout passes the gate [macos]

- **WHEN** the repository is checked out on a macOS machine prepared per `SETUP.md`
- **THEN** every command of the `definition-of-done` gate runs and exits successfully

#### Scenario: The iOS target compiles [ios]

- **WHEN** the gate runs the iOS target compile check
- **THEN** the crate compiles for `aarch64-apple-ios` without errors or warnings

#### Scenario: The gate runs without a running app

- **WHEN** the gate runs on a checkout where no app process is running
- **THEN** Rust tests, typechecking, linting, unit/component tests and the E2E suite all run headlessly

### Requirement: The app boots on the iOS simulator and on macOS

**TOOL-2.** The scaffold SHALL launch on an iOS simulator and on macOS and SHALL present its first
screen without crashing, so that native boot and lifecycle behaviour can be verified while native
smoke testing is manual (see `testing-strategy`).

#### Scenario: The app launches on the iOS simulator [ios]

- **WHEN** the scaffold is started on an iOS simulator
- **THEN** the app launches and its first screen is presented
- **AND** it does not crash during launch

#### Scenario: The app launches on macOS [macos]

- **WHEN** the scaffold is started on macOS
- **THEN** the app launches and the same first screen is presented
- **AND** it does not crash during launch

### Requirement: Bindings are generated reproducibly and drift fails the build

**TOOL-3.** The typed frontend bindings SHALL be generated from the Rust contract by a headless
export that needs no running app, and SHALL be committed. Regenerating them from unchanged source
SHALL produce no diff. When the generated file differs from what the source produces, the build
SHALL fail and SHALL name that file. The scaffold SHALL satisfy this with an **empty** command and
event set, so the pipeline is proven before any command exists.

#### Scenario: Bindings export without launching the app

- **WHEN** the bindings target is regenerated
- **THEN** the bindings file is produced or confirmed unchanged
- **AND** no application window is created

#### Scenario: The empty command set produces a stable artefact

- **WHEN** the scaffold's empty command and event set is exported
- **THEN** a bindings file is produced
- **AND** exporting it a second time produces no diff

#### Scenario: A stale bindings file fails the build

- **WHEN** the committed bindings file differs from what the current source produces
- **THEN** the gate fails
- **AND** the failure names the bindings file

#### Scenario: A hand edit is reverted by regeneration

- **WHEN** the bindings file is edited by hand
- **THEN** regenerating removes that edit
- **AND** the gate fails until the file matches the source

### Requirement: The route tree is generated reproducibly and drift fails the build

**TOOL-4.** The route tree SHALL be generated from the file routes by the router plugin and SHALL
be committed. Regenerating it after a build SHALL produce no diff, and a route tree that differs
from what the file routes produce SHALL fail the build, naming that file.

#### Scenario: Route generation is reproducible

- **WHEN** a build regenerates the route tree from the file routes
- **THEN** the committed route tree file is unchanged

#### Scenario: A stale route tree fails the build

- **WHEN** the committed route tree differs from what the file routes produce
- **THEN** the gate fails
- **AND** the failure names the route tree file

### Requirement: The test harness runs at both form-factor baselines

**TOOL-5.** Unit/component tests SHALL run against both the 390×844 and the 1280×800 baseline, and
the E2E suite SHALL drive a WebKit browser at both viewports with the IPC layer mocked through the
documented bootstrap. Asserting adaptive navigation at both baselines SHALL be possible without a
native device or a real shared service.

#### Scenario: Component tests exercise the compact baseline [mobile]

- **WHEN** the component suite runs
- **THEN** it exercises a 390×844 viewport

#### Scenario: Component tests exercise the expanded baseline [desktop]

- **WHEN** the component suite runs
- **THEN** it exercises a 1280×800 viewport

#### Scenario: E2E runs in WebKit with mocked IPC

- **WHEN** the E2E suite runs
- **THEN** it drives a WebKit browser with the IPC layer mocked
- **AND** no real shared service and no native device are required

#### Scenario: Adaptive navigation is asserted at the compact baseline [mobile]

- **WHEN** a test runs at 390×844
- **THEN** it can assert the compact navigation arrangement, not only that content rendered

#### Scenario: Adaptive navigation is asserted at the expanded baseline [desktop]

- **WHEN** a test runs at 1280×800
- **THEN** it can assert the expanded navigation arrangement, not only that content rendered

### Requirement: The design system is established and its status recorded before dependent UI work

**TOOL-6.** Design tokens and a shared component inventory SHALL exist before any UI work that
depends on them: the primitives SHALL live in the shared component location, and the design-system
skill's status line SHALL record that the system is defined. UI work that needs tokens or shared
primitives SHALL NOT start while that status is unrecorded.

#### Scenario: The design-system status is recorded

- **WHEN** the design system's tokens and component inventory are defined
- **THEN** the design-system skill's status line records the system as defined

#### Scenario: UI work waits for the recorded status

- **WHEN** a UI task needs design tokens or shared primitives
- **THEN** that task starts only after the status has been recorded

### Requirement: Continuous integration runs the same gate as the definition of done

**TOOL-7.** A CI workflow on a macOS runner SHALL run the same gate as `definition-of-done`,
including both generated-artefact drift checks and the E2E suite, and SHALL fail when any gate
command fails or a generated file differs after regeneration. Third-party actions SHALL be pinned,
the workflow SHALL NOT add platform jobs beyond iOS and macOS, and it SHALL neither sign, notarise
nor distribute the app.

#### Scenario: CI runs the gate [macos]

- **WHEN** a push or pull request triggers the workflow
- **THEN** every command of the `definition-of-done` gate runs on a macOS runner

#### Scenario: CI fails on generated drift

- **WHEN** regeneration produces a different bindings file or route tree
- **THEN** the workflow fails

#### Scenario: CI covers no platform beyond iOS and macOS

- **WHEN** the workflow runs
- **THEN** it contains no Android, Windows or Linux job

#### Scenario: CI neither signs nor distributes

- **WHEN** the workflow runs
- **THEN** it performs no signing, no notarisation and no distribution step

### Requirement: The toolchain is pinned exactly and supports both platforms

**TOOL-8.** Every third-party dependency the scaffold introduces SHALL be pinned to an exact
version in the committed manifest, and SHALL be usable on both iOS and macOS. Adding a dependency
SHALL require a one-line justification, and a dependency that supports only one of the two
platforms SHALL NOT be added. Crates that must move together SHALL be pinned to matching versions
in the same change.

#### Scenario: Versions are exact

- **WHEN** the manifests are resolved
- **THEN** every third-party dependency is recorded at an exact version, not a range

#### Scenario: Both platforms are supported

- **WHEN** a dependency is added
- **THEN** it is usable on iOS and on macOS
- **AND** it is justified in one line in the change's `proposal.md` or `design.md`

#### Scenario: Paired crates move together

- **WHEN** one of the bindings crates is bumped
- **THEN** it and its paired crate are pinned to matching versions in the same change
