# Questions — add-shared-household-foundation (P0)

**Settled before implementation:** Q1–Q9 were answered by the product owner and recorded as decisions
in `design.md` (D0.12 … D0.20); they are tabulated below and were never reopened.

**Raised during implementation and resolved the same day (2026-10-16):** Q10–Q12, by the product
owner. The decisions the loop follows are in "Resolutions" below; the evidence each was raised on is
kept in "History".

**Current open items:** Q15 (an environment blocker for task 0.27's iOS simulator boot, recorded rather
than worked around) and Q16 (two configuration decisions task 0.14 recorded instead of settling alone —
the explicit `es2021` build target, and the generated route tree's style against Biome's defaults; both
need an owner, neither blocks 0.14). Q17 and Q18 are implementation findings rather than ambiguities,
recorded because they change what later tasks must do: Q17 hands the git half of TOOL-4's drift check to
the orchestrator, which made it meaningful in task 0.15's commit, and Q18 measures what task 0.18 will
actually find. Q19 corrects a completeness claim in task 0.18's own text — the checkable surface is not
just the frontend plus Rust — and records the measured boundary decision (D0.25) rather than leaving it
to be rediscovered. Q13 was resolved by the product owner on 2026-10-16 (harden the generated
capability, no second file) and Q14's decision is implemented, with the obligations it carries
recorded against task 0.10.

New ambiguity discovered while implementing is appended here as a new question, per project hard
rule 7.

| Former question | Topic | Decision |
|---|---|---|
| Q1 | storage-allowance exhaustion must be reportable distinctly | `design.md` D0.12 |
| Q2 | concrete bounded times and the offline failure threshold | `design.md` D0.13 |
| Q3 | client-generated request ids, deduplicated by the service | `design.md` D0.14 |
| Q4 | bundle identifier and display name | `design.md` D0.15 |
| Q5 | minimum iOS/macOS deployment targets are derived from the installed SDKs | `design.md` D0.16 |
| Q6 | Biome as the single lint/format tool | `design.md` D0.17 |
| Q7 | the `tauri-specta` `2.0.0-rc.x` pin is accepted, together with `specta` | `design.md` D0.18 |
| Q8 | the app mark is generated after the design tokens exist | `design.md` D0.19 |
| Q9 | `ios init` keeps its documented position, with a fallback | `design.md` D0.20 |

## Resolutions (2026-10-16, product owner)

### Q10 — decided: fix the harness configuration first, then delegate

Task work was paused until the project-defined subagents could execute their allowlisted commands.
The product owner then granted the orchestrator `edit` on `.opencode/agents/*.md` (so the allow lists
could be repaired) plus `bun install *`, `rustup *`, `xcodebuild *`, `xcrun *` and `SETUP.md`.

**Verified outcome:** specialist sessions now run their allowlisted commands. `cargo install
tauri-cli --help` and then the full `cargo tauri init …` invocation both ran in an `architect`
session, and `cargo --version` / `pwd` were still correctly refused there — so the allow list is
enforced again, last-match-wins, exactly as the V2 documentation describes.

**One lasting finding, recorded in `.opencode/agents/architect.md`:** `cargo tauri …` needs a `tauri *`
rule *in addition to* `cargo tauri *`. With only `cargo tauri *` present, `cargo tauri --version` was
refused while `cargo tauri init --help` and `cargo tauri init --ci` ran; adding `tauri *` made the
full `tauri init` invocation with all its flags run. The scanner's exact form is undocumented, so the
rule stands on that observation. The orchestrator's own session reaches the same commands through
`cargo *` and did not need the extra rule.

### Q11 — decided: create `SETUP.md` now — done

Root `SETUP.md` documents the macOS setup TOOL-1's scenario and task 0.1's precondition refer to:
Xcode with a simulator runtime, the `aarch64-apple-ios` rustup target, bun, and the one-off local
`cargo install tauri-cli --version "^2" --locked` bootstrap. It was created through the built-in
`general` agent — the only session with write access to a root path, since no project agent's edit
allow list owns a root-level file — and committed with these resolutions. It carries the iOS-target
check (`rustup target list --installed`), the `rustup target add aarch64-apple-ios` step, the
`bun install` step and the `definition-of-done` gate list, and points the deployment-target question at
D0.16 rather than naming a version. The ownership gap it exposed is **closed** by the same grant:
`SETUP.md` is now in the orchestrator's edit allow list.

