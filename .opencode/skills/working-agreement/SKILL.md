---
name: working-agreement
description: Shared rules for every specialist agent — scope discipline, the smallest-complete-change ladder, stop-and-report, report shape, and who runs the gate. Load before starting any task.
---

# Working agreement

Every specialist agent loads this first. The orchestrator enforces it.

## Scope

1. **Read the task and the code it touches before writing.** Query the graph
   (`graphify query "..."`), then grep, then read only what those located.
2. **Edit only the paths named in your delegation.** Need a file outside them?
   Stop and report. Do not refactor neighbours, rename for taste, or fix
   unrelated bugs.
3. **Never read** `src/bindings.ts`, `src/routeTree.gen.ts`, `graphify-out/`,
   lockfiles, `node_modules/`, build output. Generated, large, not yours.
4. **Never edit** generated files or `AGENTS.md`. Regenerate; do not hand-write.

## The smallest complete change

The spec decides *whether* something is built; this decides *how little* code
builds it. Adapted from Ponytail (`DietrichGebert/ponytail`, MIT).

Take the first rung that fully works:

1. Already in this codebase — a helper, component, service, pattern? Use it the
   way the surrounding code already does.
2. Standard library or platform feature? Use it, unless the project has its own.
3. An installed dependency? Use it. Never add one for a few lines.
4. Can it be one line a reader gets at a glance? One line.
5. Otherwise: the minimum code that works, in the structure the codebase has.

- No wrapper, abstraction, option, config flag or "for later" code the task does
  not name. Deletion beats addition. The shortest working diff wins.
- Finish everything the task *does* need: callers, fixtures, tests, error paths.
- Bug fix: find every caller first, fix the root cause once in the shared code.
- Comment only the why the code cannot show, in one line. A deliberate shortcut
  gets `shortcut: <the limit>, <when to upgrade>` — do not hide it.

**Never cut:** validation at trust boundaries, error handling that prevents data
loss, security, accessibility, anything a requirement or scenario names.

If the smallest change and the spec disagree, the spec wins — and you say so in
your report instead of quietly shipping less.

## Reporting

- **Do not run the full gate.** Run only the suite you touched
  (`cargo test <filter>`, `bun run test -- <path>`, one targeted e2e). The
  orchestrator runs the whole gate, once, and does not trust your claim.
- **Report shape** — nothing else, no file dumps, no code walkthroughs:

  ```
  Files: <path> (new|changed)
  Tests: <file>::<test> — covers "<scenario>"
  Ran: <command> → <pass|fail>
  Skipped: <what and why>   Risk: <one line, or "none">
  ```

- Ambiguous, missing contract, spec conflict, or a file outside scope?
  **Stop and report in one paragraph.** Do not guess. One blocker costs less than
  three retry loops.
