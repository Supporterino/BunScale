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
  another member recorded moments ago may not yet be present locally. This is settled, not open:
  confirming a dose **forces a fresh read of that medication's doses** before the check is evaluated
  (D5.13), which is exactly what the guard exists for, and R1 is resolved rather than carried.

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

Two decisions fix what it compares (D5.12, D5.13), and neither is left to the implementation:

- **The anchor is the timestamp entered for the new dose, and the window extends both ways.** The
  window is measured from the instant that will be recorded — not from the moment the person taps
  confirm — and a dose anywhere inside the approximately 15 minutes on **either side** of that instant
  is a candidate. The backward-only reading misses the case the guard is most needed for: a person
  back-dates a dose to 09:00 when another member's dose was logged at 09:05, and the record ends up
  with two doses five minutes apart while nothing warned.
- **The comparison is made against a fresh read of that medication's doses.** Confirming a dose
  re-reads that medication's doses from the household's shared collection before the window is
  evaluated, instead of trusting the copy the device already holds, because a dose another member
  recorded minutes ago is precisely the dose a stale copy will not hold.

The duration itself — approximately 15 minutes — is given by MED-7 and was never in question. Nothing
here blocks: the warning is a prompt for a human decision, the person continues or cancels, and the
check is safe to repeat.

### D5.6 — Finished and archived are two read-only causes, both enforced at the write boundary (MED-12, MED-13)

Two distinct conditions make medication records read-only, and they are not the same condition:

- **Finished** — a medication whose end date has passed, and one that an end date of today finishes
  immediately (MED-2). Its dose history is frozen and every one of its own attributes is read-only
  **except its end date**, which stays correctable so a mistyped end date is never frozen (MED-13,
  D5.11). Exactly **two** exceptions are permitted while a medication is finished, and no others: that
  end-date correction, and **recording a dose that was actually given**, carrying its own timestamp
  (D5.15). Neither touches an **existing** dose: no dose is ever edited, deleted or retargeted on a
  finished medication, and recording a given dose changes nothing else about the course.
- **Archived rabbit** — every medication and dose of the rabbit is read-only, the end date included,
  and **neither exception applies**: no end-date correction and no dose can be recorded. This is DM-4
  and P1's boundary; MED-12 states only the observable medication behaviour and references DM-4
  instead of restating it.

Both are enforced **where writes are accepted** — every dose edit and deletion and every
medication-detail write other than the permitted end-date correction refuses a finished target, while a
dose that was actually given is accepted against it; and every medication write, every dose write
(recording included) and the end-date correction refuses an archived rabbit — each reported through
P0's single failure surface. Both are **mirrored in the UI**, which presents no create/edit/delete
control in those states other than MED-12's two exceptions. Neither layer is allowed to be the only
one: hiding a control is not enforcement, and a refusal with a live button is a lie about what the
product accepts. **No path deletes a medication** (MED-13).

Because the two exceptions are narrower than a single read-only flag can express, the contract carries
a **derived affordance set** instead of one `is_read_only` boolean (see "Shared types"): the finished
medication still accepts a recorded dose while refusing every edit to its history, and a boolean
cannot say both. The affordances are derived on read from the derived status (D5.2) and the rabbit's
archived state, never stored, and both the medications section and the detail payload carry the same
set so they can never disagree.

The two conditions are independent, which the specs assert: restoring an archived rabbit returns write
access to its active medications, and its finished medications stay read-only except for those same
two exceptions.

### D5.7 — Units and amounts are enumerated, and no unit conversion exists

MED-1 fixes the serving unit to exactly **{mg, ml, drops, tablets}**. Unlike weight, medication
amounts have **no canonical unit**: a unit is a label carried with its value, and the product performs
no conversion between units (nothing in the requirements asks for one, and inventing mg↔ml for a drug
would be a clinical claim). Three rules follow, all settled (D5.14):

- **A dose carries its own value and its own unit, and that unit may differ from the serving's.** The
  domain model gives `Dose` both an `amountValue` and an `amountUnit`, which makes a dose
  self-describing, and a dose counted in drops against a serving recorded in ml is plausible. The dose
  form therefore carries its own unit control, the dose is stored with the unit it was declared in,
  and the serving is untouched by any dose.