### Q12 — decided: extend the orchestrator's allow list, and close the specialist gaps the same way

| Command family | Needed by | Status |
|---|---|---|
| `bun install` | tasks 0.13–0.19 (materialise `node_modules` from the hand-written exact pins) and every `bun run typecheck\|lint\|test\|test:e2e` after them | granted for the orchestrator; **`frontend-logic`'s own entry landed in `fb5a375`, and task 0.13 confirmed it** — both `bun install` and `bun install --frozen-lockfile` ran there |
| `rustup` | task 0.1's precondition, the `aarch64-apple-ios` target check, `apple-native`'s task 0.7 | granted for the orchestrator; add for `apple-native` only if it records the installed target list itself |
| `xcodebuild`, `xcrun` | task 0.7 (measure the deployment floor from the installed SDKs); re-verified by the orchestrator during the gate | already allowed for `apple-native`; granted for the orchestrator for gate verification |

Task 0.7 stays owned by `apple-native`. `rustup target list --installed` on this machine reports
`aarch64-apple-ios` and `aarch64-apple-ios-sim`, and Xcode 27.1 is installed, so task 0.1's real
preconditions hold.

## Open items

### Q13 — `cargo tauri capability new` cannot run headless, and its artefact already exists (resolved 2026-10-16: harden the generated capability, no second file)

Task 0.1 step 4 ran and failed: `Error failed to prompt input: IO error: not a terminal`, exit 1,
nothing generated. Two facts behind it:

- the command was invoked exactly as the task text gives it — `cargo tauri capability new`, with no
  identifier — and fell back to an interactive prompt, which fails without a TTY. The installed CLI does
  accept `[IDENTIFIER]`, `--description`, `--windows`, `--permission` and `-o` (verified with `--help`),
  so the command is runnable headless when given an identifier; this change simply defines none to give
  it;
- `cargo tauri init` (step 2) had already written `src-tauri/capabilities/default.json` (identifier
  `default`, `permissions: ["core:default"]`, `windows: ["main"]`) — precisely the file D0.11's
  hand-owned table assigns to task 0.6 to harden.

**Resolution taken:** step 4 is recorded in `design.md` (D0.11, "Generator pass — observed output") as
satisfied by generator output, and no second capability file was invented. **Decided (product owner,
2026-10-16):** task 0.6 hardens the generated `src-tauri/capabilities/default.json` to least privilege
and does **not** create a second capability file — with a single window there is nothing to split, and
an extra file would only widen the surface 0.6 exists to narrow. Harness note: an unargued
`capability new` needs a TTY, so an agent session must always pass an explicit identifier if the step is
ever re-run. Task 0.6's text has been corrected accordingly (`tasks.md`).

### Q14 — the generator's logging plugin broke the iOS compile check; the decision is taken and implemented

`cargo check --manifest-path src-tauri/Cargo.toml --target aarch64-apple-ios` failed on the baseline as
generated (reproduced independently by the orchestrator):

```
error[E0425]: cannot find function `mach_task_self` in crate `libc`
  --> num_threads-0.1.7/src/apple.rs:34:31
error: could not compile `num_threads` (lib) due to 1 previous error
```

Root cause, read from the resolved sources rather than inferred: the generator's `Cargo.toml` carried
`tauri-plugin-log = "2"` (resolved to 2.10.1), which depends on `time` with the `local-offset` feature;
that feature pulls `num_threads` (declared for `cfg(target_family = "unix")`, so iOS is included), and
`num_threads 0.1.7` calls `libc::mach_task_self()`, which `libc 0.2.190` defines only under
`#[cfg(target_os = "macos")]`. The host build passed solely because that gate is open on macOS.

**Decision taken (project rule 6, recorded in `design.md` D0.8 and D0.11):** drop `tauri-plugin-log`
from the generated crate and its debug-only registration from `src/lib.rs`, as a refinement of the
generated baseline. No requirement in this change asks for logging and the generated
`capabilities/default.json` grants only `core:default`, so the plugin was not even reachable from the
frontend. Pinning around it was rejected: that keeps an unrequested plugin and adds pins or a patch for
no product benefit. No replacement dependency was added.

**Implemented and verified:** the removal is in task 0.1's own commit; both `cargo check` commands
(host and `aarch64-apple-ios`) pass on the committed tree, so TOOL-1's "[ios] The iOS target compiles"
scenario is met. Two obligations carry forward:

