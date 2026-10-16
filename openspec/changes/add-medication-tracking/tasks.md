# Tasks — add-medication-tracking (P5)

One task = one commit = one owner. Format: `- [ ] <n>.<m> [agent] <imperative>`.
Order: contract → backend → frontend-logic → frontend-ui → tests → review.
Tick a task only after the `definition-of-done` gate passes for it.
Commit message: `feat(add-medication-tracking): <task>`.
Every UI task captures screenshots at 390×844 and 1280×800 into `.artifacts/screens/` (gitignored).

No `tauri-security` task appears here: this change alters no capability, CSP, scope, entitlement or
permission string (design D5.9). Task 1.3 records that finding rather than leaving it implicit.

Medication editing is implemented; medication deletion never is. The source documents never state that a
medication's own details are editable or that a medication is deletable — this is a deliberate addition
(**MED-13**, `specs/medication-tracking/spec.md`) reached as a product-owner decision, and that decision
rejects full CRUD: a medication course is **never** deleted, so no task builds a delete affordance and no
contract command deletes one. An active course's details are correctable, and a finished course's end
date stays correctable so a mistyped end date is never frozen (MED-2, MED-12).

MED-12 carries exactly **two** bounded exceptions, and no task builds a third. The end-date correction is
MED-13's; the other is that a dose **actually given** stays recordable against a finished course, with
its own timestamp, while no existing dose is ever edited, deleted or retargeted there and the course's
other attributes stay read-only (MED-12, design D5.15). An **archived rabbit** gets neither exception
(DM-4).

## 1. Contract

- [ ] 1.1 [architect] Extend the P0 IPC contract in `src-tauri/src/ipc/` with the medication,
  schedule and dose commands, events and shared types listed in the design's "IPC contract" section,
  reusing P0's `CommandError` / `SaveCause` / `Instant` / `MemberId` and P1's `RabbitId` unchanged.
  Model `Schedule` as one neutral tagged value type with the four variants, `TimeOfDay` as a wall-clock
  time with no date and no offset, and the 1-to-4 slot invariant; keep the type free of any reminder or
  export concern (P4 consumes it for both). Reconcile the active-medication badge field with P1's and
  P2's dashboard row type instead of adding a second dashboard command. Carry a dose's **own unit
  together with its own value**, so a dose may name a unit other than the serving's, and type the
  positive-amount rule with no upper bound (D5.14). Type `check_recent_dose(medication_id, at)` so `at`
  is the **timestamp entered for the dose it guards** and so it reports the doses of that medication
  inside the approximately 15-minute window **on both sides of `at`**, read afresh by that call
  (D5.12, D5.13). Replace the single `is_read_only` flag with the derived affordance set that expresses
  MED-12's two bounded exceptions (D5.6, D5.15). Expose
  `update_medication(medication_id, patch)` for the medication's own details — name, reason, start date,
  end date, serving value and unit, schedule and note — and **no command that deletes a medication at
  all** (MED-13). Freeze the names and add typed stubs that return a `not implemented` error.
- [ ] 1.2 [architect] Record in `design.md` (and in `docs/adr/` only if a mechanism-level choice is
  made) the schedule value type and its slot invariants, the derived-never-stored active/finished
  status, the dose-giver attribution path, the near-simultaneous check as a read-then-confirm rule that
  performs no write — including that its window is measured from the timestamp entered for the new dose,
  covers doses on both sides of that instant, and is evaluated only after a fresh read of that
  medication's doses (D5.12, D5.13) — the amount rules (a dose's own unit, which may differ from the
  serving's; positivity; no plausibility bound; no conversion, D5.14), and where the human-readable
  schedule summary is produced. Record the corrected read-only boundary (MED-13) as carrying **two**
  bounded exceptions and no others: an active course's details are correctable, a finished course's end
  date stays correctable and a dose actually given stays recordable against it while its other
  attributes and its whole dose history stay read-only with no existing dose edited, deleted or
  retargeted (D5.15), an archived rabbit is correctable in nothing and gains neither exception, and no
  path deletes a medication.
