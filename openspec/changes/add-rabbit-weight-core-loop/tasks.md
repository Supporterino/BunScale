# Tasks — add-rabbit-weight-core-loop (P1)

One task = one commit = one owner. Format: `- [ ] <n>.<m> [agent] <imperative>`.
Order: contract → backend → frontend-logic → frontend-ui → tests → review.
Tick a task only after the `definition-of-done` gate passes for it.
Commit message: `feat(add-rabbit-weight-core-loop): <task>`.

Every UI task (section 4) captures screenshots at **390×844 and 1280×800** into
`.artifacts/screens/`, per `definition-of-done`.

`tauri-security` has no task in this change: P1 adds no capability, CSP entry, scope or entitlement
(no photo-library access — see `design.md` D1.11).

## 1. Contract

- [ ] 1.1 [architect] Extend the P0 IPC contract in `src-tauri/src/ipc/` with the P1 commands, events
  and shared types listed in `design.md` ("IPC contract"): rabbit create/list/get/update/archive/
  restore/delete, weight entry create/list/get/update/delete, the dashboard read, `Rabbit`,
  `RabbitSummary`, `DashboardRow`, `WeightEntry`, `WeightEntrySummary`, `WeightRange`,
  `WeightTrend`, `Sex`, `NeuterStatus`, plus typed stubs returning a `not implemented` error. Reuse
  P0's `CommandError`, `SaveCause`, `Instant`, `WeightUnit` and `HouseholdId` unchanged. Declare the
  fixed value sets with the types: `Sex` = `male | female | unknown`, `NeuterStatus` =
  `intact | neutered | spayed | unknown` (D1.14). Decide and record whether the entry-side unit
  conversion happens in `api.ts` or in the weight commands. Freeze the names for this phase.
  (RAB-1 … RAB-10, WT-1 … WT-8, DASH-1 … DASH-9, DM-1)
- [ ] 1.2 [architect] Record the P1 decisions in `design.md`: the derived (never stored) drop alert
  with the threshold travelling with the trend series (D1.3), range selection as route state with two
  independent ranges (D1.4), archived immutability enforced at the write boundary (D1.5), the alert
  ordering — timestamp order with the earlier-created entry as the previous entry on a tie (D1.12) —
  the alert as a pure derivation over the current entry set that an edit or a deletion re-evaluates
  (D1.13), and the fixed `Sex` and `NeuterStatus` value sets (`male | female | unknown`;
  `intact | neutered | spayed | unknown`, D1.14). (RAB-1, WT-6, WT-7, DM-4)
- [ ] 1.3 [architect] Confirm in `design.md` that P1 opens no new capability, CSP, scope or
  entitlement — the rabbit photo reference is carried and rendered but selecting a photo is PHOTO-2
  in P2 — and state that no shared-service dependency is added beyond P0's selected mechanism.
  (RAB-1, DASH-1)

## 2. Backend (Rust)

- [ ] 2.1 [rust-backend] Implement rabbit profile creation with the full field set: name, optional
  photo reference, optional breed, optional date of birth with an explicit "unknown" marker, sex,
  neuter status and optional adoption date; unsupplied fields recorded as absent (RAB-1).
- [ ] 2.2 [rust-backend] Enforce non-blank rabbit names on create and update, treating
  whitespace-only input as blank and refusing it through P0's failure surface with a validation cause
  (RAB-2).
- [ ] 2.3 [rust-backend] Implement multiple rabbits per household with no fixed upper limit, each
  belonging to exactly one household (RAB-3).
- [ ] 2.4 [rust-backend] Implement editing every field of an active rabbit's profile, including the
  weight-drop threshold, and share the change with all members through P0's collection write path;
  refuse an edit to an archived rabbit (RAB-5, HS-8).
- [ ] 2.5 [rust-backend] Implement archiving a rabbit: excluded from the active list, included in the
  archived list, no data discarded (RAB-6, RAB-9).
- [ ] 2.6 [rust-backend] Implement restoring an archived rabbit to the active list with its history
  intact (RAB-7).
