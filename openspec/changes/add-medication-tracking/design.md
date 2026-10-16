# Design — add-medication-tracking (P5)

## Context

P0 (`add-shared-household-foundation`) established the contract surface, the storage boundary, the
reachability-gated write path, the loaded-data read path, the single failure surface and the member
identity. P1 (`add-rabbit-weight-core-loop`) added rabbits, the rabbit dashboard with its History
section, and the archived-rabbit read-only boundary (DM-4). P2
(`add-health-and-photo-history`) composed the remaining history views onto that dashboard. No code
exists yet; P0's typed stubs in `src-tauri/src/ipc/` are the starting point.

P5 is the change that records **treatment**: what a rabbit is on, how much, how often, what was
actually given, who recorded it, and — the requirement the capability exists for — a warning before
the same dose is given twice. It is delivered **before** `add-vet-export-and-reminders` (P4), which
consumes this change's schedule model twice over: once for the unified reminder mechanism (D3,
MED-9 … MED-11) and once for the vet-handoff export (EXP-1, EXP-2, EXP-4). That is why the schedule
model is built here as one neutral value type and why P5 carries no reminder surface of its own.

Platforms: iOS (primary) and macOS (Phase 1). Baselines: 390×844 first, 1280×800 second. Android,
Windows and Linux are later, so everything stays `cfg`-clean.

Stack: Tauri v2, Rust in `src-tauri/`, React + TypeScript + Vite in `src/`, TanStack Router +
Query, generated bindings in `src/bindings.ts` (never hand-edited), bun as package manager.

