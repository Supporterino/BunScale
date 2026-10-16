# Design — add-shared-household-foundation (P0)

## Context

Greenfield: no code exists. `openspec/specs/` and `openspec/changes/` were empty. This change is
the first, so it decides the shapes every later phase inherits: the IPC contract, the storage
boundary, the read/write model and the failure surface.

Platforms: iOS (primary) and macOS (Phase 1). Baselines: 390×844 first, 1280×800 second. Android,
Windows and Linux are later, so everything stays `cfg`-clean.

Stack: Tauri v2, Rust in `src-tauri/`, React + TypeScript + Vite in `src/`, TanStack Router +
Query, generated bindings in `src/bindings.ts` (never hand-edited), bun as package manager.

## Decisions

### D0.1 — Connectivity-required write model (implementation of D1)

Writes are attempted only while the shared service is reachable and the account is valid. There is
no local write queue, no optimistic write, no deferred reconciliation. A refused write is surfaced
with its cause and may be retried by the person.

Consequences accepted: logging care while offline is impossible; the UX must make the requirement
obvious before the person invests effort in a form. Offline dose logging (US-MED-23) is dropped.
Reads are unaffected — they are served from the locally kept copy of the last loaded collection.

### D0.2 — Loaded-data read model (implementation of SYNC-3)

The device keeps a local copy of each household collection it has loaded. Reads always resolve
against that copy first. A start or foreground return with connectivity refreshes it. "Nothing ever
loaded + unreachable" is a distinct state with its own surface, never an empty list.

The loaded-data path is deliberately unoptimised in P0 — serving it first, batching the refresh and
invalidating only affected data is the subject of `add-read-path-performance` (P6, NFR-7). P0 only
fixes the *boundaries* (where the cache lives, what a refresh replaces) so P6 can improve behaviour
without moving them.

### D0.3 — Owner-dependent sharing, no transfer (implementation of D2)

A household records the member that created it as its owner. Ownership is used only to gate
destructive household deletion, which arrives in P3 (HS-10). No ownership transfer and no
backup/restore flow. HS-13 stays an accepted risk. The owner field is part of the P0 model because
retrofitting it later would touch every household record.

### D0.4 — One canonical value representation

Values with a single canonical representation are stored only in that form (weight in grams; DM-1
belongs to P1) and converted only at the entry/display boundary. P0 establishes the rule at the
storage boundary and implements it for the one preference it ships: the display weight unit (SET-4).

### D0.5 — Absolute-instant timestamps

Timestamps are absolute instants (serialised as RFC 3339 with an offset, normalised to UTC on
store), never local-time strings or offsets. Default is "now" at creation; a supplied instant is
stored unchanged.

### D0.6 — One failure surface

Every state-changing operation reports failure through one typed error surface carrying a cause
code, whether the failure is retryable, and a message key. The UI renders cause-specific copy from
that key. No operation reports a bare "something went wrong" (NFR-8), and no failure path is
allowed to fall through to a success state (SYNC-2). The surface also carries the client-generated
`request_id` that identifies the operation (D0.14) — that is what makes the retry the surface offers
safe, because the shared service deduplicates on it.

### D0.7 — Technology-agnostic specs, concrete mechanism chosen here

The capability specs in this change name only "the shared service" and "the account" (D6). The
concrete mechanism — which product provides one shared collection per household with per-item
last-write-wins and per-device local caching — is selected and justified by `architect` in task 1.1
and recorded in `docs/adr/0002-shared-service-mechanism.md` (`0001` is taken by the settled
scope decisions, `docs/adr/0001-reimplementation-scope-decisions.md`). Until that task lands, the mechanism
is an **open item**, and every statement below treats it as an interface with five capabilities:

1. address a household collection by an opaque, stable id;
2. read the collection and read per-item changes since a marker;
3. write an item with a client-supplied version/updated-at for last-write-wins;
4. report availability and account state in a way the product can map to cause codes;
5. report storage-allowance and rate-limit conditions, where an exhausted owner allowance is
   reportable as **its own condition**, distinct from a generic write error (D0.12);
6. accept a client-supplied idempotency key and deduplicate on it, so a re-issued write is safe by
   construction (D0.14).

Any candidate that cannot report 4 and 5 *distinctly*, or that cannot deduplicate a request by its
key, cannot satisfy SYNC-2's cause list or SYNC-1's "exactly one copy" rule and must be rejected in
task 1.1. The mechanism is therefore not an open item in the sense of an unknown criterion: the
criteria are fixed here, and 1.1 names the candidate that meets them.

### D0.8 — Scaffold baseline and exact pins (section 0)

`tasks.md` section 0 creates the workspace, because none exists: no `package.json`, no `src/`, no
`src-tauri/`, no Vite/TypeScript config, no `.gitignore`, no CI workflow. It is folded into this
change as section 0 rather than split into its own change: phase labels and archive order stay as
they are, and this change remains P0 with no predecessor change. The capability that states it is
`project-toolchain` (TOOL-1 … TOOL-8), an explicitly enabler-only addition — no product behaviour,
no source-document ID.

Section 0 does not hand-write that baseline: task 0.1 generates it with the official Tauri v2 CLI
(D0.11) and the tasks after 0.1 refine the generated output. The exact pin set below is what the
tasks then impose on the committed manifests; it is not what the generator emits.

Baseline: bun as package manager, React + TypeScript + Vite, TanStack Router + Query, Tauri v2,
Rust in `src-tauri/`. Every third-party dependency is pinned to an **exact** version (no ranges) in
the committed manifests; `tauri-specta` and `specta` are pinned to matching versions because
`tauri-specta` is still `2.0.0-rc.x` and the two must move together (`tauri-ipc-contract`). That
pinned release candidate is **accepted**, with the bump discipline recorded in D0.18 —
`tauri-typed-ipc` is not evaluated first. The lint and format tool is **Biome** (D0.17); the app
identity is settled (D0.15); the minimum iOS and macOS deployment targets are **derived from the
installed SDKs** (D0.16) rather than chosen. Rule 6's one-line justification per dependency is
recorded in the pin tables below (TOOL-8 allows `proposal.md` **or** `design.md`; `proposal.md`
keeps a coarser, partial capability-level table, and the complete per-dependency justification set
is the one here), together with the confirmation that each dependency is usable on both iOS and
macOS. Section 0 adds **no Tauri plugin**, so there is no plugin whose platform support must be
confirmed; the scaffold's own pinnings are platform-neutral Rust crates plus JavaScript that is
either runtime-neutral or host/CI dev tooling. The one plugin the generator's template does add
is removed again inside section 0 — see the recorded refinement below.

##### Exact pin set (task 0.2, TOOL-8)

These pins are recorded in **exact-pin form**: Cargo `=x.y.z` for the crates, bare versions (no `^`,
no `~`) for the npm packages. They **replace** the generator's caret ranges from task 0.1 in
`src-tauri/Cargo.toml` — they do not sit beside them. That manifest is hand-owned (D0.11) and task
0.10 is what writes the pins into it, so this record is 0.10's input rather than a parallel set of
versions. `package.json` **does not exist yet**: tasks 0.13–0.19 create the frontend workspace from
these pins (0.13 the manifest and its scripts, 0.18 Biome), so nothing in the npm table describes a
file already committed. Every dependency carries its rule-6 one-line justification; entries marked
**grouped** say so and are grouped only because they are the same kind of housekeeping dependency.

| Crate | Pin | One-line justification (rule 6) |
|---|---|---|
| `tauri` | `=2.12.4` | The v2 runtime the whole app is built on; `max_stable_version`, and the version the generated crate already resolves. |
| `tauri-build` | `=2.7.3` | The v2 build script that emits the `mobile`/`desktop` cfg gates and the Tauri config for both platforms. |
| `serde` | `=1.0.229` | Derives the (de)serialisation of every IPC payload type and shared type. |
| `serde_json` | `=1.0.154` | The JSON encoding Tauri IPC and the shared service's payloads use. |
| `tauri-specta` | `=2.0.0-rc.25` | Generates the typed bindings from the Rust contract, which is what keeps `src/bindings.ts` generated rather than hand-written (TOOL-3, D0.9), with its `typescript` feature (which gates the `specta-typescript` dependency — see the module shape below). |
| `specta` | `=2.0.0-rc.25` | The type-introspection crate `tauri-specta` is built on; pinned equal to it by that crate's own exact requirement. |
| `specta-typescript` | `=0.0.12` | The TypeScript exporter `Builder::export` is called with, named **directly** by `src-tauri/src/ipc/` because `tauri-specta` does not re-export `specta_typescript::Typescript`; pinned explicitly so the manifest stays exact. |