- **Amounts are positive.** A serving amount and a dose amount SHALL each be greater than zero; a
  non-positive amount is refused with that reason and never stored. Weight has the same rule (WT-4,
  P1) and there is no reason a medication amount should differ.
- **No plausibility bound exists.** WT-4 confirms an implausible weight, but no single plausibility
  threshold holds across mg, ml, drops and tablets — 0.2 and 500 mean opposite things in ml and in mg.
  No upper bound is enforced, no implausible-value confirmation is specified for a serving or a dose,
  and none may be invented into the specs or the tests.

**No unit conversion is performed anywhere.** A dose in drops against a serving in ml is stored and
displayed as drops against ml; the product never converts one to the other, and no derived value is
computed across units (a "total given" that summed drops and ml would be a clinical claim, not a
derivation).

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

### D5.12 — The guard's window is anchored on the entered timestamp and extends both ways (MED-7)

The approximately 15-minute window is measured from **the timestamp entered for the new dose** — the
instant that will be recorded — and covers doses of that medication **both before and after** that
instant. Two readings were rejected:

- **Anchoring on the moment of confirmation** breaks exactly when a person corrects a forgotten dose:
  MED-5 and DM-6 let the timestamp be edited, including to the past, so a dose recorded now for this
  morning would be compared against fifteen minutes of wall-clock time instead of against the morning
  it claims to belong to.
- **Considering only preceding doses** misses the collision the guard exists to catch: a person
  back-dates a dose to 09:00 while another member's dose was logged at 09:05, and the record ends up
  with two doses five minutes apart while nothing warned. The backward-only reading sees no conflict
  at all.

The window is therefore the approximately 15 minutes before **and** the approximately 15 minutes after
the entered timestamp, and the existing dose the warning names may lie on either side. The duration
itself is MED-7's given value and is not a decision taken here. The check remains the read-then-confirm
rule D5.5 fixes: it performs no write, and the person continues or cancels.

### D5.13 — Confirming a dose forces a fresh read of that medication's doses (MED-7; resolves R1)

Before the window is evaluated, confirming a dose **re-reads that medication's doses** from the
household's shared collection (P0's loaded-copy read path) rather than evaluating the window against
the copy the device already holds. The device's copy may be minutes old, and the dose the guard exists
to catch — another member's, recorded four minutes ago — is precisely the one a stale copy will not
hold; a guard that consults the local copy first is defeated by the situation it was built for.

The cost is accepted deliberately: the confirmation is slower, and a dose write is a
connectivity-required save anyway (D5.1, P0's reachability gate), so the fresh read rides on a path
that already requires a reachable shared service. This is the decision that **resolves R1**. The
residual is not freshness but the unavoidable race between the read and the confirm, which is inherent
to warning rather than locking (D5.5 refuses a stored lock for good reasons) and is accepted.

### D5.14 — Amounts are positive, a unit is per-dose, and no plausibility bound exists (MED-1, MED-5)

Three rules, one subject — what an amount is:

- **A dose may carry a different unit from the medication's serving.** `Dose` carries both an
  `amountValue` and an `amountUnit` (§5), so a dose is self-describing, and a drop count against a
  serving in ml is a real case. The dose form carries a unit control, the recorded dose stores the unit
  it was declared in, and the medication's serving is never changed by a dose (MED-5's existing
  scenario already asserts that).
- **Amounts must be positive.** A serving amount and a dose amount are each required to be greater
  than zero; zero and negative amounts are refused with that reason and never stored. This is the same
  rule weight carries (WT-4, P1).
- **No plausibility bound is invented.** WT-4 also confirms an implausible weight, but medication
  amounts have no comparable threshold: mg, ml, drops and tablets do not share a scale, so no
  "implausible" amount can be defined across them without inventing clinical meaning. An unusually
  large amount is therefore accepted, and neither the specs nor the tests add a plausibility
  confirmation. This is a deliberate asymmetry with WT-4, not an omission.

No conversion between units exists anywhere (D5.7), so the per-dose unit is a label with its value and
never a quantity to be converted.