1. **Task 0.10** must not reintroduce `tauri-plugin-log` (or any dependency that enables
   `time/local-offset`) when it imposes the exact pins — `time` itself stays in the graph via `cookie`,
   `plist`, `serde_with` and `tauri-build`, and only the `local-offset` feature was the problem. The
   plugin's removal also orphaned the generated `log = "0.4"` dependency; the same rule-6 argument
   applied to it, so it was dropped in the same commit — no `log::` reference remained anywhere in
   `src-tauri/src/`, and the `log` crate itself stays in the graph only as a transitive dependency of
   other crates.
2. **Review expectation:** the record of this deviation lives in `design.md` D0.8/D0.11 and here; the
   generator's own artefact list is otherwise unchanged.

### Q15 — CoreSimulator does not respond in this environment (open; blocks task 0.27's iOS evidence)

Task 0.7's measurement found it, and it is not a defect in this change: `xcrun simctl list runtimes` and
`xcrun simctl list runtimes -j` both hang with no output (killed at 120 s, then again at 300 s), so the
installed simulator runtimes cannot be inventoried from an agent session. Task 0.27 requires booting the
scaffold on an **iOS simulator** and recording what was observed, so its iOS half cannot be evidenced
until CoreSimulator responds.

What is measured and unaffected: the simulator SDK's own floor equals the device floor —
`SupportedTargets.iphoneos.MinimumDeploymentTarget` is `15.0` in `iPhoneOS27.1.sdk`, and
`SupportedTargets.iphonesimulator.MinimumDeploymentTarget` is `15.0` in `iPhoneSimulator27.1.sdk` —
and the generated project already pins `ARCHS: [arm64]` with
`EXCLUDED_ARCHS[sdk=iphoneos*]: x86_64`, so an arm64 simulator build is the shape to expect.

Nothing is worked around and nothing is claimed: 0.7's floors come from the SDKs' own metadata, not from
a simulator run. If CoreSimulator stays unresponsive, 0.27's iOS half must be reported as blocked rather
than recorded from a `tauri ios dev` run.

### Q16 — two decisions task 0.14's config carries that the gate cannot decide (open, `frontend-logic`, 2026-10-17)

Raised while implementing task 0.14 (`index.html`, `tsconfig.json`, `vite.config.ts`). Neither item
blocks 0.14 — the task's gate is green — but both need an owner before the tasks that act on them.

**1. `build.target: 'es2021'` in `vite.config.ts` is not in 0.14's required-value list.** Vite 8's
default is `'baseline-widely-available'`, which is Chrome/Edge 111+, Firefox 114+, **Safari 16.4+**
(read from the pinned `vite@8.3.4` `BuildEnvironmentOptions.target` doc comment). The floor task 0.7
measured is iOS 15.0 / macOS 12.0 (D0.16) — Safari 15, ES2021-complete but not all of ES2022 — so the
default would emit syntax that floor cannot parse, which is what TOOL-2's `[ios]` launch scenario is
about. `es2021` was therefore set explicitly, in one line and reversibly, and `tsconfig.json`'s
`target`/`lib` were set to `ES2021` to agree with it. Recorded rather than silently decided: if the
intent is instead to **raise** the deployment floor, that belongs to D0.16's owner and not to this
file, and the two settings must move together. **The guarantee is narrower than it reads, and that is
worth stating rather than discovering at 0.27:** `build.target` is consumed only by the build-time
transpile step, so it governs what ships — `tauri ios dev` serves source untransformed, and
`tsconfig.json`'s `target` is `noEmit`-only and therefore type-level. A source file using an ES2022
construct that `tsc` accepts at `ES2021` would still reach a Safari-15 simulator in dev even though the
build lowers it. So the floor is enforced for build output, not for the dev loop, and task 0.15's code
must not lean on the config to keep dev-loop syntax inside the floor.

**2. The generated route tree's style and Biome's defaults will fight.** The pinned
`@tanstack/router-generator`'s defaults are `quoteStyle: 'single'`, `semicolons: false`,
`enableRouteTreeFormatting: true`; task 0.14 deliberately does not override them. Biome's defaults are
double quotes and tabs, which is why `bun run lint` already reports `format` diagnostics for
`tsconfig.json`, `vite.config.ts` and `package.json` — and why it would report them for
`src/routeTree.gen.ts` once task 0.15 commits it. One of two tasks must settle it: task 0.15 (by
overriding the generator's style) or task 0.18 (by excluding the generated file from Biome's checked
set, as TOOL-4's "regeneration produces no diff" and the "generated files are read-only" rule point
to). Task 0.18's text has been extended to name **both** generated files, since it already had to
decide the same question for `src/bindings.ts`.

