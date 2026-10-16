# Questions — add-shared-household-foundation (P0)

**No open questions remain.** The former Q1–Q9 were settled by the product owner; each is recorded as
a decision in `design.md` (D0.12 … D0.20) and reflected in the tasks and scenarios that consume it.
No task in this change is blocked on an answer, and settled decisions (D1–D6) were never reopened
here.

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

New ambiguity discovered while implementing is appended here as a new question, per project hard
rule 7; nothing has been added since Q9.

## Resolutions (2026-10-16, product owner)

**Q10 — decided: fix the harness configuration first, then delegate.** Task work stays paused until
the project-defined subagents can execute their allowlisted commands; task 0.1 stays unchecked.

Diagnostic the orchestrator added after the refusal, because it narrows the fix sharply: the built-in
`general` subagent — the one agent with **no** `permissions` block — *can* run shell (`openspec list`
succeeded in its session, with the graphify plugin notice and the change list returned), while
`architect` and `rust-backend` are refused **every** command. So child sessions are not
shell-restricted by the harness; the refusal comes from the project's own agent files. Every one of
the eleven `.opencode/agents/*.md` files opens its `permissions` array with

```
- action: edit
  resource: "*"
  effect: deny
- action: shell
  resource: "*"
  effect: deny
```

and then lists specific `allow` entries. Ordering is **ruled out** as the cause: `rust-backend.md`
allows `cargo *` *after* that deny and `cargo --version` was still refused in its session, while the
identical array shape resolves correctly in the orchestrator's primary session (`cargo *`,
`openspec *`, `bun run *` and the `git` verbs run; `ls`, `pwd` and piped commands are refused). The
specific `shell` allow entries are simply not being applied for subagent sessions. Candidate fixes
for the config owner, to be verified empirically before 0.1 is re-delegated: express the agent
permission blocks in the object/map form current OpenCode expects rather than the array-of-rules
form; or remove the blanket `resource: "*"` shell deny from the subagent files and keep only explicit
allows and denies; or file it upstream against subagent permission application. Verification probe:
`architect` must be able to run `cargo tauri --version` and `apple-native` must be able to run
`xcrun --show-sdk-path` in their own sessions.

**Q11 — decided: create `SETUP.md` now.** Root `SETUP.md` documents the macOS setup TOOL-1's scenario
and task 0.1's precondition refer to: Xcode with a simulator runtime, the `aarch64-apple-ios` rustup
target, bun, and the one-off local `cargo install tauri-cli --version "^2" --locked` bootstrap.

**Done** — created at the repository root and committed with the resolutions (first commit of
2026-10-16 in this change). It was written through the built-in `general` agent, the only session with
write access to a root path, since no project agent's edit allowlist owns a root-level file and the
orchestrator may edit only `tasks.md` and `questions.md`. The document carries the iOS-target check
(`rustup target list --installed`), the `rustup target add aarch64-apple-ios` step, the
`bun install` step and the `definition-of-done` gate list. It points the deployment-target question at
D0.16 rather than naming a version. The ownership gap it exposed is **not** closed: future edits to
root documentation still have no owner, and the config change should name one.

**Q12 — decided: extend the orchestrator's allowlist**, and close the two specialist gaps the same
way. Answers to "who runs the gate's non-gate commands":

| Command family | Needed by | Allowlist entry required |
|---|---|---|
| `bun install` | tasks 0.13–0.19 (materialise `node_modules` from the hand-written exact pins) and every `bun run typecheck\|lint\|test\|test:e2e` after them | `.opencode/agents/frontend-logic.md` (has `bun run *` and `bun add *`, which cannot install a hand-written manifest) **and** `.opencode/agents/orchestrator.md` (the gate) |
| `rustup` | task 0.1's precondition, the `aarch64-apple-ios` target check, `apple-native`'s task 0.7 | `.opencode/agents/orchestrator.md`; `apple-native.md` if it is to record the installed target list itself |
| `xcodebuild`, `xcrun` | task 0.7 (measure the deployment floor from the installed SDKs); re-verified by the orchestrator during the gate | already allowed for `apple-native.md`; add to `.opencode/agents/orchestrator.md` if the orchestrator is to verify the floor independently |

