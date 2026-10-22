---
name: graphify-usage
description: When and how to query the Graphify codebase graph and when to refresh it. Load at the start of any task that reads or edits existing code.
---
# Graphify usage

Graph output lives in `graphify-out/` (`graph.json`, `graph.html`, report).

## Query (agents)

```
graphify query "where is settings persistence handled" --graph graphify-out/graph.json
graphify query "what connects SettingsStore to the ipc layer" --graph graphify-out/graph.json
```

Use before creating a module, command, component or hook (does it exist?), before editing (callers, dependencies, blast radius) and to locate where a requirement is already implemented. Order: graph query → targeted grep → file reads.

## Build and refresh (orchestrator / human)

| Action | Command |
|---|---|
| Install CLI | `uv tool install graphifyy` (double y) |
| Register for opencode | `graphify install --platform opencode` |
| Always-on rules + plugin | `graphify opencode install` (appends to `AGENTS.md`, adds `.opencode/plugins/graphify.js`) |
| Build graph | `/graphify .` in opencode |
| Auto-refresh on commit | `graphify hook install` |
| Auto-refresh on change | `graphify --watch` in a background terminal |

Refresh after archiving a change and after large moves. Not mid-task; a slightly stale graph is acceptable.

## Reporting

Include graph findings (paths, symbols) in delegation messages so specialists do not rediscover them.

## Context cost

- Refresh at change boundaries: after an archive, or after a large move. Not per task, not per commit.
- The commit hook rewrites `graph.json` whole, so a large diff to it on every commit is expected. Nobody diffs or opens `graphify-out/` in review; pass findings as `file:symbol` lines instead.
- Query narrow. Read `GRAPH_REPORT.md` only when query/path/explain came back empty.

## Limits

The graph is a map, not truth. If it contradicts the code, trust the code and flag a refresh.