- [ ] 1.3 [architect] Confirm and record that this phase requires no capability, CSP, scope,
  entitlement or permission-string change, and that the notification permission belongs to
  `add-vet-export-and-reminders` (P4) rather than here.

## 2. Backend (Rust)

- [ ] 2.1 [rust-backend] Implement medication creation through the P0 reachability-gated save path:
  name, optional reason, start date defaulting to today and editable, optional end date, amount value
  with a unit from exactly {mg, ml, drops, tablets}, optional note; name, start date and amount
  required; **the amount required to be positive, with no plausibility confirmation**; several
  medications concurrently active (MED-1, D5.14).
- [ ] 2.2 [rust-backend] Implement the derived active/finished status from the end date and the present
  moment — active with no end date or a future one, finished once passed, finished immediately when the
  end date is set to today — with nothing stored on the medication and the dose history retained
  (MED-2).
- [ ] 2.3 [rust-backend] Implement the four schedule variants as one value type: times per day with 1
  to 4 explicit slot times, every X days with the interval counted fixed from the medication's start
  date at a chosen time, weekly with exactly one weekday plus a time, and as needed with an optional
  maximum-per-day hint and no slots; enforce the slot invariants where a schedule is accepted (MED-3).
- [ ] 2.4 [rust-backend] Implement the human-readable schedule summary derivation from the structured
  schedule, covering all four variants and their parameters, rendering with and without recorded doses
  and never failing for any variant (MED-4).
- [ ] 2.5 [rust-backend] Implement dose recording: the medication, a timestamp defaulting to now and
  supplied unchanged when given, an amount pre-filled from the serving at the form boundary and accepted
  as supplied **together with its own unit, which may differ from the serving's**, an optional note, a
  refusal of a non-positive amount with no plausibility confirmation and no conversion between units,
  and the recording member captured as the giver (MED-5, D5.14).
- [ ] 2.6 [rust-backend] Implement the chronological dose list read, each dose carrying its timestamp,
  its amount and its giver, resolved from the household's collection (MED-6).
- [ ] 2.7 [rust-backend] Implement dose editing of the amount, the note and the timestamp, and dose
  deletion, both shared through the household collection (MED-6).
- [ ] 2.8 [rust-backend] Implement giver attribution at the storage boundary: a stored dose always
  carries exactly one giver, the giver is a member of the dose's own household and never a member of
  another household, the giver is written from the recording identity and is not patchable, and no
  permitted dose edit retargets it (DM-5).
- [ ] 2.9 [rust-backend] Implement the near-simultaneous check as a read that performs no write: given a
  medication and the timestamp entered for the dose being confirmed, read that medication's doses
  **afresh** from the household's shared collection for this call and report every dose of the same
  medication inside the approximately 15-minute window **on both sides** of the entered timestamp,
  returning them, and record nothing — so a back-dated dose colliding with a later one is caught as well
  as one logged minutes earlier, and neither anchor nor freshness is left to the data the device happens
  to hold (MED-7, D5.12, D5.13).
- [ ] 2.10 [rust-backend] Enforce read-only at the write boundary with MED-12's **two** exceptions and no
  others: refuse every edit and every deletion of an existing dose, and every medication-detail write
  other than a correction of the end date, targeting a finished medication; **accept recording a dose
  that was actually given against a finished medication**, with its own timestamp and without touching
  that medication's other attributes or any existing dose; and refuse every medication write, every dose
  write — recording a dose included — and the end-date correction targeting a rabbit that is archived;
  reporting each refusal through P0's single failure surface, while leaving the records readable
  (MED-12, MED-13, D5.15).
- [ ] 2.11 [rust-backend] Extend P1's dashboard read with the derived active-medication badge field:
  set when the rabbit has at least one active medication, clear when none is active, never stored
  (DASH-5).