Task 0.7 stays owned by `apple-native`, whose file already allows `xcodebuild *` and `xcrun *`; the
orchestrator entries are for gate verification only. None of these entries takes effect until the
Q10 refusal is fixed, so Q10 is the gating change.

## Q10 — Specialists cannot execute shell commands in this harness (2026-10-16, orchestrator)

**Blocking for:** task 0.1 and every later task whose deliverable is produced by running a command
rather than by editing a file — the generator pass (0.1), the SDK/deployment-floor measurement (0.7),
regenerating bindings (0.16) and the icon set (0.22), the iOS compile check and every gate run
(0.26, 5.18, 6.x), and the manual boot on the simulator (0.27).

**Observed evidence.** Task 0.1 was delegated to `architect` and completed with no command executed:
every `shell` call in a subagent session is refused at the tool layer with
`{"error":{"type":"permission.rejected","message":"Permission denied: shell"}}`. This is not an
allowlist miss: the same refusal was returned for `cargo install tauri-cli --version "^2" --locked`,
which `.opencode/agents/architect.md` explicitly allows, and a second probe in a `rust-backend`
session refused both `cargo --version` and `openspec list`, which that agent's own allowlist permits.
File edits in subagent sessions do work (the `architect` session successfully edited `design.md`),
and the orchestrator's session does enforce its own allowlist (`cargo *`, `bun run *`, `openspec *`,
`graphify *`, `git status|diff|add|commit|log|rev-parse *` all run; anything else is refused).

**Consequence.** D0.11's premise that "`architect` is granted shell access for
`cargo install tauri-cli *` and `cargo tauri *` for exactly this task" does not hold in this
environment, so task 0.1 cannot be completed by its owner as written. Either the harness/config is
fixed so specialist sessions may execute their allowlisted commands, or every command-running step
of every task is executed by the orchestrator while the owning specialist supplies the file edits —
which is a delegation model change, not a task-level deviation.

**Awaiting a product-owner/configuration decision.** No task output has been faked; `src-tauri/` does
not exist and task 0.1 remains unchecked. The `architect` session recorded the non-execution in
`design.md` (D0.11) rather than an invented observed-output note.

## Q11 — `SETUP.md` does not exist (2026-10-16, orchestrator)

TOOL-1's scenario ("A fresh checkout passes the gate [macos]") says the repository is "prepared per
`SETUP.md`", and task 0.1 lists "the `aarch64-apple-ios` rustup target (`SETUP.md`)" as a
precondition. The repository has **no `SETUP.md`**: the root holds `AGENTS.md`, `opencode.jsonc`,
`openspec/`, `docs/` and `graphify-out/` only.

Neither the precondition nor the scenario can currently be read from a document, and no agent's edit
allowlist owns a root-level `SETUP.md`. Open item: should the documented macOS setup (Xcode, the
`aarch64-apple-ios` rustup target, bun) be written as `SETUP.md`, and by whom? Task 0.1's real
precondition — Xcode plus the iOS rustup target being installed — is unaffected by the missing file
and is checked directly instead.

## Q12 — Orchestrator gate-command permissions are incomplete (2026-10-16, orchestrator)

The `definition-of-done` gate and section 0 need three command families that no agent in this
repository is permitted to run:

| Needed by | Command | Permitted for |
|---|---|---|
| tasks 0.13–0.19 and every `bun run typecheck\|lint\|test\|test:e2e` after them | `bun install` | nobody (`bun run *` only, which cannot install) |
| task 0.7 (measure the deployment floor from the installed SDKs) | `xcodebuild`, `xcrun`, `rustup` | nobody |
| task 0.1 precondition and the iOS compile check | `rustup target list` / `rustup target add aarch64-apple-ios` | nobody |

The orchestrator can run `cargo check --target aarch64-apple-ios` once a crate exists, which is
enough to *detect* a missing iOS target but not to install it. Task 0.7 as currently scoped cannot be
executed by any agent. Awaiting a decision: extend the orchestrator's (and/or `apple-native`'s)
allowlist, or accept a shell-capable human step for these commands.