### D5.15 — A dose actually given stays recordable against a finished course (MED-12, second exception)

MED-12 freezes a finished medication, and a new dose is a write, so read strictly it would make a
course that was ended — including by the end date being set to today (MED-2) — permanently unable to
receive another dose. Two ordinary situations collide with that reading:

- a course ended at the wrong moment, or ended in the product while doses continued in reality;
- a dose given on the last day of a course and logged after midnight, when the end date has passed.

The decision: **a dose that was actually given stays recordable against a finished course.** The
give-dose action remains available for a finished medication, the recorded dose carries its own
timestamp (DM-6, MED-5), and the course's other attributes stay read-only. This is a **second
deliberate exception to MED-12**, alongside MED-13's end-date correction, and it is bounded as tightly:

- **Neither exception touches an existing dose.** No dose on a finished medication can be edited,
  deleted or retargeted — the dose history stays frozen — and DM-5's giver is not patchable on a new
  dose either, since the giver is captured from the recording identity (D5.4).
- **The end-date correction stays the only correction to the course's own attributes.** Recording a
  dose corrects nothing: it adds one record with its own timestamp and leaves the course's name,
  reason, dates, serving, unit, schedule and note untouched.
- **An archived rabbit gains neither exception.** DM-4 freezes it everywhere, so no dose can be
  recorded against its finished course and its end date cannot be corrected either (D5.6).

It is deliberately **not** a second edit path and **not** a reopening of the record: MED-6's edits
(amount, note, timestamp) do not become available on a finished course, and the correction of a
course's details does not. Rejected alternatives were a time-bounded grace window after the end date
(the UI froze the record by the calendar rather than by what is true, and a genuinely late log fell
outside it) and restarting the course with a new start date (which duplicates the course and splits
its dose history across two records). The exception is expressed in the contract as part of the
derived affordance set (D5.6), not as a change to MED-2's active/finished derivation, so what "active"
and "finished" mean is untouched — P4's reminder lifecycle reads that derivation and stays valid.