- [ ] 2.12 [rust-backend] Implement the medications section read for a rabbit: every medication with
  its derived status and its schedule summary, finished medications included, plus the medication
  detail payload carrying the fields, the status, the structured schedule and the **derived affordance
  set of MED-12's two exceptions, which the section and the detail share** (DASH-7, D5.6).
- [ ] 2.13 [rust-backend] Implement the medication-detail correction path for an active course — name,
  reason, start date, end date, amount per serving with its unit, schedule and note — applied through the
  same reachability-gated save and shared through the household collection, with the end date remaining
  correctable while the medication is finished so a mistyped end date can be repaired and the course
  returns to active under the derived status; add **no command and no path that deletes a medication**,
  and assert a correction leaves every recorded dose untouched (MED-13).

## 3. Frontend logic

- [ ] 3.1 [frontend-logic] Run `bun run gen:bindings` against the frozen contract and add
  `src/features/medications/api.ts` wrappers; no `invoke` call anywhere outside these files.
- [ ] 3.2 [frontend-logic] Implement the medication data layer: medication-by-rabbit and medication
  detail queries, the dose list query, the medication create and correction mutations, the dose
  record/edit/delete mutations, household-scoped query keys, and invalidation on `medication:changed`
  and `dose:changed` only.
- [ ] 3.3 [frontend-logic] Implement the schedule form state for all four types: the times-per-day slot
  count limited to 1 to 4 with editable pre-filled times, the every-X-days interval and time, the
  weekly weekday and time, and the as-needed optional maximum per day, with the required-field rules of
  MED-1, the serving amount held to a positive value with the unit restricted to the four supported
  units, and **no plausibility confirmation on the serving amount** (MED-1, MED-3, D5.14).
- [ ] 3.4 [frontend-logic] Implement the dose mutation layer: the confirmation payload pre-filled from
  the medication's amount per serving with an editable amount **and its own editable unit, which may
  differ from the serving's**, an editable timestamp defaulting to now and an optional note, a refusal
  of a non-positive amount with no plausibility confirmation, and the given medication bound to the
  confirmation (MED-5, MED-8, D5.14).
- [ ] 3.5 [frontend-logic] Implement the near-simultaneous warning state: run the check against the
  timestamp **entered for the dose** — never the moment the person taps confirm — so doses **before and
  after** that instant are candidates, run it **on a fresh read** of that medication's doses rather than
  on the data the query cache already holds, hold the returned existing dose or doses for presentation,
  allow continue (which records exactly one dose) or cancel (which records nothing), and never treat the
  check itself as a write (MED-7, D5.12, D5.13).
- [ ] 3.6 [frontend-logic] Extend the dashboard row data with the derived active-medication badge field
  from P1's and P2's dashboard query, and add the medications section query with each medication's
  status and schedule summary, so no second list request is needed (DASH-5, DASH-7).
- [ ] 3.7 [frontend-logic] Implement the read-only state plumbing from the contract's derived affordance
  set: an archived rabbit gates every medication and dose mutation including recording a dose and the
  end-date correction; a finished medication gates every dose edit and deletion and every
  medication-detail change except the correction of its end date, **while recording a dose that was
  actually given stays available**; and the affordances are exposed to the presentation layer with reads
  left available (MED-12, MED-13, D5.6, D5.15).
- [ ] 3.8 [frontend-logic] Add the P5 i18n keys: medication field labels, the four schedule types and
  their summaries, the four unit labels, the dose list and edit labels, the double-dose warning and its
  continue/cancel actions, with no P5 string hard-coded (MED-4, MED-7). The full German/English
  coverage stays NFR-5 in P4; the scaffold is P0's.
- [ ] 3.9 [frontend-logic] Implement the medication correction mutation layer: correct an active
  medication's details, and a finished medication's end date only, both through the reachability-gated
  save path with the result invalidated on `medication:changed`; propose no delete mutation for a
  medication anywhere (MED-13).

## 4. Frontend UI

