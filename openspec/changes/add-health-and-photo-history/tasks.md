# Tasks — add-health-and-photo-history (P2)

One task = one commit = one owner. Format: `- [ ] <n>.<m> [agent] <imperative>`.
Order: contract → backend → frontend-logic → frontend-ui → tests → review.
Tick a task only after the `definition-of-done` gate passes for it.
Commit message: `feat(add-health-and-photo-history): <task>`.
Every UI task captures screenshots at 390×844 and 1280×800 into `.artifacts/screens/` (gitignored).
Photo-source tasks state their platform explicitly: **iOS**, **macOS**, or **both**.

## 1. Contract and security surface

- [ ] 1.1 [architect] Extend the P0 IPC contract in `src-tauri/src/ipc/` with the health, tag-catalog,
  history and photo commands, events and shared types listed in the design's "IPC contract" section,
  reusing P0's `CommandError` / `SaveCause` / `Instant` and P1's `RabbitId` / `WeightEntry` /
  `WeightUnit`. Reconcile `list_rabbit_history` with P1's weight-only history and the dashboard-flag
  fields with P1's dashboard row type instead of adding parallel commands. Freeze the names, add typed
  stubs that return a `not implemented` error, and decide the photo-content transfer shape (device-local
  file reference versus bytes) plus the ordering/idempotency story that prevents an orphaned photo
  (design R3, D0.14).
- [ ] 1.2 [architect] Record in `design.md` (and `docs/adr/` if a mechanism-level choice is made) the
  photo storage/local-copy boundary, the offline availability decision that drives the SYNC-4
  placeholder, the lazy idempotent tag-catalog establishment, the derived (never stored) dashboard
  health flag, and the 7-day window definition once Q3 is answered.
- [ ] 1.3 [tauri-security] Declare the capability, CSP, scope, entitlement and permission-string
  changes that photo-source access needs on **both** iOS (photo-library permission) and macOS
  (photo-source scope); keep them least-privilege, add no permission the phase does not use, and keep
  both platform configurations in step.
- [ ] 1.4 [apple-native] Implement the photo-source bridge for **iOS** (PHPicker / PhotosUI) behind the
  frozen contract, including the not-granted path that attaches no photo and surfaces the reason.
- [ ] 1.5 [apple-native] Provide the photo-source bridge for **macOS** (Photos library / file source),
  `cfg`-clean alongside the iOS bridge and sharing the same contract surface.

## 2. Backend (Rust)

- [ ] 2.1 [rust-backend] Implement health-entry creation: zero or more household canonical tags, a
  severity of Mild/Moderate/Severe, an optional note, a photo list, and a P0 absolute-instant timestamp
  defaulting to now — through the P0 reachability-gated save path (HL-1, HL-2).
- [ ] 2.2 [rust-backend] Implement the household tag catalog: establish the nine canonical categories
  on first need, idempotently on the canonical category, identical for every member, never readable
  from or referenced by another household (HL-6, DM-2).
- [ ] 2.3 [rust-backend] Implement the health-entry read: list by rabbit with narrowing by tag,
  severity and date range, alone or combined, altering no stored entry (HL-4).
- [ ] 2.4 [rust-backend] Implement health-entry edit and delete, shared through the household
  collection, and refuse both for a health entry of an archived rabbit (HL-5).
- [ ] 2.5 [rust-backend] Implement the health-entry detail payload: severity, all tags, full
  untruncated note, timestamp, photos, and the archived state that makes it read-only (HL-7).
- [ ] 2.6 [rust-backend] Implement the interleaved history read: one timeline item model merging weight
  entries and health entries ordered by timestamp for the combined view, and the health-only narrowing
  by tag, severity and date range, resolved from the already-loaded collection (HL-3, DASH-7).