- [ ] 2.7 [rust-backend] Implement permanent deletion of an archived rabbit with all its weight
  entries, cascading within the household collection and shared with all members; refuse permanent
  deletion of an active rabbit through P0's failure surface, since RAB-10 makes archival the
  precondition (RAB-10).
- [ ] 2.8 [rust-backend] Enforce archived-rabbit immutability on every write path at the storage
  boundary: creating, editing or deleting any weight entry of an archived rabbit is refused, and the
  refusal is reported through P0's failure surface (DM-4, RAB-8).
- [ ] 2.9 [rust-backend] Implement weight entry creation: canonical grams storage with no per-entry
  unit, timestamp defaulting to now or taken from the supplied instant unchanged, optional note
  (WT-1, DM-1, WT-3).
- [ ] 2.10 [rust-backend] Implement weight value validation: reject zero and negative values through
  P0's failure surface, and mark a value above the plausibility bound as requiring confirmation
  (WT-4).
- [ ] 2.11 [rust-backend] Implement editing and deleting a past weight entry, shared with all
  members; both re-derive the alerts of the entries around the change, so after a deletion the
  following entry is compared against the entry that now precedes it (D1.13); refuse both for an
  archived rabbit (WT-5, WT-7, DM-4).
- [ ] 2.12 [rust-backend] Implement the weight trend and history read: a `WeightRange` window of
  7/30/90/all returning the entries inside it, the rabbit's threshold alongside, and the two ranges
  served independently so one query cannot influence the other (WT-6).
- [ ] 2.13 [rust-backend] Implement the drop-alert derivation: order a rabbit's entries by timestamp
  and take the earlier-created entry as the previous entry when two share a timestamp (D1.12); flag
  an entry when its drop against that previous entry exceeds the rabbit's threshold (default 5%);
  derive on read over the current entry set so a threshold change, an edit or a deletion re-evaluates
  the entries around it without rewriting stored entries (D1.13, WT-7).
- [ ] 2.14 [rust-backend] Implement the dashboard read model: non-archived rabbits with name,
  thumbnail reference, last-weighed timestamp or never-weighed indication, sorted by most recent
  weigh-in with never-weighed rabbits last, and the derived weight-drop flag per row (DASH-1, DASH-2,
  DASH-3, DASH-4).
- [ ] 2.15 [rust-backend] Implement the weight entry detail read: value in grams, timestamp,
  untruncated note, derived drop-alert flag and the entry's photo references (WT-8).

## 3. Frontend logic

- [ ] 3.1 [frontend-logic] Run `bun run gen:bindings` against the frozen P1 contract and add
  `src/features/rabbits/api.ts`, `src/features/weight/api.ts` and `src/features/dashboard/api.ts`
  wrappers; no `invoke` call anywhere outside these files (project hard rule 2).
- [ ] 3.2 [frontend-logic] Add the rabbit query keys and hooks: the active list, the archived list,
  one rabbit's profile and the dashboard rows (DASH-1, DASH-6, RAB-6).
- [ ] 3.3 [frontend-logic] Add the weight query hooks parameterised by range, separately for the
  trend and for the weight-only history, so the two carry independent range state in the route's typed
  search parameters (WT-6).
- [ ] 3.4 [frontend-logic] Add the rabbit mutations (create, update, archive, restore, delete) with
  invalidation of only the affected queries (RAB-1, RAB-5, RAB-6, RAB-7, RAB-10).
- [ ] 3.5 [frontend-logic] Add the weight mutations (create, update, delete) with the display-unit
  parse/format at the entry boundary and invalidation of only the affected trend, history, detail and
  dashboard queries (WT-1, WT-3, WT-5, DM-1).
- [ ] 3.6 [frontend-logic] Implement weight form validation state: non-positive values rejected with a
  prompt, values above the plausibility bound held behind a confirmation, and programmed save failing
  visibly through P0's cause-specific surface (WT-4).
- [ ] 3.7 [frontend-logic] Implement the derived alert and dashboard-flag state from the contract
  payload so a threshold edit re-derives the flags without a full reload (WT-7, DASH-4).
- [ ] 3.8 [frontend-logic] Add the P1 i18n string keys — including the "never weighed" indication,
  archive/restore/delete labels, confirmation copy stating consequences, and the out-of-range
  confirmation — so no P1 string is hard-coded (DASH-2, RAB-10, WT-4, NFR-9).

