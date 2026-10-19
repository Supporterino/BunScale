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
`src-tauri/`, no Vite/TypeScript config, no CI workflow, and no `.gitignore` beyond the single
`graphify-out/cache` entry the tooling already keeps in one. It is folded into this
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
0.10 has since carried these pins into it, so this record is the input that produced the committed
versions rather than a parallel set; the later tasks still read it as the specification behind
them. `package.json` **now exists**: task 0.13 created it from these pins and `bun install` resolved
exactly them (the "Materialised npm manifest" subsection below records that outcome), so the npm
table is likewise the specification behind a committed file rather than a plan, and the remaining
tasks 0.14–0.19 add the configs, the test wiring and Biome it names. Every dependency carries its
rule-6 one-line justification; entries marked **grouped** say so and are grouped only because they
are the same kind of housekeeping dependency.

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

##### Materialised npm manifest (task 0.13)

The npm half of the table above now has a committed manifest, so those pins are measured against a
file rather than pending one. `package.json` is named **`bunscale`** — D0.15's product name in npm's
mandatory lowercase form, **not** the crate name (`app` stays deliberately, D0.11) — is
`"private": true`, and declares `"type": "module"` because the pinned Vite 8, TypeScript 7 and
`@vitejs/plugin-react` require ESM. **No `packageManager` field is added deliberately**:
`frontend-conventions` does not ask for one, and the bun pin is stated once, where CI acts on it
(`setup-bun: bun-version: 1.4.3`, D0.22); a field here would be a second statement of the same pin
with no consumer. **No `version` field is added either**: D0.15 makes `src-tauri/tauri.conf.json`'s
`version` the single source of truth for the app's version, and a second statement of it here would
drift from that one.

**Every one of the 21 pins resolved to exactly its pinned version**, with no widening and nothing
missing from the registry: the five runtime dependencies and the sixteen dev dependencies in the
table above, **175 installs across 236 packages** (bun's own figures,
`bun install --frozen-lockfile`), and the lockfile lists exactly those 21 as its direct
dependencies — nothing entered as a direct entry that this change did not name. The platform
binaries the iOS/macOS paragraph above predicted did arrive, as `optionalDependencies` of the pinned
packages, each in its `darwin-arm64` form: `@tauri-apps/cli-darwin-arm64` and
`@biomejs/cli-darwin-arm64` (direct), `@rolldown/binding-darwin-arm64` and
`lightningcss-darwin-arm64` (via `vite`), and `@typescript/typescript-darwin-arm64` (via
`typescript`).

The lockfile is **`bun.lock`**, in text form with `lockfileVersion: 2`; **no `bun.lockb` exists**.
That closes the caveat D0.22 records about the CI cache key: `hashFiles('bun.lock', 'bun.lockb')` no
longer evaluates to `''`, so from task 0.13 the bun cache is real rather than a constant key.
`bun install --frozen-lockfile` succeeds on that lockfile — verified twice — which is the step CI
runs; bun does **not** freeze automatically when `CI` is set, so the flag is load-bearing.

The eight script bodies, as the task fixed them in prose:

| Script | Body |
|---|---|
| `gen:bindings` | `cargo run --manifest-path src-tauri/Cargo.toml --example export_bindings` |
| `typecheck` | `tsc --noEmit` |
| `lint` | `biome check --error-on-warnings .` |
| `test` | `vitest run` |
| `test:e2e` | `VITE_E2E_MOCK=1 playwright test` |
| `dev` | `vite` |
| `build` | `vite build` |
| `tauri` | `tauri` |

`gen:bindings` stays the bare command, so `cargo run`'s exit status **is** the script's and nothing
is printed on success — the "propagates a non-zero exit" property D0.9 requires. `lint` is Biome's
**single** entry point, which lints **and** format-checks: that is how one gate command carries
D0.17's two jobs without a second tool. Since task 0.18 it also passes `--error-on-warnings`,
because `check` exits 0 on a warning-only tree while the recommended set contains warn-severity
rules (`style/useConst`) and `suspicious/noExplicitAny` is warn **by default** — so without the flag
the gate's only lint command could print a diagnostic and still pass; with it a warning fails the
run, at a measured cost of **zero** warnings across the checked set today. No CI edit follows,
because `.github/workflows/ci.yml` invokes `bun run lint` and the flag reaches it through
`package.json`. This is the **only** one of the eight bodies task 0.18 changed; the other seven
stand as task 0.13 fixed them. `test` is `vitest run`, never watch. `test:e2e` sets the mock
flag the E2E IPC layer keys on (TOOL-5). `tauri` resolves to `node_modules/.bin/tauri` — the pinned
`@tauri-apps/cli` — and **not** to task 0.1's global `cargo install`, which installs `cargo-tauri`
and no bare `tauri`, so the pinned CLI is what runs; that is what D0.22's assertion and task 0.22's
`bun run tauri icon` depend on. `dev` and `build` carry **no** port or output flags: `devUrl`
`http://localhost:1420` and `frontendDist` `../dist` live in `tauri.conf.json`, and task 0.14's
`vite.config.ts` is what satisfies them.

