# Design — add-read-path-performance (P6)

## Context

NFR-7 is a performance requirement, and performance requirements fail in a predictable way: they
are written as aspirations ("the dashboard should open quickly") and then cannot be verified. The
goal of this design is to make every clause of NFR-7 assertable, in counts and scopes rather than
in wall-clock impressions.

P0 (`openspec/changes/add-shared-household-foundation/design.md`, **D0.2 — Loaded-data read
model**) explicitly left the loaded-data path unoptimised and deferred this work:

> The loaded-data path is deliberately unoptimised in P0 — serving it first, batching the refresh
> and invalidating only affected data is the subject of `add-read-path-performance` (P6, NFR-7).
> P0 only fixes the *boundaries* (where the cache lives, what a refresh replaces) so P6 can
> improve behaviour without moving them.

That paragraph is the constraint of this change as much as its mandate.

Platforms: iOS (primary) and macOS (Phase 1), baselines 390×844 first and 1280×800 second. Stack:
Tauri v2, Rust in `src-tauri/`, React + TypeScript + Vite in `src/`, TanStack Router and TanStack
Query, generated `src/bindings.ts` (never hand-edited). Project hard rule 2 holds: the frontend
reaches IPC only through `src/features/*/api.ts` over generated bindings, so every claim about
"a remote request" in this design is observable at two seams — the `api.ts` wrapper (frontend) and
the shared-service boundary (Rust).

## Two cache layers, two jobs

NFR-7 spans two layers, and conflating them is how this optimisation goes wrong:

| Layer | What it holds | What it decides | Kinds of clause it serves |
|---|---|---|---|
| **Local copy in the product's own storage (Rust)** | The last loaded household collection, persisted across launches | Whether a read must contact the shared service at all, and whether the freshness window has elapsed | bounded cold load; reuse inside the freshness window; invalidation scope |
| **In-memory query cache (frontend)** | Query results per household and data area, for the current session | Whether navigation re-invokes a read at all, and which cached areas a change signal invalidates | opening a rabbit does not re-download; no cold reload on the next visit |

The frontend cache is **derived**: if it is empty, the product falls back to the local copy, and
the local copy remains the only cache of record (P0 D0.2). Nothing in this change adds persistence
of its own.

## Decisions

### D6.1 — A cold dashboard load is one fixed, parallel load

A cold dashboard load is defined as one bounded set of remote requests covering the data areas the
dashboard presents, issued so that no request in the set waits for the previous one to finish
("not serialized into many", NFR-7). The load covers a **fixed** set of areas per product version,
independent of collection size: the count for a household with one rabbit is the same as for a
household with many. Anything a person opens later that was not part of the dashboard load is
requested on demand, scoped to the rabbit in question (D6.3), never as a per-rabbit sweep.