## 4. Frontend UI

- [ ] 4.1 [frontend-ui] Build the dashboard rabbit list: name, thumbnail with placeholder fallback,
  last-weighed date or never-weighed indication, sorted with never-weighed last; laid out for 390×844
  first and 1280×800 second (DASH-1, DASH-2, DASH-3).
- [ ] 4.2 [frontend-ui] Build the dashboard empty state prompting the person to add their first
  rabbit, with the add control present from it (DASH-8, RAB-4).
- [ ] 4.3 [frontend-ui] Build the rabbit-creation form reachable from the dashboard at all times:
  name, optional photo, breed, date of birth with an "unknown" option, sex offered as exactly
  `male`/`female`/`unknown`, neuter status offered as exactly `intact`/`neutered`/`spayed`/`unknown`
  (D1.14) and adoption date, with inline blank-name rejection (RAB-1, RAB-2, RAB-4).
- [ ] 4.4 [frontend-ui] Build the rabbit profile detail with a directly visible edit control and the
  pre-filled edit form covering every field including the weight-drop threshold (RAB-5).
- [ ] 4.5 [frontend-ui] Build the archive control, the archived-rabbit list entry point, and the
  restore and permanent-delete controls; the permanent-delete control lives on the archived list and
  in the archived rabbit's read-only surface and is absent from an active rabbit's profile, deleting
  requires a confirmation stating the consequence, and archiving does not (RAB-6, RAB-7, RAB-10,
  NFR-9).
- [ ] 4.6 [frontend-ui] Build the read-only presentation of an archived rabbit: its dashboard, its
  history and its entry details without any control that adds, edits or deletes its weight data (the
  lifecycle controls — restore and permanent delete — belong to the archived list, task 4.5)
  (RAB-8, DM-4, DASH-7).
- [ ] 4.7 [frontend-ui] Build the per-row quick-weight control that opens the weight form pre-bound to
  that row's rabbit, as a directly visible touchable control within the row (DASH-9, WT-2, NFR-9).
- [ ] 4.8 [frontend-ui] Build the quick-entry control with a rabbit picker that opens the same weight
  form bound to the chosen rabbit (WT-2).
- [ ] 4.9 [frontend-ui] Build the weight entry form: value in the preferred display unit with **no
  unit picker**, editable timestamp defaulting to now, optional note, non-positive rejection and the
  out-of-bound confirmation (WT-1, WT-3, WT-4).
- [ ] 4.10 [frontend-ui] Build the weight entry detail view: display-unit value, timestamp, full
  untruncated note, the drop-alert marker when it applies, the entry's photos as a full-width pager
  when present and none when absent, plus Edit opening the pre-filled form and a confirmed Delete
  (WT-8).
- [ ] 4.11 [frontend-ui] Build the rabbit dashboard: the weight trend chart with the 7d/30d/90d/all
  range control and **no baseline or healthy-range band** (D5), and the History section contributing
  the weight-only history entry point with its own independent range control and the drop-alert marker
  on affected entries (DASH-6, DASH-7, WT-6, WT-7).
- [ ] 4.12 [frontend-ui] Build the dashboard drop-alert flag on rabbit rows, visually distinct from an
  informational badge and cleared when no entry is flagged (DASH-4).
- [ ] 4.13 [frontend-ui] Apply the NFR-9 conventions across the P1 surfaces: every primary and
  destructive action is a directly visible control, no action is reachable only through a long press
  or a hidden menu, and every destructive action is confirmed with its consequence (NFR-9).
- [ ] 4.14 [frontend-ui] Capture the consolidated P1 screenshot set at 390×844 and 1280×800 into
  `.artifacts/screens/`, and verify touch targets, safe areas and the adaptive layout on both
  baselines (DASH-1, DASH-9, DASH-7).

## 5. Tests

