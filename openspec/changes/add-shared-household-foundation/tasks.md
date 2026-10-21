# Tasks — add-shared-household-foundation (P0)

One task = one commit = one owner. Format: `- [ ] <n>.<m> [agent] <imperative>`.
Order: section 0 (workspace scaffold, the prerequisite for everything below:
architect → tauri-security → apple-native → release-ci → rust-backend / frontend-logic →
frontend-ui → tests), then contract → backend → frontend-logic → frontend-ui → tests → review.
Tick a task only after the `definition-of-done` gate passes for it. In section 0 the gate is applied
incrementally, because the workspace the gate runs against is what this section creates: a task is
ticked when every gate command that is runnable at that point passes, a gate command that runs but reports
an empty suite belongs to the task that supplies the suite, and a gate command whose
precondition is a later section-0 deliverable belongs to that task (the bun commands from 0.13 onwards,
the bindings drift check from 0.16 and the route-tree drift check from 0.15 — the two are not the same
task, because each check only means something once the file it guards is tracked, and `src/routeTree.gen.ts`
is tracked from 0.15 while `src/bindings.ts` waits for 0.16 — the
E2E suite in 0.17). Concretely, task 0.1 was ticked on: `cargo check` for the macOS host **and** for
`aarch64-apple-ios` both pass; `cargo clippy --all-targets -- -D warnings` and `cargo test` pass;
`cargo fmt --check` was red on the generator's own 2-space style, which task 0.12 fixed; and the
bun-family and drift-check commands still have no `package.json` to run against. Since task 0.12 the
whole Rust half of the gate is green with **no** carve-out left, and the only commands still waiting on
a precondition are the bun-family ones (0.13), the two drift checks (0.16) and the E2E suite (0.17).
Task 0.13 is ticked on the same rule, and what it measured is worth stating plainly rather than
leaving "the bun commands from 0.13" to imply that all of them became 0.13's: of the bun family only
`bun run gen:bindings` is runnable at 0.13, and it passes — exit 0, silent, with
`git diff --exit-code src/bindings.ts` clean. The other three exit 1 for a reason owned by a later
task and **not** by the manifest: `typecheck` has no `tsconfig.json` until 0.14, `test` has no vitest
wiring until 0.17, and `lint` has no `biome.json` until 0.18 — with no config, `biome check .` checked
1512 files and reported 1502 errors, the same numbers task 0.18 carries. So the bun commands do not
all belong to 0.13; each belongs to the task that supplies what it needs, and 0.13 owns
`gen:bindings` plus its manifest's two pinned properties (the 21 exact pins and the eight script
bodies). Task 0.14 was ticked on the same rule and is the first task to make a previously-red bun
command green: `bun run typecheck` now exits 0, because the `tsconfig.json` it adds is the thing that
command was missing. `lint` stays red for 0.18 and `test` stays red for 0.17 — though at 0.14 `test`'s
reported cause changed to the router plugin's generator failing to `scandir` `src/routes`, a directory
0.15 creates, because configuring that plugin **is** 0.14's requirement (Q16). Task 0.15 is where a
drift check first becomes meaningful rather than vacuous: the route tree is generated, and because the
generated file is committed with that task, a rebuild followed by `git diff --exit-code
src/routeTree.gen.ts` is a real assertion — it was run and passed (TOOL-4). Its other three commands are
unchanged: `typecheck` green, `gen:bindings` green while its own diff stays vacuous until 0.16 commits
`src/bindings.ts`, and `lint` and `test` still red for 0.18 and 0.17. That task also recorded the first
specialist-side limitation the incremental rule runs into: a specialist session may not stage a file
(Q17), so the git half of any task whose evidence depends on a tracked file — 0.15's route tree, 0.16's
bindings, 0.19's `.gitignore` — is the orchestrator's step, and neither halves nor fakes were needed.
Task 0.16 then made the second drift check meaningful the same way, and settled the one thing the earlier
tasks could not: because `git diff --exit-code src/bindings.ts` had been vacuous since task 0.11, no
comparison against a **frozen** blob had ever run. Task 0.11 did run the exporter and record the hash of
what it produced (D0.9), and task 0.12 confirmed the builder rewrites nothing under `src/`, but there was
no tracked blob to compare a fresh export against — so the file could have been altered in the interim
and every check would still have passed. Staging it and regenerating closed that: the file is
byte-identical to the export over the empty command and event set, so it is genuinely generated output,
and the check now passes for a real reason (TOOL-3). What 0.16 could not do is prove TOOL-3's remaining
two scenarios — that a stale bindings file fails the build and that a regeneration reverts a hand edit —
because neither the orchestrator nor `frontend-logic` holds edit permission on a generated file and
`git checkout` is unavailable; those are assigned to task 0.24, which is the first task that can assert
them.
Task 0.17 configured the last piece of the test harness, and it is one of the section-0 tasks whose
commands stay red **because they are empty rather than because something is missing**: `bun run test`
and `bun run test:e2e` both exist, resolve their configs and run — they simply have no specs yet, so
vitest reports "no test files found" and Playwright "no tests found". Neither `passWithNoTests` nor
`--pass-with-no-tests` was added to paper over that, because a passing gate that proves nothing is worse
than a red command honestly attributed to the tasks that supply the specs: 0.23 and 0.24 for the
component layer, 0.25 for the E2E layer. 0.23 is in the same position for the same reason, so this is a
state two tasks share rather than a quirk of this one. The harness itself was verified by running it
rather than by reading its config — a temporary spec was shown green at both baselines, and the E2E
layer launched WebKit in both projects with `page.viewportSize()` equal to each project's declared
viewport (390×844 compact, 1280×800 expanded) and `--project` selecting one baseline, and the
`VITE_E2E_MOCK=1` chain was observed end to end from the committed script through the dev server into
the page. One limit of that evidence is worth stating rather than glossing: the **component** layer has
no declared baseline, because `vitest.config.ts` fixes the environment (`jsdom`) and deliberately
declares no viewport, so the temporary spec set its own window size — "a spec can run at both
baselines" is what was shown, not "the suite does". Making the latter true is task 0.23's, which now
owns the baseline helper TOOL-5's first two scenarios depend on. Its decisions are recorded as D0.24,
including why the test layer deliberately does not reuse the app's Vite config: that config carries the
router plugin, which renders the tracked `src/routeTree.gen.ts`, and a test run must never rewrite a
generated file the gate checks.
Task 0.18 closed the last gate command that was red for want of configuration: `bun run lint` is green,
and its script body is the only one of the eight task 0.18 changed — it now passes
`--error-on-warnings`, because `biome check` exits 0 on a warning-only tree while the recommended rule
set contains warn-severity rules and `suspicious/noExplicitAny` is warn by default, so without the flag
the gate's single lint command could print a diagnostic and still pass. The other seven bodies stand as
task 0.13 fixed them, and `.github/workflows/ci.yml` needed no edit because it invokes the script. The
checked set is an allow-list rather than an exclude-list, which is the opposite failure mode on purpose:
with no config the command checked 1512 files and reported 1502 errors at 0.13, and 1526 files / 5090
errors was the no-config state this task inherited (Q18); with the list it checks **12 first-party
files** cleanly. Both generated files are
excluded — the route tree because it attracts a `format` diff and a `noExplicitAny` from the generator's
own `} as any)`, and `src/bindings.ts` because it is clean today but becomes the long emitted line Biome
would want to break once task 1.3 adds commands, so the exclusion is a decision about what those files
become as much as about what they are. Nothing was reformatted to satisfy the tool: the config matches
the repository's existing style, so the diff is the config plus one script line. Its decisions are
recorded as D0.25, and D0.8's `lint` row was corrected with it.
Task 0.19 closed section 0's file-writing surface. The root `.gitignore` now covers `node_modules`,
`dist`, `src-tauri/target/`, `test-results/`, `playwright-report/`, `.artifacts/` and `.tanstack/tmp/`
alongside the `graphify-out/cache` entry the graph tooling keeps there, and it does **not** match either
generated file — which matters because both are *tracked*, and an ignore rule can only ever hide an
untracked path: `src/bindings.ts` and `src/routeTree.gen.ts` stay committed, so the two drift checks
keep biting rather than passing vacuously. Two things were found and deliberately not covered, both
reported rather than silently swallowed: the dated `graphify-out/<date>/` snapshots (an earlier one is
committed, so hiding new ones would end that convention while leaving the committed copy listed as
modified forever, and graph-churn policy is not this task's), and `src-tauri/gen/schemas/`, which the
generator's own `src-tauri/.gitignore` already covers — so no `src-tauri/gen/` pattern was added at
root, because that would also hide the Xcode project task 0.8 deliberately commits. `.tanstack/tmp/` is
the one path added beyond the task's list, on evidence: the pinned router generator creates that
directory lazily as per-session scratch space.
Task 0.20 established the design system and set the status line, so TOOL-6's precondition is satisfied
for every UI task that follows: `src/styles/` now holds the three layers as separate files —
`primitives.css` (values), `semantic.css` (purpose, both themes, platform facts) and `components.css`
(per-component decisions) — behind a single `index.css` entry point, and the component inventory plus the
global conventions are recorded in `.opencode/skills/design-system/SKILL.md` with `status: frozen` in its
frontmatter. The layers are plain CSS custom properties, deliberately not a TS mirror or a generated
bridge: the runtime consumers are CSS, no dependency may be added, and a second representation would
drift. They are consumed from one place, `src/main.tsx`, which is what makes the bundler walk the whole
`@import` chain — before that wiring landed the stylesheet had only been through Biome, and a broken
import path or a bad at-rule would have surfaced in 0.21 instead of here. Two measurements carried the
task rather than judgement: 484 tokens resolve per theme with every `var()` chain defined and no cycles,
and all 25 documented contrast pairs pass AA in both themes, which is why the palette is orange-700
rather than orange-500 and red-700 rather than red-600 on the sunken surface. No screenshots were owed,
because no styled surface exists until 0.21 — the token layers carry no element selectors and no reset,
so importing them restyles nothing, and the build emitting a 22.18 kB stylesheet is evidence of the
chain resolving, not of a visual result. The record lives in the skill rather than as a new `D0.x`
because D0.10 already fixes that as the design system's home.
Task 0.21 built the primitives and gave the product its first rendered surface. `src/components/ui/` holds
the 23 inventory primitives (plus three internal modules that are not inventory entries), built only from
the frozen tokens, and the placeholder screen lives in `src/features/design-preview/` — which is what
let the two route files stay thin, as `tanstack-router-conventions` requires: `src/routes/index.tsx` is
now a `createFileRoute` plus an imported component, and `src/routes/__root.tsx` imports the not-found
surface instead of declaring it inline. Both of those files are `frontend-logic`'s, so this task's commit
carries a deliberate cross-owner step, recorded here the way Q17 records the same arrangement: the screens
were built by `frontend-ui`, and the two imports that make them reachable were written by
`frontend-logic`. The measurement that proves the wiring is the build: **161 modules before, 189 after**,
with a 16.47 kB lazy route chunk — the primitives were simply not in the bundle until the routes pointed
at them, which is also why the first screenshot pass showed the old scaffold and was re-taken.
Evidence is measured rather than asserted, because a screenshot cannot answer what the requirement asks:
25 visible interactive elements at each baseline, every one **≥ 44pt in both dimensions**, the smallest
exactly 44; the adaptive arrangement verified from `getComputedStyle` and visibility rather than by eye —
tab bar laid out and sidebar `display: none` at 390×844, the reverse at 1280×800, and at the 844×390
landscape band the sidebar arrangement too, which is what made a safe-area hole reachable; the safe-area
insets applied at every box flush to a viewport edge on the inline axis as well as the block axis, since
only the top and bottom insets were applied until the review caught the horizontal pair; and 24 rendered
foreground/background pairs read off live elements, all matching their token value, with the only pair
below 4.5:1 being the disabled button — exempt, and still labelled. Two claims are stated carefully
rather than loosely, because the review found both overstated. The `navigation` landmarks are **two in the
DOM and exactly one laid out** at each width: the attribute says which landmark is which, not which one is
on screen, and jsdom cannot settle it either — not because it lacks a CSS pipeline, which it has, but
because its media evaluation matches plain media types only and the app stylesheet is never loaded in a
Vitest run — so a component test sees both. The
arrangement assertion belongs in E2E, where Playwright's viewports are real (noted in 0.25). And the
keyboard inset is **defined and reserved but not yet assigned**: `Screen` consumes it while nothing sets
it, there is no `visualViewport` code anywhere, and the app-shell assignment is deferred to task 4.1, the
first screen with a real keyboard (Q21). The compact capture was **re-taken** after this task was
committed: two files of the fix round were left unstaged, the reviewed PNG had been rendered from that
uncommitted state rather than from the commit, and once the owner restored the files byte-exact the image
was captured again so the evidence matches the code (Q24). Four screenshots sit in `.artifacts/screens/` — the preview and the not-found surface at both baselines —
gitignored as evidence rather than committed, per `definition-of-done`. The build stays at 47 linted files
and the route set is unchanged, so `src/routeTree.gen.ts` needed no regeneration.
Task 0.22 replaced Tauri's default mark with one drawn from the design system. The source artwork is
committed at `src-tauri/icons/app-mark.svg` — beside the set it generates rather than in `src/styles/`,
because the iconset is hand-owned (D0.11) and the mark is an input to it, not a stylesheet — and it is a
flat four-shape rabbit head: two rotated ear ellipses behind a head circle, with the accent inner ear in
each, which is legible down to 16px. Every colour is a brand token resolved through the layers
(`--app-mark-tile` → `--color-brand-tile` → `--color-orange-700`, and the same chain for glyph and
accent); the hex is inlined only because the icon is rasterised outside the document and cannot read a
custom property, with the provenance recorded in the file's `<desc>`. Generation is `bun run tauri icon
src-tauri/icons/app-mark.svg` through the pinned CLI — no global install, no new dependency — and it
replaced **34 tracked files**: the 16 desktop entries, the 18 iOS `AppIcon` slots, and the PNGs embedded
in `icon.icns` and `icon.ico`. Verification was pixel-level rather than visual: a scratch decoder read the
raster back at every desktop size, all 18 iOS catalog slots — 13 distinct raster sizes — and both
container formats, and found the exact token values; the two honest exceptions are ICO's 16px and 24px
entries, where the accent is sub-pixel blended by downsampling. `tauri.conf.json` was not touched and its five `bundle.icon` entries all still exist, so
the replacement is in place rather than merely alongside. Two judgement calls are recorded rather than
implied: the task could not put the artwork in `src-tauri/icons/` without a write grant, so
`frontend-ui`'s allow list gained **one narrow path** (`src-tauri/icons/*`, after the blanket deny, with
the reason in the file) rather than the artwork being parked in a stylesheet directory; and the CLI emits
`icons/android/**` and a `64x64.png` unconditionally, which are **kept** rather than hand-deleted, because
deleting part of generated output would make re-running the documented command produce a diff and D0.11
says regenerate rather than edit. Phase 1 ships neither Android nor Windows, so that is dead weight by
design, not a platform commitment (Q22).
Task 0.23 delivered the harness both test layers rest on, and it is the task after which the remaining red
gate command is red for want of a **subject** rather than wiring: both suites still exit non-zero with
"no test files found" and "no tests found", but nothing is missing from the harness any more — what is
missing is the specs themselves, which 0.24 and 0.25 own. `src/test/setup.ts` registers the jest-dom
matchers, installs Testing Library's `cleanup` by hand (its auto-cleanup hides behind a global `afterEach`
that `globals: false` does not provide), and exports the query-client wrapper with `retry: false`, the
memory-history and router builders, and — the piece nothing else supplied — the **baseline helper** that
makes "this suite runs at 390×844" and "…at 1280×800" true for every spec that opts in, with the numbers in
one place and the viewport applied in `beforeEach` and restored in `afterEach`. Its limits are documented
rather than implied, because they are what stops 0.25 being asked for the wrong thing: jsdom evaluates
only plain media types and never loads the app's stylesheet, so a feature query never matches and the
adaptive arrangement cannot be asserted in the component
layer at all, and with no layout engine `getBoundingClientRect()` returns zeroes, so the ≥44pt and
safe-area assertions stay E2E subjects. `src/test/mock-ipc.ts` is the other half, and it is what turns the
`VITE_E2E_MOCK` seam from designed into verified: with a flagged dev server the app boots,
`window.__TAURI_INTERNALS__` carries `invoke` and `transformCallback`, an `invoke` for an unregistered
command rejects naming that command rather than resolving to nothing, and the page logs no unhandled
rejection — the positive case 0.17 could not reach, now exercised through the committed module. Its
registry is one entry per command, so task 1.3's commands slot in without restructuring it. The one
cross-file step again belonged to `frontend-logic` (`setupFiles` in `vitest.config.ts`, since
`test-engineer` cannot edit that file). Two findings came out of this task and are recorded rather than
absorbed: the build stays at **189 modules with no `mock-ipc` chunk**, which is the D0.24 claim that 0.26
must assert, verified here by folding the flag off at build time; and an **unflagged dev server on 1420**
makes a local E2E run silently *unmocked*, because `playwright.config.ts` reuses an existing server and
that one lacked the flag, so the app rendered with no mock installed. **Two** such servers were found and
stopped during this task (`18674` and `17814`, both this repository's Vite; the first left by task 0.21's
screenshot work, the second appearing later and most likely started by an agent for screenshots), and Q23
records the hazard plus the mitigating fixture task 0.25 must now supply — one shared assertion, not a
per-spec convention.
Task 0.24 made `bun run test` green for the first time since 0.17 — **6 spec files, 27 tests, exit 0** —
and it is the task where the scaffold's invariants stop being prose. Six areas, each asserting something
observable rather than restating a requirement: bindings reproducibility (read, export, read, compare
bytes — no `git` needed for the stability half); the hand-edit case, which appends a marker and runs the
gate's own `git diff --exit-code`, asserting a non-zero exit that names the file, then regenerates and
asserts the original bytes are back, with the restore in a `finally` so a failing assertion cannot leave a
hand-edited generated file behind; route-tree reproducibility (read, build, read, compare); exact pins in
**both** manifests with the expectations **parsed from D0.8's tables** rather than hard-coded, so a
legitimate bump changes one place, plus the paired-crate rule; the `tauri.conf.json` ↔ `vite.config.ts`
agreement task 0.14 warns about, including `server.host` still wired to `TAURI_DEV_HOST`; and the design
system's token invariants — every `var()` resolving, no cycles, no duplicate inside one cascade context,
and all 25 documented AA pairs in both themes within 0.02 of their recorded ratio. That last one is the
assertion that matters most, for a reason worth stating: Vite does not resolve custom properties, so a
typo in a token name is emitted verbatim, the build stays green and the browser silently falls back —
nothing else in the gate would catch it.
Two things about that suite are worth keeping in the record beyond the pass. It carries **negative tests of
its own checkers** — a spec that feeds every checker a bad input and asserts it fails — because a
green checks-suite that cannot fail is the same problem as a lint config that matches nothing; and every
high-value assertion was demonstrated **failing** under perturbation before being accepted, with the exact
messages recorded (a caret pin, an unresolvable `var()`, a same-context duplicate, an orange-500 palette
dropping two pairs below AA, a 1421 port against a 1420 `devUrl`, and a marker line in the bindings). The
review then found one hole worth naming, because it is the kind that hides behind green: the Cargo reader
silently skipped a `[dependencies.name]` sub-table that carried no `version`, so a crate declared by `git`
or `path` alone entered neither manifest map and passed both the extra-key and the missing-key checks — a
demonstration against the pre-fix code showed zero issues for exactly that input. It now throws, the
sub-table stays in scope until the next header, and two negative tests cover both that shape and a field
appearing after `version`; the residual blind spots (dotted-key declarations, a duplicated `version` key)
are listed in the module rather than left implicit. Three
TOOL scenarios are deliberately outside a bun test and are recorded as such rather than dropped: the stale
route tree cannot be provoked because the gate builds before it diffs, so the gate step implements it
(0.9's CI, evidenced at 0.15, run by 0.26), and "both platforms are supported" is a constraint on adding a
dependency rather than a manifest property, enforced by D0.8's per-dependency review. What is still red is
`bun run test:e2e` alone, which 0.25 owns.
Task 0.25 closed section 0's test surface, and with it **the whole gate is green for the first time in
this change** — every command of `definition-of-done` passes locally, with no command left red for want of
a configuration, a subject or a suite. Four E2E specs in `e2e/`, plus the shared fixture, cover TOOL-5's E2E scenarios and the flows
task 0.21 built, at **16 tests, 8 per project**, so each baseline is exercised separately and
`--project=webkit-compact` runs one; three component specs under `src/**` add the component layer's half,
every one of them inside `describeAtBothBaselines` so both baselines are exercised by construction rather
than by convention. The shared fixture is the part worth describing, because it is the hazard from 0.23
turned into an assertion: it is **auto**, so it runs before every E2E spec's body without any spec
requesting it, it boots the start screen and asserts `window.__TAURI_INTERNALS__.invoke` is a function,
and every spec imports `test`/`expect` from it rather than from `@playwright/test`. That import is a
**rule, not a hope**: a guard spec under `src/test/` reads every `e2e/*.spec.ts` and fails if one
value-imports `test` or `expect` from anywhere but `./fixtures`, fails if the fixture stops exporting
`expect`, and fails when it finds no spec files at all — so a renamed directory cannot pass by scanning
nothing. That mechanical half is what the review demanded, and it is the difference between a convention
and a guard: a future spec importing from `@playwright/test` would lint, typecheck and run with the fixture
silently absent, which is Q23's unmocked-green state in a new form. It was demonstrated in the direction
that matters: with an unflagged dev server
deliberately left on 1420, all eight tests in a project failed **in the fixture** with the named cause
("VITE_E2E_MOCK=1 is not in effect: the page rendered with no IPC mock…"), which is the difference between
a green run and a green run that means something — a spec asserting D0.2's never-loaded surface would
otherwise pass *because* the mock was absent.
The arrangement assertion exists **only** in Playwright, and the component specs assert the reason it
cannot: they check that two "Primary" landmarks are in the jsdom DOM, which is what makes a component-layer
`display` read a green assertion proving nothing. The E2E side scopes by `data-navigation-arrangement` and
asserts computed `display` per band, so no assertion depends on class names. Two smaller findings came out
of this task. The first is the copy defect 0.25's work exposed — the preview rendered "Retry pressed 1
times" — which was fixed by its owner as a recorded cross-owner step rather than left for a reviewer to
find, and which is why the two preview captures were **re-taken** as insurance rather than because the old
frames were wrong: the changed caption sits in a section below the fold at both viewports and reads
"0 times" at the captured state, so the images would probably have been identical — but the source of a
rendered surface changed, and Q24's rule is to capture after the change is final rather than to reason
about whether a pixel moved. The second is cosmetic and left
alone deliberately: mounting a router under jsdom logs a `Not implemented: Window's scrollTo()` notice
per router-mounting test in `bun run test`, from the router's scroll restoration, and silencing them with a
stub would be asserting the stub rather than testing the app.
Commit message: `feat(add-shared-household-foundation): <task>`.
Every UI task captures screenshots at 390×844 and 1280×800 into `.artifacts/screens/`
(gitignored), per `definition-of-done`.

## 0. Workspace scaffold

The repository has no workspace yet: no `package.json`, no `src/`, no `src-tauri/`, no
`vite.config.ts`, no `tsconfig.json` and no CI workflow (the root `.gitignore` exists, but holds only
the `graphify-out/cache` entry the graph tooling keeps there, so task 0.19 extends it rather than
creating it). Every gate command and
every task from section 1 onwards assumes one, so section 0 creates it, and it is the scaffolding
prerequisite for the rest of P0 (`proposal.md` → Dependencies).

Section 0 does not hand-write the baseline. Task 0.1 (owned by `architect`, the first task here)
generates it with the **official Tauri v2 CLI** — `cargo install tauri-cli --version "^2" --locked`
→ `cargo tauri init --ci` → `cargo tauri ios init` → `cargo tauri capability new` — and commits that
generated output as **one** commit. `cargo tauri icon` is deliberately **not** part of that pass: the
app mark is drawn from the design-system tokens and generated by `frontend-ui` in task 0.22
(`design.md` D0.19), so 0.1 ships Tauri's default iconset as a temporary placeholder that 0.22
replaces inside section 0, before anything is released. Everything after 0.1 **refines that
baseline** instead of creating files from nothing; the baseline-then-refine model, the rejected
alternative (`create-tauri-app`) and the generator-owned versus hand-owned artefact split are
recorded in `design.md` (D0.11). Note deliberately: `tauri init` generates **only** `src-tauri/` and
never a frontend, so there is no framework template to delete and no cleanup task below.

Order inside section 0: `architect` (the generator bootstrap, then dependency choices and
justifications and the bindings pipeline design) → `tauri-security` (app identity, least privilege)
→ `apple-native` (task 0.7, the deployment floor measured from the installed SDKs) → `tauri-security`
(the iOS target's usage strings and deployment targets) and `release-ci` (CI) → `rust-backend` /
`frontend-logic` → `frontend-ui` → `test-engineer`. Section 0 now contains **one `apple-native`
task**: the deployment targets must be derived from the installed Xcode/SDKs (`design.md` D0.16) and
`tauri-security` has no `xcodebuild`/`xcrun` permission, so that measurement cannot live in task 0.8.
Everything else about `apple-native` is unchanged — the scaffold introduces no custom Swift plugin
and leaves `src-tauri/plugins/` empty, so this one task is its only surface here; the native
configuration the scaffold does need (`tauri.conf.json`, the iOS target's `Info.plist`,
entitlements) belongs to `tauri-security`. There is no separate review section for section 0: 6.1 and
6.2 review it as well.

Platforms: iOS (primary) and macOS, in step, `cfg`-clean for later platforms. Requirements: the
enabler set `TOOL-1 … TOOL-8` in `specs/project-toolchain/spec.md` (no source-document ID; see that
file's header).

- [x] 0.1 [architect] Bootstrap the workspace with the official Tauri v2 CLI and commit the
  generated baseline as one commit (message marks it as generated output). Preconditions, all
  satisfied before this task starts: macOS with Xcode, the `aarch64-apple-ios` rustup target
  (`SETUP.md`), and network access for the CLI install. Then: `cargo install tauri-cli --version
  "^2" --locked`; `cargo tauri init --ci` in the repository root — the documented "Initialize a
  Tauri project in an existing directory" path; `create-tauri-app` is NOT used (this directory is
  non-empty, and its React template has no TanStack Router file-based routing, no Query, no
  vitest/Playwright, no lint tooling and no `tauri-specta`, so it would be discarded) — passing the
  frontend values that tasks 0.13/0.14 MUST match exactly: `-D ../dist`, `-P
  http://localhost:1420`, `--before-dev-command "bun run dev"`, `--before-build-command "bun run
  build"`. Do NOT invent the app identity: take the `--ci` defaults for `--app-name` and
  `--window-title` and leave the real product name, window title and bundle identifier to task 0.5,
  which sets them to the values settled in `design.md` D0.15. Then `cargo tauri ios init` (the iOS
  target; it keeps its documented position per D0.20 — if it fails because no frontend dist exists
  yet, it moves after task 0.14 within this same pass under this same owner and the correction is
  reported as a blocker rather than worked around), and `cargo tauri capability new` (Q13: it generated
  nothing in this environment — it needs an identifier and this change defines none to give it; the
  capability file task 0.6 hardens is the generated `src-tauri/capabilities/default.json` from
  `tauri init`). The `cargo tauri icon` step is deliberately NOT part of this pass, so
  this commit ships Tauri's default iconset as a temporary placeholder; task 0.22 replaces it inside
  section 0 from the design-system tokens (D0.19), before anything is released. Record the
  generator-owned versus hand-owned artefact split in `design.md` (D0.11). `cargo check` passes on
  the macOS host. Both platforms (TOOL-1, TOOL-2).

- [x] 0.2 [architect] Record the workspace scaffold baseline in `design.md` (D0.8): bun as package
  manager, React + TypeScript + Vite, TanStack Router + Query, Tauri v2, Rust in `src-tauri/`, and
  the exact pin set for `tauri-specta` + `specta` (matching versions, not ranges — the `2.0.0-rc.x`
  release candidate is accepted, D0.18), vitest, Testing Library, Playwright, **Biome** as the single
  lint and format tool (D0.17) and the Tauri v2 JS/CLI packages. One-line justification per new
  dependency (project rule 6) and a confirmation that each is usable on iOS and macOS. The root
  tooling files (`package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `.gitignore`) are
  inside `frontend-logic`'s edit allowlist, so they need no escalation to the orchestrator. State
  that these pins replace the generator's caret ranges from task 0.1 rather than sitting beside them.

- [x] 0.3 [architect] Record the bindings pipeline design in `design.md` (D0.9): the headless
  export entry point (the target form settled in D0.9 — a cargo example at
  `src-tauri/examples/export_bindings.rs`, satisfying `tauri-ipc-contract`'s headless-export
  requirement), the empty command/event set
  it ships with, why it ships empty (so the scaffold's own gate can pass with no diff, and task 1.3
  extends the same pipeline instead of replacing it), and the two drift checks CI runs
  (`src/bindings.ts`, `src/routeTree.gen.ts`). Record the accepted `tauri-specta` `2.0.0-rc.x` pin
  and its bump discipline: bump it together with `specta`, regenerate `src/bindings.ts`, review the
  bindings diff deliberately (D0.18); `tauri-typed-ipc` is not evaluated first. No ADR number is
  taken by section 0: `docs/adr/0002` stays reserved for task 1.2 (TOOL-3, TOOL-4).

- [x] 0.4 [architect] Define the scaffold's typed IPC surface in `src-tauri/src/ipc/`: the
  `tauri_specta` builder with an **empty** `collect_commands![]` / `collect_events![]` and the
  export function, shaped so task 1.3 adds the P0 commands without changing the pipeline. Stubs
  only, no logic. `cargo check` passes (TOOL-3).

- [x] 0.5 [tauri-security] Set the app identity in the generated `src-tauri/tauri.conf.json` (the
  `--ci` defaults from task 0.1 are placeholders, not the product): product name **`BunScale`**,
  bundle identifier **`com.bunscale.app`**, display name `BunScale` with **no localized display
  name** (English only), and version, all as the single source of truth, iOS and macOS targets
  configured, the macOS window geometry brought to the **1280×800** desktop baseline (no other
  section-0 task owns it, and iOS ignores the window block), and no permission the scaffold does not
  use. These values are settled in `design.md` D0.15 and MUST NOT be invented or made
  platform-specific; this task is not blocked. Both platforms (TOOL-1, TOOL-2).

- [x] 0.6 [tauri-security] Harden the generated default capability and the generated CSP: least
  privilege, no permission the scaffold does not use, one capability per window, no `unsafe-eval`,
  and the iOS and macOS configurations in step. The capability file is the generated
  `src-tauri/capabilities/default.json` that `tauri init` wrote (Q13: `cargo tauri capability new`
  generated nothing in this environment) and the CSP is the one `tauri init` wrote into
  `tauri.conf.json`; both are treated as defaults to narrow, not as a set to extend. The capability
  must also carry its `platforms` key (`["iOS","macOS"]`), so it cannot apply to a platform this
  phase does not ship, and its editor-only `$schema`, which points at the desktop schema, may be
  repointed while you are there. Justify any widening in one line in `design.md`. Both platforms
  (TOOL-1).

- [x] 0.7 [apple-native] Determine the minimum iOS and macOS deployment targets **empirically from
  the installed Xcode and SDKs** and record the floor. Do not hand-pick a version and do not type one
  from memory: inspect the toolchain that is actually installed — for example `xcodebuild -showsdks`,
  and the installed SDK's own deployment-target floor read from its `SDKSettings.plist` after `xcrun
  --sdk <sdk> --show-sdk-path` — and take the lowest target each installed SDK still supports, for
  iOS and for macOS separately. Record both floors, the exact commands and their observed output in
  the change (`design.md` D0.16) so task 0.8 applies them instead of guessing and later phases can
  read them. `apple-native` measures; `architect` writes the D0.16 record, because `design.md` is the
  only place the change's decisions live and `apple-native`'s writable paths cover plugins only, so
  the measured values are handed over rather than the file permission widened. `tauri-security` cannot perform this measurement (no `xcodebuild`/`xcrun` permission),
  which is why this task exists and why it sits before 0.8. No configuration file is changed here —
  this task measures and records; 0.8 applies. Both platforms (TOOL-1, TOOL-2).

- [x] 0.8 [tauri-security] Set the iOS `Info.plist` usage strings, the macOS entitlements and the
  deployment targets on the generated iOS target (the target task 0.1 created with `cargo tauri ios
  init`): a usage string only for a capability that is actually present, only the entitlements the
  scaffold actually uses, and the minimum iOS and macOS deployment targets **measured and recorded by
  task 0.7** (D0.16) — consume that recorded floor, never a chosen version — so the two platforms stay
  in step. Apply the floor through `tauri.conf.json`, which is its **only** home: set
  `bundle.iOS.minimumSystemVersion` **and** `bundle.macOS.minimumSystemVersion` — the macOS key is the
  one that is actually unset today, so without it macOS keeps building against Tauri's inherited 10.13
  default — from which `tauri-build` derives `IPHONEOS_DEPLOYMENT_TARGET` and
  `MACOSX_DEPLOYMENT_TARGET`, so no environment variable is set by hand and no Rust file changes for the
  floor. The iOS half then reaches the generated project by recreation, never by hand-editing anything
  under `src-tauri/gen/apple/` (D0.11). Note how the generator actually behaves before relying on it:
  `cargo tauri ios init` renders **write-once** templates — it skips files that already exist, so
  re-running it against the task-0.1 project changes nothing — while `tauri ios dev` and
  `tauri ios build` synchronise `PRODUCT_BUNDLE_IDENTIFIER` and `PRODUCT_NAME` into the Xcode project
  from `tauri.conf.json` at build time. So when generated **files** must carry a value, recreate the
  generated project instead of re-running init in place (remove `src-tauri/gen/apple/` and
  initialise it again — the CLI's own documented repair, still regenerate-never-hand-edit): this task
  does that for the measured floor and for task 0.5's identity, whose values the task-0.1 project
  held as `com.tauri.dev` and `Tauri App` until this task's recreation: the regenerated project now
  carries `bundleIdPrefix`/`PRODUCT_BUNDLE_IDENTIFIER` `com.bunscale.app` and `PRODUCT_NAME` `BunScale`
  (verified in `project.yml` and in `app.xcodeproj/project.pbxproj`), with `com.tauri.dev` and
  `Tauri App` absent from the whole directory. Record which values are baked at init and which are
  synchronised at build time. Both platforms (TOOL-2).

- [x] 0.9 [release-ci] Add `.github/workflows/ci.yml` on `macos-latest` running exactly the
  `definition-of-done` gate — Rust fmt, clippy, tests, the iOS target compile check,
  `bun run gen:bindings` with a no-diff check on `src/bindings.ts`, the route-tree no-diff check,
  typecheck, lint, unit/component tests and `test:e2e` — with third-party actions pinned and caches
  for `~/.cargo`, `src-tauri/target` and the bun cache. The workflow uses the exactly pinned
  `@tauri-apps/cli` dev dependency (task 0.13), not the `cargo install tauri-cli` of task 0.1, which
  is a one-off local bootstrap step. No signing, no TestFlight and no Android, Windows or Linux job.
  Both platforms (TOOL-1, TOOL-7).

- [x] 0.10 [rust-backend] Reconcile the generated crate with the exact pins from D0.8 and mount the
  `ipc` module: `src-tauri/Cargo.toml` (the generator's caret ranges replaced by the pinned
  versions), `build.rs`, `src/main.rs` and `src/lib.rs` as task 0.1 generated them, wired to the ipc
  module and the builder, `cfg`-clean so later platforms stay possible, and honouring the deployment
  floor recorded by task 0.7 (D0.16) — which reaches Rust **only** through task 0.8's `tauri.conf.json`
  keys, because `tauri-build` emits `MACOSX_DEPLOYMENT_TARGET`/`IPHONEOS_DEPLOYMENT_TARGET` from them,
  so nothing in `Cargo.toml` carries the floor. Note, though, that every gate command is a **dev**
  build, for which `tauri-build` does not set the macOS variable at all, and `cargo check` never links —
  so the **macOS** floor first bites in the first **non-dev** build, task 0.26's `bun run tauri build`,
  where a deployment-target or linker failure must not be misread as an IPC problem. The **iOS** floor
  needs no such caution: it is baked into the committed generated project
  (`IPHONEOS_DEPLOYMENT_TARGET = 15.0`) and its environment variable is emitted for `*ios*` targets
  regardless of dev or prod. This task carries task 0.4's evidence and is the
  first **non-vacuous** compile of `src-tauri/src/ipc/`: 0.4's `cargo check` passed on a tree where nothing
  declared the module, so add the three IPC crates at the D0.8 pins — `tauri-specta =2.0.0-rc.25`
  **with its `typescript` feature**, `specta =2.0.0-rc.25`, and `specta-typescript =0.0.12` as a
  direct dependency, since the module names `specta_typescript::Typescript` and `tauri-specta` does
  not re-export it — declare `pub mod ipc;` in `src/lib.rs`, and mount `ipc::builder()` with commands
  through its `invoke_handler` and events through its `mount_events` **inside this task's own `setup`
  closure** (a helper that installs its own `setup` would clobber it, because `tauri`'s
  `Builder::setup` assigns rather than composes). Then re-run `cargo check` for the host and for
  `aarch64-apple-ios`; those two runs are 0.4's real evidence. `cargo check` passes for the macOS host
  and for `aarch64-apple-ios`. The generator's `tauri-plugin-log` and its debug registration were already
  removed in task 0.1 (Q14, D0.8), together with the `log = "0.4"` dependency their removal orphaned:
  do not reintroduce either, or anything that enables `time/local-offset`. Both platforms (TOOL-1,
  TOOL-8).

- [x] 0.11 [rust-backend] Add the headless bindings export entry point chosen in D0.9 — the cargo
  example `src-tauri/examples/export_bindings.rs`, with its export path resolved against
  `CARGO_MANIFEST_DIR` rather than the working directory — so `bun run gen:bindings` can export
  `src/bindings.ts` from the empty command and event set without launching the app, and so the drift
  checks stay enforceable. Both platforms (TOOL-3).

- [x] 0.12 [rust-backend] Make the crate `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` clean, and put the `src-tauri/tests/` integration
  harness in place with one integration target. While the crate is being cleaned, also give the
  binary-edge `expect("error while building tauri application")` in `src/lib.rs` the invariant comment
  `rust-conventions` requires for an `expect` outside tests (a task-0.10 review finding), or replace it
  with a propagating entry point if that reads better. Both platforms (TOOL-1).

- [x] 0.13 [frontend-logic] Add the root `package.json` (bun), replacing the generator's implied
  frontend values with our stack and exact pins: it exposes exactly the scripts the gate needs —
  `gen:bindings`, `typecheck`, `lint`, `test`, `test:e2e` — plus `dev`, `build` and `tauri`, at the
  exact pinned dependency versions from D0.8 (Biome for lint, D0.17), and it MUST add
  **`@tauri-apps/cli` as an exactly pinned dev dependency** so local runs and CI use that pinned CLI
  instead of the global `cargo install` from task 0.1 and so task 0.22 can run `bun run tauri icon`
  without a global install. The `gen:bindings` script must pass the export example's exit status
  through (`cargo run`'s) and must not expect success output: the exporter exits 0 **silently** when it
  works (task 0.11) and only a non-zero exit signals failure, so a script written to parse output would
  look for success strings that never arrive. Both platforms (TOOL-1, TOOL-8).

- [x] 0.14 [frontend-logic] Add `index.html`, a strict `tsconfig.json` and `vite.config.ts`,
  replacing the generator's config values with our stack: the dev-server port and the build output
  directory MUST match the `--dev-url` and `--frontend-dist` values task 0.1 passed to `tauri init`
  (change whichever side is wrong, never let the two drift), the dev server MUST set
  `server.host = process.env.TAURI_DEV_HOST` so the iOS simulator can reach it, and the router
  plugin MUST be configured. `tauri init` generates no frontend, so no template output is being
  replaced or deleted here — these values are ours from the start. Both platforms (TOOL-1, TOOL-2).

- [x] 0.15 [frontend-logic] Add `src/main.tsx` with the TanStack Query client provider and a minimal
  route tree under `src/routes/` that the router plugin renders into `src/routeTree.gen.ts`; **commit
  the generated `src/routeTree.gen.ts`** — otherwise `git diff --exit-code` against it passes
  vacuously while the file is untracked — then confirm a rebuild leaves it unchanged. Never edit the
  generated file by hand. Both platforms (TOOL-4).

- [x] 0.16 [frontend-logic] Wire `gen:bindings` to the headless export entry point, run it, and
  commit the empty-command-set `src/bindings.ts`; a second run and
  `git diff --exit-code src/bindings.ts` must produce no diff. The commit is what makes that check
  mean anything — `git diff --exit-code` exits 0 with no output on an **untracked** path, so an
  uncommitted file would make the drift check pass vacuously — and task 0.19 must keep the file out
  of `.gitignore` for the same reason. Both platforms (TOOL-3).

- [x] 0.17 [frontend-logic] Configure the test harness: vitest + Testing Library, and a Playwright
  **WebKit** project with the two required viewports 390×844 and 1280×800; wire `test` and
  `test:e2e`, and bootstrap the IPC mock only when `VITE_E2E_MOCK=1` — the `test:e2e` script sets
  that variable itself, so the local and the CI invocation are byte-identical **as far as the mock
  environment goes**: CI injects no mock variable of its own, and the one variable CI does set, `CI`,
  only tightens the harness (`forbidOnly` refuses a committed `.only`, and `reuseExistingServer` stops
  CI reusing a server someone else started) rather than supplying anything the local run lacks. Both
  platforms (TOOL-5).

- [x] 0.18 [frontend-logic] Add the lint/format tooling so `bun run lint` exists and passes, using
  **Biome** for both linting and formatting (D0.17). A second linter and a separate formatter
  (Prettier) are NOT added **as dependencies**: `cargo fmt` already covers Rust, and an extra JS tool
  would churn the exactly pinned dependency set TOOL-8 requires. Because the gate command is
  `biome check .` run from the repository root, the config MUST narrow the checked set (`files.includes`
  or `files.ignore`) so it cannot reach `node_modules/`, `src-tauri/target/**`, `graphify-out/**` or
  `.opencode/skills/**`: with no config that command checked **1512 files and reported 1502 errors**
  across exactly those trees (measured in task 0.13), and it must also decide what to do with **both**
  generated files, `src/bindings.ts` and `src/routeTree.gen.ts`. Measure both before deciding, because
  task 0.15 found them to be in opposite states (Q18): `src/routeTree.gen.ts` already attracts **two**
  diagnostics — one `format` and one `lint/suspicious/noExplicitAny` from the `} as any)` the generator
  emits — while `src/bindings.ts` attracts **none** today, being a single comment line over the empty
  command and event set, and only becomes "a single long emitted line that Biome's formatter would want
  to break" once task 1.3 adds commands. `src/routeTree.gen.ts` is the harder of the two and is the
  reason this task names it: the pinned `@tanstack/router-generator` emits it with
  `quoteStyle: 'single'`, `semicolons: false` and `enableRouteTreeFormatting: true`, while Biome's
  defaults are double quotes and tabs, so unless this task either excludes the generated file from the
  checked set or matches the generator's style, TOOL-4's "regeneration produces no diff" and this
  task's "`bun run lint` passes" fight each other forever (Q16). The generated file asks for the first
  option in its own banner — "you should also exclude this file from your linter and/or formatter to
  prevent it from being checked or modified" — and excluding the two generated artefacts is also the
  reading the project's own "generated files are read-only, regenerate never edit" rule points to;
  state which you chose and why. Every remaining `format` diagnostic in the repository is this task's
  to settle in the same pass, including the ones on `package.json`, `tsconfig.json`, `vite.config.ts`
  and the five files task 0.15 adds. Consider `--error-on-warnings`, since Biome's warnings do not fail
  `check` by default and the gate has exactly one lint command to carry both jobs. Both platforms
  (TOOL-1). The boundary is the frontend tree and its root configs, and it is stated as a decision
  rather than left as an omission: `src-tauri/**` is deliberately outside the allow-list. The two
  hand-owned JSON files there were measured by temporarily listing them — `src-tauri/tauri.conf.json`
  is Biome-clean, and `src-tauri/capabilities/default.json` would need one cosmetic `format` change
  (collapsing `["main"]` and `["iOS", "macOS"]` to inline). They stay out on ownership and remit:
  `capabilities/default.json` is `tauri-security`'s, hardened by task 0.6 so its permission diff is
  reviewable, and a frontend-owned formatter rewriting a security file is the cross-ownership hazard
  D0.11 exists to prevent; and Tauri's own `$schema` and capability validation is a stronger
  correctness signal for those files than JSON formatting. `opencode.jsonc` is likewise a known
  one-line follow-up rather than an oversight. D0.25 records all of it.

- [x] 0.19 [frontend-logic] Extend the root `.gitignore` to cover everything this change's own gate
  commands and tools write — `node_modules/`, `src-tauri/target/`, `dist/` (`bun run build`),
  `test-results/` and `playwright-report/` (Playwright: the first `test:e2e` run already left
  `test-results/.last-run.json` in the tree, before task 0.17's config existed), and `.artifacts/` —
  while **not** ignoring the two generated files `src/bindings.ts` and `src/routeTree.gen.ts`. The file
  already exists and is not empty — it holds the `graphify-out/cache` entry the graph tooling keeps
  there plus whatever the environment has already added while the earlier tasks ran (`node_modules`,
  and `dist` by the time task 0.15 built the frontend; read the file rather than trusting this list,
  and note that 0.19 is what commits those lines, since nothing before it did). So this task extends
  the file and commits what it holds rather than creating it, and it must keep the `graphify-out/cache`
  entry. Both platforms (TOOL-1).

- [x] 0.20 [frontend-ui] Bootstrap the design system: define the three-layer tokens in
  `src/styles/` (primitive → semantic → component) and the component inventory, using
  `ui-ux-pro-max` for direction only — an iOS-first, native-feeling app that scales to macOS, not a
  website — then record the result in `.opencode/skills/design-system/SKILL.md` and set its status
  line to `frozen` (the file currently carries no status line, and that line is what the
  orchestrator's UI precondition reads), so TOOL-6 is satisfied before any UI task that depends on
  the system. These tokens are also the source of the app mark drawn in task 0.22 (D0.19). No
  rendered surface exists yet at this point, so `.artifacts/screens/` evidence is not applicable to
  this task; the first screenshots come from 0.21. iOS-first, both platforms (TOOL-6).

- [x] 0.21 [frontend-ui] Build the `src/components/ui/` design-system primitives from that
  inventory (touch targets ≥ 44pt, safe areas, keyboard inset, AA contrast, screen-reader labels, no
  hover-only affordance) and a placeholder route that renders them. Screenshots at 390×844 and
  1280×800 in `.artifacts/screens/`. Both platforms (TOOL-6). This task also owns the two unstyled
  scaffolding placeholders task 0.15 left behind, because no other task does: the index route's
  component in `src/routes/index.tsx` (which this task's placeholder route replaces) and the root
  not-found surface in `src/routes/__root.tsx` that `tanstack-router-conventions` requires every route
  tree to have. Present both from the primitives, or record in `design.md` why one is deliberately left
  bare — what must not happen is a screen the product can reach that no task has ever been asked to
  design. When you touch `src/routes/index.tsx`, also leave its **file shape** as
  `tanstack-router-conventions` requires it — `createFileRoute` plus a thin `component` reference that
  imports the real component from a feature, not a component body declared inline in the route file.
  The inline component is tolerable only as a scaffold placeholder, so replacing the placeholder must
  replace the shape too, or the convention violation outlives the thing that excused it. Two things
  task 0.20 handed you, so this task does not have to rediscover them: the token layers are already
  wired — `src/main.tsx` imports `./styles/index.css`, which imports the three layers in order — so what
  is missing is the element-level layer, `src/styles/base.css` (reset, `body` background and text,
  font smoothing, `color-scheme`, default focus, scroll behaviour), imported from `index.css` **after**
  the three token files rather than added to them; and the component inventory in the skill's project
  record is 21 rows covering 23 primitives, with the `List`/`ListSection`/`ListRow` trio sharing one row,
  and it is deliberately limited to
  P0 needs with a named deferred list, so build from that list rather than inventing component types.

- [x] 0.22 [frontend-ui] Produce the app mark and generate the platform icon set, replacing the
  Tauri default iconset that task 0.1 shipped as a placeholder: draw a minimal flat mark from the
  design-system tokens of task 0.20 (a squared PNG or SVG with transparency, committed as the source
  artwork), then run `bun run tauri icon <source artwork>` through the exactly pinned
  `@tauri-apps/cli` dev dependency from task 0.13 — no global CLI install and no new permission.
  Confirm the generated iOS and macOS assets match the source mark and that the build still succeeds.
  Screenshots of the mark rendered from the tokens at 390×844 and 1280×800 in `.artifacts/screens/`.
  Both platforms (TOOL-1, TOOL-2).

- [x] 0.23 [test-engineer] Add `src/test/setup.ts` and `src/test/mock-ipc.ts` — the latter
  bootstrapped only when `VITE_E2E_MOCK=1` — with the `QueryClientProvider` (`retry: false`) and
  `createMemoryHistory` helpers the frontend suites use. Both platforms (TOOL-5). Task 0.17 left two
  seams this task closes, and both are stated in D0.24: the `VITE_E2E_MOCK` bootstrap is already wired
  in `src/main.tsx` as an `import.meta.glob('./test/mock-ipc.ts')` seam, so **creating that file is all
  it takes** — do not edit `src/main.tsx` — and this is the task that turns that half from designed into
  verified, because task 0.17 could only exercise the seam with the file absent, where an empty glob and
  an unsupported glob pattern are indistinguishable. The seam deliberately **throws** while the file is
  absent, so before this task landed an E2E run logged that throw from the page instead of booting an
  unmocked app. This task also owns the component layer's **baseline** mechanism, which TOOL-5's first
  two scenarios need and nothing yet provides: `vitest.config.ts` fixes the environment (`jsdom`) but
  deliberately declares no viewport, so a component spec must set the window size itself, and the
  uniform helper that makes "the suite runs at 390×844" and "…at 1280×800" true for every spec belongs
  here, in `src/test/setup.ts` or beside it, with 0.25 then consuming it. This task's **one** cross-file
  step is **`frontend-logic`'s to make in this task's commit**, because `vitest.config.ts` is not in
  `test-engineer`'s edit allow list: adding the single `setupFiles: ['./src/test/setup.ts']` line
  (`globals` stays off: `tsconfig.json` does not list `vitest/globals` in `types`, so specs import from
  `vitest` explicitly) — which landed, and the split is recorded rather than left unowned (Q17 is the
  precedent). The baseline helper above lives in `src/test/`, which `test-engineer` does own. This task must
  land **before 0.25**, because 0.25's E2E specs cannot render the app until the mock file exists.

- [x] 0.24 [test-engineer] Smoke tests for the scaffold artefacts: the Rust integration target
  (`src-tauri/tests/library_surface.rs`, the harness task 0.12 created) runs,
  `bun run gen:bindings` leaves `src/bindings.ts` unchanged, and a build leaves
  `src/routeTree.gen.ts` unchanged. Cover TOOL-3's other two scenarios as well rather than leaving
  them asserted in prose, because they are the ones that prove the drift check actually bites: hand-add
  a marker line to `src/bindings.ts`, assert the gate's own check now fails
  (`git diff --exit-code src/bindings.ts` exits non-zero), then regenerate and assert the marker is gone
  and the check passes again. The orchestrator could not demonstrate this during task 0.16 — neither the
  orchestrator nor `frontend-logic` holds edit permission on a generated file, which is the guardrail
  working, and `git checkout` is likewise unavailable — so this is the first place it can be asserted.
  Two things the assertion must get right, because it deliberately leaves the tree dirty in the middle:
  restore the file inside a `finally` (regenerate on both the success and the failure path) so an
  assertion that fails between the edit and the regeneration cannot leave a hand-edited generated file
  behind for a later check to trip over, and note that `bun run test` is the **last** command of the gate
  while the bindings drift check precedes it, so this in-test mutation cannot invalidate that run's own
  drift check. The two `bun run` assertions belong in a bun step, **never** inside
  the Rust integration target: `cargo test` precedes the drift checks in the gate, so a Rust test that
  reached the exporter or the script would rewrite the very files those checks exist to verify (D0.9).
  Also assert TOOL-8's "Versions are exact" scenario in that same bun step, because task 0.13 created
  the manifests but no test task owned it: that scenario speaks of **the manifests**, so assert both
  halves — read `package.json` and fail on any dependency whose range carries a `^`, a `~` or any other
  non-exact operator, and read `src-tauri/Cargo.toml` and fail on any entry under `[dependencies]`,
  `[dev-dependencies]` or `[build-dependencies]` whose version does not begin with `=` (the crate pins
  are `=x.y.z`, D0.8) — and fail in either half on a key that is missing from the corresponding D0.8
  table. This is the owning test for that scenario — it needs no `invoke`, no viewport and no compiled
  Rust, but it does need task 0.17's `bun run test` wiring and task 0.23's harness, which is why it
  belongs here rather than to task 0.13. Also assert the agreement task 0.14 requires and warns about —
  "change whichever side is wrong, never let the two drift" — because hand-reconciled values are
  exactly what rots: read `src-tauri/tauri.conf.json`, take the port out of `build.devUrl` and the
  directory out of `build.frontendDist`, and fail unless `vite.config.ts`'s `server.port` equals that
  port (and `server.host` is still wired to `TAURI_DEV_HOST`) and its `build.outDir` resolves to the
  same place as `frontendDist`. (TOOL-1, TOOL-2, TOOL-3, TOOL-4, TOOL-8). Also assert the design
  system's **token invariants**, because task 0.20 measured them with a throwaway script that no longer
  exists and nothing at build time guards them: fail on any `var()` reference under `src/styles/**` that
  does not resolve to a defined custom property, on any reference cycle, and on any custom property
  defined twice **in the same cascade context** — a genuine duplicate is two definitions of one name in
  one block, and the check must not flag the intended overrides, because `semantic.css` deliberately
  redefines around 47 tokens inside its `prefers-color-scheme: dark`, `prefers-reduced-motion: reduce`,
  `@supports` and breakpoint blocks, so a naive first-definition-wins check would fail on the very file it
  exists to guard; and assert the documented AA contrast pairs still hold, since the palette was chosen
  around two tight
  ones (`--color-primary` on `--color-primary-subtle-pressed` at 4.52:1, and on
  `--color-surface-sunken` at 4.75:1). This is the assertion that matters most in this task for a reason
  worth stating: Vite does not resolve custom properties, so a typo in a name is emitted verbatim, the
  build stays green, and the browser silently falls back — nothing else in the gate would catch it. The
  skill's project record already promises that a later test can do this; this is that test. (TOOL-6).
  Three scenarios are **deliberately not** covered by a bun test, and are recorded here so they are not
  read as oversights later. The first is the first line of this task — "the Rust integration target
  (`src-tauri/tests/library_surface.rs`) runs" — which **the gate's own `cargo test` step owns**: running
  it again from the bun step would duplicate a command that has already run earlier in the same gate, and
  `cargo test` reports its three tests by name. The second is TOOL-4's "a stale route tree fails the
  build", which cannot be provoked from a test because the gate builds *before* it diffs, so a hand-edited
  tree is regenerated before the check runs — the case that actually bites is "routes changed and the
  regenerated tree was not committed", which is implemented by the gate step itself (task 0.9's CI runs
  `bun run build` then `git diff --exit-code src/routeTree.gen.ts`) and evidenced at 0.15, and 0.26 runs it
  on a clean checkout. The third is TOOL-8's "both platforms are supported", a constraint on *adding* a
  dependency rather than a property a manifest can state, enforced by the per-dependency review recorded
  in D0.8. Also note the suite carries its own negative tests — a spec file that feeds each of the four
  decision checkers a bad input and asserts it fails, with the three remaining exported helpers covered
  positively in the specs that exercise them — because a green
  checks-suite that cannot fail is the same problem as a lint config that matches nothing.

- [x] 0.25 [test-engineer] Component and E2E smoke tests against the placeholder UI at 390×844 and
  1280×800, asserting the adaptive navigation arrangement rather than only that content rendered.
  Both platforms (TOOL-5). Task 0.17 fixed where these live and what they must import: the E2E specs go
  in **`e2e/`** (`playwright.config.ts`'s `testDir`, and the path design.md's scaffold-surface table
  records — the directory does not exist yet, so creating it is part of this task), the component specs
  anywhere under `src/**` matching `*.{test,spec}.{ts,tsx}` (`vitest.config.ts`'s `include`, which
  explicitly excludes `e2e/**` because Vitest's and Playwright's default naming overlap exactly).
  `e2e` is in `tsconfig.json`'s `include`, so these specs are inside the gate's typecheck program —
  a type error in them fails `bun run typecheck`, not just the run. The component specs must use the
  baseline helper task 0.23 adds (TOOL-5's first two scenarios are that requirement, and nothing else
  declares the two baselines for the component layer), rather than each spec setting its own window
  size. **Assert the adaptive navigation arrangement in the E2E layer, and nowhere else**, and
  note why in the spec: jsdom cannot settle it — not because it lacks a CSS pipeline, which the pinned
  jsdom has, but because its media evaluation matches plain media types only (so no `min-width` query in
  the stylesheet ever matches, and even a computed `display` read returns the base rule at both
  baselines) and because the app's stylesheet is never loaded in a Vitest run. So the `display: none` that
  hides one of the two navigation landmarks never applies, both are in the DOM, and a component test's
  role query finds two. Task 0.21 gave each landmark a `data-navigation-arrangement` attribute so the
  assertion does not depend on class names; use it **in Playwright**, where the viewports are real — scope
  by the attribute and assert the computed `display` or visibility there. Reading that value under jsdom
  would be a green assertion proving nothing, which is why there is no component-layer variant of it. The two E2E projects are
  `webkit-compact` (390×844, `deviceScaleFactor` 3, touch) and `webkit-expanded` (1280×800,
  `deviceScaleFactor` 2), one per baseline so a failure names its baseline and `--project=…` runs one.
  The suite needs no native device: it drives the frontend against the app's own dev server, which
  Playwright starts as a child of `playwright test`, with the IPC layer mocked through 0.23's
  `src/test/mock-ipc.ts` — which is why 0.23 must land first. **This task must supply one shared fixture
  (or a `globalSetup`) that asserts the mock is installed, and every E2E spec must consume it** — check
  `window.__TAURI_INTERNALS__.invoke` is a function there, before any spec exercises anything that depends
  on it. It belongs in one place rather than in each spec's own preamble for the same reason the baseline
  helper exists: a per-spec convention is exactly what task 0.23 was created to remove. The reason it
  matters is that a local run can be silently *unmocked* — `playwright.config.ts` reuses an existing dev
  server, and one started without `VITE_E2E_MOCK=1` makes the app render with no mock at all — and a spec
  asserting D0.2's never-loaded surface would then pass *because* the mock is absent. Q23 records the
  mechanism, the two unflagged servers found on port 1420 during task 0.23 and stopped, and why the
  config-level alternative (`reuseExistingServer: false`) was declined. Use the baseline
  helper task 0.23 exports for the component layer rather than setting window sizes by hand, and read its
  documented limits before writing a component assertion: it makes the viewport real to JavaScript
  (`innerWidth`, `clientWidth`) but carries nothing else — no app stylesheet, no layout engine (so
  `getBoundingClientRect()` returns zeroes), jsdom's `devicePixelRatio` of 1 rather than the compact
  project's 3, no touch or coarse-pointer capability, and no `visualViewport`. So it can make a spec *run*
  at a baseline but cannot assert geometry, density, pointer behaviour or the adaptive arrangement — all of
  which belong to the E2E projects above.

- [ ] 0.26 [test-engineer] Run the full `definition-of-done` gate on the scaffold
  (`cargo fmt --check`, `cargo clippy`, `cargo test`, `cargo check --target aarch64-apple-ios`,
  `bun run gen:bindings` with no diff, the route-tree no-diff check, `bun run typecheck`,
  `bun run lint`, `bun run test`, `bun run test:e2e`) and record the results in the change; this is
  the evidence that the workspace builds from a clean checkout. It is also the only place the CSP can
  actually be exercised: every other suite runs against the Vite dev server, where Tauri attaches no
  CSP at all (D0.21), so load a **built** artefact here — `bun run tauri build`, then run the produced
  macOS app — and confirm the shipped policy does not break the UI, recording what was loaded and what
  was observed. iOS cannot evidence this (its dev path is the proxied dev server), which is exactly why
  the built macOS load matters. That same built `.app` is also where the macOS deployment floor becomes
  user-visible: check its `Info.plist` `LSMinimumSystemVersion` against the floor task 0.7 recorded
  (D0.16), because neither the config key nor the build environment proves the shipped value.
  Both platforms (TOOL-1). One more assertion belongs here, because this is the cheapest place the E2E
  seam's **positive** case can be checked and task 0.17 could only exercise it with the mock file
  absent — where an empty glob and an unsupported glob pattern are indistinguishable, `{}` either way:
  with the mock file present, run a **non**-`VITE_E2E_MOCK` `bun run build` and assert the emitted
  bundle contains no `mock-ipc` chunk, then assert the `test:e2e` path does load the mock. That is what
  turns "task 0.23 fills the seam with no edit to `src/main.tsx`" from a design assumption into a
  checked claim, and it also confirms the seam stays inert for the shipped build.

### Manual verification

- [ ] 0.27 [test-engineer] Manual verification (native boot is manual at phase 1 per
  `testing-strategy`): boot the scaffold on an **iOS simulator** and on **macOS** with the app mark
  task 0.22 generated, confirm the first screen is presented and the app does not crash, and record
  the outcome — the simulator/macOS versions used and what was observed — in the change. Both
  platforms (TOOL-2).

## 1. Contract and security surface

- [x] 1.1 [architect] Select and justify the shared-service mechanism in `design.md` (D0.7): one
  shared collection per household, per-item versioning for last-write-wins, per-device local
  caching, forced availability and account reporting, forced allowance/rate-limit reporting where an
  exhausted owner allowance is reportable as **its own condition**, distinct from a generic write
  error (D0.12), and support for client-generated idempotency keys the service deduplicates on
  (D0.14). Reject any candidate that cannot report the SYNC-2 cause list distinctly, or that cannot
  deduplicate a re-issued request by its key. Record the alternatives rejected and why. Confirm it
  supports both iOS and macOS with a one-line dependency justification.
- [x] 1.2 [architect] Record the mechanism, the connectivity-required write model (D1/D0.1) and the
  loaded-data read model (D0.2) in `docs/adr/0002-shared-service-mechanism.md`.
- [x] 1.3 [architect] Define the P0 IPC contract in `src-tauri/src/ipc/`: the commands, events,
  `ServiceStatus`, `SaveCause`, `CommandError`, `Household`, `Member`, `Identity`, `WeightUnit` and
  `Instant` types listed in the design's "IPC contract" section, plus typed stubs that return a
  `not implemented` error. Every state-changing command takes a client-generated `request_id`, and
  the contract states that `CommandError.request_id` is **semantic** — the operation's identity for
  the service's deduplication, which is what makes a retry safe — not a diagnostic field (D0.14).
  Freeze the names for this phase. Because `tauri-specta`'s `derive` feature is exactly what gates its
  `Event` derive macro, enabling that feature is part of this task **if and only if** the contract
  derives `tauri_specta::Event` for its event types: section 0 pins `features = ["typescript"]` alone
  (D0.8), so adding `derive` here is a deliberate pin change rather than a discovery left to the
  implementer.

- [x] 1.4 [tauri-security] Declare the capability, CSP, scope and entitlement changes the chosen
  mechanism requires; keep them least-privilege, add no permission the phase does not use, and keep
  the iOS and macOS configurations in step. This task is also the **owner of the core permissions the
  P0 contract needs**, because section 0 deliberately ends with a capability that grants nothing
  (D0.21): `plugin:event|*` is always ACL-checked, and the P0 event subscriptions (consumer: task 3.6)
  are the first thing that needs it, so grant the narrowest event permission that satisfies the
  contract rather than restoring `core:default` — and the same for any other core module the frontend
  imports (`path`, `window`, `app`). The app's own commands need no entry. If `src-tauri/permissions/`
  or an app ACL manifest is ever introduced, say so here: that alone flips every custom command to
  ACL-gated, which is the trap D0.21 records.

## 2. Backend (Rust)

- [x] 2.1 [rust-backend] Implement the household domain: create with non-blank name validation
  (whitespace-only treated as blank), creator recorded as first member, created household becomes
  active (HS-1). Creation is **idempotent on the command's client-generated `request_id`**: re-issuing
  a create with the same `request_id` returns the household that already exists instead of creating a
  second one (D0.14).
- [x] 2.2 [rust-backend] Implement holding multiple households per person with exactly one active,
  switching the active household, and re-electing an active household when the active one is no
  longer held (HS-3, DM-3).
- [x] 2.3 [rust-backend] Implement the household collection layout: stable opaque ids, per-item
  version/updated-at, and one collection per household that a second load cannot duplicate
  (SYNC-1).
- [x] 2.4 [rust-backend] Implement the reachability-gated save path: refuse writes without
  reachability or a valid account, map each failure to its `SaveCause`, and never report success for
  a write that did not complete (SYNC-2, NFR-1). A save attempt that receives no outcome within the
  **15-second `SAVE_ATTEMPT_TIMEOUT`** is refused with the `timeout` cause — a real cause with a
  bound, not a placeholder (D0.13) — and an exhausted storage allowance is reported as
  `allowance_exceeded` from the service's own report of that condition, never as a generic write
  error (D0.12).
- [x] 2.5 [rust-backend] Implement the read path: serve the locally kept copy first, refresh from the
  shared service on a start or foreground return with connectivity, and produce an explicit
  never-loaded state when nothing was ever loaded (SYNC-3).
- [x] 2.6 [rust-backend] Implement last-write-wins merge on the collection: exactly one surviving
  version per item, no duplicates, and no conflict surfaced to the person as a failure
  (SYNC-5, NFR-2).
- [x] 2.7 [rust-backend] Enforce the storage-boundary invariants: an item belongs to exactly one
  household and is never readable from another, a canonical value is stored only in its canonical
  form, and a write to an entity marked immutable is refused (NFR-10).
- [x] 2.8 [rust-backend] Implement absolute-instant timestamps: default to the creation moment,
  store an explicitly supplied instant unchanged, and round-trip independently of the device offset
  (DM-6).
- [x] 2.9 [rust-backend] Implement identity resolution: account display name when it resolves, the
  locally known member name as first fallback, and an unresolved marker that does not disturb
  callers (SET-2).
- [x] 2.10 [rust-backend] Implement the per-device display weight-unit preference, its regional
  default, and the conversion boundary that applies it only on entry/display (SET-4).
- [x] 2.11 [rust-backend] Implement the single failure-reporting surface: every state-changing
  command returns a `CommandError` with a specific `SaveCause`, a `retryable` flag, a message key and
  the command's `request_id` as the operation's identity — no bare "something went wrong" path
  (NFR-8, D0.14).
- [x] 2.12 [rust-backend] Implement the redacted diagnostics sink: identifiers, counts and outcomes
  only; no rabbit names, note text, photo content or weight values; assert no third-party analytics
  transport exists (NFR-3). (deferred: no production recorder until P3 — see Q37 and task 7.1)
- [x] 2.13 [rust-backend] Implement reachability re-evaluation hooks for both platform lifecycles:
  foreground return on iOS and continuation-while-running on macOS, both `cfg`-clean (NFR-1). Apply
  the fixed bounds of D0.13: a probe that receives no answer within the **5-second
  `REACHABILITY_PROBE_TIMEOUT`** counts as a failed probe, and the status is reported as offline only
  after **3 consecutive failures without an intervening success** (`OFFLINE_FAILURE_THRESHOLD`, NFR-6's
  rule adopted in P0 so P3 inherits it). (deferred: no production recorder until P3 — see Q37 and
  task 7.1)

## 3. Frontend logic

- [x] 3.1 [frontend-logic] Run `bun run gen:bindings` against the frozen contract and add
  `src/features/households/api.ts`, `src/features/settings/api.ts` and `src/features/sync/api.ts`
  wrappers; no `invoke` call anywhere outside these files. Generate a `request_id` for every
  state-changing command in these wrappers and reuse the same one when a failed command is retried
  (D0.14).
- [x] 3.2 [frontend-logic] Add household-scoped query keys and the loaded-data-first cache so a read
  renders the local copy and a refresh replaces it rather than duplicating items (SYNC-1, SYNC-3).
- [x] 3.3 [frontend-logic] Implement the active-household state, house switching, and the route guard
  that skips first-run onboarding whenever a household exists (HS-2, HS-3, HS-4, DM-3).
- [x] 3.4 [frontend-logic] Implement the service-status store: map every `SaveCause` to its message
  key, keep `unknown` until a fresh outcome exists, set the offline state only after **3 consecutive
  failed checks without an intervening success** (D0.13), and hold retry state for a failed save
  (SYNC-2, NFR-8).
- [x] 3.5 [frontend-logic] Implement the Settings data hooks: household list with the active one
  indicated, identity with its fallback chain, and the display-unit preference (HS-4, SET-2, SET-4).
- [x] 3.6 [frontend-logic] Subscribe to `household:changed` and `service:status-changed` and
  invalidate only the affected queries (SYNC-1, NFR-1).
- [x] 3.7 [frontend-logic] Add the i18n scaffold with the P0 string keys, including the localized
  "You"-style placeholder and the cause-specific failure messages, so that no P0 string is
  hard-coded (SET-2, NFR-8). Full German/English coverage stays NFR-5 in P4.

## 4. Frontend UI

- [x] 4.1 [frontend-ui] Build the first-run household creation screen: name field, blank and
  whitespace-only rejection with an inline prompt, submit creating and activating the household
  (HS-1, HS-2). Screenshots at 390×844 and 1280×800 in `.artifacts/screens/`. This is also the task that
  **first assigns `--keyboard-inset`**, which task 0.21 defined and `Screen` already consumes but nothing
  sets (Q21): this is the change's first screen with a real keyboard, so it owns the `visualViewport`
  measurement that writes the token, and it must confirm on a simulator that a focused field is not left
  behind the keyboard — the token cannot be exercised by a desktop WebKit window, where `env()` and the
  keyboard inset both resolve to zero.
- [x] 4.2 [frontend-ui] Build the dashboard shell with the gear-shaped control opening Settings &
  Profile as its own screen, laid out for 390×844 first and 1280×800 second (SET-1). Screenshots at
  390×844 and 1280×800 in `.artifacts/screens/`.
- [x] 4.3 [frontend-ui] Build the Settings & Profile screen: the household list with the active
  household indicated, switching, creating an additional household without onboarding, the
  read-only name with its fallbacks, and the display weight-unit selector (HS-4, SET-2, SET-4).
  Screenshots at 390×844 and 1280×800 in `.artifacts/screens/`.
- [x] 4.4 [frontend-ui] Build the connectivity surfaces: the explicit connectivity-required state
  when nothing was ever loaded, and a visible indication when loadable data is being served from the
  local copy because the service is unreachable (SYNC-3). Screenshots at 390×844 and 1280×800 in
  `.artifacts/screens/`.
- [x] 4.5 [frontend-ui] Build the save-failure surface: cause-specific copy per `SaveCause`, a retry
  action, and no optimistic or success-looking state for a save that did not complete
  (SYNC-2, NFR-8). Screenshots at 390×844 and 1280×800 in `.artifacts/screens/`.

## 5. Tests

- [x] 5.1 [test-engineer] Rust tests for household creation — covers "Create the first household" and
  "Blank name is rejected" (HS-1) and the onboarding guard scenarios (HS-2).
- [x] 5.2 [test-engineer] Rust tests for multiple households — covers "A second household coexists
  without mixing data", "Switching the active household changes the presented data" (HS-3),
  "Selecting a household leaves exactly one active" and "Active household is replaced when it is no
  longer held" (DM-3).
- [x] 5.3 [test-engineer] Rust tests for collection identity — covers "Every member resolves the same
  items" and "Refreshing does not duplicate an item" (SYNC-1), including that a write re-issued with
  the same `request_id` after the service already accepted it adds no second item (D0.14).
- [x] 5.4 [test-engineer] Rust tests for the write path — covers "Save refused without connectivity",
  "Save succeeds when the service is reachable" and "Save failure names its cause and offers a
  retry" (SYNC-2), plus "Writes are accepted only while reachable and identified" and "Writes are
  refused while unreachable, reads are not" (NFR-1), including that an exhausted allowance is named
  as its own cause (D0.12), that a stalled attempt resolves with the `timeout` cause inside the
  15-second bound, and that the status reports offline only after three consecutive failures
  (D0.13).
- [x] 5.5 [test-engineer] Rust tests for the read path — covers "Previously loaded data is shown while
  unreachable", "A start with connectivity refreshes loaded data" and "Nothing loaded and
  unreachable" (SYNC-3).
- [x] 5.6 [test-engineer] Rust tests for conflict convergence — covers "Two members edit the same
  item", "Convergence does not duplicate or fail" (SYNC-5) and "Concurrent edits resolve without
  failing", "Resolution leaves no duplicate" (NFR-2).
- [x] 5.7 [test-engineer] Rust tests for the storage boundary — covers "No item is readable from
  another household", "Canonical representation survives presentation changes" and "Writes to
  immutable entities are refused" (NFR-10).
- [x] 5.8 [test-engineer] Rust tests for timestamps — covers "A timestamp defaults to now", "A
  supplied instant is stored unchanged" and "Stored instants do not shift with the device offset"
  (DM-6).
- [x] 5.9 [test-engineer] Rust tests for diagnostics — covers "Diagnostics carry no content" and
  asserts no third-party analytics transport is configured (NFR-3), including a case where a save
  fails with a health-payload-like input.
- [x] 5.10 [test-engineer] Rust tests for the failure surface — covers "A failed save is visible and
  specific" and "No bare failure messages", asserting every `SaveCause` maps to a distinct message
  key and every P0 command has a cause-specific error path (NFR-8).
- [x] 5.11 [test-engineer] Rust tests for identity and the display unit — covers "Resolved account
  name is shown read-only" and "Unresolved identity falls back without breaking the screen" (SET-2),
  plus "Unit choice is stored on the device", "Changing the unit leaves stored values unchanged" and
  "Default unit follows the device region" (SET-4).
- [x] 5.12 [test-engineer] Component tests for onboarding and Settings — covers "Create the first
  household", "Blank name is rejected" (HS-1), "Create an additional household without onboarding",
  "All households are listed with the active one indicated" (HS-4) and "Gear control opens Settings
  as its own screen" (SET-1).
- [x] 5.13 [test-engineer] Component tests for the connectivity and failure surfaces — covers
  "Nothing loaded and unreachable", "Previously loaded data is shown while unreachable" (SYNC-3),
  "Save refused without connectivity", "Save failure names its cause and offers a retry" (SYNC-2)
  and "A failed save is visible and specific" (NFR-8).
- [x] 5.14 [test-engineer] E2E (Playwright WebKit, mocked IPC, 390×844): launch with no household →
  onboarding refuses a blank name → create → dashboard → gear → Settings → switch household; covers
  the `[mobile]` scenario of SET-1 and the US-HS-01/US-HS-04 flows.
- [x] 5.15 [test-engineer] E2E: offline launch with previously loaded data, and offline launch with
  nothing ever loaded, asserting the explicit connectivity-required state; covers the two SYNC-3
  offline scenarios.
- [x] 5.16 [test-engineer] E2E: save failure for each cause with a visible cause-specific message and
  a working retry, asserting the entry is never shown as saved; covers SYNC-2 and NFR-8.
- [x] 5.17 [test-engineer] E2E at 1280×800: gear → Settings as its own screen with the dashboard still
  reachable, and household switching; covers the `[desktop]` scenario of SET-1 and HS-3.
- [ ] 5.18 [test-engineer] Run the full gate (`cargo fmt --check`, `cargo clippy`, `cargo test`,
  `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` with no diff, `git diff
  --exit-code src/routeTree.gen.ts` after a build, `bun run typecheck`, `bun run lint`,
  `bun run test`, `bun run test:e2e`) and record the results in the change.

## 6. Review

- [ ] 6.1 [reviewer] Review the P0 diff for contract conformance: frontend touches IPC only through
  generated bindings and `api.ts`, no optimistic write path exists, and no offline write queue was
  introduced (D1). Every state-changing command carries a client-generated `request_id` and a retry
  reuses it (D0.14). Confirm for every UI task the touch targets, the safe areas and the adaptive
  layout on both baselines.
- [ ] 6.2 [reviewer] Verify the delta contains exactly HS-1…HS-4, SYNC-1/2/3/5, SET-1/2/4,
  NFR-1/2/3/8/10 and DM-3/DM-6 in the five product capabilities, **plus exactly TOOL-1…TOOL-8 in
  `project-toolchain`** — the `TOOL-` set is the section-0 workspace-scaffold enabler and is in
  scope by decision of the product owner (it has no source-document ID; see the delta header and
  `proposal.md`). Anything beyond those two lists, and any requirement implemented without a
  scenario, is scope creep and fails the review; confirm no requirement was added to a capability
  this change does not own. Verify the honesty of the offline/loaded-data and failure surfaces
  against NFR-1 and NFR-8, that the app identity matches D0.15, that the deployment targets are the
  floor task 0.7 measured rather than a chosen number (D0.16), that the icon set is the one task 0.22
  generated (D0.19, no Tauri default icon remains), and verify every UI task's `.artifacts/screens/`
  screenshots are present at 390×844 and 1280×800.

## 7. Follow-up fixes (owner-approved 2026-10-21)

These close the recorded minors Q37, Q42, Q44, Q45, Q46, Q48 and Q49. The product owner approved them
on 2026-10-21 **before this change is archived**, so none of them arrives after the fact as a review
finding, and all seven land inside the same P0 delta. Q37 is closed by a documentation correction plus
an explicit deferral of the production recorder (`questions.md` → Q37); the other six are small fixes
in code each already has an owner for. Every task below is one owner, one commit and under an hour, and
names the scenario or the `questions.md` entry it closes.

- [ ] 7.1 [rust-backend] Correct the module docs in `src-tauri/src/diagnostics/mod.rs` and
  `src-tauri/src/sync/mod.rs` so they state that `diagnostics::record` has **no production call site
  yet** and that the sink is exercised only by tests, instead of naming the save path (2.4), a load
  (2.5) and the probe (2.13) as its recorders or each other as its owner. Documentation only: no
  behaviour change, no new recorder, no call site wired — the three production recorders stay deferred
  to the change that first needs diagnostics (P3). Closes Q37; NFR-3's "Diagnostics carry no content"
  keeps its current test-composed evidence.
- [ ] 7.2 [rust-backend] Extract the settings pair's `PreferenceError → CommandError` mapping in
  `src-tauri/src/ipc/commands/settings.rs` as a pure helper (Q42's suggested
  `local_fault_cause(request_id) -> CommandError`) and assert it in a unit test: cause `unknown`, the
  message key, and `retryable: false`. This is the one cause path asserted nowhere today — 5.10
  records it as a named gap — and the test reads the real mapping rather than a re-implementation.
  Closes Q42; NFR-8's "No bare failure messages".
- [ ] 7.3 [test-engineer] Assert at the component layer that a display-unit failure (cause `unknown`,
  non-retryable) shows the re-select guidance and offers **no retry control**, so "the recovery is not
  a retry button" is proved by a spec instead of recorded in a design note. Extend the existing
  Settings component specs; no product code changes unless the assertion fails. Closes Q44; SET-4's
  "Display weight unit is per-device and applied at the boundary".
- [ ] 7.4 [frontend-logic] Make the Settings read-failure retry refetch the active household together
  with the household list — the held-set pair — so the Active marker cannot be lost after a refresh
  when the active id is no longer in the list. Unreachable in P0 today and made reachable by P3's
  deletion, which is why it lands now rather than being inherited. Closes Q45; HS-4's "All households
  are listed with the active one indicated".
- [ ] 7.5 [frontend-ui] Give the Settings read retry a busy state while the refetch is in flight —
  the disabled control or the inline indicator, whichever the conventions already use for a pending
  read — so a second tap cannot land and the screen says it is working. Screenshots at 390×844 and
  1280×800 in `.artifacts/screens/` if the rendered result changes. Closes Q46; HS-4's Settings read
  path.
- [ ] 7.6 [frontend-ui] Make `LoadingIndicator` announce copy from the i18n catalog — a required
  `label`, or a `messageKey` resolved through `t()` — update every call site including the pending
  entry surface that passes none, and **remove the hard-coded English "Loading" fallback**, so no P0
  string is announced outside the catalog. Confirm the visual result is unchanged at both baselines
  and capture screenshots. Closes Q48; 3.7's no-hard-coded-copy rule. **Depends on 7.8**: the four
  loading-state strings this needs do not exist in the catalog yet, and `src/features/i18n/**` is
  `frontend-logic`'s.
- [ ] 7.7 [frontend-logic] Wire the first real `recordFailedSave` caller — the household creation flow
  — and mount `SaveFailureSurface` at the route shell, so `src/features/sync/save-retry.ts` is no
  longer dead code. Keep `SaveFailureNotice` as the **single** failure presentation rather than showing
  two surfaces for one failed save, and update `questions.md` Q49 when done. Closes Q49; SYNC-2's "Save
  failure names its cause and offers a retry".