| Package | Pin | Role and one-line justification (rule 6) |
|---|---|---|
| `react` | `19.3.1` | Runtime. The UI runtime; React 19 is the version the whole peer set below accepts. |
| `react-dom` | `19.3.1` | Runtime. The DOM renderer for that runtime, pinned to the same version as `react`. |
| `@tauri-apps/api` | `2.12.4` | Runtime. The v2 client API the generated bindings call into; the only Tauri-specific runtime package here, while the other runtime packages ship platform-neutral JavaScript. |
| `@tanstack/react-router` | `1.170.44` | Runtime. File-based routing: one route tree for all form factors, no forked routes per platform. |
| `@tanstack/react-query` | `5.104.3` | Runtime. The query/cache layer the loaded-data read model (D0.2) needs. |
| `typescript` | `7.0.2` | Dev. The typechecker `bun run typecheck` runs. |
| `vite` | `8.3.4` | Dev. The bundler and dev server the Tauri CLI drives, and the host for the router plugin. |
| `@vitejs/plugin-react` | `6.1.2` | Dev. React fast refresh for the dev server. |
| `@tauri-apps/cli` | `2.12.3` | Dev. Pinned so local runs and CI use this CLI instead of task 0.1's one-off global `cargo install` (0.13), and so 0.22 can run `bun run tauri icon`. |
| `@tanstack/router-plugin` | `1.168.46` | Dev. The Vite plugin that renders `src/routeTree.gen.ts` (TOOL-4). |
| `@biomejs/biome` | `2.5.15` | Dev. The single lint **and** format tool (D0.17), behind `bun run lint`; no second linter and no Prettier. |
| `vitest` | `5.0.5` | Dev. The unit/component runner (TOOL-5). |
| `jsdom` | `30.1.3` | Dev. The DOM environment the vitest component layer runs in. |
| `@testing-library/react` | `16.3.3` | Dev. Renders components for the component layer of the pyramid. |
| `@testing-library/jest-dom` | `7.0.1` | Dev. The DOM matchers those component assertions use. |
| `@testing-library/user-event` | `14.6.8` | Dev. Drives the pointer/touch and keyboard interaction the component tests assert on. |
| `@testing-library/dom` | `10.4.2` | Dev. The peer `@testing-library/react` and `jest-dom` build on, pinned explicitly so the set stays exact. |
| `@playwright/test` | `1.64.1` | Dev. The E2E layer, driving WebKit at both baselines (TOOL-5). |
| `@types/react` / `@types/react-dom` / `@types/node` | `19.3.0` / `19.3.0` / `26.6.5` | **Grouped** dev type declarations: React, its renderer and Node, required by `bun run typecheck` over `src/` and the config files. |

`@tanstack/react-router-devtools` is **not** in the set: nothing in this design or in
`tanstack-router-conventions` calls for devtools.

##### Compatibility constraints (from the published metadata, not inferred)

- **`tauri-specta` ⇄ `specta` are a matched pair at `2.0.0-rc.25`, and that is enforced.** The
  published `tauri-specta` metadata declares `specta = "=2.0.0-rc.25"` as a normal dependency — an
  exact requirement, not a convention. The stable lines do **not** pair (`tauri-specta` 1.0.2 against
  `specta` 1.0.5), which is why D0.18 accepts the `2.0.0-rc.x` line rather than waiting for stable.
  `specta-typescript` 0.0.12 keeps the rc line too: `tauri-specta` declares it as an optional `^0.0.12`
  behind its `typescript` feature, and `specta-typescript` 0.0.12 itself requires
  `specta = "=2.0.0-rc.25"`.
- **Tauri v2 alignment holds across the two ecosystems.** npm `@tauri-apps/api` 2.12.4 equals the
  cargo `tauri` pin 2.12.4, and npm `@tauri-apps/cli` 2.12.3 equals the locally installed `tauri-cli`
  2.12.3. Only the `3.0.0-alpha.x` `next` tags sit outside this line.
- **Vite 8 is forced by the intersection, not preferred.** `@vitejs/plugin-react` 6.1.2 declares
  `peerDependencies.vite: "^8.0.0"` (Vite 8 only), while `vitest` 5.0.5 accepts
  `"^6.4.0 || ^7.0.0 || ^8.0.0"`; Vite 8 is the only version in both. Vitest majors do **not** track
  Vite majors 1:1, so no lockstep should be read into `vitest` 5 beside `vite` 8.
- **React 19 is accepted across the set.** `@tanstack/react-router` 1.170.44 peers
  `react: ">=18.0.0 || >=19.0.0"`, `@tanstack/react-query` 5.104.3 peers `"^18 || ^19"`, and
  `@testing-library/react` 16.3.3 peers `"^18.0.0 || ^19.0.0"`.
- **TanStack router and its Vite plugin are *not* version-equal, and must not be read that way.**
  `@tanstack/router-plugin` 1.168.46 declares `"@tanstack/react-router": "^1.170.44"` as an
  **optional** peer, which `react-router` 1.170.44 satisfies; `@tanstack/react-router` 1.168.46 does
  not exist, so pinning both at the plugin's number is impossible. The two numbers above are the
  correct pair — this is the one pair in the set where "matching versions" would be wrong.
- **Node runtime floors are satisfied locally.** `vitest` requires Node
  `^22.12.0 || ^24.0.0 || >=26.0.0` and `jsdom` requires `^22.22.2 || ^24.15.0 || >=26.0.0`; the
  machine runs **Node v24.18.1** and **bun 1.4.3**. bun does not enforce npm `engines`, so these
  floors bind the Node install rather than bun.
- **Rust version.** `tauri` and `tauri-build` declare `rust_version: "1.90"`; the local toolchain is
  `cargo`/`rustc` **1.93.1**, so both are satisfied. The `2.0.0-rc.25` crates publish **no**
  `rust_version`, so **no MSRV is claimed for them**.

##### iOS and macOS support (TOOL-8, TOOL-1, TOOL-2)

Confirmed for every dependency, and the reasoning is written down rather than asserted:
`@tauri-apps/api` is the only Tauri-specific **runtime** package — the one bound to Tauri's
native side while the app runs. The other runtime packages (`react`, `react-dom`,
`@tanstack/react-router`, `@tanstack/react-query`) are pure JavaScript and ship
platform-neutrally. Every **Dev.** npm package is host/CI tooling, and each one that carries a
native binary publishes a darwin-arm64 variant — `@tauri-apps/cli` (`@tauri-apps/cli-darwin-arm64`),
`@biomejs/biome` (`@biomejs/cli-darwin-arm64`), `typescript` 7 (a per-platform native compiler,
`@typescript/typescript-darwin-arm64`, which the top-level `bin/tsc` Node shim loads), `vite`
(transitively `rolldown` → `@rolldown/binding-darwin-arm64` and `lightningcss` →
`lightningcss-darwin-arm64`) and `@playwright/test` (its WebKit is pulled explicitly by
`playwright install`, with no postinstall script). The Rust crates are platform-neutral and are what
produce the app binary itself. Nothing in the set is desktop-only, and nothing targets a platform
this app does not ship.

Local environment for the record: **macOS 27.0.1 on arm64**, **Xcode 27.1**, `rustc`/`cargo`
**1.93.1**, `tauri-cli` **2.12.3**, the **`aarch64-apple-ios`** rustup target installed, **Node
v24.18.1** and **bun 1.4.3**.

##### Pin provenance

Every version above was read on **2026-10-16** from the crates.io API
(`https://crates.io/api/v1/crates/<name>` → `max_stable_version`, `max_version`, and the published
dependency requirements) and from the npm registry (`https://registry.npmjs.org/<name>` →
`dist-tags.latest`, `peerDependencies`, `peerDependenciesMeta`, `engines`, `optionalDependencies`),
so a reviewer can re-verify each pin against the same endpoints rather than against this document.

**Recorded refinement (project rule 6): the generator's logging plugin is removed.** `tauri init`'s
template writes `tauri-plugin-log = "2"` into the generated `src-tauri/Cargo.toml` and registers it
debug-only in `src/lib.rs`, but no requirement in this change asks for logging, and its
`time` (`local-offset`) → `num_threads` → `libc::mach_task_self()` chain (that symbol is gated to
`target_os = "macos"`) is exactly what fails `cargo check --target aarch64-apple-ios` — the failure
found by task 0.1's own step 6 and breakdown recorded in D0.11 — leaving TOOL-1's
"[ios] The iOS target compiles" scenario unmet. `tauri-plugin-log` and its registration are therefore
removed as a refinement of the generated baseline (D0.11, "baseline, then refine"), landed inside
**task 0.1's own baseline commit** with `rust-backend` as the implementing specialist — it is not
deferred to task 0.10, whose artefacts (`Cargo.toml`, `lib.rs`) the removal happens to touch. The
verification runs on that committed tree: `cargo check --manifest-path src-tauri/Cargo.toml` and
`cargo check --manifest-path src-tauri/Cargo.toml --target aarch64-apple-ios` both pass (the
orchestrator reproduced both), so TOOL-1's "[ios] The iOS target compiles" scenario is met by task 0.1
itself. **No replacement dependency is added**, and nothing was reachable through the plugin in any
case, since the generated `capabilities/default.json` grants only `core:default`. **The now-unused
`log = "0.4"` dependency goes with it, in the same task-0.1 commit and under the same rule-6 argument**
("no requirement in this change asks for logging"): its only consumer was the removed plugin's `setup`
closure, and no `log::` reference remains anywhere in `src-tauri/src/`. Task 0.10 therefore inherits
only a **forward obligation on the reconciled crate**: it MUST NOT reintroduce `tauri-plugin-log`, nor
anything that switches on `time/local-offset` (`time` itself stays in the graph without that feature,
via `cookie`, `plist`, `serde_with` and `tauri-build`).