Scope held: **13 requirement IDs — MED-1 … MED-8, MED-12 and MED-13 (10), the two named cross-phase
dashboard requirements (2), DM-5 (1).** MED-13 is a deliberate product-owner addition with no
source-document counterpart (see the delta's header note and D5.11). MED-9, MED-10 and MED-11 are P4's
and appear nowhere here.

## Decisions

### D5.1 — Writes stay connectivity-required; nothing is queued (implementation of D1)

Every medication and dose write goes through P0's reachability gate. There is no local write queue, no
optimistic write, no deferred reconciliation and no "pending dose" state. A refused write is surfaced
with its cause from P0's `CommandError`/`SaveCause` and may be retried by the person.

Two consequences are accepted explicitly, because medication is where they bite hardest:

- **US-MED-23 (offline dose logging) stays dropped.** A dose cannot be recorded while the shared
  service is unreachable, even though a dose is the one record a person is most likely to want to make
  in a barn or a car. The dose form must therefore make the requirement obvious *before* effort is
  invested in it, and must never present an unrecorded dose as recorded (P0's NFR-8 surface).
- **The double-dosing guard is only as good as the data it compares against.** Because reads are
  served from the household's locally kept copy (P0's D0.2) and refreshed when reachable, a dose
  another member recorded moments ago may not yet be present locally. Whether the guard must force a
  fresh read before confirming is a genuine open question (`questions.md` Q2) and is a risk below
  (R1), not a decision invented here.

### D5.2 — Active and finished are derived from the end date, never stored

A medication carries an end date or none. Its status — active while the end date is absent or in the
future, finished once it has passed — is computed on read from that date and the current time, and is
never persisted as a status field. P1 made the same call for the weight-drop alert (D1.3), for the
same reasons: a stored status has to be rewritten for every medication when the clock passes a date,
can disagree with the date it was computed from, and would put a background job in the critical path
of a read.

This is what makes MED-2's "setting the end date to today finishes the course immediately" true by
construction: the edit changes the date, and the next read derives the new status. It is also what
lets P4 raise no reminders for a finished course (MED-11) against the same derivation rather than
against a second source of truth. Because the status is derived rather than frozen, finishing a course
never strands a wrong end date: the end date stays correctable while the medication is finished, so
correcting it — to a future date, or clearing it — returns the medication to active on the next read
(MED-13, D5.6, D5.11).

The dashboard badge (DASH-5) and the medications section's status column are the same derivation
surfaced twice, so the badge and the section can never disagree.

### D5.3 — One neutral schedule value type, four variants, no reminder and no export coupling

The schedule is modelled as **one Rust value type with four variants**, matching §5's `Schedule`:

- `TimesPerDay { slots: [TimeOfDay] }` — 1 to 4 slots, each an explicit time of day;
- `EveryXDays { interval_days, time_of_day }` — the interval counted fixed from the medication's
  start date;
- `Weekly { weekday, time_of_day }` — exactly one weekday plus a time;
- `Prn { max_per_day? }` — no slots, an optional maximum-per-day hint.

Three rules hold this shape:

1. **The wire carries the structured model, never only a rendered string.** The human-readable
   summary (MED-4) is a **derivation** from the model, produced in one place, not the stored form of
   the schedule. P4 needs the parameters themselves — to fire a reminder at each slot, and to print
   the schedule into an export that must succeed for *every* type including `everyXDays` and `prn`
   (EXP-4) — and a rendered string cannot be un-rendered.
2. **The model is coupled to neither consumer.** It carries no reminder flag, no reminder enabled
   state, no snooze state, no next-fire time and no export field. Reminder configuration is
   per-device and per-person (MED-9), which makes it the opposite of household medication data, and
   it belongs to P4's model. Export formatting belongs to P4 too. Nothing in this change may store a
   reminder preference on a medication, and `reviewer` checks that (task 6.1).
3. **The slot invariant is enforced where a schedule is accepted**, not only in the form: a
   times-per-day schedule has 1 to 4 slots, a weekly schedule carries exactly one weekday, and a
   `prn` schedule carries no slots. The UI and the accepted model agree rather than one trusting the
   other.

A slot is a **wall-clock time of day with no date and no offset**, which is what §5's `timeOfDay`
means and what a person setting "08:00 and 20:00" intends. Interpreting that wall-clock time at
firing time is P4's concern; P5 must not resolve it into an instant or a device offset, because
resolving it here would bake one device's timezone into shared household data.

`prn` means "no fixed times" and **nothing else** in this change. That PRN schedules no reminders is
MED-11 and is specified in P4.

### D5.4 — The giver is captured from the recording member, never chosen (DM-5)

A dose's giver is the member who recorded it, taken from the authenticated identity behind the save
(P0's identity resolution, SET-2), and stored as that member's **id** — not as a copied display name.
Consequences, all of which are the point:

- There is **no giver picker**. A person cannot attribute a dose to another member, so "the member who
  recorded it" (MED-5) and "a named member of the household" (DM-5) are true by construction rather
  than by validation. The `giver_display_name` shown in the list is resolved at read time through
  P0's existing fallback chain, so a renamed or unresolved member's past doses still resolve correctly
  (R7) and a rename never rewrites history.
- The giver is **not** among the attributes MED-6 makes editable (amount, note, timestamp), so the
  edit path cannot retarget attribution, and DM-5's non-retargeting scenario is satisfied where writes
  are accepted.
- A giver is always a member of the dose's own household because a member can only ever record a dose
  through the reachability gate while acting in the active household (P0's DM-3 and NFR-10).
- The member set shrinks only through household deletion (HS-10), which cascades to doses. No member
  removal flow exists in the requirements, so no orphaned giver is reachable through an ordinary flow.
  `reviewer` records the assumption rather than a task inventing a dangling-reference repair.

### D5.5 — The near-simultaneous guard informs, it does not block (MED-7)

The guard is a **read-then-confirm rule**. Confirming a dose runs a check that answers "was a dose of
this medication already logged inside the recent window?", and the answer is presented as a warning
the person can continue past or cancel. It is deliberately **not**:

- a stored lock or a duplicate-suppression rule on the dose table — the window is a prompt for a
  human decision, and the product has no business refusing a dose a vet asked for;
- a uniqueness constraint on (medication, time bucket) — MED-6 allows a dose to be edited to a
  back-dated timestamp, so a bucket constraint would reject corrections;
- a value persisted on a dose — a stored "was warned" flag would make the same data read differently
  later, and the warning is a property of the confirmation, not of the record.

The check is a separate read that performs **no write**, so cancelling at the warning leaves the
collection untouched, and the check is safe to repeat.

What the window is measured from, and whether it extends forward for a back-dated dose, is a genuine
unknown (`questions.md` Q1), as is whether the check consults freshly fetched data (Q2). Until both
are answered, backend task 2.9 and its tests compare the doses the device currently resolves, using
the entered timestamp as the reference and only preceding doses as candidates — the narrowest reading
of "already logged" — and neither answer is invented into the specs.

### D5.6 — Finished and archived are two read-only causes, both enforced at the write boundary (MED-12, MED-13)

Two distinct conditions make medication records read-only, and they are not the same condition:

- **Finished** — a medication whose end date has passed, and one that an end date of today finishes
  immediately (MED-2). Its dose history is read-only and no new dose is added to it, and every one of its
  own attributes is read-only **except its end date**, which stays correctable so a mistyped end date is
  never frozen (MED-13, D5.11). This single exception is deliberate and closed: nothing else about a
  finished medication is writable.
- **Archived rabbit** — every medication and dose of the rabbit is read-only, the end date included.
  This is DM-4 and P1's boundary; MED-12 states only the observable medication behaviour and references
  DM-4 instead of restating it.

Both are enforced **where writes are accepted** — every dose write and every medication-detail write
other than the permitted end-date correction refuses a finished target, and every medication and dose
write refuses an archived rabbit, each reported through P0's single failure surface — and both are
**mirrored in the UI**, which presents no create/edit/delete control in those states other than the
end-date correction. Neither layer is allowed to be the only one: hiding a control is not enforcement,
and a refusal with a live button is a lie about what the product accepts. **No path deletes a
medication** (MED-13).

The two conditions are independent, which the specs assert: restoring an archived rabbit returns write
access to its active medications, and its finished medications stay read-only except for that same
end-date correction.

### D5.7 — Units and amounts are enumerated, and no unit conversion exists

MED-1 fixes the serving unit to exactly **{mg, ml, drops, tablets}**. Unlike weight, medication
amounts have **no canonical unit**: a unit is a label carried with its value, and the product performs
no conversion between units (nothing in the requirements asks for one, and inventing mg↔ml for a drug
would be a clinical claim). A dose carries its own amount and unit (the domain model's `Dose` has both
fields), and whether that unit may differ from the serving's is a genuine unknown (`questions.md` Q3),
as is what validity applies to an amount. Neither is guessed.

### D5.8 — The dashboard gains an informational badge, not a second warning

DASH-5's badge says "this rabbit is on something", which is information, not alarm. It is therefore
rendered in informational styling, visually distinct from the warning flag that P1's weight-drop alert
and P2's recent severe health entry raise, and it never adopts warning styling. A rabbit can carry
both at once, so the two must be distinguishable side by side and the specs assert them together
rather than in isolation.

The badge is a pure derivation from the rabbit's derived medication status (D5.2) and is not stored or
editable. Because the badge is a row indicator on a touch-first dashboard, it is presented without
requiring a hover: hover is progressive enhancement under the project's mobile-first rule.

### D5.9 — No security surface changes in this phase

This change adds no capability, CSP entry, scope, entitlement or permission string. It opens no
notification surface — raising and handling reminders is P4's, and the notification permission belongs
there — and no photo, file or new network surface. Medications and doses travel as data through P0's
collection and P0's already-declared mechanism. There is therefore **no `tauri-security` task**;
`architect` confirms and records the finding (task 1.3) so its absence is a recorded decision rather
than an omission.

### D5.10 — Delivery order: this change before P4, and reminders deliberately not here

§12 of the user stories proposes "P4 export/reminders, then P5 medications". That order is circular —
export needs the medication model (EXP-1, EXP-2, EXP-4), and medication reminders need the reminder
mechanism (MED-9 … MED-11). The product owner resolved it, and the order is
**P0 → P1 → P2 → P3 → P5 → P4 → P6**. Medication reminders are deliberately specified in P4, under
D3's one unified mechanism, alongside the weigh reminders that share it. Nothing in this change
depends on P4, and nothing in this change reserves, requires or forward-references a reminder
requirement.

### D5.11 — The boundary amendment: a finished course's end date stays correctable (MED-13)

This is the one place where the source documents are deliberately amended, so it is stated plainly.

MED-2 finishes a course immediately when its end date is set to today, and MED-12 freezes a finished
medication's record and dose history. Read literally together, **the very edit that creates a mistyped
end date is the edit that freezes it**, and the owner's requirement — that after any end date is set,
including today, the person can still correct it — would be false. The source documents say nothing
about a medication's own details being editable at all, so the correction path had to be added, not
found.

The **minimal amendment** is a split of what MED-12 freezes, not a repeal of it:

- **The medication's own details** are correctable while it is active (MED-13), and once it is finished
  **only its end date remains correctable** — name, reason, start date, serving, unit, schedule and note
  stay read-only, exactly as MED-12 states.
- **The dose history** — the clinical record — is frozen from the moment the medication finishes, with
  no exception: no dose is added to it, and no dose is edited, deleted or retargeted. MED-12's dose
  clauses are untouched.

The rejected alternative was to treat read-only as beginning only once the end date lies strictly *in
the past*. It was rejected for three reasons: it leaves the whole record, doses included, writable for a
day after the course is clinically over; it still freezes a typo that the person notices the next
morning, which does not meet the owner's stated intent without qualification; and it makes the
correction window a function of the calendar rather than of the record's state. Keeping the end date
correctable for as long as the medication exists has no time window, and it protects the one field a
vet-facing export prints — P4's export presents finished courses with their date range (EXP-1, EXP-2),
so a wrong end date must never become permanent.

Consequences, all intended:

- Correcting a finished course's end date to a future date, or clearing it, returns the medication to
  **active** under D5.2's derivation, because the course never actually ended. Correcting it to another
  past date leaves it finished with the corrected date.
- A correction changes no dose: every recorded dose keeps its timestamp, its amount and its giver
  (DM-5). The correction is a repair of a data-entry error in metadata, not an alteration of clinical
  history.
- An **archived rabbit** gains nothing from this amendment: DM-4 freezes it everywhere, the end date
  included (D5.6).
- A medication is **never deletable** — no affordance removes one and the contract proposes no command
  that does. Deleting a course would take its recorded dose history out of the record, which is the one
  thing this capability exists to keep. Full CRUD is explicitly rejected.
- MED-2 and MED-12 are marked **in place** in the delta with the clause this amendment gives them; no
  `## MODIFIED Requirements` section is used, because both requirements are stated by this change.

The dose-after-the-end-date case is **not** resolved by this amendment and remains `questions.md` Q4.

## State and boundaries

```
Rust (src-tauri/)                                    Frontend (src/)
  src-tauri/src/ipc/              P0 contract + P5 commands, events, shared types → src/bindings.ts (generated)
  src-tauri/src/domain/medication.rs   medication record, serving, derived active/finished,
                                       the detail correction path (MED-13) and the write-boundary
                                       read-only refusals (finished except the end date, archived)
  src-tauri/src/domain/schedule.rs     the four schedule variants, slot invariants,
                                       the human-readable summary derivation
  src-tauri/src/domain/dose.rs         dose recording and editing, giver attribution,
                                       the near-simultaneous check (read-only, performs no write)
  src-tauri/src/domain/dashboard.rs    (P1) row assembly + the derived active-medication badge field
  src-tauri/src/storage/          (P0) boundary: archived immutability, household scoping, instants
  src-tauri/src/sync/             (P0) reachability gate, loaded copy, change events
                                                  → src/features/medications/ api.ts + hooks
                                                  → src/routes/ (medication form with schedule editor,
                                                    medication detail, give-dose confirmation,
                                                    dose list, dose edit)
```

Rules held here:

- The frontend never calls `invoke` directly; only `src/features/*/api.ts` wrappers over generated
  bindings (project hard rule 2). Regenerating `src/bindings.ts` leaves no diff.
- Medications and doses are household-scoped data and live in the household's shared collection.
  Nothing about them is stored per device, and in particular **no reminder preference is stored on a
  medication** (D5.3) — per-device configuration is P0's boundary and P4's content.
- Every derivable value — active/finished status, the dose count, the dashboard badge, the schedule
  summary — is derived on read from stored canonical data. Nothing derived is persisted.
- The schedule is one neutral value type consumed by the UI, by P5's summary derivation, and later by
  P4's reminders and export alike. No consumer's concern leaks into the type (D5.3).
- Slot times are wall-clock times of day, never instants and never offsets (D5.3).
- A dose's giver is captured from the recording identity, never supplied by the caller, and is not an
  editable attribute (D5.4).
- Read-only enforcement for finished medications and archived rabbits lives at the write boundary and
  is mirrored in the UI; neither is allowed to be the only one (D5.6). A finished medication's **end
  date** is the single stated exception and is correctable in both layers (MED-13, D5.11).
- A medication is **never deleted** by any layer: no affordance offers it and the contract declares no
  command for it (MED-13).
- Timestamps are P0's absolute instants (DM-6); P5 supplies them for doses and medication dates and
  never re-interprets them.

## IPC contract — to be defined by architect

**Status: proposal only. `architect` owns this section and the typed stubs in `src-tauri/src/ipc/`
(tasks 1.1–1.3). Nothing may be implemented against these names until 1.1 is done and
`bun run gen:bindings` regenerates `src/bindings.ts`.**

Naming style extends P0's and P1's contract: snake_case commands, `noun:changed` events, shared types
in PascalCase, and P0's `CommandError` / `SaveCause` / `Instant` / `MemberId` plus P1's `RabbitId`
reused unchanged rather than redeclared. `SaveCause.validation` (P0) carries MED-1's missing-required-
field and schedule-invariant refusals; no new cause code is introduced unless 1.1 justifies one.

### Commands the frontend needs in this phase

| Command (proposed) | Purpose | Requirements |
|---|---|---|
| `create_medication(rabbit_id, medication_input)` | Create a medication with its serving and its schedule; required fields validated, schedule invariants enforced | MED-1, MED-3 |
| `update_medication(medication_id, patch)` | Correct a medication's own details — name, reason, start date, end date, serving value and unit, schedule and note. Accepted while the medication is active, and for the **end date only** while it is finished; refused for an archived rabbit | MED-13, MED-12 |
| `get_medication(medication_id)` | Detail payload: the medication's fields, its derived status, its structured schedule and its schedule summary | MED-1, MED-2, MED-4 |
| `list_rabbit_medications(rabbit_id, include_finished)` | The medications section: each medication with its derived status and its schedule summary | DASH-7 |
| `record_dose(medication_id, amount, timestamp?, note?)` | Record a dose; the timestamp defaults to now; the giver is the recording member, never a parameter | MED-5, MED-8, DM-5 |
| `check_recent_dose(medication_id, at)` | Whether a dose of the same medication falls inside the near-simultaneous window before `at`; returns the existing dose if so; **performs no write** | MED-7 |
| `list_doses(medication_id)` | The chronological dose list with timestamp, amount and giver | MED-6 |
| `update_dose(dose_id, patch)` | Edit a dose's amount, note or timestamp; the giver is not patchable | MED-6, DM-5 |
| `delete_dose(dose_id)` | Delete a dose; shared with members | MED-6 |
| *(type extension, not a new command)* | P1's dashboard read gains the derived active-medication badge field; `architect` reconciles the row type with P1 and P2 rather than adding a second dashboard command | DASH-5 |

**No `delete_medication` command is proposed here, and none may ever be added.** A medication is
**never deletable** (MED-13): its recorded dose history is the clinical record, so there is no delete
affordance and no delete command, and every correction path preserves every dose. `update_medication` is
the only medication-mutating command beyond `create_medication`, and it exists because a mistyped end
date — finished by MED-2 and otherwise frozen by MED-12 — must be correctable (D5.11). It is not an
opening for full CRUD.

`architect` also decides, and records in 1.1/1.2, where the schedule summary is produced (a derived
field on the payload versus a frontend derivation from the structured schedule). The structured
`Schedule` must travel either way (D5.3), and the summary must be derived from it in exactly one place
so the detail, the section and later the export cannot disagree.

### Events the frontend needs in this phase

| Event (proposed) | Payload | Purpose |
|---|---|---|
| `medication:changed` | `{ household_id, rabbit_id, medication_id, kind: created \| updated \| status_changed }` | Invalidate only the affected medication, section and dashboard-badge queries |
| `dose:changed` | `{ household_id, rabbit_id, medication_id, dose_id, kind: created \| updated \| deleted }` | Invalidate only the affected dose list, medication detail and badge queries |

Both are household-scoped like P0's `household:changed` and P1's `rabbit:changed`, so a dose recorded
by one member reaches the others through the ordinary propagation path — which is also what makes the
double-dosing guard meaningful when the local copy is fresh (R1, Q2). No P5 event replaces a P0 event.

### Shared types

| Type (proposed) | Shape | Notes |
|---|---|---|
| `MedicationId`, `DoseId` | opaque, stable strings | same style as P0's `HouseholdId` and P1's `RabbitId` |
| `MedicationUnit` | `mg \| ml \| drops \| tablets` | exactly the four supported units (MED-1); no conversion exists (D5.7) |
| `TimeOfDay` | wall-clock `HH:MM`, **no date and no offset** | a slot is a time of day (D5.3); never an `Instant` |
| `Weekday` | `monday` … `sunday` | the weekly variant carries exactly one |
| `Schedule` | tagged value type: `{ kind: times_per_day, slots: [TimeOfDay] }` (1–4) / `{ kind: every_x_days, interval_days, time_of_day }` / `{ kind: weekly, weekday, time_of_day }` / `{ kind: prn, max_per_day? }` | one neutral model, four variants; carries no reminder and no export concern (D5.3); **P4 consumes this type for both reminders and export** |
| `MedicationStatus` | `active \| finished` | derived from the end date and the present moment, never stored (D5.2) |
| `Medication` | `{ id, rabbit_id, name, reason?, start_date, end_date?, amount_value, amount_unit, schedule, note? }` | no status field (D5.2), no reminder field (D5.3) |
| `MedicationSummary` | `{ id, rabbit_id, name, status, schedule, schedule_summary, is_read_only }` | the medications section row; `is_read_only` folds the finished and archived causes (D5.6) — no dose and no medication detail is written — with MED-13's end-date correction on a finished medication as the one permitted write |
| `Dose` | `{ id, medication_id, timestamp, amount_value, amount_unit, note?, giver_member_id }` | `giver_member_id` is written from the recording identity and is not patchable (DM-5, D5.4) |
| `DoseSummary` | `{ id, timestamp, amount_value, amount_unit, giver_member_id, giver_display_name? }` | the chronological list; the display name resolves through P0's identity fallback chain (SET-2) |
| `RecentDoseCheck` | `{ within_window: bool, window_minutes, existing_dose?: DoseSummary }` | MED-7; a read that performs no write (D5.5) |
| reuse | `CommandError`, `SaveCause`, `Instant`, `HouseholdId`, `MemberId` (P0); `RabbitId` (P1) | unchanged; no new failure cause is invented |

## Traceability: user stories → scenarios

| Story | Requirement(s) | Capability / scenario |
|---|---|---|
| US-MED-01 | MED-1 | medication-tracking: "A medication is recorded with its treatment details" |
| US-MED-02 | MED-1 | medication-tracking: "Several medications are active at the same time" |
| US-MED-03 | MED-1 | medication-tracking: "Only the required fields are needed" |
| US-MED-04 | MED-2 | medication-tracking: "No end date keeps the medication active", "A passed end date finishes the medication" |
| US-MED-05 | MED-2 | medication-tracking: "Setting the end date to today finishes the course immediately" |
| US-MED-06 | MED-3 | medication-tracking: "Times per day offers one to four pre-filled editable slots" |
| US-MED-07 | MED-3 | medication-tracking: "The every-X-days interval is counted from the start date" |
| US-MED-08 | MED-3 | medication-tracking: "A weekly schedule is one weekday plus a time" |
| US-MED-09 | MED-3 (configuration half) | medication-tracking: "An as-needed schedule needs no time and accepts a maximum per day". **The other half of this story traces MED-11, which is `add-vet-export-and-reminders` (P4)'s — see the delta's header note.** |
| US-MED-10 | MED-4 | medication-tracking: the eight MED-4 summary scenarios, in particular "An every-X-days summary shows the interval", "An as-needed summary shows the as-needed nature and the maximum-per-day hint" and "Every schedule type renders a summary with no recorded doses" |
| US-MED-11 | MED-8 | medication-tracking: "Give dose opens the pre-filled confirmation and records on confirm", the `[mobile]` scenario |
| US-MED-12 | MED-5, DM-5 | medication-tracking: "A dose is recorded with the serving amount, the current time and the recording member"; data-integrity: "Every dose names a household member as its giver" |
| US-MED-13 | MED-5 | medication-tracking: "The pre-filled amount can be changed", "A dose can be back-dated" |
| US-MED-14 | MED-6, DM-5 | medication-tracking: "The dose list is chronological and shows timestamp, amount and giver"; data-integrity: "A renamed member does not rewrite past doses" |
| US-MED-15 | MED-6 | medication-tracking: "A dose is deleted and the deletion is shared" |
| US-MED-16 | MED-7 | medication-tracking: all eight MED-7 scenarios (inside and outside the window, continue, cancel, own dose, different medication, no earlier dose, the `[mobile]` case) |
| US-MED-22 | MED-12 | medication-tracking: "A finished medication's record is visible without edit or delete actions", "A finished medication's dose history is visible without edit or delete actions", "An archived rabbit's medication section offers no write action", "Restoring the rabbit restores write access, but a finished medication stays read-only" |
| US-DASH-06 | DASH-5 | dashboard: all six scenarios of `Rabbits with active medications carry an informational badge` |
| US-DASH-08 (medications part) | DASH-7 medications part | dashboard: all five scenarios of `Medications section on the rabbit dashboard` |
| US-QUAL-09 (DM-5 clause) | DM-5 (NFR-10 via P0) | data-integrity: "A dose is never stored without a giver", "A giver is never a member of another household", "The giver is not changed by the edits a dose permits" |
| US-MED-17 … US-MED-21 | MED-9, MED-10, MED-11 | **P4** (`add-vet-export-and-reminders`) — not restated here |
| US-MED-23 | — | **Dropped** by decision D1 (offline dose logging) |

Stories with no P5 scenario: **none** among the stories whose requirements this change owns. Two
requirement IDs sit in a sibling capability by design and are counted once: **DASH-5** is
`Rabbits with active medications carry an informational badge` and the medications part of **DASH-7**
is `Medications section on the rabbit dashboard`, both in `dashboard`. **US-DASH-08** is also partly
P2's (the health history views) and is covered there; this change covers only its medications part.
**DM-5** has no story of its own; it is exercised by US-MED-12 and US-MED-14 and is stated as an
invariant with directly testable scenarios anyway. **MED-13** has no story of its own either — it is a
deliberate product-owner addition with no source-document counterpart (see the delta's header note) —
and its scenarios are covered by task 5.17 and by the read-only coverage US-MED-22 already names; it
restates no story and no sibling requirement.

## Risks

| # | Risk | Mitigation in this change |
|---|---|---|
| R1 | **The double-dosing guard is defeated by a stale read.** It compares the person's confirmation against the household collection the device currently holds, and another member's dose from four minutes ago may not be in it — which is exactly the situation the guard exists for. | D5.5 makes the check a separate read so it *can* force a fresh fetch; the freshness question is raised as `questions.md` Q2 and is not silently assumed; task 5.7 tests both the inside-window and outside-window behaviour so the guard is proven to work on the data it is given. |
| R2 | **A wrongly set end date could be frozen in the UI by the very edit that sets it.** MED-2 finishes a course immediately when the end date is set to today, and MED-12 freezes a finished record — so without an amendment the mistyped date would have no in-product correction path, and §6.7 never says whether a medication's own details are editable at all. | **Resolved by amendment, not carried.** MED-13 (a deliberate product-owner addition) makes an active course's details correctable and keeps a finished course's **end date** correctable for as long as the medication exists, while its dose history and every other attribute stay read-only (D5.11); `update_medication` is the contract's only medication-mutating command beyond creation, and **no delete command exists**. The residual, genuinely open part — whether a dose actually given after the end date may be recorded — stays `questions.md` Q4 and is not silently answered. |
| R3 | **The schedule model is coupled to P4's consumers.** Adding a reminder flag, a next-fire time or an export field to `Schedule` while building P5 would make P4's requirements load-bearing here and would bake one device's reminder configuration into shared household data. | D5.3 fixes the model as one neutral value type with no reminder and no export concern; the wire carries the structured model so P4 can consume it; `reviewer` checks the contract for reminder/export fields (task 6.1). |
| R4 | **The summary and the export disagree.** If the human-readable summary is computed in more than one place, the app's rendering and P4's export can describe the same schedule differently. | D5.3 requires the summary to be derived from the structured schedule in exactly one place, and `architect` records where (1.1/1.2); MED-4's scenarios pin the content of the summary for all four types, with and without doses. |
| R5 | **Overlapping capability files collide at archive.** `dashboard` is opened by P1, P2 and this change; `data-integrity` by all four phases; `medication-tracking` is created here and extended by P4. | Cross-phase requirements use the exact names fixed in `proposal.md`; this change adds to `dashboard` and `data-integrity` under `## ADDED Requirements` only, restates no sibling requirement and references P0's NFR-10, DM-6, P1's DM-4 and P2's DM-2 by name; the review task checks the delta for exactly the 13 owned IDs. |
| R6 | **Scope creep into P4 through MED-2's reminder clauses.** §6.7 puts reminder behaviour inside MED-2 and MED-3's PRN story, and the natural next step from a derived status is "and fire a reminder when it applies". | MED-2 is scoped to the observable lifecycle and the reminder consequences are named as P4's; MED-11 is not mentioned by any requirement here; a "no reminders" statement appears nowhere in this delta, not even negatively, except as an explicit boundary note outside the requirements. |
| R7 | **Giver attribution drifts.** Storing a display name, or resolving the giver through a per-device name, would let a rename or an unresolved identity rewrite what the record says. | D5.4 stores the member id and resolves the display name at read time through P0's fallback chain; DM-5 and its scenarios assert non-retargeting and rename-independence; the edit payload carries the giver as non-patchable. |
| R8 | **Slot times are misread as instants.** Resolving "08:00" into a timestamp or a device offset would put one device's timezone into shared household data and would make the same medication mean different things to different members. | `TimeOfDay` carries no date and no offset (D5.3), the type is distinct from `Instant`, and the firing interpretation is explicitly left to P4. |
| R9 | **Back-dated doses disturb both orderings.** MED-5 and MED-6 allow a dose's timestamp to be edited, including to the past, and MED-7's window is measured in minutes — so a correction can change what the list looks like and whether a later confirmation would have warned. | The list sorts by the dose timestamp rather than creation order; the window's anchor is raised as `questions.md` Q1 rather than assumed; task 5.7's cases are stated relative to the recording member rather than to a clock fixture. |

## Verification

- Specs: `openspec validate add-medication-tracking --strict`. **This could not be run in the
  authoring session — the shell was unavailable (permission denied).** Structural checks were done by
  hand instead: one `## ADDED Requirements` header per delta file, at least one `#### Scenario:` per
  requirement, the `- **WHEN** / **THEN** / **AND**` bullet form throughout, and every task matching
  `- [ ] <n>.<m> [agent] <imperative>`.
- Gate (per `definition-of-done`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` (no diff),
  `git diff --exit-code src/routeTree.gen.ts` after a build, `bun run typecheck`, `bun run lint`,
  `bun run test`, `bun run test:e2e`.
- UI tasks additionally: screenshots at 390×844 and 1280×800 in `.artifacts/screens/`, plus reviewer
  confirmation of touch targets, safe areas and adaptive layout.
- Every requirement above has at least one scenario named by a test task in `tasks.md`; MED-4, MED-7 and
  MED-13 each have a test task of their own (5.4, 5.7 and 5.17).
- Cross-check at review: the delta contains exactly **MED-1 … MED-8, MED-12, MED-13, DASH-5, the
  medications part of DASH-7 and DM-5** — no more, no less; **no MED-9, MED-10 or MED-11**, no
  US-MED-23, no reminder scope, no export scope and no restated P0/P1/P2 requirement.
