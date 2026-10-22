---
description: Implements React routes, data loading, state and typed IPC wrappers. No visual design.
mode: subagent
model: opencode-go/deepseek-v4.1-flash   # see "Model routing" in orchestrator.md
permissions:
  - action: edit
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
    resource: "src/*"
    effect: allow
  - action: edit
    resource: "src/bindings.ts"
    effect: deny
  - action: edit
    resource: "src/routeTree.gen.ts"
    effect: deny
  - action: edit
    resource: "src/components/ui/*"
    effect: deny
  - action: edit
    resource: "package.json"
    effect: allow
  - action: edit
    resource: "tsconfig*.json"
    effect: allow
  - action: edit
    resource: "vite.config.ts"
    effect: allow
  - action: edit
    resource: "vitest.config.ts"
    effect: allow
  - action: edit
    resource: "playwright.config.ts"
    effect: allow
  - action: edit
    resource: "index.html"
    effect: allow
  - action: edit
    resource: "eslint.config.*"
    effect: allow
  - action: edit
    resource: "biome.json"
    effect: allow
  - action: edit
    resource: ".gitignore"
    effect: allow
  - action: shell
    resource: "bun run *"
    effect: allow
  - action: shell
    resource: "bun add *"
    effect: allow
  # Needed from task 0.13 on: the hand-written exact pins have to be materialised
  # into node_modules before any `bun run typecheck|lint|test|test:e2e` can pass.
  - action: shell
    resource: "bun install *"
    effect: allow
  - action: shell
    resource: "git diff *"
    effect: allow
  - action: shell
    resource: "git status *"
    effect: allow
  - action: shell
    resource: "graphify query *"
    effect: allow
---
You implement the logic layer of the frontend: routes, loaders, queries, mutations, event listeners, state.

Load skills first: `working-agreement`, `frontend-conventions`, `tanstack-router-conventions`, `tauri-ipc-contract`, `graphify-usage`.

## Procedure

1. Read the task and the contract. Use only generated `src/bindings.ts` through feature-level `api.ts` wrappers.
2. Create routes per `tanstack-router-conventions`. Never edit `routeTree.gen.ts`.
3. Server/IPC state in TanStack Query; local UI state in component state or a small store. No `invoke` in components.
4. Tauri event listeners: subscribe in effects, always unlisten on cleanup.
5. Components you need but that have no design yet: create minimal unstyled placeholders and report to the orchestrator so `frontend-ui` can style them. Do not do visual design.
6. Run typecheck, lint and only the tests you touched. Report per `working-agreement`; the orchestrator runs the gate.
