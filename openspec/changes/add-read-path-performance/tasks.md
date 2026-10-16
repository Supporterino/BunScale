# Tasks — add-read-path-performance (P6)

One task = one commit = one owner. Format: `- [ ] <n>.<m> [agent] <imperative>`.
Order: contract → backend → frontend-logic → frontend-ui → tests → review.
Tick a task only after the `definition-of-done` gate passes for it.
Commit message: `feat(add-read-path-performance): <task>`.
Scope: NFR-7 only. No new command or event may be introduced (design D6.5).
Settled values this change owns: cold load = **two** remote requests (D6.1); freshness window =
**30 s, one global value** (D6.2). CI asserts the **count** (tasks 5.1 and 5.8); the wall-clock
figure is **recorded evidence**, never a CI threshold (D6.7, task 5.11).
Every UI task captures screenshots at 390×844 and 1280×800 into `.artifacts/screens/` (gitignored).

## 1. Contract and measurement design

- [ ] 1.1 [architect] Confirm in `design.md` that NFR-7 needs no new IPC command or event, and
  freeze whether `household:changed` with `kind: refreshed` plus per-item `version`/`updated_at` is
  sufficient for scoped invalidation — or, if it is not, specify the minimal payload extension of
  that existing event. No bulk or preload endpoint is permitted.
- [ ] 1.2 [architect] Fix the bounded cold load in `design.md` (D6.1): exactly **two** remote
  requests — the household's collection plus one service-status check — issued concurrently, and the
  rule that the count never grows with the number of rabbits. Record the value alongside P0's D0.13
  constants rather than as an open question.
- [ ] 1.3 [architect] Fix the freshness window policy in `design.md` (D6.2) as one named, configurable
  value of **30 seconds** applying globally to all data areas alike: where it is evaluated, what it
  gates, that a change signal is honoured regardless of it, that it never makes a cache hit look like
  a service status, and that P0's cache boundaries (D0.2) are unchanged. Record the value alongside
  P0's D0.13 constants.
- [ ] 1.4 [architect] Name the test seams that make each NFR-7 clause measurable — request counting
  at the shared-service boundary, wrapper-call counting for reuse, per-key refetch counting for
  invalidation scope, and a controlled clock for the freshness window — and confirm that none of
  them changes the product surface.
- [ ] 1.5 [architect] Define the wall-clock measurement of task 5.11 as **recorded evidence**,
  separate from the CI-asserted request count: the metric, the reference device and conditions, and
  where the figure is recorded. State that no timing threshold is asserted in CI (D6.7, risk R6).

## 2. Backend (Rust)

- [ ] 2.1 [rust-backend] Serve a household load from the kept local copy while it is inside the
  **30-second** freshness window (D6.2, task 1.3) and contact the shared service only once the window
  has elapsed. Covers "Already-loaded data is reused inside the freshness window", "One freshness
  window applies to all loaded data".
- [ ] 2.2 [rust-backend] Issue the cold load's **two** independent remote requests — the household's
  collection and the service-status check — concurrently and hold the load to that count for any
  number of rabbits. Covers "Cold dashboard load stays within a fixed request bound", "A cold load is
  not a serialised chain".
- [ ] 2.3 [rust-backend] Refresh once the **30-second** window has elapsed with the same fixed,
  concurrent two-request shape as a cold load. Covers "Data is refreshed once the freshness window
  has elapsed".
- [ ] 2.4 [rust-backend] Report the items a refresh actually changed by per-item
  `version`/`updated_at`, so invalidation can be scoped to the affected data only. Covers "A remote
  change invalidates only the affected data".
- [ ] 2.5 [rust-backend] Return already-loaded data immediately while a refresh runs and replace it
  when the refresh completes, so no read blocks on a slow service. Covers "Already-loaded data is
  served while a refresh is outstanding".
- [ ] 2.6 [rust-backend] Scope per-rabbit reads to the data the household collection does not
  already hold, so opening an already-loaded rabbit performs no remote request. Covers "Opening a
  rabbit re-downloads nothing the dashboard already loaded", "Opening a rabbit loads only the data
  that was not already loaded".

## 3. Frontend logic

- [ ] 3.1 [frontend-logic] Run `bun run gen:bindings` and keep the read path behind
  `src/features/*/api.ts`; introduce no `invoke` call outside those wrappers (hard rule 2).
- [ ] 3.2 [frontend-logic] Give each data area a household-scoped query key with a load-once
  reuse policy, so a second mount of the same key reuses the cached result instead of issuing a
  remote request. Covers "Opening a rabbit re-downloads nothing the dashboard already loaded",
  "Already-loaded data is reused inside the freshness window".
- [ ] 3.3 [frontend-logic] Apply the **30-second, global** freshness window of task 1.3 in the query
  cache so data inside the window is reused and data past it is refreshed, without ever suppressing
  an invalidation. Covers "Already-loaded data is reused inside the freshness window", "One freshness
  window applies to all loaded data", "Data is refreshed once the freshness window has elapsed".
- [ ] 3.4 [frontend-logic] Prefetch the dashboard's fixed set of data areas — the collection and the
  status check — together so the cold load issues its **two** requests concurrently with no
  per-rabbit request chain. Covers "Cold dashboard load stays within a fixed request bound", "A cold
  load is not a serialised chain".
- [ ] 3.5 [frontend-logic] On `household:changed` with `kind: refreshed`, diff the per-item versions
  and invalidate only the query keys covering the changed items, keeping the rest of the cache.
  Covers "A remote change invalidates only the affected data".