Two structural notes for the orchestrator. First, ownership of the root tooling files the scaffold
creates is settled: `frontend-logic`'s edit allowlist covers exactly these paths (`package.json`,
`tsconfig*.json`, `vite.config.ts`, `vitest.config.ts`, `playwright.config.ts`, `index.html`,
`biome.json`, `.gitignore`), so tasks 0.13–0.19 stay in scope with no escalation —
unlike `.github/workflows/ci.yml`, which belongs to `release-ci`. Second, no ADR number is taken
by section 0: `docs/adr/0002` stays reserved for task 1.2, and section 0's decisions live here.

A third structural note: section 0 now contains **one `apple-native` task** (0.7, D0.16). It exists
because the deployment floor must be measured from the installed Xcode/SDKs and `tauri-security` has
no `xcodebuild`/`xcrun` permission; task 0.8 consumes the floor it records. `frontend-ui` also owns
a section-0 task (0.22, D0.19) that draws the app mark from the design-system tokens and runs the
icon generation, after the tokens (0.20/0.21) and the `package.json` task (0.13) it depends on.

### D0.9 — The bindings pipeline ships with an empty command set

The scaffold ships the whole bindings pipeline **and** an empty command/event set, deliberately.
A headless export entry point — the form this record settles below — calls
`builder.export(Typescript::default(), "../src/bindings.ts")` with `collect_commands![]` and
`collect_events![]` empty and no running app. It ships in section 0 because the scaffold's own gate
must pass: `bun run gen:bindings` followed by `git diff --exit-code src/bindings.ts` has to produce
no diff on the scaffold, and a pipeline that only works once a command exists could not do that.
Task 1.3 then extends the same builder with the P0 commands — it does not introduce or rework the
pipeline. Both `src/bindings.ts` and `src/routeTree.gen.ts` are committed, generated and never
hand-edited; CI fails on a diff after regeneration (TOOL-3, TOOL-4). This is what the "IPC contract"
section below means by "nothing may be implemented against these names until 1.3 is done": the
pipeline exists from section 0, the names do not.

##### The export entry point is a cargo example, not a test