- [ ] 2.7 [rust-backend] Implement the derived dashboard health flag: a rabbit is flagged when a
  Severe health entry falls within the last 7 days, and the flag recomputes away when it is edited to a
  lower severity, deleted, or ages out; nothing is persisted onto the rabbit (DASH-4 severe clause).
- [ ] 2.8 [rust-backend] Implement photo attachment to a health entry and to a weight entry as part of
  the entry save: upload gated on reachability, content saved into the household collection, removal
  also a save, failures mapped to P0's `SaveCause` with no new cause (PHOTO-1).
- [ ] 2.9 [rust-backend] Implement the rabbit profile photo: settable, stored with the household
  collection, and exposed for a rabbit's thumbnail everywhere the rabbit is presented (PHOTO-2).
- [ ] 2.10 [rust-backend] Implement the per-rabbit gallery read: the rabbit's entry photos and its
  profile photo, with entry photos ordered by entry date, available for an archived rabbit
  (PHOTO-3).
- [ ] 2.11 [rust-backend] Ensure a photo attached to an entry or a rabbit is part of the household's
  shared collection so every member resolves the same photo, with exactly one copy per photo (PHOTO-4).
- [ ] 2.12 [rust-backend] Implement the photo local-copy boundary: keep the local copy of a displayed
  photo, resolve a held photo while unreachable, and report an explicit unavailable-offline state when
  it is not held — with no write queued and no success reported for an unmade change (SYNC-4).

## 3. Frontend logic

- [ ] 3.1 [frontend-logic] Run `bun run gen:bindings` against the frozen contract and add
  `src/features/health/api.ts`, `src/features/photos/api.ts` and `src/features/history/api.ts`
  wrappers; no `invoke` call anywhere outside these files.
- [ ] 3.2 [frontend-logic] Implement the health data layer: entry list and detail queries, the
  household category catalog hook, health-entry create/edit/delete mutations through the P0 save path,
  and invalidation on `health:changed` (HL-1, HL-2, HL-5, HL-7).
- [ ] 3.3 [frontend-logic] Implement the health filter and history-view state: tag, severity and date
  range kept independently of the stored data, plus the health-only and combined history hooks over
  `list_rabbit_history` (HL-4, HL-3, DASH-7).
- [ ] 3.4 [frontend-logic] Implement the photo pipeline: attach/detach during an entry save, the
  profile-photo mutation, the gallery query, and the availability state that distinguishes a locally
  held photo from an unavailable-offline one for the SYNC-4 placeholder (PHOTO-1, PHOTO-2, PHOTO-3,
  SYNC-4).
- [ ] 3.5 [frontend-logic] Extend the dashboard row data with the derived health flag from the P1
  dashboard query, so the dashboard renders one warning flag per rabbit without a second list request
  (DASH-4 severe clause).
- [ ] 3.6 [frontend-logic] Add the P2 i18n keys: category names, severity labels, filter labels, the
  history view titles, the cannot-load placeholder and the missing-photo state, with no P2 string
  hard-coded (NFR-5 stays P4's, the scaffold is P0's).

## 4. Frontend UI

- [ ] 4.1 [frontend-ui] Build the health-entry form: severity selection, multi-select category tags
  that allow none, optional note, a timestamp pre-filled with now and editable, and photo attach
  (HL-1, HL-2); screenshots at 390×844 and 1280×800 into `.artifacts/screens/`.
- [ ] 4.2 [frontend-ui] Build the health-entry detail view: severity, all tags, the full untruncated
  note, the timestamp, Edit and Delete for an active rabbit and neither for an archived one (HL-7);
  screenshots both viewports.
- [ ] 4.3 [frontend-ui] Build the health-only history view with the tag, severity and date-range
  filters, showing entries only and restoring hidden entries when a filter is removed (HL-4);
  screenshots both viewports.
- [ ] 4.4 [frontend-ui] Build the full combined history timeline: weight and health entries in one
  timestamp-ordered list, no first-entry prompt when only one kind exists, read-only for an archived
  rabbit (HL-3, DASH-7); screenshots both viewports.