- [ ] 4.1 [frontend-ui] Build the medication form: name, reason, start date pre-filled with today,
  end date, amount with the four-unit selector that refuses a non-positive amount and applies no
  plausibility confirmation, note, and the four-way schedule editor whose times-per-day slots arrive
  pre-filled and editable; screenshots at 390×844 and 1280×800 into `.artifacts/screens/`
  (MED-1, MED-3, D5.14).
- [ ] 4.2 [frontend-ui] Build the medication detail: the medication's fields, its active or finished
  status, its schedule summary in full, and the primary give-dose action as a directly visible control;
  screenshots both viewports (MED-2, MED-4, MED-8).
- [ ] 4.3 [frontend-ui] Build the give-dose confirmation: the amount pre-filled from the serving and
  editable together with **its own unit**, an editable timestamp defaulting to now, an optional note, a
  refusal of a non-positive amount with no plausibility confirmation, and a single primary confirm that
  records the dose and updates the presented list in place — the same confirmation whether the course is
  active or finished, since a dose actually given stays recordable against a finished course; screenshots
  both viewports (MED-5, MED-8, MED-12, D5.14, D5.15).
- [ ] 4.4 [frontend-ui] Build the near-simultaneous warning presentation: shown before the dose is
  confirmed, naming the existing dose or doses inside the window — including one that lies **after** the
  timestamp entered for the new dose — offering continue and cancel as directly visible controls;
  screenshots both viewports (MED-7, D5.12).
- [ ] 4.5 [frontend-ui] Build the chronological dose list with each dose's timestamp, amount and giver,
  plus per-dose edit (amount, note, timestamp) and delete behind a consequence confirmation;
  screenshots both viewports (MED-6).
- [ ] 4.6 [frontend-ui] Build the medications section on the rabbit dashboard: each medication with its
  active/finished status and its schedule summary, an entry point to its detail and dose history, and
  the presentation for a rabbit with no medications; screenshots both viewports (DASH-7).
- [ ] 4.7 [frontend-ui] Render the active-medication badge on the dashboard row in informational
  styling, visually distinct from the warning flag and never warning styled, absent when no medication
  is active, and visible without a hover; screenshots both viewports (DASH-5).
- [ ] 4.8 [frontend-ui] Build the read-only presentations from the derived affordances: a finished
  medication's detail and dose list offering no control that edits or deletes an existing dose and no
  control that changes any attribute other than the end date, **while the give-dose action stays visible
  for a dose that was actually given**, and an archived rabbit's medications section offering none at
  all, with reads still available; screenshots both viewports (MED-12, MED-13, D5.15).
- [ ] 4.9 [frontend-ui] Build the medication correction affordance: a directly visible edit control on an
  active medication's detail reaching all of its details, and on a finished medication only its end date,
  with **no delete control for a medication anywhere**; screenshots at 390×844 and 1280×800 into
  `.artifacts/screens/` (MED-13).

## 5. Tests

- [ ] 5.1 [test-engineer] Rust tests for medication creation — covers "A medication is recorded with
  its treatment details", "Several medications are active at the same time", "Only the required fields
  are needed", "The start date defaults to today and is editable", "The serving unit is one of the
  four supported units" and "A non-positive serving amount is refused", including a refusal for a unit
  outside the four and asserting that no plausibility confirmation is applied to a serving amount
  (MED-1, D5.14).
- [ ] 5.2 [test-engineer] Rust tests for the lifecycle — covers "No end date keeps the medication
  active", "A passed end date finishes the medication", "A future end date keeps the medication
  active", "Setting the end date to today finishes the course immediately" and "A finished medication
  is still reachable with its dose history" (MED-2).
- [ ] 5.3 [test-engineer] Rust tests for the four schedules — covers "Times per day offers one to four
  pre-filled editable slots", "The times-per-day slot count is limited to one to four" (including a
  refusal at zero and at five slots), "The every-X-days interval is counted from the start date",
  "A weekly schedule is one weekday plus a time" and "An as-needed schedule needs no time and accepts a
  maximum per day" (MED-3).
