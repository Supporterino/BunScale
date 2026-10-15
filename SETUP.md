# Setup (once, on the Mac that runs the agents)

Requires macOS, Xcode, Rust (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim`), bun, Node, Python 3.10+, uv.

| Step | Command |
|---|---|
| 1. Copy kit | unzip into repo root (`AGENTS.md`, `opencode.jsonc`, `.opencode/`) |
| 2. OpenSpec | `npm i -g @fission-ai/openspec@latest && openspec init` (choose opencode) |
| 3. ui-ux-pro-max | `npm i -g ui-ux-pro-max-cli && uipro init --ai opencode` (needs Python 3). Read its SKILL.md and scripts first; confirm skill ID `ui-ux-pro-max` |
| 4. Graphify | `uv tool install graphifyy && graphify install --platform opencode && graphify opencode install` |
| 5. Graph build | in opencode: `/graphify .`, then `graphify hook install` |
| 6. Models | set `model:` in each agent (`provider/model#variant`) |
| 7. Smoke test | start opencode, check `orchestrator` is default, run a read-only prompt per agent, confirm an edit outside its scope is denied |
| 8. Start | feed requirements + stories to `spec-writer`, review specs and `questions.md`, then run `orchestrator` |

`graphify opencode install` appends its own block to `AGENTS.md`; keep it below the project rules.