**Measured at 0.14, no decision needed:** `bun run test` no longer fails as "No test files found" — it
now exits 1 inside `@tanstack/router-generator`'s generator (`ENOENT … scandir '…/src/routes'`)
because vitest resolves `vite.config.ts` and configuring the plugin *is* 0.14's requirement. That cause
clears the moment task 0.15 creates `src/routes/`; 0.17's missing wiring is still why the command is
red overall. `bun run lint` moved from 1512 files / 1502 errors to 1516 / 1505, and the only
diagnostics naming a file 0.14 owns are the two `format` ones above — no lint or correctness
diagnostic — so `lint` remains attributable to 0.18.

### Q17 — a specialist session cannot stage a file, so TOOL-4's drift check was still vacuous at 0.15 (open, `frontend-logic`, 2026-10-17)

Raised while implementing task 0.15. The task prescribes staging `src/routeTree.gen.ts` and then
running a second build plus `git diff --exit-code src/routeTree.gen.ts`, because that command exits 0
with no output on an **untracked** path and the check would otherwise mean nothing.
`frontend-logic`'s allow list grants `bun run *`, `bun add *`, `bun install *`, `git diff *`,
`git status *` and `graphify query *` only: `git add src/routeTree.gen.ts` was refused at the tool
layer, so the precondition the task asks for cannot be created in a specialist session, and
`graphify update .` is refused for the same reason, so the graph is not refreshed for the files 0.15
adds. Task 0.15 produced substitute evidence (four builds, a byte-identical `src/routeTree.gen.ts`, and
identical bundle content hashes across all four), but the **git half of TOOL-4's scenario is the
orchestrator's to make meaningful**, and it did: the file is committed with task 0.15, after which the
rebuild-and-diff check is a real assertion. Tasks 0.24 and 0.26 assert the same check again, and the
graph is refreshed after the commit.

**Who owns this, recorded once so it stops being re-improvised per task.** The orchestrator, and no
allow-list widening is needed for it: by `AGENTS.md` hard rule 4 and the loop this project runs on,
**staging and committing are the orchestrator's steps, never a specialist's** — a specialist implements
and runs the gate, and a task whose text says "commit the generated file" is satisfied when the task's
commit contains that file, which the orchestrator makes. So the git half of 0.15's evidence, and the
equivalent halves of 0.16 (which commits `src/bindings.ts`) and 0.19 (which commits the `.gitignore`
lines), are orchestrator work by design rather than a gap in three different task texts. Widening
`frontend-logic`'s allow list with `git add *`/`git commit *` was considered and rejected: it would let
a specialist stage or commit outside its own task boundary, for no evidence the orchestrator cannot
produce itself, and no other thing `frontend-logic` owns needs git write access. What was genuinely
missing was this record — the three task texts never say who performs the git half.

### Q18 — the generated route tree does attract a lint diagnostic, and `src/bindings.ts` does not (measured for 0.18, `frontend-logic`, 2026-10-17)

Measured with the pinned `@biomejs/biome` 2.5.15 and no `biome.json`, which is the state task 0.18
inherits. `bun run lint` (`biome check .`) exits 1 at 1526 files / 5090 errors / 3083 warnings / 170
infos, up from 0.13's 1512/1502 and 0.14's 1516/1505, and the new `src/` files are why the count moved.
Attribution from `--reporter=summary` and `--reporter=github`:

- `src/routeTree.gen.ts` attracts **two** diagnostics — one `format` and one
  `lint/suspicious/noExplicitAny` from the `} as any)` the generator itself emits — so Q16's prediction
  holds and the generated tree keeps fighting Biome's defaults until 0.18 excludes it. The file asks to
  be excluded in its own banner: "you should also exclude this file from your linter and/or formatter to
  prevent it from being checked or modified".
- `src/bindings.ts` attracts **nothing**, neither lint nor format, because today it is a single comment
  line over the empty command and event set. Task 0.18's text describes it as "a single long emitted
  line that Biome's formatter would want to break", which describes what it becomes once task 1.3 adds
  commands and not what 0.18 will actually find; that text has been corrected.