- [ ] 4.5 [frontend-ui] Add photo attach and remove to the weight-entry and health-entry forms, with
  the iOS not-granted outcome surfaced and the entry still savable without photos (PHOTO-1);
  screenshots both viewports.
- [ ] 4.6 [frontend-ui] Build the swipeable full-width photo pager in an entry's detail view: paging
  within the detail, no pager when the entry has no photos, a directly visible next/previous control on
  a fine pointer, and the cannot-load placeholder in place of an unavailable photo (PHOTO-5, SYNC-4);
  screenshots both viewports.
- [ ] 4.7 [frontend-ui] Wire the timeline thumbnail to open the entry's detail view rather than any
  separate full-screen viewer (PHOTO-5); screenshots both viewports.
- [ ] 4.8 [frontend-ui] Build the rabbit profile-photo control and the shared thumbnail presentation
  with its non-photo placeholder, used by the rabbit list and the rabbit dashboard (PHOTO-2);
  screenshots both viewports.
- [ ] 4.9 [frontend-ui] Build the per-rabbit gallery ordered by entry date, reachable from an archived
  rabbit's timeline (PHOTO-3); screenshots both viewports.
- [ ] 4.10 [frontend-ui] Render the derived severe-health flag on the dashboard row as the same warning
  flag the weight-drop clause uses, clearing when no recent Severe entry remains (DASH-4 severe
  clause); screenshots both viewports.

## 5. Tests

- [ ] 5.1 [test-engineer] Rust tests for health-entry creation and the tagless case — covers "A
  structured health entry is created with tags and a severity", "An entry can be created with no note
  and no photo", "Severity is one of the three defined levels", "The timestamp defaults to now and is
  editable" (HL-1) and "A note-only entry is accepted", "A tagless entry displays without an error
  state" (HL-2).
- [ ] 5.2 [test-engineer] Rust tests for the tag catalog — covers "Each household owns exactly one copy
  of the canonical categories", "Establishing the catalog again creates no duplicate category",
  "A tag is never referenced by another household's entries" and "All members of a household resolve
  the same catalog" (HL-6, DM-2), including a concurrent establishment by two members.
- [ ] 5.3 [test-engineer] Rust tests for health filtering — covers "Filtering by tag shows only entries
  carrying it", "Filtering by severity shows only that severity", "Filtering by date range restricts to
  that range", "Filters can be combined" and "Removing a filter restores the entries it hid" (HL-4).
- [ ] 5.4 [test-engineer] Rust tests for edit and delete — covers "Editing a health entry is shared
  with members", "Deleting a health entry removes it for every member" (HL-5) and "An archived rabbit's
  entry cannot be edited or deleted".
- [ ] 5.5 [test-engineer] Rust tests for the health detail payload — covers "The detail shows every
  recorded field" (a note long enough to be truncated by any naive rendering), "The detail shows the
  entry's photos" and "An archived rabbit's health detail is read-only" (HL-7).
- [ ] 5.6 [test-engineer] Rust tests for the interleaved history — covers "The full history interleaves
  weight and health entries by timestamp", "A rabbit with only health entries shows no empty-state
  prompt", "The health-only history shows only health entries", "The health-only history by tag,
  severity and date range" and "An archived rabbit's history views are available without add, edit or
  delete actions" (HL-3, DASH-7).
- [ ] 5.7 [test-engineer] Rust tests for the dashboard health flag — covers "A Severe health entry
  within the last 7 days flags the rabbit", "A Severe health entry older than 7 days does not flag the
  rabbit", "A Mild or Moderate entry does not flag the rabbit" and "The flag clears when no recent
  Severe entry remains" (DASH-4 severe clause).