- [ ] 5.1 [test-engineer] Rust tests for rabbit profiles — covers "A profile is created with the
  supplied identifying details", "Optional fields may be left unset", "Date of birth may be marked
  unknown", "A neutered and a spayed rabbit are recorded distinctly", "Sex and neuter status may each
  be recorded as unknown" (RAB-1), "An empty name is rejected", "A whitespace-only name is treated as
  blank" (RAB-2) and "Additional rabbits are created without restriction" (RAB-3).
- [ ] 5.2 [test-engineer] Rust tests for profile editing and rabbit lifecycle — covers "A saved edit
  including the threshold is shared with every member" (RAB-5), "Archiving removes the rabbit from
  the active list" (RAB-6), "Restoring returns the rabbit to the active list", "Restoration leaves the
  history unchanged" (RAB-7) and "Confirmed deletion removes the rabbit and its data", "An active
  rabbit cannot be permanently deleted", "The deletion is shared with every member" (RAB-10).
- [ ] 5.3 [test-engineer] Rust tests for archived immutability and retention — covers "No new entry
  can be added to an archived rabbit", "Archived entries cannot be edited or deleted" (RAB-8),
  "Archived history survives an arbitrary interval", "Archiving discards nothing" (RAB-9) and "No
  weight data of an archived rabbit is mutable", "The archive is retrievable and its data is retained
  by default" (DM-4), asserting the refusal through the command path and not only through the UI.
- [ ] 5.4 [test-engineer] Rust tests for weight entries — covers "An entry is recorded with the
  current time", "An entry records an edited timestamp", "The note is optional" (WT-1), "Every stored
  weight is in grams", "A weight entry carries no unit of its own", "A value entered in kilograms is
  stored as grams" (DM-1), "A zero value is rejected", "A negative value is rejected", "A confirmed
  outlier is saved" (WT-4) and the WT-5 edit and delete sharing.
- [ ] 5.5 [test-engineer] Rust tests for the drop alert — covers "A drop exceeding the threshold is
  flagged", "A drop within the threshold is not flagged", "Raising the threshold clears an existing
  flag", "Lowering the threshold raises a flag", "Entries sharing a timestamp are ordered by creation",
  "Deleting an entry re-derives the following entry's alert", "Editing an entry re-derives the alerts
  around it" (WT-7), asserting that no stored entry is rewritten when the threshold changes and that
  the dashboard flag (DASH-4) is derived from the same result.
- [ ] 5.6 [test-engineer] Rust tests for the dashboard read model — covers "Both active rabbits are
  listed with their details", "An archived rabbit is not listed" (DASH-1), "A rabbit with no entries
  is marked as never weighed" (DASH-2) and "The most recently weighed rabbit sorts first", "A
  never-weighed rabbit sorts last" (DASH-3), including a back-dated entry case.
- [ ] 5.7 [test-engineer] Rust tests for trend and history ranges — covers "Selecting a range limits
  the trend to that window", "The full-history range presents every entry", "The weight history range
  is independent of the trend range" (WT-6).
- [ ] 5.8 [test-engineer] Rust tests for the weight entry detail read — covers "The detail shows the
  value in the display unit and the untruncated note" and "The drop-alert flag appears in the detail
  when it applies" (WT-8), asserting the note is returned whole.
- [ ] 5.9 [test-engineer] Rust tests for equal member access — covers "A member edits data created by
  another member", "A joining member writes into the same shared data" and "No member is offered a
  reduced view of household data" (HS-8).
- [ ] 5.10 [test-engineer] Component tests for the dashboard — covers "Both active rabbits are listed
  with their details", "An archived rabbit is not listed" (DASH-1), "A rabbit with no entries is
  marked as never weighed" (DASH-2), "The most recently weighed rabbit sorts first", "A never-weighed
  rabbit sorts last" (DASH-3), "A rabbit with a flagged drop is flagged", "The flag clears when the
  threshold is raised" (DASH-4), "The empty state prompts for the first rabbit" (DASH-8) and "The row
  control opens the form bound to that rabbit" (DASH-9).