- The five hand-written files 0.15 adds appear **only** in the format list, for the same reason
  `tsconfig.json` and `vite.config.ts` already did at 0.14 (two-space indent and single quotes against
  Biome's tab and double-quote defaults). One `assist/source/organizeImports` error on
  `src/routes/__root.tsx` fired on the first run and was fixed by ordering the imported names, and the
  next run confirmed it gone. Every `format` diagnostic in the repository is 0.18's to settle in one
  pass, whichever way it settles them.

### Q19 — task 0.18's "every remaining format diagnostic" clause assumed a repository that is frontend plus Rust (resolved by measurement, `frontend-logic` + orchestrator, 2026-10-19)

Task 0.18 says "Every remaining `format` diagnostic in the repository is this task's to settle in the
same pass". That clause reads as though the checkable surface were the frontend tree plus Rust, which
`cargo fmt` covers. It is not: `src-tauri/` carries two first-party **hand-owned JSON** files that Biome
2.5.15 can parse — `tauri.conf.json` (D0.11 makes it hand-owned, task 0.5 set it) and
`capabilities/default.json` (hand-owned, hardened by task 0.6 per D0.21) — and neither is Rust nor
generated, so the allow-list's completeness claim was false as first written.

Measured by temporarily listing both and running `bun run lint` (`Checked 14 files`, proving they were
genuinely checked, then reverted byte-for-byte): `tauri.conf.json` is **Biome-clean**;
`capabilities/default.json` has **one `format` diagnostic and no lint diagnostic**, wanting its two
short arrays collapsed to inline (`["main"]`, `["iOS", "macOS"]`).

**Resolution recorded as decision D0.25, not left as an omission:** `src-tauri/**` stays outside the
allow-list, on ownership (`capabilities/default.json` is `tauri-security`'s, hardened so its permission
diff is reviewable, and a frontend-owned formatter rewriting a security file is the cross-ownership
hazard D0.11 prevents) and on remit (Tauri's `$schema` and its capability validation are a stronger
correctness signal for those files than JSON formatting). `opencode.jsonc` remains the third
checkable-but-unlisted file, a known one-line follow-up. The task text has been corrected accordingly,
so no later task inherits the false completeness claim.

## History — how Q10–Q12 were raised (2026-10-16, orchestrator)

### Q10 — Specialists could not execute shell commands in this harness

Task 0.1 was delegated to `architect` and completed with no command executed: every `shell` call in a
subagent session was refused at the tool layer with
`{"error":{"type":"permission.rejected","message":"Permission denied: shell"}}`. It was not an
allow-list miss: the same refusal came back for `cargo install tauri-cli --version "^2" --locked`,
which `.opencode/agents/architect.md` explicitly allows, and a second probe in a `rust-backend` session
refused both `cargo --version` and `openspec list`, which that agent's own allow list permits.

The orchestrator's diagnostic narrowed it: the built-in `general` subagent — the one agent with **no**
`permissions` block — *could* run shell (`openspec list` succeeded in its session), while every
project-defined subagent was refused **every** command, even though file edits in subagent sessions did
work (the `architect` session successfully edited `design.md`). The orchestrator's own primary session
enforced its allow list throughout (`cargo *`, `bun run *`, `openspec *`, `graphify *` and the `git`
verbs ran; `ls`, `pwd` and piped commands were refused). Ordering was ruled out as the cause:
`rust-backend.md` allowed `cargo *` after the blanket deny and `cargo --version` was still refused
there. Resolved as above; no task output was ever faked.

### Q11 — `SETUP.md` did not exist

TOOL-1's scenario ("A fresh checkout passes the gate [macos]") says the repository is "prepared per
`SETUP.md`", and task 0.1 lists "the `aarch64-apple-ios` rustup target (`SETUP.md`)" as a precondition,
while the root held `AGENTS.md`, `opencode.jsonc`, `openspec/`, `docs/` and `graphify-out/` only.
Resolved as above; task 0.1's real precondition was checked directly instead (Xcode 27.1 present,
`aarch64-apple-ios` installed).

### Q12 — Orchestrator gate-command permissions were incomplete

No agent was permitted to run `bun install` (required before any `bun run typecheck|lint|test|test:e2e`
can pass), `xcodebuild`/`xcrun` (task 0.7's deployment-floor measurement) or `rustup` (the iOS target
precondition). Resolved as above; the orchestrator can now run all of them, which is also how the two
`cargo check` results recorded in Q14 were reproduced independently.