The dose-after-the-end-date case is **deliberately not part of this amendment**. It is decided
separately, as MED-12's second bounded exception, in **D5.15**.

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
                                       the near-simultaneous check (fresh read, anchored on the entered
                                       timestamp, performs no write)
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
  is mirrored in the UI; neither is allowed to be the only one (D5.6). MED-12's **two** exceptions are
  correctable in both layers — a finished medication's **end date** (MED-13, D5.11) and **recording a
  dose that was actually given** (D5.15) — and an archived rabbit gets neither.
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
| `get_medication(medication_id)` | Detail payload: the medication's fields, its derived status, its structured schedule, its schedule summary and its derived affordances | MED-1, MED-2, MED-4 |
| `list_rabbit_medications(rabbit_id, include_finished)` | The medications section: each medication with its derived status and its schedule summary | DASH-7 |
| `record_dose(medication_id, amount_value, amount_unit, timestamp?, note?)` | Record a dose; the amount carries its own unit, which may differ from the serving's (D5.14); the timestamp defaults to now; the giver is the recording member, never a parameter. Accepted for an active medication and for a finished one, where it records a dose that was actually given (D5.15); refused for an archived rabbit | MED-5, MED-8, MED-12, DM-5 |
| `check_recent_dose(medication_id, at)` | The `at` it is given is **the timestamp entered for the dose being confirmed**, and it reports the doses of the same medication inside the approximately 15-minute window **on both sides** of `at`, read afresh for this call (D5.12, D5.13); **performs no write** | MED-7 |
| `list_doses(medication_id)` | The chronological dose list with timestamp, amount, unit and giver | MED-6 |
| `update_dose(dose_id, patch)` | Edit a dose's amount, note or timestamp; the giver is not patchable; refused for a finished medication's dose and for an archived rabbit | MED-6, DM-5 |
| `delete_dose(dose_id)` | Delete a dose; shared with members; refused for a finished medication's dose and for an archived rabbit | MED-6 |
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
by one member reaches the others through the ordinary propagation path — but that propagation is not
what makes the guard meaningful: the guard forces its own fresh read at confirmation (D5.13, which is
how R1 is resolved) rather than waiting for a change event to arrive. No P5 event replaces a P0 event.

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
| `MedicationAffordances` | `{ can_record_dose, can_correct_details, can_correct_end_date, can_edit_dose, can_delete_dose }` | the derived affordances, computed on read from the derived status and the rabbit's archived state and never stored (D5.6). Replaces a single `is_read_only` flag, which cannot express that a finished course accepts a dose that was actually given (D5.15) while refusing every edit to its history; `can_record_dose` is false only for an archived rabbit, `can_correct_details` true only while active and not archived, `can_correct_end_date` true unless the rabbit is archived, and the two dose-editing affordances true only while active and not archived |
| `MedicationSummary` | `{ id, rabbit_id, name, status, schedule, schedule_summary, affordances }` | the medications section row; it carries the same derived affordances as the detail payload (D5.6) so the section and the detail can never disagree |
| `Dose` | `{ id, medication_id, timestamp, amount_value, amount_unit, note?, giver_member_id }` | the unit travels with the value and may differ from the serving's, with no conversion (D5.14); `giver_member_id` is written from the recording identity and is not patchable (DM-5, D5.4) |
| `DoseSummary` | `{ id, timestamp, amount_value, amount_unit, giver_member_id, giver_display_name? }` | the chronological list, the dose's own unit included (D5.14); the display name resolves through P0's identity fallback chain (SET-2) |
| `RecentDoseCheck` | `{ window_minutes, window_start, window_end, within_window: bool, existing_doses: [DoseSummary] }` | `window_start`/`window_end` are the approximately 15 minutes on either side of the entered timestamp; the doses of the same medication inside them, chronological and possibly empty, are what the warning names (MED-7, D5.12); a fresh read that performs no write (D5.5, D5.13) |
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
| US-MED-13 | MED-5 | medication-tracking: "The pre-filled amount can be changed", "A dose can be back-dated", "A dose may carry a different unit from the serving" |
| US-MED-14 | MED-6, DM-5 | medication-tracking: "The dose list is chronological and shows timestamp, amount and giver"; data-integrity: "A renamed member does not rewrite past doses" |
| US-MED-15 | MED-6 | medication-tracking: "A dose is deleted and the deletion is shared" |
| US-MED-16 | MED-7 | medication-tracking: all eleven MED-7 scenarios (inside and outside the window, continue, cancel, own dose, different medication, no earlier dose, the entered-timestamp anchor, a dose after the entered timestamp, the fresh read, and the `[mobile]` case) |
| US-MED-22 | MED-12 | medication-tracking: "A finished medication's record is visible without edit or delete actions", "A finished medication's dose history is visible without edit or delete actions", "A dose that was actually given is recordable against a finished course", "Recording a given dose leaves the frozen dose history frozen", "An archived rabbit's medication section offers no write action", "An archived rabbit gains neither exception", "Restoring the rabbit restores write access, but a finished medication stays read-only" |
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

Four groups of scenarios carry a **product-owner decision that no story states**, so they trace to no
story: MED-1's positive-amount refusal and its absent plausibility confirmation, MED-5's per-dose unit,
positive-amount refusal and absent plausibility confirmation, MED-7's entered-timestamp anchor, its
dose-after-the-entered-timestamp case and its fresh read, and MED-12's recordable dose against a
finished course. Each is a decision taken here and recorded as D5.12 … D5.15, and each is covered by a
test task (5.1, 5.5, 5.7, 5.9). They invent no story, and no story is left unrepresented by them.

## Risks