- [ ] 3.6 [frontend-logic] Keep already-loaded data rendered while a refresh is in flight, and never
  clear the cache on a navigation or a change signal. Covers "Already-loaded data is served while a
  refresh is outstanding", "A remote change does not force a cold reload on the next dashboard visit
  `[ios]`", "A remote change does not force a cold reload while the product stays open `[macos]`".

## 4. Frontend UI

- [ ] 4.1 [frontend-ui] Render already-loaded data immediately on the dashboard and rabbit screens
  with no full-screen cold-load state, laid out for 390×844 first and 1280×800 second, with
  screenshots at 390×844 and 1280×800 captured into `.artifacts/screens/`. Covers
  "Already-loaded data is served while a refresh is outstanding", "Opening a rabbit re-downloads
  nothing the dashboard already loaded".
- [ ] 4.2 [frontend-ui] Keep a refresh running behind already-loaded data non-blocking and legible,
  so no screen replaces loaded data with a blocking loading state while it happens, checked at
  390×844 first and 1280×800 second with screenshots at both viewports captured into
  `.artifacts/screens/`. Covers "Already-loaded data is reused inside the freshness window".

## 5. Tests

- [ ] 5.1 [test-engineer] Rust tests counting requests at the shared-service boundary: assert the
  cold load issues exactly **two** requests and the identical count for a household with one rabbit
  and with many. Covers "Cold dashboard load stays within a fixed request bound", "A cold load is not
  a serialised chain".
- [ ] 5.2 [test-engineer] Rust tests for the **30-second global** freshness window with a controlled
  clock: assert zero requests inside the window, a bounded refresh after it has elapsed, and that
  every data area loaded together ages by the same window. Covers "Already-loaded data is reused
  inside the freshness window", "One freshness window applies to all loaded data", "Data is refreshed
  once the freshness window has elapsed".
- [ ] 5.3 [test-engineer] Rust tests asserting the **absence** of redundant per-rabbit requests:
  open a rabbit whose data the collection already holds and assert zero remote requests, and open a
  never-loaded rabbit and assert the household collection is not requested again. Covers "Opening a
  rabbit re-downloads nothing the dashboard already loaded", "Opening a rabbit loads only the data
  that was not already loaded".
- [ ] 5.4 [test-engineer] Rust tests for scoped invalidation: apply a one-item remote change and
  assert only that item's data is invalidated, asserting the absence of a full-collection refresh.
  Covers "A remote change invalidates only the affected data".
- [ ] 5.5 [test-engineer] Component tests counting mocked wrapper calls across dashboard → rabbit →
  back: assert **zero** additional calls for already-loaded data, not merely that the screen
  rendered. Covers "Opening a rabbit re-downloads nothing the dashboard already loaded",
  "Already-loaded data is reused inside the freshness window".
- [ ] 5.6 [test-engineer] Component tests for next-visit reuse: apply a mocked remote change, leave
  and return, and assert the loaded data is reused while no cold-load call sequence runs. Covers
  "A remote change does not force a cold reload on the next dashboard visit `[ios]`", "A remote
  change does not force a cold reload while the product stays open `[macos]`", "A remote change
  invalidates only the affected data".
- [ ] 5.7 [test-engineer] Component tests for the non-blocking refresh: assert already-loaded data
  is on screen while a deliberately unresolved refresh is in flight and that no full-screen loading
  state replaces it. Covers "Already-loaded data is served while a refresh is outstanding".
- [ ] 5.8 [test-engineer] E2E (Playwright WebKit, mocked IPC, 390×844) cold-load budget: record
  every IPC call the dashboard makes and assert the exact count of **two** for a household with
  several rabbits; covers "Cold dashboard load stays within a fixed request bound" and "A cold load
  is not a serialised chain".
- [ ] 5.9 [test-engineer] E2E at 1280×800: dashboard → open a rabbit → back → apply a mocked remote
  change → revisit, asserting by IPC call count that already-loaded data is reused and that no full
  cold load runs; covers "A remote change invalidates only the affected data" and both "A remote
  change does not force a cold reload …" scenarios.
- [ ] 5.10 [test-engineer] Run the full gate (`cargo fmt --check`, `cargo clippy`, `cargo test`,
  `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` with no diff,
  `git diff --exit-code src/routeTree.gen.ts` after a build, `bun run typecheck`,
  `bun run lint`, `bun run test`, `bun run test:e2e`) and record the results in the change.
- [ ] 5.11 [test-engineer] **Recorded evidence, not a CI assertion.** Measure the cold dashboard
  load's wall-clock figure once on the reference device defined in task 1.5, using the metric stated
  there, and record it in this change against NFR-7's bounded promise and US-QUAL-07's "loads in a
  bounded time". Assert no timing threshold in CI: the count assertions of tasks 5.1 and 5.8 stay
  the only gate (D6.7, risk R6).

## 6. Review

- [ ] 6.1 [reviewer] Review the read-path diff against P0 D0.2: the cache boundaries are unchanged,
  the frontend query cache stays derived and is never a record of truth, the frontend touches IPC
  only through generated bindings and `api.ts`, no new command or event was added, and no write path
  or write queue was introduced (D1).
- [ ] 6.2 [reviewer] Verify the delta adds exactly NFR-7 and no requirement from another phase, that
  every scenario is named by a test task, that the bound (**two** requests, D6.1) and the window
  (**30 s, global**, D6.2) carry the settled values and are asserted as independent of collection
  size, and that the CI-asserted count (tasks 5.1 and 5.8) is not conflated with the recorded
  wall-clock measurement (task 5.11); confirm the section 4 UI tasks captured screenshots at 390×844
  and 1280×800 in `.artifacts/screens/`, and confirm touch targets, safe areas and adaptive layout
  for those UI tasks.