The concrete number is **not derivable from the source documents** (NFR-7 says only "a small,
fixed number of remote requests") and is recorded as open question Q1. The spec therefore asserts
the *shape* — fixed, and independent of the number of rabbits — and the number is settled by task
1.2 before backend work starts.

### D6.2 — Serve from already-loaded data first, with a freshness window

Already-loaded data is presented without waiting for a refresh, and a load that is inside its
freshness window does not contact the shared service at all (IMP-3: "serve-from-loaded-data-first
with a freshness window"). The window is **one named, configurable value** — not a scattered set
of ad-hoc timeouts — so that it can be set to zero in tests and asserted deterministically. The
concrete duration is **not in the source documents** (IMP-3 names the policy, not a duration) and
is recorded as open question Q2; P0 D0.2's "a start or foreground return with connectivity
refreshes it" remains the outer refresh trigger.

Two rules bound the window so it cannot become a truthfulness bug:

1. A change signal (D6.4) is honoured regardless of the window. The window governs *unprompted*
   refresh, never requested invalidation.
2. Freshness is never claimed. Serving data inside the window shows the data that was loaded; it
   does not present it as freshly refreshed. The "syncing"/status truthfulness of NFR-6 is P3 and
   is not restated here.

### D6.3 — Load once, per data area, scoped to what is missing

Each data area (household summary, rabbit list, a rabbit's detail or history) is loaded once and
reused. Opening a rabbit whose data the dashboard already loaded performs no remote request for
that data (NFR-7, clause 2; US-QUAL-07). Opening a rabbit whose detail was never loaded requests
only the data that is missing for that rabbit, not the household collection again. Reuse is keyed
by household and data area, so switching the active household cannot serve one household's cached
area for another (the P0 isolation invariant, NFR-10).

### D6.4 — A remote change invalidates only the data it affects

When a change arrives from another member, invalidation is scoped to the data covering the changed
items. The mechanism chosen is to reuse the existing `household:changed` event with
`kind: refreshed` and compare per-item `version`/`updated_at` — which P0's collection model already
carries — so the frontend invalidates only the query keys whose items actually changed, and keeps
the rest of the cache. That is what prevents NFR-7's third clause from failing: an unscoped
"refresh everything" signal would reload the whole dashboard on the next visit.

Two properties are asserted, both testable: the unaffected areas are **not** re-requested, and no
full cold-load sequence runs after a change.

### D6.5 — The optimisation lives behind the existing contract

This change adds no command, no event, no type and no capability. The read path is improved behind
`load_household` and the surrounding read commands, and the frontend-side reuse is a cache policy.
The design deliberately avoids a "preload everything" command: an explicit bulk endpoint would be
a contract change in service of an implementation detail, and the same bound is reachable by
batching and concurrency behind the command that already exists.

### D6.6 — P0's boundaries are used, not moved

| P0 D0.2 boundary | How this change treats it |
|---|---|
| The household collection is the only source of truth; the local copy is a cache, never a competing record | Unchanged. Reuse reads the copy; it never writes to it as if it were authoritative. |
| A refresh replaces the loaded copy rather than adding to it | Unchanged. Batching and concurrency change when and how the refresh runs, not what it replaces. |
| "Nothing ever loaded + unreachable" stays a distinct state with its own surface | Unchanged. Reuse applies only to data that was genuinely loaded; a freshness window can never manufacture loaded data. |
| Per-device preferences live on the device and never in the collection | Unchanged. The freshness window is a device-local read policy, not household data. |
| Reachability is derived from a fresh outcome, never a stale status | Unchanged. A cache hit says nothing about reachability and must not be read as one. |

## State and boundaries

```
Rust (src-tauri/)                                    Frontend (src/)
  src-tauri/src/ipc/          unchanged: load_household, get_service_status,
                              household:changed, service:status-changed
  src-tauri/src/sync/         batched, concurrent load · freshness window ·
                              item-level change reporting · per-rabbit scoping
  src-tauri/src/storage/      unchanged: the loaded local copy (cache of the
                              household collection, P0 D0.2)
                                                       → src/features/*/api.ts (unchanged surface)
                                                       → src/features/*/ query keys: household-scoped,
                                                         one key per data area, load-once reuse
                                                       → src/routes/ dashboard and rabbit screens:
                                                         render loaded data immediately, no blocking
                                                         cold-load state
```

Rules held here:

- Reuse never crosses households: cache keys carry the household id (NFR-10).
- A cache hit is not evidence about the service: the freshness window and reachability are
  separate concepts and must not be conflated in the status surfaces.
- The freshness window is configurable so tests can drive it to zero and to "never", rather than
  sleeping.
- No new persistence: the frontend cache is in memory and does not survive a launch; what survives
  is the P0 local copy.

## How each clause becomes measurable

| NFR-7 clause | Observable behaviour | How it is asserted |
|---|---|---|
| Cold load is bounded | A fixed number of remote requests, independent of rabbit count | Count calls at both seams — the shared-service boundary in Rust tests, the `api.ts` wrappers in component tests, and mocked IPC calls in E2E — for a household with one rabbit and with many, and compare. Assert the count *and* the absence of additional calls. |
| Cold load is not serialised | The load's requests are in flight together | Record the order of requests into the seam and assert the independent areas are opened before earlier ones complete (a test double that only resolves when all are requested is sufficient; no wall-clock timing). |
| Reuse inside the freshness window | No remote request while the window is open | Drive the window with a controlled clock; assert zero calls into the seam, then a bounded refresh once the window elapses. |
| Opening a rabbit does not re-download | Zero calls for data already loaded | Navigate dashboard → rabbit → back with a call counter on the wrappers; assert **zero** additional calls for the already-loaded areas, not merely that the screen rendered. |
| Opening a rabbit loads only what is missing | Only the missing area is requested | Load the dashboard, open a never-loaded rabbit, and assert exactly the missing area's calls appear and the collection is not reloaded. |
| Remote change invalidates only affected data | Only the affected key refetches | Apply a one-item change signal, then assert the affected key's refetch count is one and every other loaded key's count is unchanged. |
| No cold reload on the next visit | Reused data, no cold-load sequence | After a change signal, leave and return; assert loaded data renders and the cold-load call sequence does not run again (both lifecycle variants). |
| Already-loaded data is served while a refresh is outstanding | Content before the refresh resolves | Use a deliberately unresolved (slow) refresh double; assert loaded content is on screen while the refresh is still pending. |

Assertions are on **counts and call order**, never on elapsed wall-clock time, so the suite stays
deterministic (see risk R6 and open question Q3).

## IPC contract — to be defined by architect

**Status: no contract change is proposed. `architect` confirms this in task 1.1 before backend
work starts.**

No new command, no new event, no new type and no capability change is required for NFR-7. The
optimisation sits behind commands that P0 already froze:

| Existing surface (P0) | How P6 uses it |
|---|---|
| `load_household(household_id)` | The single entry point for loading a household's data; carries the batching, concurrency, freshness window and per-rabbit scoping internally |
| `get_service_status()` | Unchanged. A cache hit is not a status signal |
| `household:changed` `{ household_id, kind }` | The change signal used for invalidation. `kind: refreshed` carries (or is accompanied by) per-item `version`/`updated_at`, which the P0 collection already stores |
| `service:status-changed` | Unchanged |

The one decision `architect` must freeze in 1.1 is whether `household:changed` with
`kind: refreshed` plus per-item version comparison is **sufficient** for scoped invalidation. If
it is not, the only candidate is a payload extension of that existing event — an affected-item
identifier — and that decision belongs to `architect`, not to this design. **What is not
proposed, and what backend and frontend tasks must not introduce, is a new command, a new event, a
"preload/bulk" endpoint or a manual cache-control command.** The frontend additionally touches IPC
only through generated bindings via `src/features/*/api.ts` (hard rule 2), so no `invoke` call
appears in this change.

## Traceability: user stories → scenarios

| Story / source | Requirement | Scenarios in `specs/product-quality/spec.md` |
|---|---|---|
| US-QUAL-07 AC1 ("loads in a bounded time rather than a long serialized chain") | NFR-7 | "Cold dashboard load stays within a fixed request bound", "A cold load is not a serialised chain" |
| US-QUAL-07 AC2 ("open that rabbit … NOT loaded again") | NFR-7 | "Opening a rabbit re-downloads nothing the dashboard already loaded", "Opening a rabbit loads only the data that was not already loaded" |
| US-QUAL-07 AC3 ("only affected data reloads and already-loaded data is reused") | NFR-7 | "A remote change invalidates only the affected data", "A remote change does not force a cold reload on the next dashboard visit `[ios]`", "A remote change does not force a cold reload while the product stays open `[macos]`" |
| IMP-3 direction ("serve-from-loaded-data-first with a freshness window") | NFR-7 | "Already-loaded data is reused inside the freshness window", "Data is refreshed once the freshness window has elapsed", "Already-loaded data is served while a refresh is outstanding" |

Stories uncovered by this change: **none**. Every acceptance criterion of US-QUAL-07 maps to at
least one scenario, and the IMP-3 direction is expressed as observable behaviour rather than as an
implementation instruction.

Platform tagging: NFR-7's behaviour is the same on both platforms except where the product's
lifecycle differs, so only the "next visit" pair carries `[ios]` / `[macos]`. A slow or limited
connection is not a platform difference, so the slow-refresh scenario is deliberately untagged: it
holds on both. No `[mobile]`/`[desktop]` scenario is added, because the bound and the reuse policy
are form-factor independent; the layout consequence (already-loaded data rendering immediately at
390×844 and at 1280×800) is UI work in task 4.1 rather than a behavioural difference.

## Risks

| # | Risk | Mitigation in this change |
|---|---|---|
| R1 | **The bound has no number**, so "bounded" degrades into an aspiration again. | The spec asserts the bound's *shape* (fixed, independent of rabbit count) and task 1.2 must settle the number before backend work; the gap is recorded as open question Q1 rather than invented. |
| R2 | **A freshness window hides a change.** Serving a window-stale copy could mask a remote edit and violate the write/read honesty of D1/SYNC-3. | A change signal is honoured regardless of the window (D6.2 rule 1); the window governs only unprompted refresh; the window duration is an open question (Q2) with the honesty constraint attached. |
| R3 | **Coarse invalidation.** An unscoped "refreshed" signal recreates the historical full reload on the next visit — exactly the symptom IMP-3 records. | D6.4 scopes invalidation by per-item version and asserts the *absence* of refetches for unaffected areas (task 5.4). |
| R4 | **Two caches drift.** The frontend cache and the local copy could disagree about what was loaded. | The frontend cache is derived and never a record (D6.6); reuse falls back to the local copy; task 3.6 forbids clearing the cache on navigation or on a change signal. |
| R5 | **Parallelism multiplies traffic on a limited connection.** Batching could be read as "fetch everything eagerly", inflating requests on cellular. | The fixed bound is independent of connection type; the load covers a fixed set of areas and never adds per-rabbit fallbacks; the slow-connection behaviour is asserted explicitly. |
| R6 | **Flaky performance tests.** Wall-clock assertions on a simulator are non-deterministic and would be quarantined. | Every assertion is on request counts, call order or controlled-clock window state (see the measurement table); Q3 records that no latency target exists in the docs. |
| R7 | **Cache growth.** Many rabbits, two devices, long sessions could grow the in-memory cache unboundedly. | Eviction is the frontend cache's concern and must not change the observable behaviour the scenarios assert; the bound and the reuse are asserted by count, not by cache size. Correctness of the P0 local copy is unaffected. |
| R8 | **Optimisation regresses a P0 invariant.** Isolation, "no loaded data is invented", or "a refresh replaces rather than duplicates". | Task 6.1 reviews the diff specifically against P0 D0.2's boundaries, and task 5.3 asserts the absence of redundant requests rather than only the rendered result. |
| R9 | **Scope creep.** P6 lands last and is tempting as a place to "polish" anything. | The delta is ADDED-only, exactly NFR-7; NFR-4/NFR-5 (P4), NFR-6 (P3), NFR-9 (P1) and the write path are explicitly out; task 6.2 checks the delta contains nothing else. |
| R10 | **Phase-order dependency.** Landing before the phases that populate the read path leaves nothing to bound. | Dependencies and the archive order are stated in `proposal.md`; this change archives last. |

## Verification

- Specs: `openspec validate add-read-path-performance --strict`.
- Gate (per `definition-of-done`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` (no diff in
  `src/bindings.ts`), `git diff --exit-code src/routeTree.gen.ts` after a build,
  `bun run typecheck`, `bun run lint`, `bun run test`, `bun run test:e2e`.
- Every scenario in the delta is named by at least one test task in `tasks.md`, and at least one
  test asserts the **absence** of redundant remote requests rather than only the rendered result.
- Cross-check at review: the delta contains exactly NFR-7 — no requirement from P0 (NFR-1/2/3/8),
  P1 (NFR-9), P3 (NFR-6) or P4 (NFR-4/NFR-5) is restated, modified or removed.