| # | Risk | Mitigation in this change |
|---|---|---|
| R1 | **The double-dosing guard is defeated by a stale read.** It compares the person's confirmation against the household collection the device currently holds, and another member's dose from four minutes ago may not be in it — which is exactly the situation the guard exists for. | **Resolved by decision, not carried.** D5.13 forces a fresh read of that medication's doses before the window is evaluated, on a path that already requires a reachable shared service (D5.1), and D5.12 anchors the window on the entered timestamp so the comparison is made against the right instant. Task 5.7 covers the fresh read and both sides of the window. The residual is the unavoidable race between the read and the confirm — inherent to warning rather than locking (D5.5), and accepted. |
| R2 | **A wrongly set end date could be frozen in the UI by the very edit that sets it.** MED-2 finishes a course immediately when the end date is set to today, and MED-12 freezes a finished record — so without an amendment the mistyped date would have no in-product correction path, and §6.7 never says whether a medication's own details are editable at all. | **Resolved by amendment, not carried.** MED-13 (a deliberate product-owner addition) makes an active course's details correctable and keeps a finished course's **end date** correctable for as long as the medication exists, while its every other attribute stays read-only and no existing dose is ever edited, deleted or retargeted (D5.11); `update_medication` is the contract's only medication-mutating command beyond creation, and **no delete command exists**. The second question this row used to leave open — whether a dose actually given after the end date may be recorded — is now settled as MED-12's second bounded exception (D5.15): a given dose stays recordable while nothing about the course's own attributes except its end date does. |
| R3 | **The schedule model is coupled to P4's consumers.** Adding a reminder flag, a next-fire time or an export field to `Schedule` while building P5 would make P4's requirements load-bearing here and would bake one device's reminder configuration into shared household data. | D5.3 fixes the model as one neutral value type with no reminder and no export concern; the wire carries the structured model so P4 can consume it; `reviewer` checks the contract for reminder/export fields (task 6.1). |
| R4 | **The summary and the export disagree.** If the human-readable summary is computed in more than one place, the app's rendering and P4's export can describe the same schedule differently. | D5.3 requires the summary to be derived from the structured schedule in exactly one place, and `architect` records where (1.1/1.2); MED-4's scenarios pin the content of the summary for all four types, with and without doses. |
| R5 | **Overlapping capability files collide at archive.** `dashboard` is opened by P1, P2 and this change; `data-integrity` by all four phases; `medication-tracking` is created here and extended by P4. | Cross-phase requirements use the exact names fixed in `proposal.md`; this change adds to `dashboard` and `data-integrity` under `## ADDED Requirements` only, restates no sibling requirement and references P0's NFR-10, DM-6, P1's DM-4 and P2's DM-2 by name; the review task checks the delta for exactly the 13 owned IDs. |
| R6 | **Scope creep into P4 through MED-2's reminder clauses.** §6.7 puts reminder behaviour inside MED-2 and MED-3's PRN story, and the natural next step from a derived status is "and fire a reminder when it applies". | MED-2 is scoped to the observable lifecycle and the reminder consequences are named as P4's; MED-11 is not mentioned by any requirement here; a "no reminders" statement appears nowhere in this delta, not even negatively, except as an explicit boundary note outside the requirements. |
| R7 | **Giver attribution drifts.** Storing a display name, or resolving the giver through a per-device name, would let a rename or an unresolved identity rewrite what the record says. | D5.4 stores the member id and resolves the display name at read time through P0's fallback chain; DM-5 and its scenarios assert non-retargeting and rename-independence; the edit payload carries the giver as non-patchable. |
| R8 | **Slot times are misread as instants.** Resolving "08:00" into a timestamp or a device offset would put one device's timezone into shared household data and would make the same medication mean different things to different members. | `TimeOfDay` carries no date and no offset (D5.3), the type is distinct from `Instant`, and the firing interpretation is explicitly left to P4. |
| R9 | **Back-dated doses disturb both orderings.** MED-5 and MED-6 allow a dose's timestamp to be edited, including to the past, and MED-7's window is measured in minutes — so a correction can change what the list looks like and whether a later confirmation would have warned. | The list sorts by the dose timestamp rather than creation order; **the window is anchored on the entered timestamp and covers both directions (D5.12)**, which is what makes a back-dated dose collide visibly rather than silently; task 5.7's cases are stated relative to the recording member rather than to a clock fixture. |

## Verification

- Specs: `openspec validate add-medication-tracking --strict`. It **passes** in the revision that
  settles D5.12 … D5.15: one `## ADDED Requirements` header per delta file, at least one
  `#### Scenario:` per requirement, the `- **WHEN** / **THEN** / **AND**` bullet form throughout, and
  every task matching `- [ ] <n>.<m> [agent] <imperative>`. An earlier authoring session could not run
  it (the shell was unavailable) and checked the structure by hand instead.
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