- [ ] 5.4 [test-engineer] Rust tests for the schedule summary, one case per type — covers
  "A times-per-day summary shows the daily slot count and their times", "An every-X-days summary shows
  the interval", "A weekly summary shows the weekday and the time", "An as-needed summary shows the
  as-needed nature and the maximum-per-day hint", "An as-needed summary renders when no
  maximum-per-day hint is set", "Every schedule type renders a summary with no recorded doses" and
  "Every schedule type renders a summary with recorded doses", asserting that no type fails and no
  parameter is omitted (MED-4).
- [ ] 5.5 [test-engineer] Rust tests for dose recording — covers "A dose is recorded with the serving
  amount, the current time and the recording member", "The pre-filled amount can be changed", "A dose
  can be back-dated", "The note is optional", "A dose may carry a different unit from the serving",
  "A non-positive dose amount is refused", "No plausibility confirmation is applied to a dose amount"
  and "A dose recorded by one member is resolved by every member" (MED-5), asserting the medication's
  serving is unchanged by a dose edit and that no conversion between units is attempted (D5.14).
- [ ] 5.6 [test-engineer] Rust tests for the dose list and its edits — covers "The dose list is
  chronological and shows timestamp, amount and giver", "A dose's amount is edited and the change is
  shared", "A dose's note and timestamp are edited and the change is shared" and "A dose is deleted
  and the deletion is shared" (MED-6), including a back-dated dose ordering into the list.
- [ ] 5.7 [test-engineer] Rust tests for the near-simultaneous guard, one case per scenario — covers
  "A dose inside the window raises a warning", "Continuing past the warning records the dose",
  "Cancelling at the warning records nothing", "A dose outside the window records without a warning",
  "The person's own recent dose raises the warning too", "A dose of a different medication does not
  raise the warning", "A medication with no earlier dose raises no warning", "The window is measured
  from the timestamp entered for the new dose", "A dose after the entered timestamp inside the window
  raises the warning too" and "The check reads the medication's doses afresh before it is evaluated",
  asserting the check itself records nothing, that its anchor is the entered timestamp rather than the
  moment of confirmation, and that a dose logged on another device seconds ago is seen because the check
  reads afresh (MED-7, D5.12, D5.13).
- [ ] 5.8 [test-engineer] Rust tests for giver attribution — covers "Every dose names a household member
  as its giver", "A dose is never stored without a giver", "A giver is never a member of another
  household", "The giver is not changed by the edits a dose permits" and "A renamed member does not
  rewrite past doses", through the command path rather than the screen (DM-5).
- [ ] 5.9 [test-engineer] Rust tests for read-only enforcement — covers "A finished medication's record
  is visible without edit or delete actions" (asserting its end date is the only correctable attribute),
  "A finished medication's dose history is visible without edit or delete actions", "A dose that was
  actually given is recordable against a finished course", "Recording a given dose leaves the frozen
  dose history frozen", "An archived rabbit's medication section offers no write action", "An archived
  rabbit gains neither exception", "An archived rabbit's course is not correctable" and "Restoring the
  rabbit restores write access, but a finished medication stays read-only", asserting the refusals
  happen at the write boundary and not only in the UI, that no existing dose of a finished medication
  can be edited, deleted or retargeted, and that recording a dose is refused only for an archived rabbit
  (MED-12, MED-13, D5.15).
- [ ] 5.10 [test-engineer] Rust tests for the dashboard badge derivation — covers "A rabbit with an
  active medication carries the informational badge", "The badge is absent when every medication is
  finished", "A rabbit with no medications carries no badge", "The badge is distinct from the warning
  flag" and "The badge clears when the last course finishes" (DASH-5).
- [ ] 5.11 [test-engineer] Component tests for the medication form and the schedule editor — covers
  MED-1's creation scenarios and MED-3's four schedule configurations, including that the
  times-per-day slots arrive pre-filled, editable and limited to 1 to 4 (MED-1, MED-3).
- [ ] 5.12 [test-engineer] Component tests for the medication detail, the give-dose flow and the dose
  list — covers "A finished medication is still reachable with its dose history" (MED-2), the MED-4
  summaries as rendered, "Give dose opens the pre-filled confirmation and records on confirm" (MED-8),
  MED-5's recording scenarios and MED-6's list and edit scenarios.