`tauri-ipc-contract` names two forms — "a Rust test or small bin that calls `builder.export(…)`" — and
what the requirement needs is a **headless caller of `builder.export`**. This record settles a third
form deliberately: a cargo **example** at `src-tauri/examples/export_bindings.rs` (task 0.11), whose
`main` does nothing but call the export function of `src-tauri/src/ipc/`. It is run as
`cargo run --manifest-path src-tauri/Cargo.toml --example export_bindings` — the exact invocation
this record requires `bun run gen:bindings` to use (the script is task 0.13's artefact; see "The single
regeneration entry point" below). A cargo example *is* that headless caller: the Tauri CLI's bundler
never enumerates examples, so the target is
not built as a bundle binary and cannot end up inside or be signed into the macOS `.app`, and —
unlike a `required-features`-gated bin — it is still compiled and linted by the gate's
`cargo clippy --all-targets`, so it stays under the lint gate instead of escaping it. A second
`src/bin/` target was considered and rejected on the same evidence: `tauri-cli` 2.12.3 enumerates
every `src/bin/` entry as a binary of the app (`get_binaries`), so the exporter would have been
compiled in release and copied into and signed inside the macOS `.app` (`tauri-bundler`,
`bundle/macos/app.rs`), and a second bin would also have left the CLI unable to identify a main
binary to run. Neither consequence exists for the example, and the gate's
`cargo check --target aarch64-apple-ios` runs without `--all-targets` and therefore never builds the
example, so it cannot affect the iOS target.

The reason for a separate entry point at all, rather than a test, is the drift check, not taste. A
`cargo test` integration test would run inside the gate's own `cargo test` step, which precedes the
bindings check in `definition-of-done`, and would therefore rewrite `src/bindings.ts` from the current
source *before* that check looked at it — repairing exactly the drift TOOL-3 requires the build to
fail on ("A stale bindings file fails the build", "A hand edit is reverted by regeneration"). A
build-script-driven exporter is worse, not better: a `build.rs` runs on `cargo check`, `cargo build`
and `cargo test` alike, so the macOS check and the iOS check would each mutate a generated source
file, and the `aarch64-apple-ios` compile check would write to the working tree while presenting
itself as read-only. The example is run only when it is asked for, so **no gate step that precedes the
bindings drift check writes `src/bindings.ts`**; that invariant is what makes the no-diff check mean
something. It also imposes an explicit rule on the rest of the crate: no `#[test]` under
`src-tauri/` — a unit test beside the code, an integration test in task 0.12's `src-tauri/tests/`
harness, or a smoke test like task 0.24's — may call that export function or the `bun run
gen:bindings` script, because such a test would run at the gate's own `cargo test` step, which
precedes the bindings drift check, and would rewrite the very file that check is about to verify;
the export path is reachable only through the explicit `cargo run --example` invocation, and the
drift checks that exercise the script are `bun run` steps, outside `cargo test`.

The exported path is the repository's `src/bindings.ts` — the `../src/bindings.ts` the skill names,
one level above the crate root — resolved from the crate root (`CARGO_MANIFEST_DIR`) rather than from
the process working directory, because the script is invoked from the repository root while the file
sits above `src-tauri/`. Exporting builds the builder and serialises it; it creates no window and
starts no app (TOOL-3, "Bindings export without launching the app").

##### One builder, mounted and exported

`src-tauri/src/ipc/` owns a single builder constructor:
`Builder::<tauri::Wry>::new().commands(collect_commands![…]).events(collect_events![…])`, with events
mounted in `setup`. Task 0.10 mounts that same constructor in `src/lib.rs`; task 0.11's example calls
it to export. There is no second, export-only builder, and that is the mechanism by which task 1.3
**extends** the pipeline — adding entries to the two `collect_*!` lists and the types they reference —
instead of replacing it, and by which an export can never disagree with what the running app
registers.

##### The empty command/event set it ships with

Section 0 ships exactly `collect_commands![]` and `collect_events![]`: zero commands, zero events,
and therefore no shared type of its own. Exporting that produces an empty but valid TypeScript
module — a bindings file with no command function and no event binding, and nothing in it naming a
command that does not exist yet. Its exact contents are whatever the pinned `tauri-specta` exporter
emits; the requirement here is only that the file exists, is committed, and is byte-stable across
runs (TOOL-3, "The empty command set produces a stable artefact"). If an empty invocation of either
macro turns out not to compile in the pinned release candidate, task 0.4 records that as a question
(rule 7) rather than inventing a placeholder command or event: a placeholder would put a name into
the contract that task 1.3 has not approved, which is exactly what this design forbids.

##### Why it ships empty

Two reasons, both in the task's own terms. First, the scaffold's own gate has to pass with **no
diff**: `bun run gen:bindings` followed by `git diff --exit-code src/bindings.ts` becomes a
`definition-of-done` step from task 0.16 on and one of the two checks CI runs from task 0.9 on, and it
can only pass on the scaffold if the pipeline is exercised against the set the scaffold actually
ships — the empty one — and the committed file is what a re-run reproduces (hard rule 3: generated
files are read-only and regeneration leaves no diff). A pipeline that needed a command to exist could
not be gated at all before task 1.3. Second, task 1.3 extends this pipeline rather than replacing it:
the exporter, the example, the script and both drift checks are written once, in section 0, and 1.3's own
diff stays confined to the contract surface in `src-tauri/src/ipc/` plus the regenerated
`src/bindings.ts`. TOOL-3 states it directly — the scaffold satisfies the requirement with an empty
command and event set "so the pipeline is proven before any command exists".

##### The single regeneration entry point

`bun run gen:bindings` is the only producer of `src/bindings.ts` (AGENTS.md's command table). End to
end, that script: runs from the repository root; invokes the export example as
`cargo run --manifest-path src-tauri/Cargo.toml --example export_bindings`, naming the target
explicitly so no target has to be guessed; lets the example write exactly one file,
`src/bindings.ts`, and nothing else; and propagates a non-zero exit, so the gate stops instead of
comparing a file that was never written. Writing exactly one path is what lets CI attribute a diff to
that file alone. The script itself does not exist yet: `package.json` is task 0.13's artefact, and
this record is part of
its requirement — it MUST expose the script under exactly this name and MUST NOT introduce a second
bindings target. Task 0.16 wires the script to the example and commits the first (empty-set)
`src/bindings.ts`; task 1.3 regenerates through the same script, not a new one.

##### The two drift checks CI runs

Both follow one shape — regenerate, then fail on any diff — and they are the two generated-artefact
checks the gate and CI run (TOOL-3, TOOL-4). `release-ci` designs them into
`.github/workflows/ci.yml` in task 0.9, on `macos-latest` with third-party actions pinned; nothing
here assumes that workflow exists yet.

| Check | Regeneration step | Fails when | File it names |
|---|---|---|---|
| Bindings | `bun run gen:bindings`, headless, no app process | `git diff --exit-code src/bindings.ts` exits non-zero | `src/bindings.ts` |
| Route tree | the frontend build, `bun run build`, which is what runs the router plugin | `git diff --exit-code src/routeTree.gen.ts` exits non-zero | `src/routeTree.gen.ts` |

Each check is a step of its own, run against a single path, so a failure names the file the
requirement says it must name (TOOL-3, "the failure names the bindings file"; TOOL-4, "naming that
file"). Three preconditions make them meaningful rather than decorative. Both files are **tracked**,
and neither may be excluded by `.gitignore` (task 0.19). The route-tree check runs after the build in
the same job, because a dev server never exits — and the build is also the only step that regenerates
that file. The bindings check is not preceded by any step that writes `src/bindings.ts`, which holds
by construction because the exporter is an example rather than a test, the reason recorded above, and
stays true only under the rule recorded there: that no `#[test]` under `src-tauri/` may call the
export function or the `bun run gen:bindings` script. What the checks prove is reproducibility, not
correctness: a contract change that regenerates cleanly still goes to the reviewer (D0.18).

##### The `tauri-specta` pin and its bump discipline

The pin is D0.8's and the acceptance is D0.18's; neither is re-derived here. What this record adds is
that the whole pipeline — the export example, the `bun run gen:bindings` script and the bindings
drift check — is written against that pinned `{tauri-specta, specta}` pair with its
`specta-typescript` companion, so a bump is a change to the pipeline's inputs and is handled as one:
bump `tauri-specta` and
`specta` **together** in the same change (TOOL-8, "Paired crates move together"), regenerate
`src/bindings.ts` through the same script, and review the resulting bindings diff deliberately as part
of that change (D0.18) — a bump that changes the generated file's shape is a reviewed change, never
silent drift the no-diff check would attribute to the contract. `tauri-typed-ipc` is not evaluated
first (D0.18) for two operative reasons: the accepted `2.0.0-rc.x` `tauri-specta`/`specta` line
already generates the bindings the pipeline needs, and standing up a second generation pipeline
alongside it would churn TOOL-8's exact-pin set for no gain in this phase. That is a deferral, not a
dismissal — the evaluation the skill expects lands as an ADR numbered **after** `0002`, in the change
that owns it, since section 0 itself takes no ADR number (below).

##### No ADR number is taken by section 0

This record lives in `design.md`, not in an ADR. As D0.8 already states, section 0 takes no ADR
number: `docs/adr/0002` stays reserved for task 1.2 (the shared-service mechanism, D0.7). Section 0's
decisions, this one included, are recorded here so that 1.2 can take `0002` without renumbering.

##### The module shape and the empty macro form (task 0.4)

`src-tauri/src/ipc/` ships as **one file**, `mod.rs`, holding exactly two public items and no domain
type: `builder() -> tauri_specta::Builder<tauri::Wry>` — the single constructor of "One builder,
mounted and exported" above — and `export_bindings() -> Result<(), specta_typescript::Error>`, which
builds from it and exports. It is a directory from the start, so task 1.3 adds its commands, events
and shared types as sibling modules without renaming the parent or touching the pipeline.

**The export path is resolved inside `export_bindings`**, not by the caller:
`Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/bindings.ts")`. Task 0.11's example `main` is
therefore one call with no argument, and no caller can pass a bare `../src/bindings.ts` that would
resolve against the working directory — the defect `tauri-ipc-contract` names, where the script runs
from the repository root and the exporter would create that directory rather than fail.

**The empty macro invocation compiles in the pinned release candidate**, so TOOL-3's "The empty
command set produces a stable artefact" needs no placeholder name (which this design forbids).
That was read from the pinned sources rather than assumed. `specta` 2.0.0-rc.25 declares an explicit
empty arm for the macro `tauri-specta` expands to:

```rust
macro_rules! _collect_functions {
    ($(,)?) => {{
        fn export(_: &mut $crate::Types) -> Vec<$crate::datatype::Function> { vec![] }
        export
    }};
    // …the non-empty arm…
}
```

and `tauri-specta` 2.0.0-rc.25's own documentation example for `Builder::export` is written in
exactly this shape — `Builder::<tauri::Wry>::new().commands(collect_commands![]).events(collect_events![])`
— with the empty lists passed to the two macros this module calls. `collect_commands![]` reaches
`tauri::generate_handler![]`, whose 2.7.3 expansion parses its input with
`parse_terminated(CommandDef::parse, Token![,])` and therefore accepts zero commands, emitting
`match cmd { _ => { return false } }`.

**What task 0.10 must declare beyond the mount:** `tauri-specta` with its `typescript` feature (the
crate's default feature set is empty, and `Builder::export` is only usable with a `LanguageExt`
implementor), and `specta-typescript` as a **direct** dependency, since `tauri-specta` does not
re-export `specta_typescript::Typescript`, the implementor the export function names. Both are
already D0.8's pins; only the direct-ness of `specta-typescript` is new, and that table's
justification now says so.

**Mounting stays one construction site.** Commands and events both come from that constructor, so
task 0.10 calls `ipc::builder()` once, hands `Builder::invoke_handler` to
`tauri::Builder::invoke_handler`, and calls `Builder::mount_events` with the app handle inside the
app's own `setup` closure. That call must live in **0.10's** `setup` closure rather than in a helper
of this module that installs a `setup` of its own: `tauri` 2.12.4's `Builder::setup` assigns —
`self.setup = Box::new(setup)` — so a second one would silently replace the event registration
instead of composing with it.

**Honest compile status.** Nothing declares `pub mod ipc;` yet (that is task 0.10's mount, and
`src-tauri/src/lib.rs` is outside task 0.4's writable paths), and the dependency pins above are
0.10's manifest work, so `cargo check` passes on the scaffold *with this module absent* and proves
nothing about it. The first compile evidence for these stubs lands with task 0.10. Nor does
`bun run gen:bindings` exist as a script yet — `package.json` is task 0.13's artefact — so nothing
here regenerates `src/bindings.ts` either: the first real export is task 0.16, and 0.4's evidence
is the two `cargo check` runs that task 0.10 must make non-vacuous.

### D0.10 — Design-system status convention

The design system is established in section 0, before any UI task that depends on it. Following the
`frontend-ui` bootstrap: `ui-ux-pro-max` is used for direction only (an iOS-first, native-feeling
app that scales to macOS, not a website), tokens are implemented as three layers (primitive →
semantic → component) in `src/styles/`, primitives land in `src/components/ui/`, and the result is
written into `.opencode/skills/design-system/SKILL.md` with its **status line** set so that the
orchestrator's UI precondition (step 3 of its loop: "task touches UI and `design-system` status is
not `frozen`") resolves instead of blocking every UI task. That status line is the recorded evidence
for TOOL-6; the screenshots required of UI tasks come from the primitives task 0.21, not from the
token/inventory task 0.20, which has no rendered surface of its own. The tokens are also what the
app mark of task 0.22 is drawn from (D0.19).

### D0.11 — The baseline is generated by the official Tauri v2 CLI, then refined

Section 0 begins with **one** bootstrap task (0.1, owned by `architect`) that generates the baseline
with the official Tauri v2 CLI and commits it as one commit whose message marks it as generated
output. Every other section-0 task refines that baseline instead of hand-writing it. `architect` is
granted shell access for `cargo install tauri-cli *` and `cargo tauri *` for exactly this task.

**`tauri init`, not `create-tauri-app`.** The two utilities are not interchangeable.
`create-tauri-app` (`bun create tauri-app`) scaffolds a *new* project directory and lays down a
framework template; both halves disqualify it here. This repository is non-empty (agent kit, `docs/`,
`openspec/`), and its React template would be discarded anyway: no TanStack Router file-based
routing, no Query, no vitest/Playwright, no lint tooling and no `tauri-specta`. The v2 CLI documents
`tauri init` as *"Initialize a Tauri project in an existing directory"*; it creates **only**
`src-tauri/` and generates no frontend, which is precisely the gap section 0 has. There is therefore
no template to delete and deliberately no task that removes one.

**The generator pass** (every command below is in the v2 CLI reference):

| Step | Command | Produces |
|---|---|---|
| 1 | `cargo install tauri-cli --version "^2" --locked` | the CLI itself (the install line the v2 docs give) |
| 2 | `cargo tauri init --ci -D ../dist -P http://localhost:1420 --before-dev-command "bun run dev" --before-build-command "bun run build"` | `src-tauri/` — the crate, `tauri.conf.json`, the capability set, and **Tauri's default iconset as a temporary placeholder** |
| 3 | `cargo tauri ios init` | the iOS target (`src-tauri/gen/apple/`) |
| 4 | `cargo tauri capability new` | nothing: the capability file 0.6 hardens is the `capabilities/default.json` `tauri init` wrote in step 2, and no second file is created (Q13 resolved) |

`cargo tauri icon` is deliberately **not** part of this pass any more. The source mark is drawn from
the design-system tokens, which do not exist at bootstrap time, so the icon step moved to a
`frontend-ui` task of its own (0.22, D0.19). The consequence is stated rather than hidden: task 0.1
therefore ships **Tauri's default iconset** temporarily, and 0.22 replaces it inside section 0,
before anything is released.

`tauri add <plugin>` is the CLI's generator-managed path for later plugin additions; section 0 adds
no plugin (D0.8), so it is not used here.

**Baseline, then refine.** The generated output is a *baseline the existing tasks refine*, not
something they replace. The frontend values step 2 passes are the contract between the bootstrap and
tasks 0.13/0.14: those two must match the dev URL and dist path exactly, since `tauri init` generates
no frontend and nothing else reconciles the two sides. `tauri init --ci` takes the defaults for app
name, window title and bundle identifier, so the bootstrap **invents no product identity and is not
blocked on one** — the real identity is set once in task 0.5 (D0.15) and consumed by the iOS
target's usage strings in task 0.8, which also applies the deployment floor measured by task 0.7
(D0.16).

**What the generator does and does not save.** It removes the native and configuration boilerplate
and is the only sane way to get the iOS target; it does **not** reduce the work of exact pinning
(TOOL-8 — generators emit caret ranges, so task 0.2's pin set and tasks 0.10/0.13 still have to
impose it), nor the frontend stack, the `tauri-specta` pipeline, the test harness, the lint tooling,
the design system or CI. All of those stay hand-owned.

**Generator-owned versus hand-owned.** This split governs later edits: a generator-owned artefact is
regenerated with the CLI, never hand-edited; a hand-owned artefact is ours once generated.

| Artefact | After generation |
|---|---|
| `src-tauri/Cargo.toml`, `build.rs`, `src/main.rs`, `src/lib.rs` | hand-owned: 0.10 reconciles the pins and mounts `ipc` |
| `src-tauri/tauri.conf.json` | hand-owned: 0.5 sets the identity, 0.6 narrows the CSP |
| `src-tauri/capabilities/*.json` | hand-owned: 0.6 hardens the generated default capability |
| the iOS target's Xcode project under `src-tauri/gen/apple/` | generator-owned: re-run `cargo tauri ios init` rather than hand-editing the project file |
| `src-tauri/gen/apple/project.yml`, `src-tauri/gen/apple/Podfile` | **generator-owned:** both are rendered by `cargo tauri ios init` and both carry deployment floors (`project.yml` `deploymentTarget.iOS: 15.0`; `Podfile` `platform :ios, '15.0'` and `:osx, '11.0'`); regenerate with `cargo tauri ios init`, never hand-edit. The iOS floor is read from `tauri.conf.json` → `bundle.iOS.minimumSystemVersion` and baked into `project.yml`/`pbxproj` at `ios init` time (the generated config sets no such key yet, so today's values are the CLI defaults), so 0.8 applies the floor 0.7 measured by setting that key and regenerating — not by hand-editing anything under `src-tauri/gen/apple/` |
| the iOS target's `Info.plist` and entitlements files under `src-tauri/gen/apple/` | hand-owned once generated: 0.8 sets the usage strings and the entitlements the scaffold actually uses; no deployment target lives in these files |
| `src-tauri/icons/**` | **hand-owned:** generated once from our own source mark by `frontend-ui` in 0.22 (`bun run tauri icon`, D0.19); Tauri's default iconset from 0.1 is a temporary placeholder replaced there, and the icons are regenerated with that same command thereafter |
| `@tauri-apps/cli` as a dev dependency | hand-owned: 0.13 pins it exactly so local runs and CI use the pinned CLI instead of the global `cargo install` |

**Ordering note.** The pass follows the CLI's documented order, which also matches the v2 docs'
statement that `tauri icon` runs *after* `ios init` has set up the Xcode project (the icon step has
since moved out of the bootstrap to task 0.22, so only `ios init`'s own position matters here).
`tauri ios init` therefore stays in its documented position, *before* any frontend exists, and the
risk of it needing one is settled by D0.20: if `cargo tauri ios init` fails for want of a frontend
dist, it moves to after the frontend task (0.14) **within the same generator pass, under the same
owner**, and the correction is reported as a blocker rather than worked around.

##### Generator pass — observed output (task 0.1)

The pass ran on 2026-10-16 with **`tauri-cli` 2.12.3** (`cargo tauri --version` prints `tauri-cli
2.12.3`). Every flag in the table above was accepted as written: `cargo tauri init --help` on the
installed CLI lists `-D/--frontend-dist`, `-P/--dev-url`, `--before-dev-command` and
`--before-build-command`, so there is no corrected invocation to record.

| Step | Command as run | Exit | Observed output |
|---|---|---|---|
| 1 | `cargo install tauri-cli --version "^2" --locked` | 0 | `cargo-tauri` 2.12.3. One warning, in the CLI's own lockfile rather than ours: `zip v8.0.0` is yanked in the registry. |
| 2 | `cargo tauri init --ci -D ../dist -P http://localhost:1420 --before-dev-command "bun run dev" --before-build-command "bun run build"` | 0 | `src-tauri/` only, nothing at the repository root: `Cargo.toml`, `Cargo.lock`, `build.rs`, `tauri.conf.json`, `capabilities/default.json`, `icons/` (16 files — the CLI template's full iconset), `src/main.rs`, `src/lib.rs`, `.gitignore`, `gen/schemas/`. No frontend, as expected. |
| 3 | `cargo tauri ios init` | 0 | `src-tauri/gen/apple/`: `app.xcodeproj`, `app_iOS/` (holding `Info.plist` and `app_iOS.entitlements`), `Sources/app/`, `Assets.xcassets/`, `assets/`, `Externals/`, `Podfile`, `project.yml`, `LaunchScreen.storyboard`, `ExportOptions.plist`, `.gitignore`. Preflight found `xcodegen`, `idevicesyslog` and `pod`. |
| 4 | `cargo tauri capability new` | 1 | **nothing generated** — `Error failed to prompt input: IO error: not a terminal`. |
| 5 | `cargo check --manifest-path src-tauri/Cargo.toml` | 0 | passes. |
| 6 | `cargo check --manifest-path src-tauri/Cargo.toml --target aarch64-apple-ios` | 101 | **fails** (below). |
| 7 | `cargo tauri --version` | 0 | `tauri-cli 2.12.3`. |

**`ios init` was not deferred.** It ran in its documented position, before any frontend exists, and
succeeded, so the D0.20 fallback is *not used* and nothing from the pass is missing on that account.
`cargo tauri icon` was not run, as decided (D0.19).

**Step 4 needed a decision, not a workaround.** `capability new` takes its identifier either as a CLI
argument or as an interactive prompt; with no TTY it refuses, and this change defines no identifier, so
none was invented. Its stated purpose is already met by generator output: step 2 wrote
`src-tauri/capabilities/default.json` (identifier `default`, `permissions: ["core:default"]`,
`windows: ["main"]`), the file the hand-owned table above already assigns to 0.6. That question is
settled (2026-10-16): 0.6 hardens the generated `src-tauri/capabilities/default.json` to least
privilege and creates no second capability file — a single window leaves nothing to split, and an extra
file would only widen the surface 0.6 exists to narrow (Q13).

**Step 6 fails, and the cause is the generator's own plugin choice.** The host check passes; the iOS
check does not:

```
error[E0425]: cannot find function `mach_task_self` in crate `libc`
  --> num_threads-0.1.7/src/apple.rs:34:31
error: could not compile `num_threads` (lib) due to 1 previous error
```

The chain, read from the resolved sources rather than inferred: the generator's `Cargo.toml` carries
`tauri-plugin-log = "2"` (resolved to 2.10.1), which depends on `time` with the **`local-offset`**
feature; `time 0.3.55`'s `local-offset` feature pulls in `num_threads` (declared for
`cfg(target_family = "unix")`, so iOS is included), and `num_threads 0.1.7` calls
`libc::mach_task_self()`, which `libc 0.2.190` defines under `#[cfg(target_os = "macos")]` and
annotates "Prohibited on iOS/tvOS/watchOS/visionOS". The host build passes only because that gate is
open there.

This contradicts D0.8's "Section 0 adds **no Tauri plugin**, so there is no plugin whose platform
support must be confirmed": the generated baseline *does* add one, and that plugin is what fails
TOOL-1/TOOL-2 on iOS. **Decided (rule 6): the plugin goes.** `tauri-plugin-log` is dropped from the
generated `Cargo.toml` and its debug-only `setup` registration from `src/lib.rs` — a recorded
refinement of the generated baseline in the "baseline, then refine" model above, justified in D0.8 and
applied inside **task 0.1's own baseline commit**, by `rust-backend` as the implementing specialist;
`Cargo.toml` and `lib.rs` are task 0.10's artefacts, but the removal is not task 0.10's work. Step 6's
iOS failure is therefore already repaired on the committed tree: re-running
`cargo check --manifest-path src-tauri/Cargo.toml` and
`cargo check --manifest-path src-tauri/Cargo.toml --target aarch64-apple-ios` both pass (the
orchestrator reproduced both), which meets TOOL-1's "[ios] The iOS target compiles" scenario.
Pinning around it was rejected: that keeps an unrequested plugin and adds pins or a patch for no
product benefit. No replacement dependency is added, and the failing chain goes with the plugin. The
now-unused `log = "0.4"` dependency the generated `Cargo.toml` still carries is dropped in the same
task-0.1 commit, for the same rule-6 reason as the plugin: its only consumer was the removed plugin's
`setup` closure and nothing in `src-tauri/src/` references `log::` any more. Task 0.10 therefore
inherits only a **forward obligation**: it MUST NOT reintroduce `tauri-plugin-log` or anything that
switches on `time/local-offset` (`time` itself remains in the graph via `cookie`, `plist`,
`serde_with` and `tauri-build`).

**Placeholder identity observed**, i.e. the `--ci` defaults this task deliberately did not override and
0.5/D0.15 replaces: `productName` `Tauri App`, identifier `com.tauri.dev`, window title `Tauri`, crate
`app` with library `app_lib`. The frontend contract values landed as passed — `frontendDist` `../dist`,
`devUrl` `http://localhost:1420`, `beforeDevCommand` `bun run dev`, `beforeBuildCommand`
`bun run build` — and are what 0.13/0.14 must match. Step 5 also produced the generator's
`src-tauri/Cargo.lock`; the **committed** lockfile holds **408** `[[package]]` entries, each resolved
to one **exact** version — a lockfile records resolved versions, never ranges. That is the post-removal
figure: dropping the logging plugin (above) took **seven** entries out of the generator's lockfile
(`android_log-sys`, `android_logger`, `env_filter`, `fern`, `num_threads`, `tauri-plugin`,
`tauri-plugin-log`), so the pre-removal figure was **415**, not the 414 an earlier measurement of this
pass recorded. The caret ranges it resolved from live in the generated
`src-tauri/Cargo.toml`, and that is the file D0.8 says 0.2/0.10 replaces with exact pins.

### D0.12 — An exhausted allowance must be reportable as its own condition (Q1)

SYNC-2's "storage allowance exceeded" cause is a **hard selection criterion** for the shared-service
mechanism, not a mapping to be improvised later. A candidate that cannot report allowance exhaustion
as its **own condition, distinct from a generic write error**, is rejected in task 1.1 (D0.7
criterion 5). SYNC-2's four-cause list is therefore fully honourable: connectivity unavailable, no
account, storage allowance exceeded and temporarily rate limited are four distinguishable outcomes,
and `SaveCause.allowance_exceeded` is produced from the service's own report rather than inferred
from a failed write.

### D0.13 — Concrete bounded times, as named constants (Q2)

The bounded times the requirements leave unnumbered are fixed here as constants:

| Constant | Value | Applies to |
|---|---|---|
| `SAVE_ATTEMPT_TIMEOUT` | **15 s** | a save attempt that gets no outcome is refused with the `timeout` cause |
| `REACHABILITY_PROBE_TIMEOUT` | **5 s** | a reachability probe that gets no answer counts as a failed probe |
| `OFFLINE_FAILURE_THRESHOLD` | **3 consecutive failures** | without an intervening success, only then is the status reported as offline |

The third value is NFR-6's threshold, adopted here in P0 as a concrete number so that P3 **inherits**
it rather than redefining it. The `timeout` cause is therefore a first-class cause with a bound, not
a placeholder; there is no unnumbered timing left in the reachability or save paths. Tasks 2.4, 2.13
and 3.4 apply these constants.

### D0.14 — Client-generated request ids, deduplicated by the service (Q3)

Every state-changing command carries a **client-generated `request_id`**, and the shared service
deduplicates on it, so a retry is **safe by construction**: re-invoking a failed command with the
same `request_id` cannot create a second item. This is a selection criterion for the mechanism in
task 1.1 (D0.7 criterion 6), it shapes the contract in task 1.3 — where `CommandError.request_id` is
**semantic, not diagnostic**: it names the operation whose identity the service must not duplicate —
and it makes household creation idempotent on `request_id` in task 2.1. Reconciliation by re-reading
the collection before a retried create is therefore not needed, and R6 is closed.

### D0.15 — App identity (Q4)

Bundle identifier **`com.bunscale.app`**, display name **`BunScale`**, and **English only**: no
localized display name is shipped, even though NFR-5 (P4) covers German and English *content*. This
is the single source of truth in `tauri.conf.json` (task 0.5) and is consumed by the iOS target's
usage strings (task 0.8). Neither task is blocked: there is no open question left on identity, and
`release-signing` can treat the identifier as fixed from section 0 on.

### D0.16 — Deployment targets are derived, not chosen (Q5)

The minimum iOS and macOS deployment targets MUST be the **lowest the installed SDKs still support**.
The product owner explicitly does not want a hand-picked number, and the value cannot be guessed, so
it is measured empirically: task 0.7 (`apple-native`, because `tauri-security` has no
`xcodebuild`/`xcrun` permission) inspects the installed Xcode and SDKs — for example
`xcodebuild -showsdks` plus the installed SDK's own deployment-target floor as reported by its
`SDKSettings.plist` after `xcrun --sdk <sdk> --show-sdk-path` — and records the resulting floor for
iOS and for macOS in this change. Task 0.8 then **applies the recorded floor** instead of asking for
a version, and the same recorded floor constrains the Rust skeleton (task 0.10). A hand-written
version number here is a defect, not a decision.

### D0.17 — Biome, for lint and format (Q6)

`bun run lint` uses **Biome**, which owns both linting and formatting for the frontend. No second
linter and no Prettier are added: `cargo fmt` already covers Rust, and adding a second JavaScript
linter (or a separate formatter) would churn the exactly pinned dependency set TOOL-8 requires.
Task 0.18 implements it; `proposal.md`'s rule-6 justification records it.

### D0.18 — The `tauri-specta` release candidate is pinned, with `specta` (Q7)

The `tauri-specta` `2.0.0-rc.x` pin is **accepted**, pinned to an exact version **together with
`specta`** (TOOL-8's "paired crates move together"). `tauri-typed-ipc` is **not** evaluated first:
the accepted `2.0.0-rc.x` line already generates the bindings this pipeline needs, and a second
generation pipeline would churn TOOL-8's exact-pin set for no gain this phase. That evaluation is
deferred, not dropped — it lands as an ADR numbered **after** `0002`, in the change that owns it;
section 0 takes no ADR number. The bump discipline this implies is part of the decision: bump both
crates together, regenerate `src/bindings.ts`, and review the bindings diff deliberately — a bump
that changes the generated file's shape is a reviewed change, never a silent drift that the
`git diff --exit-code src/bindings.ts` check would attribute to the contract. Task 0.3 records it in
the bindings-pipeline design; task 0.2 pins it.

### D0.19 — A branded placeholder icon, generated after the design tokens exist (Q8)

The app mark is produced by `frontend-ui`, **not** by the CLI bootstrap and **not** by the product
owner: a minimal flat mark drawn from the design-system tokens (a squared PNG or SVG with
transparency), fed to the icon generator. Because it depends on the tokens, `cargo tauri icon` is
**removed from the bootstrap task 0.1** and becomes task 0.22, placed after both the design-system
primitives task (0.21) and the `package.json` task (0.13) so it can call the exactly pinned
`@tauri-apps/cli` dev dependency as `bun run tauri icon <source>` — no new permission is needed and
no global `cargo tauri icon` is required. The consequence is explicit: **task 0.1 therefore ships
Tauri's default iconset as a temporary placeholder**, and that placeholder is replaced inside section
0 by 0.22 before anything is released. Q8 no longer blocks task 0.1. The icon set is **hand-owned**
in D0.11's split: generated once from our own source mark, regenerated with the same command when
the mark changes.

### D0.20 — `ios init` keeps its documented position, with a fallback (Q9)

`cargo tauri ios init` stays in the bootstrap (task 0.1) in the CLI's documented order. If it fails
because no frontend dist exists yet, it moves to after the frontend task (0.14) **within the same
generator pass and under the same owner** (`architect`), and the correction is reported as a blocker
rather than worked around. The artefact ownership recorded in D0.11 is unchanged by that move, and
the ordering risk is closed by this decision instead of living in `questions.md`.

## State and boundaries

```
Rust (src-tauri/)                                  Frontend (src/)
  src-tauri/src/ipc/            contract: commands, events, shared types  → src/bindings.ts (generated)
  src-tauri/src/domain/         household, member, ownership, invariants
  src-tauri/src/storage/        storage boundary: household isolation, canonical
                                representation, immutability, absolute instants
  src-tauri/src/sync/           reachability gate, loaded copy, refresh, LWW merge
  src-tauri/src/settings/       per-device preferences (display weight unit)
  src-tauri/src/diagnostics/    redacted diagnostics sink
  src-tauri/capabilities/       permissions (tauri-security owns)
                                                → src/features/households|settings|sync/ api.ts + hooks
                                                → src/routes/ (onboarding, dashboard shell, settings)
```

Rules held here:

- The frontend never calls `invoke` directly; only `src/features/*/api.ts` wrappers over generated
  bindings (project hard rule 2).
- Per-device preferences (display unit; later language and reminders) live on the device and are
  never part of a household collection. Household-scoped data is never stored per device only.
- The household collection is the only source of truth for household data; the local copy is a
  cache of it, never a competing record.
- Reachability is a state the *product* derives (from a fresh outcome), not a value cached from a
  previous session. The repeated-failure rule that NFR-6 (P3) depends on is adopted here as the
  concrete three-consecutive-failures threshold (D0.13), so P3 inherits it instead of redefining it;
  P0 must still never claim a stale status as current.

### Scaffold surface added by section 0

Section 0 creates the workspace the diagram above assumes. It adds no product behaviour; these are
the paths later phases inherit:

| Path | Purpose |
|---|---|
| `package.json`, `index.html`, `tsconfig.json`, `vite.config.ts`, `.gitignore` | Frontend workspace; the dev server sets `server.host = process.env.TAURI_DEV_HOST` for iOS dev |
| `src-tauri/Cargo.toml`, `build.rs`, `src/main.rs`, `src/lib.rs` | Rust skeleton; `cfg`-clean for later platforms |
| `src-tauri/tauri.conf.json`, `src-tauri/capabilities/`, `Info.plist`, entitlements | App identity and least-privilege baseline (`tauri-security`) |
| `src-tauri/src/ipc/` | The specta builder and export function, empty command/event set until 1.3 |
| `src-tauri/examples/export_bindings.rs` | The headless bindings export entry point (D0.9), run by `bun run gen:bindings` as a cargo example — never bundled, and never built by the iOS `cargo check` |
| `src-tauri/gen/apple/`, `src-tauri/icons/` | The iOS target — generator-owned, produced by the CLI bootstrap (D0.11) — and the icon set, which is **hand-owned**: Tauri's default iconset from task 0.1 is a temporary placeholder that task 0.22 replaces from the design tokens (D0.19) |
| `src-tauri/tests/`, `src/test/`, `e2e/` | Integration harness, vitest/Testing Library helpers, Playwright WebKit projects |
| `src/styles/`, `src/components/ui/` | Design tokens and the shared primitives |
| `.github/workflows/ci.yml` | The gate, on `macos-latest` |

## IPC contract — to be defined by architect

**Status: proposal only. `architect` owns this section and the typed stubs in
`src-tauri/src/ipc/` (tasks 1.1–1.4). Nothing may be implemented against these names until 1.3 is
done and `bun run gen:bindings` regenerates `src/bindings.ts`.**

Consistent with D0.9: the *pipeline* that produces `src/bindings.ts` exists from section 0 and
already runs against an **empty** command and event set, so the scaffold gate passes with no diff.
The names below are therefore not yet in the contract: until task 1.3 lands, the generated file
contains no command and no event, and no frontend code may be written against the table.

### Commands the frontend needs in this phase

| Command (proposed) | Purpose | Requirements |
|---|---|---|
| `create_household(name)` | Create a household, creator becomes first member, becomes active | HS-1, HS-2 |
| `list_households()` | Households held by the person, with the active one indicated | HS-3, HS-4, DM-3 |
| `get_active_household()` | The active household (or none) | HS-2, HS-3, DM-3 |
| `set_active_household(household_id)` | Switch the active household | HS-3, HS-4, DM-3 |
| `load_household(household_id)` | Loaded-data-first read of a household collection, refreshing when reachable | SYNC-1, SYNC-3 |
| `get_service_status()` | Last observed service/account status with the time it was observed | NFR-1 |
| `check_service_reachability()` | Fresh reachability probe, used before a write and on foreground return | SYNC-2, NFR-1 |
| `get_identity()` | Account user id and resolved display name, or unresolved | SET-2 |
| `get_display_preferences()` / `set_display_weight_unit(unit)` | Per-device display unit | SET-4 |
| `get_diagnostics_summary()` *(optional, P0)* | Redacted counts/outcomes for support; never content | NFR-3 |

Every state-changing command in the table carries a **client-generated `request_id`**, and the
shared service deduplicates on it, so a retry is safe by construction (D0.14): retry (SYNC-2) is a
re-invocation of the failed command with the *same* `request_id`, not a separate retry command.
`request_id` is therefore **semantic, not diagnostic** — it identifies the operation the service
must not perform twice. Mechanism selection is conditional on that support (task 1.1).

### Events the frontend needs in this phase

| Event (proposed) | Payload | Purpose |
|---|---|---|
| `household:changed` | `{ household_id, kind: created \| switched \| refreshed \| removed }` | Invalidate/refresh the affected household query only |
| `service:status-changed` | `ServiceStatus` | Drive the status indicator and the offline/loaded-data states |

### Shared types

| Type (proposed) | Shape | Notes |
|---|---|---|
| `HouseholdId`, `MemberId`, `AccountUserId` | opaque, stable strings | shared verbatim across members |
| `Household` | `{ id, name, created_at, owner_member_id, is_active, is_owner }` | `is_owner` gates P3's delete control |
| `HouseholdSummary` | `{ id, name, created_at, is_active, is_owner, member_count, rabbit_count }` | list view; counts may be omitted in P0 |
| `Member` | `{ id, account_user_id, display_name }` | `display_name` optional → SET-2 fallback chain |
| `Identity` | `{ account_user_id?, display_name?, resolved: bool }` | unresolved must not break Settings |
| `ServiceStatus` | `{ state: reachable \| unreachable \| unknown, account_valid: bool, cause?, observed_at }` | `unknown` until a fresh outcome exists |
| `SaveCause` | `connectivity \| no_account \| allowance_exceeded \| rate_limited \| timeout \| validation \| unknown` | drives the cause-specific copy of NFR-8 |
| `CommandError` | `{ cause: SaveCause, retryable: bool, message_key, request_id }` | one failure surface for every operation; `request_id` is **semantic** — the operation's identity for the service's deduplication — not a diagnostic field |
| `WeightUnit` | `grams \| kilograms` | display only; storage stays canonical |
| `Instant` | RFC 3339 absolute instant, normalised to UTC | DM-6 |

`SaveCause` → UI copy is one-to-one, so no cause can degrade into a generic message (NFR-8). A
`timeout` cause exists because HS-5 and NFR-1 require stalled operations to resolve with feedback
rather than hang; it is a first-class cause, not a placeholder, and its bound is the save-attempt
constant of D0.13. `allowance_exceeded` likewise reports the exhausted owner allowance as its **own**
condition (D0.12) rather than as a generic write error mapped after the fact.

## Traceability: user stories → scenarios

| Story | Requirement(s) | Capability / scenario |
|---|---|---|
| US-HS-01 | HS-1, HS-2 | household-sharing: "Create the first household", "Onboarding shown when no household exists", "Onboarding not repeated once a household exists" |
| US-HS-02 | HS-1 | household-sharing: "Blank name is rejected" |
| US-HS-03 | HS-3 | household-sharing: "A second household coexists without mixing data" |
| US-HS-04 | HS-3, HS-4 | household-sharing: "Switching the active household changes the presented data", "All households are listed with the active one indicated" |
| US-HS-05 | HS-4 | household-sharing: "Create an additional household without onboarding" |
| US-SYNC-01 | SYNC-1 | sync-and-offline: "Every member resolves the same items", "Refreshing does not duplicate an item" |
| US-SYNC-02 | SYNC-2, NFR-1, NFR-8 | sync-and-offline: "Save refused without connectivity", "Save succeeds when the service is reachable"; product-quality: "Writes are accepted only while reachable and identified", "A failed save is visible and specific" |
| US-SYNC-03 | SYNC-2, NFR-8 | sync-and-offline: "Save failure names its cause and offers a retry"; product-quality: "No bare failure messages" |
| US-SYNC-04 | SYNC-3 | sync-and-offline: "Previously loaded data is shown while unreachable" |
| US-SYNC-05 | SYNC-3 | sync-and-offline: "A start with connectivity refreshes loaded data" |
| US-SYNC-06 | SYNC-3 | sync-and-offline: "Nothing loaded and unreachable" |
| US-SYNC-09 | SYNC-5, NFR-2 | sync-and-offline: "Two members edit the same item", "Convergence does not duplicate or fail"; product-quality: "Concurrent edits resolve without failing" |
| US-SET-01 | SET-1 | settings-profile: "Gear control opens Settings as its own screen", plus the `[mobile]`/`[desktop]` scenarios |
| US-SET-02 | SET-2 | settings-profile: "Resolved account name is shown read-only" |
| US-SET-03 | SET-2 | settings-profile: "Unresolved identity falls back without breaking the screen" |
| US-SET-06 | SET-4 | settings-profile: "Unit choice is stored on the device", "Changing the unit leaves stored values unchanged" |
| US-QUAL-05 | NFR-2 | product-quality: "Conflict resolution never fails and never duplicates" |
| US-QUAL-06 | NFR-3 | product-quality: "Saved data reaches no third party", "Diagnostics carry no content" |
| US-QUAL-09 | NFR-10 | data-integrity: all three scenarios (weight unit and archive clauses land in P1/P2) |
| US-WT-04 | SET-4 (P0 part) | settings-profile: "Default unit follows the device region"; the entry-form behaviour is WT-3 in P1 |
| US-HS-23 | NFR-8 (P0 part) | product-quality: "No bare failure messages"; the deletion flow itself is HS-10 in P3 |
| US-MED-23 | — | **Dropped** by decision D1 (offline dose logging) |

Stories with no P0 scenario: **none** among the requirements this change owns.
**DM-3** and **DM-6** have no story of their own — DM-3 is exercised by US-HS-03/04 and DM-6 by the
timestamped entries added in P1/P2; both are stated as invariants with directly testable scenarios
here anyway.

### Scaffold traceability: `project-toolchain` (TOOL-1 … TOOL-8)

The `project-toolchain` requirements trace to **no user story and no source requirement**: the source
documents describe product behaviour and silently assume a codebase that does not exist, so the
product owner added this enabler capability with the section 0 workspace scaffold. They are not a
category of the story table above and are not part of the 18 product requirement IDs this change
owns; they are listed separately, and `specs/project-toolchain/spec.md` says so in its header.

| Requirement | Section-0 tasks that implement or verify it |
|---|---|
| TOOL-1 (builds from a clean checkout, full gate passes) | 0.1, 0.5, 0.6, 0.7, 0.9, 0.10, 0.12, 0.13, 0.14, 0.18, 0.19, 0.22, 0.24, 0.26 |
| TOOL-2 (boots on the iOS simulator and on macOS) | 0.1, 0.5, 0.7, 0.8, 0.10, 0.14, 0.22, 0.27 |
| TOOL-3 (bindings reproducible, drift fails the build) | 0.3, 0.4, 0.9, 0.11, 0.16, 0.24 |
| TOOL-4 (route tree reproducible, drift fails the build) | 0.3, 0.9, 0.15, 0.24 |
| TOOL-5 (test harness at both baselines) | 0.17, 0.23, 0.25 |
| TOOL-6 (design system established, status recorded) | 0.20, 0.21, 0.22 |
| TOOL-7 (CI runs the same gate) | 0.9 |
| TOOL-8 (exactly pinned, both platforms) | 0.2, 0.10, 0.13 |

Two section-0 tasks have no TOOL requirement of their own and are listed against the scaffold
requirements their artefacts serve: 0.7 (D0.16) records the deployment floor that 0.8 and 0.10 apply
— a committed part of the buildable, bootable scaffold (TOOL-1, TOOL-2) — and 0.22 (D0.19) replaces
the generator's placeholder icon set with the product's own app mark, which is a shipped scaffold
artefact (TOOL-1, TOOL-2) drawn from the design system (TOOL-6).

## Risks

| # | Risk | Mitigation in this change |
|---|---|---|
| R1 | **Sharing correctness (IMP-2)** — the historical failure mode. Multi-member convergence must be provable, not assumed. | SYNC-1/SYNC-5 stated as observable behaviour; LWW convergence and no-duplicate cases tested at unit and E2E level; invitation/acceptance end-to-end hardening scheduled as P3 rather than deferred. |
| R2 | **Last-write-wins loses an edit.** Two members editing the same item means one change is discarded silently. | Accepted by D1/D2 and NFR-2. Mitigation: keep the conflict window narrow by refreshing on load/foreground; no UI promise of merge semantics. Revisit only if the product later wants per-field merge. |
| R3 | **False or stale "offline".** Claiming offline after one transient failure, or showing a previous session's status as current, is a truthfulness bug. | `ServiceStatus.state` starts as `unknown` and "offline" is only shown after a fresh outcome; the threshold is settled as three consecutive failures without an intervening success (D0.13, NFR-6's rule adopted in P0 so P3 inherits it), and `observed_at` lets staleness be recognised. |
| R4 | **Cause list not reportable.** If the chosen mechanism cannot expose allowance/rate-limit conditions, SYNC-2's cause list cannot be honoured. | D0.7 makes availability/account reporting (4), allowance/rate-limit reporting (5) and idempotency-key support (6) mandatory selection criteria, and D0.12 requires an exhausted allowance to be reportable *distinctly*; task 1.1 rejects any candidate that cannot, so Q1's uncertainty is closed by a criterion rather than carried. |
| R5 | **Build-time cost of the mechanism on iOS.** A client library that does not support iOS/macOS, or that needs a hostile capability/CSP widening, would stall P0. | Project hard rule 6: the mechanism must support both platforms and is justified in one line; `tauri-security` (task 1.4) keeps the widening least-privilege. |
| R6 | **Duplicated household on retried create.** A create that timed out after the service accepted it, then retried, would violate SYNC-1's "exactly one copy". | Closed by D0.14: every state-changing command carries a client-generated `request_id` and the service deduplicates on it, so the retry SYNC-2 offers is safe by construction. The criterion is part of mechanism selection (1.1), the contract (1.3) and household creation (2.1). |
| R7 | **Per-device preference drift.** Unit, later language and reminders are per device; a person with two devices sees two settings. | Accepted: it is the documented product model (§5). The display unit is deliberately not a household field, and later phases must not add it to the collection. |
| R8 | **Scope creep into P1.** The dashboard shell in P0 can tempt rabbit placeholders. | The dashboard shell in P0 renders household/offline/settings surfaces only; rabbit list, empty state and quick-weigh belong to DASH-1..DASH-9 in P1. |
| R9 | **The scaffold gates everything else.** A broken pin, a missing gate script or an unrecorded design system blocks section 1 onwards *and* all six sibling changes, so it would stall the whole project rather than one task. | TOOL-1 makes the gate itself a requirement and 0.26 records a fresh full-gate run; 0.1 generates the baseline with the official CLI first, so nothing below it hand-writes native or configuration boilerplate; TOOL-8 pins exact versions and requires both-platform support, and 0.13 pins `@tauri-apps/cli` exactly so local runs and CI use the pinned CLI rather than repeating the global `cargo install`; 0.20 records the design-system status before any dependent UI work; 0.22 replaces the generator's placeholder icon set inside section 0, so no build can ship Tauri's default mark. The decisions that used to block section-0 tasks are settled (D0.12–D0.20), so no section-0 task is waiting on a product-owner answer (project rule 7 no longer applies to section 0). |

## Verification

- Specs: `openspec validate add-shared-household-foundation --strict`.
- Gate (per `definition-of-done`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` (no diff),
  `git diff --exit-code src/routeTree.gen.ts` after a build, `bun run typecheck`, `bun run lint`,
  `bun run test`, `bun run test:e2e`.
- Section 0 applies that gate incrementally, as the preamble in `tasks.md` states: a section-0 task is
  ticked when every gate command *runnable at that point* passes, and a gate command whose precondition
  is a later section-0 deliverable belongs to that task (the bun-family commands arrive with 0.13, the
  two drift checks with 0.16, the E2E suite with 0.17). Task 0.12 restores `cargo fmt --check` and the
  clippy-clean requirement — the generator's own 2-space style is red until then — and 0.26 re-runs the
  complete gate fresh. That incremental carve-out **expires at 0.12 and must not extend past it**: from
  0.12 on, every gate command that already exists is run in full against the tree, and a command that is
  still absent is owned by the task that first makes it runnable rather than deferred further.
- Every requirement above has at least one scenario named by a test task in `tasks.md`.
- Scaffold: TOOL-1 is evidenced by 0.26 (the full gate run fresh and recorded), TOOL-2 by 0.27
  (manual simulator and macOS boot, recorded under "Manual verification"), TOOL-3 and TOOL-4 by 0.24
  (both drift checks), TOOL-5 by 0.25, TOOL-6 by 0.20/0.21, TOOL-7 by 0.9 and TOOL-8 by 0.2/0.10/0.13
  — see the scaffold traceability table above. The deployment floor is measured by 0.7 and applied by
  0.8; the product's own app mark replaces the generator's placeholder in 0.22.
- Decisions: D0.12 … D0.20 record the settled answers to the former `questions.md` Q1–Q9, and every
  task that consumes one is unblocked, but `questions.md` **no longer carries "no open question"**:
  Q13 is resolved for `tauri-security` in task 0.6: the product owner decided on 2026-10-16 that 0.6
  hardens the generated `src-tauri/capabilities/default.json` to least privilege and does not create a
  second capability file — a single window leaves nothing to split, and an extra file would only widen
  the surface 0.6 exists to narrow —
  and Q14 is the iOS-compile failure caused by the generator's logging plugin, whose decision is
  recorded in D0.8/D0.11 here and applied inside task 0.1's own baseline commit by `rust-backend` as
  the implementing specialist, not deferred to task 0.10; the plugin's now-unused `log = "0.4"`
  dependency is dropped in that same commit under the same rule-6 reason (no requirement in this
  change asks for logging, and its only consumer was the removed plugin's `setup` closure). Both check
  commands pass on the committed tree, and task 0.10 inherits only the forward obligation not to
  reintroduce `tauri-plugin-log` or `time/local-offset`.
- Cross-check at review: the delta contains exactly HS-1…HS-4, SYNC-1/2/3/5, SET-1/2/4,
  NFR-1/2/3/8/10, DM-3 and DM-6 across the five product capabilities, **plus exactly TOOL-1…TOOL-8
  in `project-toolchain`** — no more, no less. The `TOOL-` set is the section-0 enabler and is in
  scope; any further requirement, or any requirement implemented without a scenario, is scope creep
  (task 6.2).