**One forward pin rule follows from what the install revealed.** The resolved graph contains packages
the manifest does not name — notably `prettier` (pulled in by the router plugin's generator, which is
why D0.17's "no Prettier" is a **manifest** statement rather than an absence from `node_modules`) and
`zod` (pulled in by the router plugin). A later task that **imports** such a package directly MUST add
it to the manifest as its own exact pin with its own rule-6 one-line justification, rather than
relying on it arriving transitively — the same discipline the three IPC crates followed when they
became direct names in `src-tauri/Cargo.toml`.

No ADR number is taken: section 0 takes none (the rule stated below), and this passage is evidence
for a decision already taken rather than a new decision.

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
anything that switches on `time/local-offset` (`time` itself stays in the graph without that feature:
the measured inverse tree attributes `time v0.3.55` to `cookie v0.18.2`, `plist v1.10.1` and
`tauri-codegen v2.7.3` — the build graph, not `tauri-build`; `serde_with` is in the lock but does **not**
enable `time` here — its build fingerprint records features `["alloc","default","macros","std"]`,
with no `time_0_3`, confirming it is not the enabler. The enabled `time` features are exactly
`alloc`, `default`, `formatting`, `macros`, `parsing` and `std` (measured by
`cargo tree -e features -i time`), so **no** `local-offset` — and `num_threads`, which
`time/local-offset` requires, is absent from the lockfile's package set, which is the meaningful
absence a lockfile scan can show, since the lockfile records resolved packages, not features).

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
`builder.export(Typescript::default(), <bindings path>)` — the second argument is the constant,
joined onto `CARGO_MANIFEST_DIR`, see below — with `collect_commands![]` and
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
`src-tauri/` — a unit test beside the code, an integration test in task 0.12's harness at
`src-tauri/tests/library_surface.rs` (whose third test asserts the adjacent "nothing else writes or
**rewrites**" half), or a smoke test like task 0.24's — may call that export function or the `bun
run gen:bindings` script, because such a test would run at the gate's own `cargo test` step, which
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
file"). Three preconditions make them meaningful rather than decorative. Both files are **tracked** —
from task 0.16 for `src/bindings.ts` and 0.15 for `src/routeTree.gen.ts`, and not before ("Tracking
state" below) — and neither may be excluded by `.gitignore` (task 0.19).
The route-tree check runs after the build in
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

**Honest compile status — as 0.4 left it, before task 0.10.** Nothing declared `pub mod ipc;` (that
was 0.10's mount, and `src-tauri/src/lib.rs` is outside task 0.4's writable paths), and the dependency
pins were 0.10's manifest work, so `cargo check` passed on the scaffold *with this module absent* and
proved nothing about it; the first compile evidence for these stubs lands with task 0.10. Nor does
`bun run gen:bindings` exist as a script yet — `package.json` is task 0.13's artefact — so nothing
here regenerates `src/bindings.ts` either: the first real export is task 0.16, and 0.4's evidence
is the two `cargo check` runs task 0.10 had to make non-vacuous.

**Task 0.10's outcome — the pins are materialised and the module is mounted, so that compile is no
longer vacuous.** The seven Rust pins are applied as written — `tauri =2.12.4`, `tauri-build =2.7.3`,
`serde =1.0.229` with `derive`, `serde_json =1.0.154`, `tauri-specta =2.0.0-rc.25` with `typescript`,
`specta =2.0.0-rc.25`, `specta-typescript =0.0.12`, the last three being the IPC crates new to the set —
and **no caret range remains** in `src-tauri/Cargo.toml`. `specta` needed no hand-added features:
`tauri-specta` requests `specta`'s `derive` and `function` features itself, so its pinned line stays
bare. One reading of the manifest is worth forestalling: `tauri-specta` depends on `tauri` with its
`specta` feature, so `tauri = { …, features = [] }` does **not** mean the final graph is specta-free —
the feature arrives transitively. The mount is the crate's **only** `setup` call, and `ipc::builder()`
is constructed **once** and **moved** into that closure rather than cloned, because
`invoke_handler(&self)` and `mount_events(&self, …)` both **borrow**: the same value that produced the
invoke handler mounts the events, which is the strongest reading of "one builder, mounted and exported"
above. `#[cfg_attr(mobile, tauri::mobile_entry_point)]` is kept and neither a `cfg(desktop)`-only nor a
`cfg!(mobile)` branch was introduced. **The non-vacuity is measured, not asserted:** before this task the
module was unreachable and the compiler's dep-info files listed only `src/lib.rs`; now both the host and
the `aarch64-apple-ios` dep-info files list `src/lib.rs` **and** `src/ipc/mod.rs`, so the gate compiles
the module. Two crate-metadata placeholders are **deliberately kept**, not left over:
`description = "A Tauri App"` and `authors = ["you"]` reach no iOS or macOS shipped artefact —
`tauri-bundler` 2.10.3's macOS plist writer reads `bundle.copyright` (not Cargo `authors`) for
`NSHumanReadableCopyright` and writes no `CFBundleGetInfoString` at all, consuming
`description`/`authors` only on the Linux/Windows packers, while `tauri-codegen` bakes both into
`tauri::PackageInfo` as runtime data nothing user-visible reads — so there is nothing to settle here, and
they surface only if a later phase enables a Linux/Windows packer.

**Task 0.11's outcome — the example exists, runs headless, and exports a one-line file.** The entry
point is `src-tauri/examples/export_bindings.rs`: `main` calls `ipc::export_bindings()` with no
argument and exits non-zero on failure. Measured from the repository root,
`cargo run --manifest-path src-tauri/Cargo.toml --example export_bindings` exits 0, starts no app and
creates no window — confirming "Exporting builds the builder and serialises it; it creates no window
and starts no app" above. It is **silent on success**, writing to stderr and exiting 1 only on
failure, so there is no success line for a script to parse: task 0.13's `gen:bindings` script must
pass the exit status through rather than grep output.

**The empty set's output, exactly:** `src/bindings.ts` is a **single line** — the `specta-typescript`
header comment, `// This file has been generated by Tauri Specta. Do not edit this file manually.` —
and nothing else: no empty object literal, no exports, no placeholder name. That is the "empty but
valid TypeScript module" this record requires, no more. The section-0 bindings file is therefore a
one-line file whose entire content is that header, and task 1.3's first command is what turns it into
a real module; a later reader must not mistake it for a truncated or broken export.

**Determinism, measured rather than eyeballed:** three successive runs produced the same git blob
hash `ca1fe9489cb42fc19cee67d21a5a02658aef607a` (`git diff --no-index --full-index /dev/null
src/bindings.ts`), and the hash was still identical after a `cargo test` run. That byte-stability is
what "The empty command set produces a stable artefact" and the no-diff check rest on.

**The D0.9 invariants held, each checked rather than assumed.** `cargo test` compiles but does **not**
execute the example — no `Running Examples` section, and the file's blob hash was unchanged
afterwards — so the gate's own `cargo test` step cannot rewrite the file before the drift check.
`cargo clippy --all-targets -- -D warnings` **does** compile and lint the example, as "The export
entry point is a cargo example, not a test" claims: a temporary injected `unused_variables` probe
failed clippy with `example "export_bindings"` in the message, then was reverted. `cargo check
--manifest-path src-tauri/Cargo.toml --target aarch64-apple-ios` does not build it, so the iOS target
is unaffected. And the prohibition above held in practice: no `#[test]` under `src-tauri/` calls the
export function — 0.11 added none, and `cargo test` left the file byte-identical.

**Tracking state.** `src/bindings.ts` is **untracked** at this point and deliberately left so. Task
0.16 commits it, and `tasks.md` 0.16 records why that matters: `git diff --exit-code` exits 0 with no
output on an untracked path, so an uncommitted file would make the drift check pass vacuously. Task
0.19 must keep it out of `.gitignore`, the precondition "The two drift checks CI runs" states above.

##### Task 0.12's outcome — the fmt carve-out closes, and the integration harness lands

**`cargo fmt --check` is green for the whole crate.** Only two files were reindented, both from the
generator's 2-space style to 4-space: `src-tauri/build.rs` (`tauri_build::build()`) and
`src-tauri/src/main.rs` (`app_lib::run();`). `src-tauri/src/lib.rs`, `src-tauri/src/ipc/mod.rs` and
`src-tauri/examples/export_bindings.rs` were already canonical, so the visible change is exactly those
two indentations. **That closes the section-0 carve-out:** from 0.12 on the whole Rust half of the gate
is green with nothing carved out, which is what "Section 0 applies that gate incrementally" under
"Verification" says it must be.

**The integration harness exists:** `src-tauri/tests/library_surface.rs` — one target, linking `app_lib`
as a library, with three tests and a `//!` doc comment naming the task and the harness's purpose. What
it asserts, and why those are the only three things assertable here:

- the context the binary edge runs: `tauri::generate_context!()` typed as `tauri::Context<tauri::Wry>`,
  asserting the identity `com.bunscale.app` and `BunScale` — so a later identity change that forgets
  this coupling fails the gate;
- the mount shape: two `invoke_handler()` handles off one builder; the `Send + Sync` half of
  `Fn(Invoke) -> bool + Send + Sync + 'static` is held **at compile time** by a bound-checking helper
  called on each handler **by reference**, and the value is then **moved across a thread**, which is the
  only half a move can hold — `Send + 'static`, the closure bound the real `setup` needs. A move cannot
  exercise `Sync`, and `assert!(moved.is_ok())` could never fail because the thread body has no panic
  path; the test's doc comment therefore states plainly that it is **two compile-time bounds plus one
  move** rather than pretending there is a run-time condition
  (`the_ipc_builder_hands_out_a_mountable_handler_and_survives_being_moved`);
- that constructing and mounting the builder creates, removes or **rewrites** no file under the
  repository's `src/` — the observable half of the prohibition above, since a test that wrote bindings
  would rewrite exactly what the drift checks verify. Regular files are compared by **content**, not by
  name: the violation D0.9 forbids — a `#[test]` calling `export_bindings()` — rewrites the
  `src/bindings.ts` that already exists, which a name-only comparison would leave identical; non-regular
  entries are compared by name and kind. A missing `src/` counts as **no entries** rather than an error,
  because a clean checkout has none — `src/bindings.ts` is untracked until task 0.16 commits it, and git
  cannot carry an empty directory — while every other `read_dir` failure still panics, so a permission
  or I/O failure stays loud. That clean-checkout correctness is verified from a **fresh worktree** (a
  checkout with no `src/` at all) as part of this task's evidence, and the test is
  `building_the_ipc_builder_creates_removes_or_rewrites_no_source_file`.

**The limits are recorded rather than worked around**, so that this small test set is not later read as
an oversight: `app_lib::run()` starts the Tauri event loop and is never called; `export_bindings()` is
never called and nothing in the harness emits bindings; and `tauri::ipc::Invoke` cannot be constructed
outside tauri, so the mounted handler cannot be invoked from a test — what proves the empty command and
event set is the exported bindings file, not the harness. Task 0.24 extends this target.

**The harness is under the gate, and that was measured rather than assumed.** `cargo clippy
--all-targets -- -D warnings` really does compile it: its first run failed *inside*
`tests/library_surface.rs` (`error: could not compile app (test "library_surface")`) before the file was
cleaned up, which is stronger evidence than a clean pass. `cargo test` runs 3 tests in that target and 0
elsewhere, and leaves `src/bindings.ts` byte-identical (blob
`ca1fe9489cb42fc19cee67d21a5a02658aef607a`, the same hash the 0.11 record above gives) — so the D0.9
invariants that record states still hold with a test target present.

**The `expect` was kept, not propagated.** `run()` is the process's last frame — `src/main.rs` calls it
and returns, and on mobile tauri's `mobile_entry_point` wrapper calls it inside `stop_unwind`, which
discards a returned value — so no caller could act on a `Result`; and the only reachable failure is
`Builder::build` failing before the event loop starts. The invariant comment now states that at the call
site, which is what `rust-conventions` requires of an `expect` outside tests, and it also records where
the panic message lands: on macOS it is what a person sees, on stderr, while on iOS it never reaches the
app edge at all — the generated `stop_unwind` catches the panic, logs
``attempt to unwind out of `rust` with err: …`` and calls `std::process::abort()` — and
`tauri::log_stdout` is not what reports it: that call only redirects stdout and stderr into the device
log **before** the wrapper body runs.

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
blocked on one** — the real identity is set once in task 0.5 (D0.15) and consumed by task 0.8, which
applied the deployment floor measured by task 0.7 (D0.16) and recreated the generated iOS target so
the identity is rendered into it (D0.11). It is *not* consumed by any usage string: nothing in the
scaffold calls a privacy-gated API, so 0.8 adds none (D0.16).

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
| the iOS target's Xcode project under `src-tauri/gen/apple/` | generator-owned: recreate it with `cargo tauri ios init` (delete the target and initialise it again) rather than hand-editing the project file — the templates are write-once, so re-running init in place changes nothing (next row) |
| `src-tauri/gen/apple/project.yml`, `src-tauri/gen/apple/Podfile` | **generator-owned:** both are rendered by `cargo tauri ios init` and both carry deployment floors (`project.yml` `deploymentTarget.iOS: 15.0`; `Podfile` `platform :ios, '15.0'` and `:osx, '11.0'`); regenerate with `cargo tauri ios init`, never hand-edit. Its templates are **write-once** (`if !path.exists() { create } else { Ok(None) }`, `src/mobile/ios/project.rs`) and `InitOptions` takes no force/overwrite flag, so re-running `ios init` against an existing `src-tauri/gen/apple/` writes nothing, and the values split into two classes. **Baked at init:** the iOS floor from `tauri.conf.json` → `bundle.iOS.minimumSystemVersion`, and the identity — `project.yml` carries `deploymentTarget.iOS: {{apple.ios-version}}`, `PRODUCT_BUNDLE_IDENTIFIER: {{app.identifier}}` and `PRODUCT_NAME: {{app.stylized-name}}`. Changing either therefore requires the generated project to be **recreated**, not init re-run in place: 0.8 deleted `src-tauri/gen/apple/` and initialised it again for the floor 0.7 measured — the **iOS** floor: `ios init` renders no macOS target, so the macOS floor is applied in `tauri.conf.json` instead (D0.16) — and for 0.5's identity, which the task-0.1 project held as `com.tauri.dev`/`Tauri App` until that recreation. **Synchronised at build time:** `synchronize_project_config` (`src/mobile/ios/mod.rs`, called from `ios/dev.rs` and `ios/build.rs`) writes `PRODUCT_BUNDLE_IDENTIFIER` from `app.identifier` and `PRODUCT_NAME` from `productName` into the generated `app.xcodeproj/project.pbxproj` on `tauri ios dev`/`tauri ios build`, so those two values take effect without regeneration. Nothing under `src-tauri/gen/apple/` is hand-edited in either class |
| the iOS target's `Info.plist` and entitlements files under `src-tauri/gen/apple/` | hand-owned once generated, but 0.8 made no edit: nothing in the scaffold calls a privacy-gated API or references an entitlement, so it invented no usage string and no entitlement (D0.16), and both files keep the content the generator wrote; no deployment target lives in these files |
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
switches on `time/local-offset` (`time` itself remains in the graph without that feature — `cookie`,
`plist` and `tauri-codegen`, the build graph; D0.8 records the attribution, the enabled features and
`num_threads`'s absence from the lockfile's package set).

**Placeholder identity observed**, i.e. the `--ci` defaults this task deliberately did not override:
`productName` `Tauri App`, identifier `com.tauri.dev`, window title `Tauri`, crate `app` with library
`app_lib`. Task 0.5 (D0.15) replaced three of them — `productName` and the window title are now
`BunScale`, the identifier `com.bunscale.app` — and the fourth is deliberately **kept as generated**,
not an unfulfilled placeholder: the crate stays `app` with library `app_lib`, because the user-visible
name comes from `productName` (and D0.15's display name), so the crate name is internal and renaming
it would churn `src/main.rs` and the generated iOS project for no user-visible gain; a rename is its
own task if it is ever wanted, not an obligation riding on 0.5. The frontend contract values landed as
passed — `frontendDist` `../dist`,
`devUrl` `http://localhost:1420`, `beforeDevCommand` `bun run dev`, `beforeBuildCommand`
`bun run build` — and are what 0.13/0.14 must match. Step 5 also produced the generator's
`src-tauri/Cargo.lock`. At the generator stage that lockfile held **408** `[[package]]` entries, each
resolved to one **exact** version — a lockfile records resolved versions, never ranges. 408 is the
post-removal figure: dropping the logging plugin (above) took **seven** entries out of the generator's
lockfile (`android_log-sys`, `android_logger`, `env_filter`, `fern`, `num_threads`, `tauri-plugin`,
`tauri-plugin-log`), so the pre-removal figure was **415**, not the 414 an earlier measurement of this
pass recorded. Task 0.10 has since materialised the exact pins and added the IPC crates, and `cargo`
locked **eight** packages to the newest Rust 1.90-compatible versions with **no** `Removing` and no
`Updating` lines — the four pre-existing pins were already the locked versions — so the **committed**
lockfile now holds **416** entries: the `specta` family (`specta`, `specta-macros`, `specta-serde`,
`specta-util`, `specta-typescript`) and `tauri-specta` at the `2.0.0-rc.25`/`0.0.12` line, plus the
`Inflector` and `paste` crates they pull in. The caret ranges it resolved from lived in the generated
`src-tauri/Cargo.toml`, the file D0.8 says 0.2/0.10 replaces with exact pins.

**Recorded refinements to the generated `tauri.conf.json` (task 0.5), beyond the identity.** Both are
refinements of generator output, not additions. `bundle.android.debugApplicationIdSuffix` (`.debug`)
was **removed** — it configures a fourth platform this phase does not ship, and removing it takes
nothing iOS or macOS needs, since a future Android task re-adds it under the generate-then-own model.
The macOS window geometry was brought from the generator's **800×600** to the design's **1280×800**
desktop baseline: the generator default no section-0 task owned.

**Second generator pass — task 0.8's recreation, run by the orchestrator.** The iOS target was
**recreated**, not initialised in place, because the values that had to change are baked at init (the
generator-owned table above and the write-once templates it records). That makes the deletion of
`src-tauri/gen/apple/` the **one destructive step in this change** — a precondition of the pass, not
an incidental cleanup — so it ran under the **product owner's explicit grant of that single deletion**.
That grant was **single-use and has since been reverted**: the deletion it permitted is done,
`src-tauri/gen/apple/` is generator output `cargo tauri ios init` re-creates on demand, nothing else in
the plan deletes generated output, and the product owner approved it as the means to run this one step
rather than as a standing capability. It has therefore been removed from the orchestrator's allow list
— no destructive permission remains — and a future regeneration of that directory would need the grant
again.

`architect`'s authoring session had no shell (the harness refuses `rm -rf` in every session), so
**the orchestrator ran the pass**; the transcript below is its output and the values after it are
read-only observations of the regenerated tree. It ran on 2026-10-16 through the `cargo tauri` CLI
installed in step 1:

```
$ rm -rf src-tauri/gen/apple
$ cargo tauri ios init
/opt/homebrew/bin/xcodegen
/opt/homebrew/bin/idevicesyslog
/opt/homebrew/bin/pod
Apple dependencies are up to date
        Info "/Users/supporterino/github/BunScale/src-tauri" relative to "/Users/supporterino/github/BunScale/src-tauri/gen/apple" is "../../"
Generating Xcode project...
⚙️  Generating plists...
⚙️  Generating project...
⚙️  Writing project...
Created project at /Users/supporterino/github/BunScale/src-tauri/gen/apple/app.xcodeproj
victory: Project generated successfully!
```

**The regenerated project carries the settled values.** The placeholders the task-0.1 project held are
gone from the whole directory — `grep -r "com.tauri.dev\|Tauri App" src-tauri/gen/apple` returns **no
matches** — and the identity the recreation was run for is present in the generated **files**, not
left to build-time substitution:

- `src-tauri/gen/apple/project.yml:3` `bundleIdPrefix: com.bunscale.app`; `:4–5` `deploymentTarget:
  iOS: 15.0`; `:13` `PRODUCT_NAME: BunScale`; `:14` `PRODUCT_BUNDLE_IDENTIFIER: com.bunscale.app`.
- `src-tauri/gen/apple/app.xcodeproj/project.pbxproj:288`, `:375` `PRODUCT_BUNDLE_IDENTIFIER =
  com.bunscale.app;`; `:289`, `:376` `PRODUCT_NAME = BunScale;`; `:341`, `:434`
  `IPHONEOS_DEPLOYMENT_TARGET = 15.0;`. The generic `PRODUCT_NAME = "$(TARGET_NAME)";` entries
  elsewhere in that file are unrelated target-template lines, not the app target's name.
- `src-tauri/gen/apple/Podfile` is **unchanged** by the pass: `platform :ios, '15.0'` and `platform
  :osx, '11.0'` — the permanent cargo-mobile2 residue D0.16 records, re-emitted exactly as the split
  above predicts.
- One more change beyond the identity values and the floor: the regenerated `project.pbxproj` also
  gained a `PBXFileReference` + `PBXGroup` pair for `src-tauri/src/ipc/mod.rs` (`:36`, `:110`,
  `:138–145`), because `gen/apple/project.yml:6`'s `fileGroups: [../../src]` enumerates
  `src-tauri/src/` **at init time** and task 0.4's `ipc/` module did not exist when task 0.1 generated
  the project — which is harmless, since that group is not in `PBXSourcesBuildPhase`, a phase that
  lists only `main.mm` (`:253–262`), and which is the evidence that the generated project's file
  listing tracks the source tree at init time, so any later `src/` reorganisation needs another
  recreation rather than a hand edit.

**Gate status on this exact tree (measured by the orchestrator after the recreation, 2026-10-16).**
`cargo check --manifest-path src-tauri/Cargo.toml` **passes**; the same with
`--target aarch64-apple-ios` **passes**; `cargo clippy --manifest-path src-tauri/Cargo.toml
--all-targets -- -D warnings` **passes**; `cargo test --manifest-path src-tauri/Cargo.toml` **passes**
(0 tests at that point — no Rust source changed in this task, so task 0.6's clippy run still holds).
`cargo fmt --check` **was red** on the generator's own style and was task 0.12's deliverable, per the
section-0 preamble; task 0.12 has since reindented the two generator-styled files and made it **green**
(its outcome record is in D0.9, above). The iOS side of the floor is provable from the committed tree itself —
`IPHONEOS_DEPLOYMENT_TARGET = 15.0` in the generated project (above) plus the
`IPHONEOS_DEPLOYMENT_TARGET` compiler env var `tauri-build` emits in the build output — whereas the
macOS floor can only be seen in a **built** artefact and stays task 0.26's check (below).

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
is the single source of truth in `tauri.conf.json` (task 0.5) and reaches the generated iOS target
through task 0.8's recreation of it (`PRODUCT_BUNDLE_IDENTIFIER`/`PRODUCT_NAME`, D0.11) — not through
any usage string, since nothing in the scaffold calls a privacy-gated API (D0.16). Neither task is
blocked: there is no open question left on identity, and
`release-signing` can treat the identifier as fixed from section 0 on.

**The pinned CLI warns about the identifier, and the warning is accepted.** Every build path that
calls `build::setup` (`tauri build`, `tauri ios build`, `tauri android build`) logs a `log::warn!` in
the pinned `tauri-cli` 2.12.3 (`src/build.rs`): *"The bundle identifier `com.bunscale.app` set in
`identifier` ends with `.app`. This is not recommended because it conflicts with the application
bundle extension on macOS."* It is recorded rather than acted on: nothing fails, the value has been
fixed from section 0 on, and re-settling it now would invalidate the App Store identity. The
alternative — choosing a different identifier purely to silence the warning — was rejected for
exactly that cost.

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

##### The measured floor (task 0.7, `apple-native`)

| Platform | SDK | Key read from its `SDKSettings.plist` | Floor |
|---|---|---|---|
| iOS | `iPhoneOS27.1.sdk` (`iphoneos27.1`) | `SupportedTargets.iphoneos.MinimumDeploymentTarget` | **15.0** |
| iOS simulator | `iPhoneSimulator27.1.sdk` (`iphonesimulator27.1`) | `SupportedTargets.iphonesimulator.MinimumDeploymentTarget` | **15.0** — the same value, so there is no lower simulator floor |
| macOS | `MacOSX27.0.sdk` (`macosx27.0`) | `SupportedTargets.macosx.MinimumDeploymentTarget` | **12.0** |
| macOS, **excluded** | `/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk` | `SupportedTargets.macosx.MinimumDeploymentTarget` | 10.13 — **not this floor** |

**Why 10.13 is excluded, recorded rather than dropped.** That CommandLineTools SDK is not the toolchain a build uses: `xcodebuild -showsdks` does not enumerate it, `xcrun --sdk macosx --show-sdk-path` does not resolve it, and it is reachable only by passing an explicit `-sdk <path>`. The floor a build can target is bounded by the SDK the toolchain builds with, which is why it is derived rather than chosen; the alternative value is stated here so the choice is visible. Both in-scope SDKs carry `DefaultDeploymentTarget` `27.0` and `RecommendedDeploymentTarget` `17.0` (iOS) / `14.0` (macOS), and their `ValidDeploymentTargets` arrays start at `15.0` and `12.0` respectively. Those three keys are **not** in the output of the four `xcrun plutil` extraction commands below, which read `MinimumDeploymentTarget` only; they are quoted from each SDK's plain-text **`SDKSettings.json` twin**, the path of which is named with that command block. Exactly one SDK version is installed per platform **on the active toolchain** — the one `xcodebuild -showsdks` enumerates and `xcrun --sdk <sdk> --show-sdk-path` resolves — so within the toolchain that will build this app there is no competing version to weigh. Other installations on the machine carry more, and the exclusion applies **only** to those: `/Library/Developer/CommandLineTools/SDKs/` holds three SDK versions — `MacOSX12.0.sdk`, `MacOSX26.5.sdk` and `MacOSX27.0.sdk` (plus aliases) — and each of those is excluded on the same ground recorded above, `MacOSX26.5.sdk` with the floor `10.13` the table above quotes and `MacOSX12.0.sdk` declaring **no floor at all** (a real directory carrying no `SDKSettings.plist`, holding only `System/` and `usr/`, so it has no `SupportedTargets.macosx.MinimumDeploymentTarget` to read). Xcode's own platform directory is **not** among the excluded installations: it holds **one** SDK version, 27.0, under three names — `MacOSX.sdk` is the one real directory there, and `MacOSX27.0.sdk` and `MacOSX27.sdk` are its alias entries, which is why `xcodebuild -showsdks` prints `macosx27.0` twice — and that is the very `MacOSX27.0.sdk` the macOS row of the table above cites, so it is the toolchain SDK whose `12.0` floor is the measured one, not a competing version to exclude. **Reasoning, not a measured value:** the generated project is arm64-only (`ARCHS: [arm64]`), and Apple silicon Macs begin at macOS 11.0, so the excluded 10.13 SDK could never have been this app's floor even on a maximalist reading — that is an inference from the arch pin plus Apple's own hardware floor, not the output of any command run here.

**The generated project versus this floor — the gap task 0.8 closes.** Measured read-only on the committed tree: `gen/apple/project.yml:5` already carries `deploymentTarget.iOS: 15.0`, matching the floor, and `app.xcodeproj/project.pbxproj` carries `IPHONEOS_DEPLOYMENT_TARGET = 15.0` twice and **no** `MACOSX_DEPLOYMENT_TARGET` anywhere. But `project.yml` has **no** `deploymentTarget.macOS` and defines only an `app_iOS` target, and `gen/apple/Podfile:9` still carries `platform :osx, '11.0'` for an `app_macOS` Pods target that `project.yml` no longer defines — **permanent** template residue, not incidental: `cargo-mobile2` hard-codes `11.0` as its default macOS version and the Tauri CLI passes only the iOS version through, so **every** recreation re-emits that line. It is below the measured 12.0 floor and absent from the SDK's `ValidDeploymentTargets`; it is therefore **not** evidence that the floor was applied, and it must not be hand-edited (D0.11). So the iOS floor is **already satisfied** by the generated project, while **the macOS floor currently has no home in `gen/apple/`**: recreating the iOS target cannot apply it and writes no `MACOSX_DEPLOYMENT_TARGET`.

**Where the macOS floor actually belongs (verified from the pinned sources).** `tauri.conf.json` is the lever, and both platforms have a key there (read from `tauri-utils` **2.10.2**, the version `tauri` 2.12.4 and `tauri-build` 2.7.3 resolve to in the committed `Cargo.lock`):

| Key | Source read | Type and default |
|---|---|---|
| `bundle.iOS.minimumSystemVersion` | `tauri-utils-2.10.2/src/config.rs:3619–3626` (`IosConfig::minimum_system_version`; field `#[serde(rename = "iOS", alias = "ios")] pub ios` at `:1704`) | `String`, default `"15.0"` (`:698–700`), alias `minimum-system-version` |
| `bundle.macOS.minimumSystemVersion` | `tauri-utils-2.10.2/src/config.rs:636–649` (`MacConfig::minimum_system_version`; field `#[serde(rename = "macOS", alias = "macos")] pub macos` at `:1701`) | `Option<String>`, default `10.13` (`:694–696`), alias `minimum-system-version`; documented as the bundle's `LSMinimumSystemVersion` plus the `MACOSX_DEPLOYMENT_TARGET` env var, and **ignored in `tauri dev`** |

The macOS key therefore exists, and the Rust side needs nothing manual: the pinned `tauri-build` 2.7.3 emits both env vars from that one config — `cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET` from `bundle.macOS.minimumSystemVersion` (only when not dev) and `cargo:rustc-env=IPHONEOS_DEPLOYMENT_TARGET` from `bundle.iOS.minimumSystemVersion` for `*ios*` targets (`tauri-build-2.7.3/src/lib.rs:779–791`), and the macOS bundler writes `LSMinimumSystemVersion` from the same key (`tauri-bundler-2.10.3/src/bundle/macos/app.rs:260` — the bundler the installed `tauri-cli` 2.12.3 pins at `=2.10.3`, not a crate this app's `Cargo.lock` contains). Setting the config key is sufficient, and **task 0.8 applied the D0.16 floor in `tauri.conf.json`:** `bundle.iOS.minimumSystemVersion` `"15.0"` and `bundle.macOS.minimumSystemVersion` `"12.0"`, set explicitly so the floor is a recorded decision rather than an inherited crate default (the config it replaced carried only `active`, `targets`, `icon`, i.e. macOS took Tauri's own `10.13` default — **below the measured 12.0 floor** — while iOS inherited a `15.0` default that happened to equal the floor). This *tightens* macOS from `10.13` to the measured `12.0` (no widening, so no narrowing was rejected); its only user-visible surface, the bundled `LSMinimumSystemVersion`, is task 0.26's check (below). No `src-tauri/Info.plist` and no entitlements file were created: nothing in the scaffold calls a privacy-gated API or references an entitlement, so an invented usage string or sandbox set would be an unverifiable claim (D0.21). The macOS App Sandbox set, by contrast, is owed by **the change that first needs a signed or distributed macOS build**, authored by `tauri-security` (AGENTS.md routes entitlements there) and validated in that release flow — not carried as a standing obligation on a sibling role.

**The simulator (task 0.27).** The simulator SDK's floor equals the device floor (both `15.0`), and the generated project pins `ARCHS: [arm64]` (`project.yml:70` — the durable anchor — and `app.xcodeproj/project.pbxproj:269/356` on the regenerated file, where the pair sat at `:259/346` before the recreation) with `EXCLUDED_ARCHS[sdk=iphoneos*]: x86_64`, which excludes the Intel slice for the **device** SDK only; an arm64 simulator build is therefore the expected shape, and the equal floor asks for no second simulator target.

**D0.16 resolution — no Tauri-imposed minimum above the floor:** no crate in the pinned set raises it. `tauri-build` 2.7.3 emits `MACOSX_DEPLOYMENT_TARGET`/`IPHONEOS_DEPLOYMENT_TARGET` verbatim from the config keys (`src/lib.rs:779-791`); every fallback it or its siblings could reach is at or below the floor — `tauri-utils` 2.10.2 defaults `10.13` (macOS) and `15.0` (iOS) (`src/config.rs:694-696`, `698-700`) and the SwiftLinker fallback is `13.0` (`src/build.rs:29`); `tauri-bundler` 2.10.3 passes the config value straight into `LSMinimumSystemVersion` (`src/bundle/macos/app.rs:260`). `tauri` 2.12.4 / `tauri-build` 2.7.3 / `tauri-utils` 2.10.2 are the versions the committed `Cargo.lock` resolves; `tauri-bundler` 2.10.3 is **not** lock-resolved — that lock carries no `tauri-bundler` and no `tauri-cli` entry at all — and comes instead from the installed CLI's own pin, `tauri-cli` 2.12.3 declaring `tauri-bundler = "=2.10.3"` in its `Cargo.toml`. The distinction is load-bearing: the bundler that writes `LSMinimumSystemVersion` is the **CLI's** bundler, not anything in the app's lock. The only hardcoded Apple-side default found is cargo-mobile2's macOS Podfile `11.0`, already recorded and generator-owned.

**The macOS floor's one user-visible surface, and an obligation on task 0.26.** The bundled `Info.plist`'s `LSMinimumSystemVersion` — written by the macOS bundler from `bundle.macOS.minimumSystemVersion` (`tauri-bundler-2.10.3/src/bundle/macos/app.rs:260`) — is the one place the macOS floor becomes **user-visible**; nothing else in the plan verifies it. Task 0.8 set the config key and the gate exercises the config, but only a **built** artefact shows the value that ships, so task 0.26's built-artefact check MUST load the built `.app` and confirm its bundled `Info.plist` carries the measured floor (`12.0`) — the config key and the `MACOSX_DEPLOYMENT_TARGET` env var do not by themselves prove the bundled value.

**Provenance.** Every value above comes from the installed SDKs' own `SDKSettings.plist` metadata, read through the toolchain on macOS 27.0.1 / arm64 / Xcode 27.1 (build 27A9275). `xcodebuild -showsdks` enumerates the same active toolchain (and no CommandLineTools SDK), and `xcrun --sdk <sdk> --show-sdk-path` resolves the two in-scope SDK paths in the table. The floor values were first read from each SDK's `SDKSettings.json` twin and then **re-derived from the binary `.plist` files by the orchestrator** through the sanctioned `xcrun plutil` invocation — bare `plutil` is outside `apple-native`'s shell allow list — which is why the numbers below are the `.plist` sources' own output. The exact commands and their observed output:

```
$ xcrun plutil -extract SupportedTargets.iphoneos.MinimumDeploymentTarget raw -o - /Applications/Xcode.app/Contents/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS27.1.sdk/SDKSettings.plist
15.0
$ xcrun plutil -extract SupportedTargets.iphonesimulator.MinimumDeploymentTarget raw -o - /Applications/Xcode.app/Contents/Developer/Platforms/iPhoneSimulator.platform/Developer/SDKs/iPhoneSimulator27.1.sdk/SDKSettings.plist
15.0
$ xcrun plutil -extract SupportedTargets.macosx.MinimumDeploymentTarget raw -o - /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX27.0.sdk/SDKSettings.plist
12.0
$ xcrun plutil -extract SupportedTargets.macosx.MinimumDeploymentTarget raw -o - /Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk/SDKSettings.plist
10.13
```

Those four invocations read `MinimumDeploymentTarget` and nothing else. The `DefaultDeploymentTarget`, `RecommendedDeploymentTarget` and `ValidDeploymentTargets` values quoted above are carried by each SDK's plain-text `SDKSettings.json` twin, beside the `.plist` read here: `/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX27.0.sdk/SDKSettings.json` and `/Applications/Xcode.app/Contents/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS27.1.sdk/SDKSettings.json`.

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

### D0.21 — Task 0.6: an empty permission set and a strict CSP (Q13)

**The empty permission set is the deliberate minimum, not an oversight — and this change is a
narrowing only: no permission and no CSP source was widened.** Task 0.6 narrowed the generated
`capabilities/default.json` from `permissions: ["core:default"]` to `permissions: []`. Section 0 ships
no Tauri plugin (D0.8), no command and no event (D0.9), so nothing invokes a core or a plugin command:
`core:default` bundles nine permission sets (menu, tray, window, webview, event, app, path, image,
resources) and none of them has a consumer. The load-bearing fact is that **the app's own commands are
ACL-exempt** — `tauri`'s deny path runs only for plugin commands, for an app that defines its own ACL
manifest, or for non-local origins (`tauri-2.12.4/src/webview/mod.rs`,
`tauri-2.12.4/src/ipc/authority.rs`), and this scaffold defines no app manifest — so an empty set
cannot block the P0 commands task 1.3 adds. **That exemption rests on a condition a later task can
break:** `has_app_manifest` is true only if `src-tauri/permissions/**` provides a manifest or
`Attributes::app_manifest` sets one, and this crate has neither (`build.rs` is a bare
`tauri_build::build()`). Introducing `src-tauri/permissions/**` therefore **flips every custom command
to ACL-gated**, so any task that adds one must revisit the empty set **in the same change**. That is
what makes `[]` the honest posture now rather than a guess, and it is why a later task must **not**
"restore" `core:default` for safety. Nothing was widened, so rule 4 asks for no one-line justification
of this decision; the deferrals below are what remains.

**The capability is platform-scoped now, and stays the only one.** `"platforms": ["iOS","macOS"]`
makes the capability inactive on any other target (`Capability::is_active`) and skipped by validation,
so it cannot apply to a platform this phase does not ship; the consequence is stated positively and
handed on: **a future shippable platform must add itself to this array in the change that ships it, or
it gets no capability at all**, because `is_active(&target)` filters the file out rather than failing
loudly. iOS and macOS are in step here: the CSP response header reaches WKWebView on both, so neither
platform needs a policy of its own. `windows: ["main"]` is correct on both platforms because the single
configured window carries no explicit label and `main` is Tauri's default. `identifier: "default"` is
unchanged, no second capability file was created (Q13, D0.11), and the description now reads "The
app's only window. Grants only the permissions its tasks need." — deliberately **de-timed**, because
the previous wording asserted a fact that expires at task 1.3, and this file is hand-owned and copied
verbatim into the generated capability manifest. The repointed `$schema` — the iOS schema instead of
the desktop one — is **not a platform decision**: both generated schemas carry the same core permission
identifiers (spot-checked: `core:menu:default` and `core:window:allow-set-skip-taskbar` appear in
both), so the iOS pick is editor-only and equivalent to the macOS one.

**The CSP is strict, and its widenings carry the rule-4 justification.** `app.security.csp` goes
from `null` to `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self';
font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none';
frame-ancestors 'none'; form-action 'none'` — no `unsafe-eval` and no `'unsafe-inline'` in any
directive. The policy is applied through the `tauri://` asset protocol (`AppManager::get_asset`), and
Tauri appends `'self'` plus per-asset nonces/hashes **per response when the asset is served**, with the
nonce token and the inline-script hashes produced at build time by `tauri-codegen`: that is why inline
scripts in the built HTML keep working with no `'unsafe-inline'` in `script-src`, and why an
`'unsafe-inline'` in `style-src` would be *defeated* by Tauri's nonce in the same directive rather than
helping. Two source entries — `ipc:` and `http://ipc.localhost` — are therefore the whole record of what
this policy permits beyond `'self'`, and the second bullet below is a decision, not a widening:

- **`connect-src` is `ipc: http://ipc.localhost` and nothing else.** These are the endpoints of
  Tauri's IPC fetch transport, and that transport is the **primary** one on both shipped platforms, not
  a desktop-only path: `tauri-2.12.4/scripts/ipc-protocol.js` takes the fetch branch first
  (`canUseCustomProtocol = osName !== 'android'`, then `fetch(convertFileSrc(cmd, 'ipc'))`), so on iOS
  **and** macOS the custom-protocol fetch carries IPC and `connect-src` governs it; `postMessage` is
  only the fallback taken when that fetch *rejects* ("IPC custom protocol failed, Tauri will now use
  the postMessage interface instead"). The narrower option — omitting them — would therefore silently
  **downgrade** IPC to the fallback rather than expose anything new, which makes this entry
  load-bearing rather than belt-and-braces. `'self'` was **removed** from the directive as a
  narrowing: the runtime issues no same-origin request at all (`ipc-protocol.js` is the only `fetch`
  in `tauri-2.12.4/scripts/`, and there is no `WebSocket`, `EventSource` or `XMLHttpRequest`), so
  `'self'` granted nothing while widening the directive — and it could never have been what kept IPC
  alive anyway, because the IPC fetch targets `ipc://localhost/{cmd}`, a different scheme and host from
  the document origin, while Tauri injects `'self'` and nonces only into `script-src`/`style-src`
  (`manager/mod.rs` `replace_csp_nonce`) and never into `connect-src`. Recorded consequence: with
  `'self'` gone from the directive, `connect-src` no longer falls back to `default-src 'self'`, so a
  later same-origin `fetch`/`XHR`/`WebSocket` issued by the document is blocked and needs `'self'` added
  back to `connect-src` in the change that introduces it. No external origin, no scheme-wide wildcard
  and no `data:` source is allowed anywhere in the policy.
- **No `devCsp` is set.** On iOS (dev proxy) and macOS (direct `devUrl` load) Tauri 2.12.4 does not
  attach the configured CSP to dev content at all: in dev `csp()` falls back to the configured `csp`
  (`dev_csp.clone().or_else(|| csp.clone())`), but the dev path never reaches `get_asset` — macOS dev
  loads `build.devUrl` directly and the iOS dev path proxies to the dev server without adding the
  header — so the key would be inert today; the dev policy must be authored by the task that creates
  the Vite config, as `devCsp` only, never by weakening `csp`.

**Neither the gate nor dev exercises this policy.** Because Tauri attaches no CSP in dev and no gate
step runs `tauri build`, the policy's strictness must **not** be assumed safe from a green gate: the
evidence is a **built** macOS artefact loaded and checked, and task 0.26 now carries that step. The
following spellings are consequently load-bearing constraints on whoever edits the CSP next:

- Keep `script-src` present **and** carrying `'self'`: wry rewrites that directive by naive string
  replacement, so a missing directive behaves differently from a permissive one.
- Do **not** introduce `script-src-attr` or `script-src-elem`: the same `.replace("script-src", …)`
  corrupts them.
- Leave `dangerousDisableAssetCspModification` at its default, or Tauri's nonce/hash injection stops.

**Obligations this posture hands on**, each where the current strictness is next tested:

- **0.13/0.19:** dev-only needs — the Vite HMR websocket and the React-fast-refresh inline preamble —
  go in `devCsp`, never in `csp`. Recorded consequence: if a later task sets a `devCsp`, Fast Refresh's
  inline preamble needs an explicit allowance **there**, not a relaxation of `csp`.
- **0.19:** the first inline `style` attribute from the UI layer is fixed with
  `style-src-attr 'unsafe-inline'` (the narrowest form, plus its own rule-4 line), **never** by putting
  `'unsafe-inline'` into `style-src`, where Tauri's nonce in that directive defeats it.
- **0.19:** the first sub-4 kB asset Vite inlines as a `data:` URI is a real widening and must be
  justified then, in `img-src`/`font-src`.
- **0.19:** self-hosted fonts, or any remote image origin, is a new source introduced by that change and
  must be justified there.
- **0.16 / asset protocol:** any task that enables `app.security.assetProtocol` or uses
  `convertFileSrc` — the change that first serves an image — must add `asset:` and
  `http://asset.localhost` to `img-src` (and `media-src` if it is used) in the same change.
- **1.4:** the first commands widen this capability for whatever core or plugin APIs they actually
  use; app commands need **no** entry (1.3 declares the core APIs the contract uses; 1.4 grants them).
- **1.4 (owner) / 3.6 (first consumer):** the narrowest permissions the P0 event subscriptions require
  are **`core:event:allow-listen`** and **`core:event:allow-unlisten`** (for the returned unlistener);
  `core:event:default` is the set containing them, and it additionally grants `emit`/`emit-to`, so 1.4
  grants the smallest thing that satisfies the contract. `plugin:event|*` is always ACL-checked, unlike
  the app's own commands, and 3.6 is where `household:changed` and `service:status-changed` are first
  subscribed to. Task 1.4 is the named owner of that entry in `tasks.md`. The same obligation covers
  every other core module the frontend later imports: `path`, `window` and `app` are equally denied
  today.
- **1.4:** the shared service's origin is the read model's network surface (R5) and is owed a
  `connect-src` entry there, with its own rule-4 line.
- **the change that first needs a signed or distributed macOS build:** the macOS App Sandbox
  entitlements set is owed there — authored by `tauri-security`, since AGENTS.md routes entitlements to
  that agent, and validated in that release flow — not carried as a standing obligation on a sibling
  role. There is no such task yet: `tasks.md` contains no sandbox task, and `release-ci`'s only task
  (0.9) explicitly disclaims signing. The iOS half of the same set already has an owner in the form the
  decision takes: `src-tauri/Info.plist` deliberately does not exist yet, and its usage strings are
  owed by whichever task first calls a privacy-gated API. Both halves are release-time artefacts, not
  scaffold ones — no entitlements file exists under `src-tauri/` and none is referenced, and inventing
  a usage string would ship a permission granted for nothing. Task 0.8 added no iOS usage string, for
  exactly that reason (D0.16).

No ADR number is taken: section 0 takes none (D0.8).

### D0.22 — Task 0.9: the CI gate is the gate, and nothing else

**The workflow runs the eight `definition-of-done` commands in their recorded order on
`macos-latest`, as eight separate steps named after the commands themselves**, plus the two drift
checks as steps of their own against a single path: `bun run gen:bindings` then
`git diff --exit-code src/bindings.ts`, and `bun run build` then
`git diff --exit-code src/routeTree.gen.ts` — the "regenerate, then fail on any diff" shape D0.9
requires, with the failure naming the file TOOL-3/TOOL-4 point at. Counting `bun run build` as the
route-tree check's own prerequisite step, those are **eleven** gate steps, and `bun run test:e2e` is
the twelfth and last, after `bunx playwright install webkit` (`--with-deps` is a no-op on macOS, so
it is not used).

**The workflow holds no secret and requests nothing beyond `permissions: contents: read`:** the gate
is headless and the E2E IPC layer is mocked; nothing signs, notarises, uploads or distributes
(TOOL-7). One job, one runner, so "no platform beyond iOS and macOS" is structural — no
Android/Windows/Linux job exists.

**Third-party actions are pinned to commit SHAs with the tag in a trailing comment:**
`actions/checkout` `3d3c42e5aac5ba805825da76410c181273ba90b1` (v7.0.1), `actions/cache`
`55cc8345863c7cc4c66a329aec7e433d2d1c52a9` (v6.1.0), `Swatinem/rust-cache`
`6323deb102c322ba6fcbdcafc7e3dddab59af2b6` (v2.9.2 — the commit its annotated tag points at),
`dtolnay/rust-toolchain` `683b588c487a69d01f6c6ddf40f036ff682cc2c8` (v1), `oven-sh/setup-bun`
`0c5077e51419868618aeaa5fe8019c62421857d6` (v2.2.0). The toolchain action's spec is passed
**explicitly** (`toolchain: stable`, `targets: aarch64-apple-ios`, `components: rustfmt, clippy`)
because that action otherwise takes its toolchain from its own tag; the rust-cache step follows it
because its key includes the rustc version. Caches cover the three required locations: rust-cache
owns `~/.cargo` and `src-tauri/target` (`workspaces: src-tauri -> target`, the
`$workspace -> $target` form a manifest outside the repository root requires), and `actions/cache`
owns `~/.bun/install/cache` keyed on the lockfile — deliberately on both `bun.lock`, which bun
1.4.x writes, and the legacy-but-still-readable `bun.lockb` — and that third location is now real
coverage rather than a constant: task 0.13 wrote `bun.lock` (text, `lockfileVersion: 2`; no
`bun.lockb` exists), so `hashFiles('bun.lock', 'bun.lockb')` resolves to that file's hash instead of
`''`, and the `restore-keys` prefix no longer matches the key exactly. This was the one coverage
claim that could not be verified before 0.13, and the reason is worth keeping on record, because it
is also why no step here can fail on it: `hashFiles` on a missing file is **not** an error — it
evaluates to `''` and the step still succeeds — so what ended was the key's constancy, never a
failure. `bun install --frozen-lockfile` honours the lockfile the exact pins resolve to, and bun
is the pinned runtime 1.4.3. No global Tauri CLI: the workflow asserts `bun run tauri --version`,
i.e. task 0.13's exactly pinned `@tauri-apps/cli` dev dependency (SETUP.md §5), while task 0.1's
`cargo install tauri-cli` stays a one-off local bootstrap. That assertion does not distinguish the
pinned dev dependency from a globally installed CLI — a script delegating to a global `cargo tauri`
would print the same version, and no gate step uses the CLI for anything — so the step matches
`SETUP.md` §5 and 0.13's promised `tauri` script without proving the pin: "the workflow uses the
pinned dev dependency" rests on `bun install --frozen-lockfile` plus the exact pin, not on this
step.

**It is the gate and deliberately not the app build:** no `tauri ios build` and no `tauri build`
here. Consequence recorded rather than hidden — **the gate never exercises the CSP and never builds
a non-dev artefact, so it never shows a shipped artefact's floor value**; its iOS step is the one
place it does compile against a floor (below). That is exactly why task 0.26's non-dev
`bun run tauri build` on macOS, its `LSMinimumSystemVersion` check and the built-artefact load are
the only evidence for those (D0.16, D0.21).

**Runner skew, recorded because the floor is derived (D0.16):** `macos-latest` is today the macOS 26
arm64 image whose default Xcode is 26.6 (17F113), while the floor was measured on a local Xcode 27.1
(iOS SDK 27.1 → 15.0, macOS SDK 27.0 → 12.0). The skew cannot break this workflow's macOS half —
every step there is a *dev* build and `tauri-build` emits `MACOSX_DEPLOYMENT_TARGET` only when
`!is_dev()` (`tauri-build-2.7.3/src/lib.rs:779-783`), while `cargo check` neither links nor bundles
— but it does reach the iOS compile check: the same `tauri-build` 2.7.3 emits
`IPHONEOS_DEPLOYMENT_TARGET` **unconditionally** for `*ios*` targets (`:786-791`), so
`cargo check --target aarch64-apple-ios` compiles the app crate **at the 15.0 floor** and an
above-floor API use would fail this workflow. What no step here can show is the floor as *shipped*:
the value reaches iOS through the generated project (`tasks.md` 0.10) and the gate compiles the app
crate against it, but no gate step builds a distribution-bound iOS artefact — locally that is task
0.27's simulator `dev` run, and in CI the future release change's `tauri ios build` (below). The
runner image's own SDK floors are **not** verified from this repository, and no step selects a
specific Xcode, because naming one the image does not carry would replace an unverifiable risk with
a certain failure. One cost line: GitHub allows at most 5 concurrent macOS jobs and bills macOS
minutes at a higher rate on private repositories, hence the concurrency group, the job timeout, and
no macOS job this phase does not need.

**The workflow is red until the frontend exists, and that is expected.** It could not pass before
task 0.12 made `cargo fmt --check` clean (that half is done — see the 0.12 outcome record in D0.9), and
it cannot pass until 0.13 adds `package.json` and its scripts, 0.14–0.15 add the
build and the route tree, 0.16 commits `src/bindings.ts`, 0.17–0.18 wire the tests and Biome, and
0.19 **extends** the root `.gitignore` — it already exists, holding the `graphify-out/cache` entry,
so 0.19 adds `src-tauri/target/`, `test-results/`, `playwright-report/` and `.artifacts/`, plus
`.tanstack/tmp/` beyond its literal list (the pinned router generator creates that directory lazily),
and commits the `node_modules` and `dist` lines the environment had already added, rather than
creating it. So this file is the evidence for TOOL-7, and task 0.26's recorded full-gate run is the
evidence for TOOL-1.

**The skill/plan tension is resolved explicitly: this change's task text governs.** `release-ci`'s
own procedure and the `release-signing` skill describe phase-1 CI as the gate **plus** a
`tauri ios build` debug simulator build and a `tauri build` macOS debug build, while task 0.9 says
"exactly the `definition-of-done` gate" and TOOL-7 forbids CI to sign or distribute. The gate only,
because a device `tauri ios build` needs signing credentials the workflow must not hold, and
`tauri build` is a non-dev build that would import the unverified runner-SDK floor risk recorded
above into CI. Those two builds are owed by a **future release change** — the one that adds
signing/TestFlight and the macOS distribution ADR — not by this change: no task here creates a
release workflow, and none exists in the repository, so the `release-signing` skill's CI bullet is
left with a named owner instead of being silently unowned. For the built-artefact evidence of this
phase: task 0.26's macOS build plus task 0.27's boot — noting that task 0.27's iOS `dev` run is what
actually builds the generated Xcode project for the simulator, so TOOL-2's "the iOS target boots" is
not left to `cargo check` alone. The `release-signing` skill is not edited here: it is not this
agent's file.

No ADR number is taken: section 0 takes none (D0.8).

### D0.23 — Task 0.15: the query cache is a derived cache, and its options are chosen

**The app's single query cache is a derived cache and never a record of truth (D0.2), and the two
non-default options it carries are decisions rather than inherited defaults.** `src/lib/query-client.ts`
(task 0.15) is the one place the cache is created, and it sets `retry: false` on queries **and** on
mutations, sets `refetchOnWindowFocus: false`, and deliberately sets **no `staleTime`** —
`@tanstack/react-query`'s own defaults are three automatic retries and a refetch on window focus, so
`retry` and `refetchOnWindowFocus` each change behaviour, and every query in the app inherits them.
They are app-wide policy, and their consumers are named here so no later task has to rediscover them:
**task 3.4** (the service-status store), **task 3.6** (the event subscription and its invalidation) and
**P6, `add-read-path-performance`** (the freshness window and the two-request cold-load bound).

**`retry: false`, because the attempt policy and its bounds are owned by the Rust side (D0.13,
D0.12).** The number of attempts and the timeouts a save or a probe may consume are fixed in Rust
(`SAVE_ATTEMPT_TIMEOUT`, `REACHABILITY_PROBE_TIMEOUT`, D0.13), and a failure is required to reach the
person with its specific cause and an explicit retry the person invokes — a re-invocation (NFR-8,
D0.6), never an automatic one. A cache-level retry would stack a second, invisible attempt policy on
top of those bounds and would hide the cause the failure surface exists to show, which is why the
option is off on both queries and mutations. `mutations.retry: false` merely restates the library
default and changes nothing; it is written out so that the whole attempt policy reads in one place.
That task 0.23's test harness also wants `retry: false` — for deterministic tests — is a separate
reason, not this one.

**`refetchOnWindowFocus: false`, because freshness is owned by the locally kept copy and the
Rust-driven refresh (D0.2, SYNC-3).** The copy is refreshed by the explicit refresh on start and on
foreground return, driven by the Rust events task 3.6 subscribes to, and invalidated by task 3.2's
load and task 3.6's event. A focus refetch would be a second, invisible refresh policy running beside
the one the Rust layer owns — and it is exactly the second policy P6 would then have to reason about
when it fixes the cold-load bound and the freshness window (D6.1, D6.2). Where the
`frontend-conventions` skill asks to "refetch on foreground", this decision does not contradict it:
the **foreground refresh** it means is the Rust-driven one task 3.6 wires, not a browser-focus
refetch, and the option is disabled so that the two cannot diverge.

**No `staleTime`, because staleness is P6's subject.** Reads resolve against the in-process local copy
(D0.2), so revalidating on mount is cheap, and a refresh is driven by invalidation once a load or a
change event has landed (task 3.2, task 3.6). Leaving the value **unset** is the honest posture rather
than an invented one: choosing a staleness policy presupposes the freshness window P6 fixes, and a
value chosen here would be a second place that window lives.

No ADR number is taken: section 0 takes none (D0.8).

### D0.24 — Task 0.17: the test layer has its own config, and an empty suite stays red

**The test layer is its own config, and it deliberately does not reuse the app's Vite config.**
`vitest.config.ts` is a standalone `defineConfig` from `vitest/config`, not a `mergeConfig` of
`vite.config.ts`: Vitest resolves `vitest.config.*` **before** `vite.config.*`, so the file *replaces*
the app's config for the test pipeline rather than extending it. What that rules out is the one plugin
that would change behaviour — the app's config carries the TanStack **router plugin**, which renders
`src/routeTree.gen.ts` as a transform side effect, and that file is a tracked generated artefact whose
drift the gate checks (TOOL-4, D0.9), so **a test run must never rewrite it**; tests exercise the
committed route tree through `src/router.tsx` instead. Nothing else is lost: the rest of
`vite.config.ts` (`server.port`, `TAURI_DEV_HOST`, `clearScreen`, `build.target`) is dev-server concern
with no meaning to tests, and `@vitejs/plugin-react` is not needed because Vitest compiles `.tsx` with
esbuild via `tsconfig.json`'s `jsx: "react-jsx"`.

**`include` is scoped to `src/`, `e2e/**` is excluded explicitly, and `globals` is off while
`setupFiles` names task 0.23's `src/test/setup.ts`.** Vitest's and Playwright's default naming overlap
exactly (`*.spec.ts` / `*.test.tsx`), so an unscoped default would make `bun run test` try to execute
the E2E specs; `configDefaults.exclude` is spread rather than replaced so Vitest's own defaults
survive. `globals` stays off because `tsconfig.json` does not list `vitest/globals` in `types`, so
global `describe`/`it`/`expect` would break `bun run typecheck`; specs import from `vitest` explicitly.
`setupFiles` was the one entry deliberately left off while the file it names did not exist — a dangling
entry would have been a second, silent way for `bun run test` to fail — and **0.23 has now landed it,
as its one cross-file step:** `vitest.config.ts` carries the single line
`setupFiles: ['./src/test/setup.ts']`, added in 0.23's own commit by `frontend-logic` (the file is not
in `test-engineer`'s edit allow list), so the jest-dom matcher registration and the after-each
`cleanup` that module performs now run for every component spec.

**The E2E layer is Playwright driving WebKit — the engine both shipped platforms actually run
(WKWebView) — in one project per baseline, not one project looping over sizes.** `webkit-compact` is
390×844 with `deviceScaleFactor` 3 and `hasTouch: true`; `webkit-expanded` is 1280×800 with
`deviceScaleFactor` 2 and no touch. The numbers are declared explicitly rather than taken from a
`devices[…]` preset, because the closest WebKit preset is 390×664 and would have to be overridden
anyway. One project per baseline, so a failure names the baseline it failed in and `--project=…` runs
exactly one baseline. `tauri-driver` is not used because it has no WKWebView driver and can drive
neither iOS nor macOS — which is why the suite drives the frontend against the dev server with the IPC
layer mocked, needing no native device and no shared service (TOOL-5); the reads it exercises resolve
against the mock's copy rather than the local copy D0.2 defines.

**The dev server is the app's own** (`bun run dev`), started by Playwright as a child of the
`playwright test` process, at the same single address stated in three places that must agree by hand:
`server.port`/`strictPort` in `vite.config.ts`, `build.devUrl` in `tauri.conf.json`, and this config's
`baseURL`/`webServer.url`. Task 0.24 asserts the first two against each other; this is the E2E side of
that same value. The determinism choices carry their reasons: `retries: 0` (a retry that hides a flake
hides the flake), `forbidOnly` on CI only (so a committed `.only` cannot silently shrink the gate), and
failure-only artefacts (`trace: 'retain-on-failure'`, `screenshot: 'only-on-failure'`) landing in
`test-results/`, which task 0.19 excludes. Every dependency these two configs name is one of D0.8's
exact pins.

**The `VITE_E2E_MOCK=1` gate is a seam in `src/main.tsx` that reads a compile-time glob, and it fails
loudly rather than silently.** The flag is set by the committed `test:e2e` script itself, so the local
and the CI invocation are byte-identical **as far as the mock environment goes**: CI injects no mock
variable of its own, and the one variable it does set, `CI`, only *tightens* the harness — it is read in
exactly two places in `playwright.config.ts`, `forbidOnly: !!process.env.CI` (a committed `.only` is
refused) and `reuseExistingServer: !process.env.CI` (CI never reuses a server someone else started) —
rather than supplying anything the local run lacks (D0.22); the mock is task 0.23's
`src/test/mock-ipc.ts`, and that file now exists. The seam awaits the mock **before the first render**,
so the mock is installed before any route loader or query can call `invoke`. It reads
`import.meta.glob('./test/mock-ipc.ts')` rather than a static or literal dynamic import because it was
written while the file did not exist: a static or literal dynamic import of a missing module fails
`bun run build` and `tsc --noEmit`, whereas the glob is resolved at build time and yields an empty
record when the path matches nothing, so the seam stayed inert and the build green until 0.23 created
the file — with no edit to `src/main.tsx`, which is still the shape a later reader meets, because 0.23
filled the seam rather than rewriting it. **The positive half is now verified rather than designed:**
with a dev server that carries the flag, the app boots, `window.__TAURI_INTERNALS__` is present with
`invoke` and `transformCallback` as functions, an `invoke` for an unregistered command rejects naming
that command, and the page logs no unhandled rejection — which is the exact-path, non-glob resolution
the seam rests on, exercised through the committed module instead of through an empty glob. What is
still owed is the build-side half: **task 0.26** carries the assertion that a non-`VITE_E2E_MOCK`
`bun run build` emits no `mock-ipc` chunk while the `test:e2e` path does load it (recorded in
`tasks.md` 0.26). The stale-pattern case remains safe rather than silent: it lands on the failure mode
below. The deliberate failure mode: **when the flag is `1` and the module is absent, the seam throws
instead of booting unmocked**, because a mocked run that silently is not mocked would look green while
testing nothing — the same reason `passWithNoTests` and `--pass-with-no-tests` were rejected below.

**A local E2E run can silently be unmocked, and that is worth knowing before trusting a green one.**
`playwright.config.ts` sets `reuseExistingServer: !process.env.CI`, so a dev server already listening
on 1420 that was started *without* `VITE_E2E_MOCK=1` is reused as-is: the page's `import.meta.env`
carries no flag, the seam takes its early return, and the app renders with no mock installed. Measured
while 0.23 was verified — **two** stale servers were found on 1420 and stopped, PIDs `18674` and
`17814`, both this repository's `node_modules/.bin/vite`, so this was a recurrent shape rather than one
lingering process. The first is the one task 0.21's screenshot work left behind, serving unflagged and
therefore invisible to the harness; the second appeared later in the session and **could not be
attributed with certainty** — most likely a dev server an agent started to take screenshots, recorded
as the probable origin rather than as an actor the evidence names. The symptom is exactly that shape: a
rendering app with `window.__TAURI_INTERNALS__` absent. CI is not exposed to it, because CI never
reuses a server, which is why the choice above is recorded as a CI-only *tightening* rather than as a
difference the local run lacks; the local hazard is the new information, and it is not visible in the
tightening. A spec that depends on the mock should therefore **assert it is installed rather than
assume it**, the same posture `src/test/mock-ipc.ts` records for itself, because that is what makes a
run *mocked* instead of merely green.

**The config-level alternative was weighed and declined, and the handling lands in a shared fixture
rather than in per-spec discipline.** Setting `reuseExistingServer: false` outright would turn this
from a silent unmocked run into a loud failure — the honest posture, and the one this record would
take if the convenience were worthless. What it costs is every local run: a dev server start becomes
mandatory, and a stale process still holding 1420 fails with Playwright's 120-second `webServer`
timeout instead of being reused. The local convenience was kept and the hazard handled where it can be
handled once: **task 0.25 is to supply one shared `e2e/` fixture (or a `globalSetup`) that asserts the
mock is installed, consumed by every E2E spec**, instead of each spec remembering to — because a
per-spec convention is the same class of thing task 0.23 exists to remove for viewports. That fixture
is 0.25's deliverable and **does not exist yet**; until it lands, the per-spec assertion above is the
only thing between a reused unflagged server and a green unmocked run.

**No `passWithNoTests` and no `--pass-with-no-tests`, recorded as a decision.** At task 0.17 there are
no vitest specs and no Playwright specs, so both commands are red; the alternative to a red command
honestly attributed to the tasks that supply the specs (0.23/0.24 for the component layer, 0.25 for
E2E) would be a passing gate that proves nothing. That is the section-0 incremental rule `tasks.md`
already states: each command goes green when the task that owns its subject lands, not before.

**The evidence these decisions rest on was obtained by running the layer, not by reasoning about it:**
the component layer ran green at both baselines with a temporary spec (since deleted) — with the limit
of that evidence stated rather than glossed, because nothing committed declared a component
**baseline** at 0.17: `vitest.config.ts` fixes the environment (`jsdom`) and deliberately declares no
viewport, so the temporary spec set its own window size, and what was shown is "**a spec** can run at
both baselines", not "**the suite** does", which is what TOOL-5's first two scenarios actually require.
That gap was owned rather than left implicit — **task 0.23** owns a uniform baseline helper (stated in
`tasks.md` 0.23) — and 0.23 has now landed it, with **0.25** consuming it, so the two scenarios are
true for every spec 0.25 writes through it. The E2E layer launched WebKit in both projects and
`page.viewportSize()` equalled each project's declared viewport (390×844 and 1280×800) with `--project`
selecting one baseline; and the `VITE_E2E_MOCK` chain was observed end to end — the script sets the
variable, the dev server hands it to the page as `import.meta.env.VITE_E2E_MOCK`, the injected env
object carries the key when set and lacks it when unset, and the mock installs only when it is `1` —
with the flagged boot against the committed module confirming the positive case, recorded above. The
browser binary is installed by `playwright install webkit`, which is what CI runs before the E2E step
(D0.22).

**The baseline helper is what makes the component layer's two baselines real.** `vitest.config.ts`
fixes jsdom and declares no viewport, so a spec sets the window size itself; 0.23 now exports one
uniform opt-in — `describeAtBothBaselines(name, body)`, expanding to one `describe` per baseline,
applying the viewport in `beforeEach` and restoring it in `afterEach`, with `COMPONENT_BASELINES`
naming 390×844 and 1280×800 in one place — so "this suite runs at both baselines" stops being a
per-spec convention. What that makes real is the viewport **to JavaScript** (`window.innerWidth`,
`document.documentElement.clientWidth`) and nothing else, and **the reason a `@media` rule never
applies under it is not that jsdom lacks a CSS pipeline** — the pinned jsdom 30.1.3 has one
(`lib/jsdom/living/css/helpers/computed-style.js`, a parsed default stylesheet, a real
`getComputedStyle` on `Window`). Two stronger reasons hold, and both are the ones `src/test/setup.ts`
records beside the helper: **jsdom's media evaluation matches plain media types only, so every feature
query in the app's styles is inert**, and **the app's stylesheet is never loaded in a Vitest run**.
`evaluateMediaList` in `MediaList-impl.js` matches `all` and `screen` and says exactly that a query
with media features does not match, so every `@media` in `src/styles/**` (`min-width` at 768 px and
1200 px, `prefers-color-scheme`, the `hover`/`pointer` pair, `prefers-reduced-motion`) never fires —
which is why even a computed-`display` assertion reads the base rule at **both** baselines, and why the
earlier "scope by the `data-navigation-arrangement` attribute and assert the computed `display`" idea
cannot discriminate. On the second reason, `css` is not enabled in `vitest.config.ts` and
`src/styles/index.css` is imported only by `src/main.tsx`, so `getComputedStyle` returns jsdom's
UA/inline-derived value rather than a token-driven one. The upshot is unchanged and the true half of
the original record stands: the adaptive arrangement does not happen here, both navigation landmarks
stay in the DOM (a role query finds two), and with no layout engine `getBoundingClientRect()` returns
zeroes, so the ≥44 pt and safe-area assertions are E2E subjects. **The arrangement assertion therefore
belongs to Playwright, which is also why task 0.25 must not be read as owing it in the component
layer.**

**Density, pointer and visual-viewport behaviour belong to E2E too, a limit recorded only now.**
Beyond the viewport numbers the helper carries nothing of a compact browser: `devicePixelRatio` is
jsdom's `1` where the E2E `webkit-compact` project sets `deviceScaleFactor: 3`; there is no touch or
coarse-pointer capability at all, since `hasTouch: true` and `pointer: coarse` exist only in a real
browser; and `window.visualViewport` is absent — consistent with Q21, which records that no
`visualViewport` code exists anywhere yet and defers that measurement to task 4.1, the first screen with
a real keyboard. A component spec that branches on density or pointer type is therefore reading jsdom's
defaults rather than the baseline it opted into, as is one that reaches for `visualViewport`; all three
belong to E2E alongside geometry and the arrangement, and all three are documented in
`src/test/setup.ts` beside the helper.

No ADR number is taken: section 0 takes none (D0.8).

### D0.25 — Task 0.18: Biome's checked set is an allow-list, and the two generated files are excluded

**The checked set is an allow-list, not an exclude-list, so "excluded" means "not listed."**
`biome.json`'s `files.includes` lists exactly `src/**`, `e2e/**`, `index.html`, `package.json`,
`tsconfig.json`, `vite.config.ts`, `vitest.config.ts` and `playwright.config.ts` (the two D0.24
records), plus `biome.json`, then negates the two generated files below. The consequence a later
reader needs is the opposite failure mode from an exclude-list: a new root-level file is **not
checked until someone adds it**, where an exclude-list would check it by default and fail on the
trees that must stay out. That trade was made deliberately, because those trees are many and keep
growing — `node_modules/`, `src-tauri/target/**`, `src-tauri/gen/**`, `graphify-out/**`,
`.opencode/skills/**`, `dist/**`, `test-results/**` and `playwright-report/**` — and an exclude-list
would have to name every one of them as it appears. The measured weight of the problem is recorded:
`biome check .` with **no config** checked **1512 files and reported 1502 errors** — the original
task-0.13 measurement — while the no-config state task 0.18 inherited was **1526 files / 5090
errors** (3083 warnings, 170 infos; Q18), grown between the two as files were added and as
`dist/assets/*.js` entered the scan. A post-`bun run build` measurement taken while preparing 0.18
read **1528 files / 5092 errors**. With the config, **12 first-party files** are checked and the run
is clean — and that clean run is not hollow: a deliberately bad file (unsorted imports, an `any`, an
unused parameter, a long double-quoted string) produced `format`, `assist/source/organizeImports`
and `lint/suspicious/noExplicitAny` **errors** plus a `lint/correctness/noUnusedFunctionParameters`
**warning**, the run ended "Some warnings were emitted while running checks" and **exited 1**, and
deleting the file returned it to `Checked 12 files` clean. That last step is the flag earning its
place: a warn-severity rule alone would not have failed the gate.

**Both generated files are excluded, and they are in opposite states — which is why the decision is
forward-looking rather than present-tense.** `src/routeTree.gen.ts` attracts **two** diagnostics: a
`format` diff and a `lint/suspicious/noExplicitAny` from the generator's own `} as any)`. Under this
repository's style that format diff is **only** the semicolons, since the generator already emits
single quotes; `src/bindings.ts` attracts **nothing today**, being one comment line over the empty
command and event set (D0.9), and it only becomes the long emitted line Biome would want to break
once task 1.3 adds commands. The exclusion is therefore a decision about what those files **become**
as much as about what they are now.

**Matching the generator instead was rejected.** It would mean `semicolons: off` repository-wide,
reformatting every first-party file to satisfy one artefact, and it would still leave `} any)`
failing: matching the generator's *linter* output is impossible without editing generated output,
which the project forbids. Excluding is what keeps TOOL-4's "regeneration produces no diff" and this
gate command's "lint passes" (D0.9) from fighting, and that agreement is verified rather than
asserted — a build followed by `git diff --exit-code` on both generated files exits 0.

**The repository's existing style wins over Biome's defaults, and that is a decision.** `biome.json`
sets `indentStyle` `space`, `indentWidth` 2, `lineWidth` 80, `lineEnding` `lf`,
`javascript.formatter.quoteStyle` `single` and `semicolons` `always`, so no first-party file was
rewritten to satisfy the formatter and the diff is the config plus genuine fixes. The rejected
alternative — adopt Biome's defaults and reformat every first-party file — would swamp a scaffolding
task's diff, touch files later tasks are reading, and still not rescue the generated route tree.
`lineEnding: lf` is Biome's default, written out as documentation rather than a change. So is the
`assist` block: `assist` is on by default in Biome 2.x, `biome check` fails on assist diagnostics and
`assist/source/organizeImports` is emitted as an **error**, so naming it documents a default — while
JSX attribute ordering (`useSortedAttributes`) is deliberately **not** imposed, which matters to
`frontend-ui`.

**`preset: "recommended"` documents a default, and the explicit `noExplicitAny: "error"` is defence
in depth rather than a second gate.** `linter.rules.recommended` is `true` **by default** in 2.5.15,
so `preset: "recommended"` is explicit documentation in the same way as the `assist` block above,
not a load-bearing switch. With the gate command now `biome check --error-on-warnings .` (D0.8), a
**warn**-severity `noExplicitAny` already fails the run, so the `"error"` override is not what puts
`any` behind the gate; it keeps the rule failing for a bare `biome check`/`biome lint` invocation
that does not pass the flag — someone running the tool directly, or an editor integration — and it
documents the intent. The default-versus-gate fact the flag rests on is recorded in D0.8 and
cross-referenced here rather than repeated.

**What is deliberately not covered, and why.** Markdown and YAML cannot be checked by Biome 2.5.15
at all: the schema's language sections are `css, graphql, grit, html, javascript, json`, so
`docs/**`, `SETUP.md`, `AGENTS.md` and `openspec/**` are uncovered by format support rather than by
choice. Rust is `cargo fmt`/`clippy` territory by D0.17. `src-tauri/**` stays deliberately outside
the frontend allow-list: `*.rs` is D0.17's and `src-tauri/gen/**` is generated, so the two hand-owned
JSON files are the ones in question, and they stay out on two grounds. **Ownership:**
`capabilities/default.json` is hand-owned and task 0.6 hardened it so its permission diff stays
reviewable (D0.21); including it either forces a cosmetic rewrite of a `tauri-security` file inside
a frontend task or leaves the gate permanently red, and a frontend-owned `biome check --write .` or
format-on-save would silently reformat a security file — the cross-ownership hazard D0.11 exists to
prevent. **Remit:** Biome is the frontend toolchain here (`src/**`, `e2e/**`, the frontend build
configs); the correctness of `tauri.conf.json` and `capabilities/default.json` is enforced by Tauri
itself — their `$schema` pointers and the capability validation `tauri build` performs — a stronger
signal than JSON formatting, so a formatter pass adds no correctness signal. Measured: adding both
to `files.includes` reported `Checked 14 files`, with `tauri.conf.json` **Biome-clean** and
`capabilities/default.json` attracting **one `format` diagnostic, no lint diagnostic** — two short
arrays Biome would collapse (`"windows": ["main"]`, `"platforms": ["iOS", "macOS"]`), pure
line-layout churn. Including only `tauri.conf.json` would churn nothing today, but that is
coincidence rather than a guarantee, and a split boundary makes the allow-list harder to reason
about for almost no gain; one stated boundary is the more defensible record. `opencode.jsonc` is
also out because it sits outside the frontend tree — a known one-line follow-up rather than an
omission, since listing it risks a red gate on a file this task may not fix. The conclusion a later
reader should draw: the covered set is the frontend tree plus its root configs; `src-tauri/**` is
deliberately out and validated by Tauri tooling; `opencode.jsonc` is out because it is outside the
frontend tree.

No ADR number is taken: section 0 takes none (D0.8).

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
| `package.json`, `index.html`, `tsconfig.json`, `vite.config.ts` | Frontend workspace; the dev server sets `server.host = process.env.TAURI_DEV_HOST` for iOS dev. `.gitignore` is not created here: the root `.gitignore` already exists and task 0.19 extends it |
| `src-tauri/Cargo.toml`, `build.rs`, `src/main.rs`, `src/lib.rs` | Rust skeleton; `cfg`-clean for later platforms |
| `src-tauri/tauri.conf.json`, `src-tauri/capabilities/` | App identity, the deployment floors (task 0.8, D0.16) and least-privilege baseline (`tauri-security`); no `src-tauri/Info.plist` and no entitlements file exist, because nothing in the scaffold calls a privacy-gated API (D0.21) |
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
- Section 0 applied that gate incrementally, as the preamble in `tasks.md` states: a section-0 task is
  ticked when every gate command *runnable at that point* passes, and a gate command whose precondition
  is a later section-0 deliverable belongs to that task (the bun-family commands arrive with 0.13, the
  two drift checks with 0.16, the E2E suite with 0.17). Task 0.12 restored `cargo fmt --check` and the
  clippy-clean requirement — the generator's own 2-space style, in `src-tauri/build.rs` and
  `src-tauri/src/main.rs`, is why it was red until then — and 0.26 re-runs the complete gate fresh. That
  incremental carve-out **expired at 0.12 and does not extend past it**: from 0.12 on, every gate
  command that already exists is run in full against the tree, and a command that is still absent is
  owned by the task that first makes it runnable rather than deferred further. It is therefore closed
  rather than merely reached: the whole Rust half of the gate is green with nothing carved out (the 0.12
  outcome record in D0.9 measures it).
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
- Task 0.6's posture (D0.21) is evidenced by a compile check rather than by a diff, and that evidence
  is of two kinds: the config and the capability are evidenced as **valid** by the `cargo check` runs
  (host and `aarch64-apple-ios`), the real check for both files because `build.rs` hands them to
  `tauri-build`, and both runs exercise a capability whose `platforms` covers the target in hand; the
  empty permission set is revisited only by the obligations D0.21 records, never by a "safety"
  re-widening. The CSP's strictness is evidenced differently — it is exercised only by a **built** macOS
  artefact, and task 0.26 now carries that step. The runnable gate status is stated, not implied:
  `cargo clippy --all-targets -- -D warnings` **passes** and `cargo test` **passes** (0 tests when this
  posture landed — the scaffold had none yet, and a config-only change cannot affect them; task 0.12's
  harness adds 3, see its outcome record in D0.9), the two `cargo check` runs pass (recorded above), and
  `cargo fmt --check` is **green** from task 0.12 — the generator's own 2-space style is why it was red
  here, and 0.12's visible diff is exactly the reindentation of `src-tauri/build.rs` and
  `src-tauri/src/main.rs`, with nothing carved out (its outcome record in D0.9).
- Cross-check at review: the delta contains exactly HS-1…HS-4, SYNC-1/2/3/5, SET-1/2/4,
  NFR-1/2/3/8/10, DM-3 and DM-6 across the five product capabilities, **plus exactly TOOL-1…TOOL-8
  in `project-toolchain`** — no more, no less. The `TOOL-` set is the section-0 enabler and is in
  scope; any further requirement, or any requirement implemented without a scenario, is scope creep
  (task 6.2).