- [ ] 5.8 [test-engineer] Rust tests for photo attachment and sharing — covers "Photos are attached to
  a health entry", "Photos are attached to a weight entry", "An entry is savable without photos",
  "Several photos can be attached to one entry" (PHOTO-1), "A profile photo appears as the rabbit's
  thumbnail across the product", "A rabbit without a profile photo shows a placeholder", "Replacing a
  profile photo changes the thumbnail" (PHOTO-2), "The gallery shows the rabbit's photos", "Entry photos
  are ordered by entry date", "The gallery remains available for an archived rabbit" (PHOTO-3) and "A
  photo attached by one member is seen by another", "Members see the same profile photo" (PHOTO-4).
- [ ] 5.9 [test-engineer] Rust tests for the photo local copy — covers "A photo that was displayed once
  is shown again offline", "A never-displayed photo shows a cannot-load placeholder offline" and "An
  entry whose photos cannot load is not presented as having no photos" (SYNC-4), asserting no write is
  queued and no save is reported as succeeded while unreachable.
- [ ] 5.10 [test-engineer] Component tests for the health form, filters and history — covers "A
  structured health entry is created with tags and a severity" (HL-1), "A note-only entry is accepted"
  (HL-2), the five HL-4 filtering scenarios, and "The full history interleaves weight and health
  entries by timestamp", "The health-only history shows only health entries" (HL-3, DASH-7).
- [ ] 5.11 [test-engineer] Component tests for the entry detail — covers "The detail shows every
  recorded field", "The detail shows the entry's photos" (HL-7), "An archived rabbit's health detail is
  read-only", and "The detail shows a swipeable full-width pager", "Selecting a timeline thumbnail
  opens the detail view", "An entry with no photos shows no pager" (PHOTO-5).
- [ ] 5.12 [test-engineer] Component tests for the offline photo placeholder — covers "A never-displayed
  photo shows a cannot-load placeholder offline" and "An entry whose photos cannot load is not
  presented as having no photos" (SYNC-4), asserting the placeholder is distinct from the no-photo
  state.
- [ ] 5.13 [test-engineer] E2E (Playwright WebKit, mocked IPC, 390×844): log a health entry with tags,
  Severe severity and a photo → open its detail and page through the photos → filter the health-only
  history by tag, severity and range → open the combined history and see it interleaved → confirm the
  dashboard row is flagged; covers the `[mobile]` flow of HL-1/HL-4/HL-7/PHOTO-5 and the DASH-4 severe
  clause.
- [ ] 5.14 [test-engineer] E2E at 1280×800: the same health detail with pager navigation by the visible
  next/previous control, the gallery ordered by entry date, and the profile thumbnail on the dashboard;
  covers "The pager is operable without a swipe gesture" (PHOTO-5), PHOTO-2 and PHOTO-3.
- [ ] 5.15 [test-engineer] Run the full gate (`cargo fmt --check`, `cargo clippy`, `cargo test`,
  `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` with no diff,
  `git diff --exit-code src/routeTree.gen.ts` after a build, `bun run typecheck`, `bun run lint`,
  `bun run test`, `bun run test:e2e`) and record the results in the change.

## 6. Review

- [ ] 6.1 [reviewer] Review the P2 diff for contract and write-model conformance: frontend touches IPC
  only through generated bindings and `api.ts`, every health/catalog/photo change goes through the P0
  reachability gate, no local write queue or optimistic write exists (D1), and the photo local copy is
  a read cache only.
- [ ] 6.2 [reviewer] Verify the delta contains exactly HL-1, HL-2, HL-4, HL-5, HL-7, PHOTO-1 … PHOTO-5,
  the DASH-4 severe-health clause, the DASH-7 health clause, HL-3, SYNC-4 and DM-2 — no added scope, no
  P1 or P5 requirement name restated or altered, no requirement without a scenario, every UI task's
  `.artifacts/screens/` screenshots present at 390×844 and 1280×800 with touch targets, safe areas and
  the adaptive layout confirmed on the reviewed surfaces, and the SYNC-4 placeholder checked against D1
  for honesty.
