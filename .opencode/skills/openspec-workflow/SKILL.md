---
name: openspec-workflow
description: OpenSpec conventions for specs, change proposals, delta format, tasks, validation and archiving. Load before touching openspec/.
---
# OpenSpec workflow

## Layout

```
openspec/specs/<capability>/spec.md        current truth
openspec/changes/<id>/proposal.md          why, what, impact
openspec/changes/<id>/design.md            decisions, IPC contract, risks
openspec/changes/<id>/tasks.md             checklist
openspec/changes/<id>/specs/<cap>/spec.md  deltas
openspec/changes/<id>/questions.md         open questions / blockers
```

`<id>`: kebab-case, verb-led (`add-settings-persistence`).

## Delta format

```
## ADDED Requirements
### Requirement: Settings persist across restarts
The system SHALL store user settings and restore them on next launch.

#### Scenario: Restore after restart
- **WHEN** the user changes the theme and restarts the app
- **THEN** the theme is the one chosen before restart
```

Use `## MODIFIED Requirements` (full replacement text), `## REMOVED Requirements`, `## RENAMED Requirements`. Every requirement needs ≥ 1 scenario.

## Tasks

- Format: `- [ ] 1.2 [agent] imperative description`.
- ≤ ~1 hour of work, one owner agent, one commit.
- Order: contract → rust-backend → frontend-logic → frontend-ui → tests.
- Tick only after the gate passes.

## Install and update

```
npm install -g @fission-ai/openspec@latest
openspec init          # select opencode; generates the slash commands/skills
openspec update        # re-run in the project after upgrading the CLI
```

Slash commands (typed in chat): `/opsx:propose`, `/opsx:apply`, `/opsx:archive`. Spelling varies per tool (some use `/opsx-propose`); check what `openspec init` generated for opencode. The orchestrator drives the CLI directly; humans can use the slash commands.

## Commands

| Action | Command |
|---|---|
| List changes | `openspec list` |
| Show | `openspec show <id>` |
| Validate | `openspec validate <id> --strict` |
| Archive | `openspec archive <id> --yes` (moves to `openspec/changes/archive/<date>-<id>/`) |
| Dashboard | `openspec view` |

> Verify `validate` and `archive --yes` exist in your installed version with `openspec --help`.

## Rules

- Specs describe observable behavior, not implementation.
- Never edit `openspec/specs/` directly; changes go through deltas and archive.
- Unclear requirement → `questions.md`, not a guess.