- [ ] 5.13 [test-engineer] Component tests for the near-simultaneous warning — covers "A dose inside
  the window raises a warning", "Continuing past the warning records the dose" and "Cancelling at the
  warning records nothing", asserting the warning is presented before any confirmation is accepted
  (MED-7).
- [ ] 5.14 [test-engineer] Component tests for the dashboard badge and the medications section —
  covers the DASH-5 badge scenarios including the informational-vs-warning distinction and the
  `[mobile]` visibility case, and the DASH-7 medications-section scenarios including a finished
  medication being listed, a rabbit with no medications, and the read-only presentation for an
  archived rabbit (DASH-5, DASH-7, MED-12).
- [ ] 5.15 [test-engineer] E2E (Playwright WebKit, mocked IPC, 390×844): add a twice-daily medication
  → the schedule summary renders → give a dose from the detail → the dose list shows the timestamp,
  amount and giver → a second dose inside the window warns and is continued → back-date a dose and see
  it ordered correctly → the dashboard shows the informational badge and the medications section;
  covers the `[mobile]` scenarios of MED-4, MED-7 and MED-8 plus MED-1, MED-5, MED-6, DASH-5 and
  DASH-7.
- [ ] 5.16 [test-engineer] E2E at 1280×800: the same medication detail and dose list at the desktop
  baseline, an as-needed and an every-X-days medication rendered with their summaries, a finished
  medication presented with no control that edits or deletes an existing dose while its give-dose action
  remains available, and a mistyped end date corrected on a finished course so it returns to
  active; covers the `[desktop]` scenario of MED-4 plus MED-3, MED-4, MED-12 and MED-13.
- [ ] 5.17 [test-engineer] Rust and component tests for the correction path — covers "An active course's
  details are corrected and the correction is shared", "A mistyped end date set to today is corrected and
  the course is active again", "A mistyped end date in the past is corrected", "A course is never
  deleted", "Correcting a course never rewrites a recorded dose" and "A finished course's end date is
  correctable while its other details and doses stay read-only", asserting the mistyped-end-date case is
  recoverable from the product and that no path removes a medication (MED-13).
- [ ] 5.18 [test-engineer] Run the full gate (`cargo fmt --check`, `cargo clippy`, `cargo test`,
  `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` with no diff,
  `git diff --exit-code src/routeTree.gen.ts` after a build, `bun run typecheck`, `bun run lint`,
  `bun run test`, `bun run test:e2e`) and record the results in the change.

## 6. Review

- [ ] 6.1 [reviewer] Review the P5 diff for contract and write-model conformance: the frontend touches
  IPC only through generated bindings and `api.ts`, every medication and dose write goes through P0's
  reachability gate, no local write queue or optimistic write exists (D1), the contract carries no delete
  command for a medication (MED-13), and the `Schedule` type and
  the contract carry no reminder or export concern (reminder configuration is per-device and P4's;
  D5.3, D5.9). Also confirm the contract expresses MED-12's read-only state as the derived affordance
  set and offers **exactly two** exceptions on a finished course — the end-date correction and recording
  a dose that was actually given — with no third write path there, no existing dose ever editable,
  deletable or retargetable, and neither exception reachable on an archived rabbit (D5.6, D5.15). For
  the UI tasks (4.1–4.9), confirm touch targets, safe areas and the adaptive layout at
  390×844 and 1280×800, per `definition-of-done`.
- [ ] 6.2 [reviewer] Verify the delta contains exactly MED-1 … MED-8, MED-12 and MED-13, the two named
  cross-phase dashboard requirements and DM-5 — no MED-9, MED-10 or MED-11, no US-MED-23, no reminder
  or export scope, no P0/P1/P2 requirement restated, no requirement without a scenario, and every UI
  task's `.artifacts/screens/` screenshots present at 390×844 and 1280×800.