- [ ] 5.11 [test-engineer] Component tests for rabbit creation, profile editing and deletion — covers
  "The add control is available whenever the dashboard is shown", "The empty state includes an
  add-first-rabbit prompt" (RAB-4), "A profile is created with the supplied identifying details", "Date of birth
  may be marked unknown" (RAB-1), "An empty name is rejected" (RAB-2), "The edit control is directly
  visible on the rabbit's detail", "An archived rabbit's profile is not editable" (RAB-5), "Without
  confirmation nothing is deleted" (RAB-10) and the NFR-9 visibility and confirmation scenarios.
- [ ] 5.12 [test-engineer] Component tests for the weight entry form — covers "A value entered in the
  preferred unit is saved in the canonical unit", "The entry form offers no unit picker", "The form
  uses the regional default when no unit was chosen" (WT-3), "A zero value is rejected", "A negative
  value is rejected", "A value above the plausibility bound requires confirmation" (WT-4) and the
  WT-1 timestamp and note cases.
- [ ] 5.13 [test-engineer] Component tests for the weight entry detail — covers "Edit opens the
  pre-filled form and Delete is confirmed" (WT-8), "An entry with no photos shows no pager" (WT-8),
  "An archived rabbit's entry detail is read-only" (WT-8) and the DM-4 archived read-only presentation.
- [ ] 5.14 [test-engineer] Component tests for the rabbit dashboard and history, including the
  screen-reader summary of the chart — covers "Selecting a rabbit opens its dashboard" (DASH-6), "The
  rabbit dashboard presents the weight trend and the history entry point", "The chart's range selection
  limits the presented trend", "An archived rabbit's dashboard is read-only" (DASH-7) and "The weight
  history range is independent of the trend range" (WT-6).
- [ ] 5.15 [test-engineer] E2E (Playwright WebKit, mocked IPC, 390×844): launch with an empty household
  → empty-state prompt → add the first rabbit → log a weight from the dashboard row in no more than
  three taps → the rabbit dashboard shows the trend → switch the chart to 30 days → set the weight
  history to 7 days; covers the `[mobile]` scenarios of WT-2 and DASH-9, and DASH-8, DASH-6, DASH-7,
  WT-6.
- [ ] 5.16 [test-engineer] E2E: record 2000 g then 1800 g at a 5% threshold, assert the entry warning
  and the dashboard flag, raise the threshold to 15% and assert both clear; covers "A drop exceeding
  the threshold is flagged", "Raising the threshold clears an existing flag" (WT-7) and "The flag
  clears when the threshold is raised" (DASH-4).
- [ ] 5.17 [test-engineer] E2E: archive a rabbit with history → it leaves the active list, the archived
  list shows it, its dashboard and entry detail carry no add/edit/delete control, then restore and
  assert the history is intact; covers RAB-6, RAB-7, RAB-8, RAB-9, DM-4.
- [ ] 5.18 [test-engineer] E2E at 1280×800: the rabbit list, opening a rabbit's dashboard, the weight
  trend with a range change and a weight entry detail with photos moved through without a touch
  gesture; covers the `[desktop]` scenarios of DASH-6 and WT-8, plus DASH-1 and DASH-7.
- [ ] 5.19 [test-engineer] Run the full gate (`cargo fmt --check`, `cargo clippy`, `cargo test`,
  `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` with no diff on `src/bindings.ts`,
  `git diff --exit-code src/routeTree.gen.ts` after a build, `bun run typecheck`, `bun run lint`,
  `bun run test`, `bun run test:e2e`) and record the results in the change.

## 6. Review

- [ ] 6.1 [reviewer] Verify the delta contains exactly RAB-1 … RAB-10, WT-1 … WT-8, DASH-1/2/3/6/8/9
  plus `Rabbit dashboard with weight trend and history entry points` and `Rabbits with a weight-drop
  alert are flagged on the dashboard`, HS-8, NFR-9, DM-1 and DM-4 — no added scope, no requirement
  implemented without a scenario, and no health-logging, medication, export or reminder requirement
  introduced.
- [ ] 6.2 [reviewer] Verify decisions D1 and D5 hold in the diff: no offline write path, no optimistic
  or pending write state, no local write queue (D1); no chart baseline, healthy-range band, per-rabbit
  baseline field or legend (D5); IPC reached only through generated bindings and `api.ts`; and archived
  immutability refused at the write boundary rather than only hidden in the UI (DM-4).
