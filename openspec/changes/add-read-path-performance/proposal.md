# add-read-path-performance

> Phase **P6 — Performance & polish** of the BunScale reimplementation.
> Source of truth: `docs/BunScale-Reimplementation-Requirements.md` (§7 NFR-7, §8 IMP-3) and
> `docs/BunScale-Reimplementation-User-Stories.md` (US-QUAL-07, §12 P6).
> This change owns **one** requirement ID — NFR-7 — and no others.

## Why

IMP-3 records the historical symptom: dashboard loads were slow because they were made of many
sequential remote requests, and the product re-fetched data that had been loaded moments earlier.
The result was a product that felt unresponsive at exactly the moment a person opens it, and a
read path whose cost grew with the size of the household.

NFR-7 is the requirement that closes this, and US-QUAL-07 is the story that carries it: a cold
dashboard load is bounded, opening a rabbit does not re-download data the dashboard already
loaded, and a remote change invalidates only what it affects rather than forcing a full cold
reload on the next visit.

**Why last:** P0 deliberately left the loaded-data path unoptimised (P0 `design.md`, D0.2). It
fixed only the *boundaries* — where the local copy lives and what a refresh replaces — so that a
later change could improve read behaviour without moving them. This is that change. It is only
meaningful once the phases that populate the read path exist, because there is nothing to make
bounded or to reuse until the dashboard and the rabbit detail actually load something.

## What changes

The read path becomes load-once and scoped: a fixed, parallel cold load; already-loaded data
reused across navigation and inside a freshness window; and invalidation limited to the data a
remote change actually touched. Nothing else about the product changes — no new screen, no new
feature, no write-path change, no new dependency.

| Capability | Requirements added | Substance |
|---|---|---|
| `product-quality` | NFR-7 | Bounded cold dashboard load, load-once reuse of already-loaded data, scoped invalidation on remote change |

This is an **ADDED-only** delta in one capability. No other capability is touched, no requirement
is modified or removed, and no requirement from another phase is restated.

Supporting work is read-path only: request batching and concurrency in the Rust read path, the
freshness window, an item-level invalidation signal, query-cache key and reuse policy on the
frontend, non-blocking rendering of already-loaded data, and tests that count remote requests
rather than only asserting the final rendered result.

### Settled decisions this change honours

- **D1 — Connectivity-required saves.** This change touches **reads only**. It adds no write
  queue, no optimistic write and no reconciliation path; the read path it improves is the
  loaded-data path D1 leaves in place.
- **D6 — Technology-agnostic specs.** The capability spec names only "the shared service", the
  product and its data. The concrete caching technology appears in `design.md` only.
- **D5 — Chart baseline band out.** Unaffected; no chart work is in scope here.
- **D2, D3, D4.** Unaffected: no ownership, reminder or migration surface is involved.

### The boundary this change must not move

P0's D0.2 is binding on this change: the local copy stays a **cache of the household collection,
never a competing record**, a refresh still replaces what was loaded, and per-device preferences
stay off the collection. This phase optimises how that boundary is *used*; it does not relocate
it. Task 6.1 checks exactly this.

## Impact

- **Rust read path** (`src-tauri/src/sync/`): batched, concurrent load; freshness window
  evaluation; item-level change reporting; per-rabbit reads scoped to what is missing.
- **Frontend data layer** (`src/features/*/`): household-scoped query keys with a load-once
  reuse policy, the freshness window applied in the cache, prefetch grouping for the cold load,
  and scoped invalidation on `household:changed`.
- **Frontend screens** (`src/routes/`): already-loaded data renders immediately and no full-screen
  cold-load state replaces it; 390×844 first, 1280×800 second.
- **IPC contract:** **no new command and no new event is required** (see `design.md`); the change
  must sit behind the P0 commands and the existing `household:changed` event.
- **Security:** no change. No new capability, scope, CSP entry or entitlement — the read path uses
  the egress the P0 mechanism already declared.
- **Dependencies added:** none. The read path uses the caching layer the frontend already has.
- **Not in this change:** any new feature or screen, the write path, the "honest service status"
  behaviour (NFR-6, P3), accessibility and localisation (NFR-4/NFR-5, P4), export and reminders
  (P4), medications (P5), and the tests of other phases' requirements.

## Dependencies

This change requires every phase that populates the read path, because it can only make bounded
and reusable what those phases cause to be loaded.

| Phase | change-id | What P6 needs from it |
|---|---|---|
| P0 | `add-shared-household-foundation` | The read path and its cache boundaries (D0.2), the IPC contract (`load_household`, `get_service_status`, `household:changed`, `service:status-changed`), the local copy, and per-item version/updated-at on the collection |
| P1 | `add-rabbit-weight-core-loop` | The dashboard and rabbit read surfaces the cold load covers, and the rabbit list the dashboard presents |
| P2 | `add-health-and-photo-history` | Per-rabbit detail and history surfaces that "opening a rabbit" resolves |
| P3 | `add-household-sharing-hardening` | The member/invitation read surfaces and the live-update subscription point this change scopes invalidation for |
| P4 | `add-vet-export-and-reminders` | The additional read surfaces its screens add to the household collection |
| P5 | `add-medication-tracking` | The medication read surfaces the dashboard may present |

**Archive order.** Several capability specs opened in P0 are modified by later phases, so archives
MUST run in order **P0 → P1 → P2 → P3 → P5 → P4 → P6** — `add-medication-tracking` (P5) archives
before `add-vet-export-and-reminders` (P4), because P4's export and medication reminders read P5's
medication model. This change archives **last**; archiving it out of order would apply its delta
against text that does not yet exist.
