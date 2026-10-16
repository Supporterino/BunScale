# add-medication-tracking

> Phase **P5 — Medications** of the BunScale reimplementation.
> Source of truth: `docs/BunScale-Reimplementation-Requirements.md` (§5 domain model, §5.1
> domain invariants, §6.7 MED) and `docs/BunScale-Reimplementation-User-Stories.md`
> (EP-7, §12).
> This change owns **13 requirement IDs and no others**: MED-1 … MED-8, MED-12 and MED-13 in the new
> `medication-tracking` capability, the two named cross-phase requirements in `dashboard`
> (`Rabbits with active medications carry an informational badge`, `Medications section on the
> rabbit dashboard`) and DM-5 in `data-integrity`. MED-13 is a **deliberate product-owner addition with
> no counterpart in either source document** — see the note in `specs/medication-tracking/spec.md` and
> design D5.11.

## Why

By the time this change lands, the product knows which rabbits exist (P1), what they weigh (P1),
what has been wrong with them (P2), and how several people share all of it (P0, P3). None of that
records *treatment*. EP-7's goal is the product's second promise: "know what each rabbit is on, when
doses are due, and whether they were given — **shared so the household never double-doses**".

That promise has three parts, and the third is the one that must not be fudged:

- **Registry** — a rabbit's courses, with name, reason, start and end date, serving and schedule,
  several running at once (MED-1, MED-2, MED-3).
- **Dosing record** — what was actually given, when, how much, and *who recorded it*, visible to
  every member and correctable when it was logged wrong (MED-5, MED-6, DM-5).
- **The double-dosing guard** — before a dose is confirmed, warn if one was already logged minutes
  ago (MED-7). This is the requirement's entire purpose; a registry that records doses but permits
  two people to give the same one has not solved the problem the household has.

The schedule model also has to be shaped for two consumers that arrive later. The unified reminder
mechanism (D3) fires per slot, and the vet-handoff export (EXP-1, EXP-2, EXP-4) prints the schedule
and its full dose history. Both are P4's and the ordering deliberately puts them *after* this
change, so the model is built once, in one place, and coupled to neither.

**Why here and not later:** P5 is the largest isolated capability, and by the resolved delivery
order it is the last piece of care data the product records. Everything after it (P4, P6) consumes
this model rather than extending it.

## Delivery order — deliberately changed

`docs/BunScale-Reimplementation-User-Stories.md` §12 proposes "P4 export/reminders, then P5
medications". That ordering is **circular**: export needs the medication model (§6.10 EXP-1/EXP-2
export medications and doses), while medication reminders need the reminder mechanism (§6.7
MED-9 — MED-11). The product owner resolved it, and the delivery order is now:

**P0 → P1 → P2 → P3 → P5 → P4 → P6**

This change is therefore delivered **before** `add-vet-export-and-reminders` (P4), and no
requirement here depends on P4. Medication **reminders** (MED-9, MED-10, MED-11) live in P4, where
D3's one unified reminder mechanism is specified, and are deliberately absent from this change's
delta — not deferred, not reserved, not forward-referenced from any requirement here.

## What changes

Specs stay technology-agnostic (D6): "the shared service" and "the account" are the only names used
for the sharing mechanism.

| Capability | Requirements added | Substance |
|---|---|---|
| `medication-tracking` *(created here)* | MED-1, MED-2, MED-3, MED-4, MED-5, MED-6, MED-7, MED-8, MED-12, MED-13 | A rabbit's courses with reason, dates, serving and note; several concurrent courses; the active/finished lifecycle with retained history; the four schedule types with editable pre-filled slots; a human-readable schedule summary for every type with and without doses; dose recording with an editable timestamp, a pre-filled editable amount, an optional note and the recording member; the chronological dose list with edit and delete; the near-simultaneous double-dosing guard; the primary give-dose action; read-only records for a finished medication; the correction of an active course's details and of a finished course's end date, and the rule that a medication is never deleted |
| `dashboard` | DASH-5 → `Rabbits with active medications carry an informational badge`, and the medications part of DASH-7 → `Medications section on the rabbit dashboard` | An informational badge on a rabbit with at least one active medication, visually distinct from the warning flag and never warning styled; a medications section on the rabbit dashboard with each medication's active/finished status, its schedule summary, an entry point to its detail and dose history, and a read-only presentation for an archived rabbit |
| `data-integrity` | DM-5 → `A dose records the giver as a member of the household` | Every dose carries exactly one giver, a member of the same household as its rabbit, captured from the recording member and not retargetable by the edits MED-6 permits |

Supporting work: the P5 IPC contract with typed stubs in `src-tauri/src/ipc/`, the medication,
schedule and dose domain in Rust, the frontend data layer (bindings wrappers, query keys, form state
for the four schedule types, the medication-detail correction path, the pre-confirmation double-dose
check), the P5 screens (medication form with the schedule editor, medication detail with its correction
affordance, give-dose confirmation, dose list and dose edit,
the dashboard badge, the medications section), an i18n key set, and tests at unit, component and E2E
level.

### Requirements this change does NOT own

| Requirement | Owner | Why |
|---|---|---|
| MED-9, MED-10, MED-11 | `add-vet-export-and-reminders` (P4) | D3's single reminder mechanism is specified there; reminders are per-device configuration, not household medication data |
| DASH-4, DASH-7 (weight parts) | `add-rabbit-weight-core-loop` (P1) | the weight-drop flag and the weight trend / weight-only history |
| DASH-4 (severe health), DASH-7 (health parts) | `add-health-and-photo-history` (P2) | the severe-health flag and the health-only / combined history |
| DM-4 | `add-rabbit-weight-core-loop` (P1) | an archived rabbit is read-only everywhere; MED-12 references it rather than restating it |
| EXP-1 … EXP-4 | `add-vet-export-and-reminders` (P4) | the export that consumes this change's model |

### Cross-phase splits observed by this change

- **`medication-tracking`** is created by this change under ten names of its own choosing. P4 later
  adds exactly three requirements to it under its own names —
  `Medication reminders are raised per device with record and snooze actions` (MED-9),
  `A missed dose window raises a single follow-up reminder` (MED-10) and
  `PRN medications schedule no reminders` (MED-11). No name chosen here collides with those, and no
  requirement here mentions reminders.
- **`dashboard`** is opened by three phases. This change adds only the two exact names above and
  restates neither P1's nor P2's requirements.
- **`data-integrity`** — this delta is **ADDED only**; it declares no `## MODIFIED Requirements`. It
  states DM-5 and references P0's `Household-scoped isolation and canonical storage at the boundary`
  (NFR-10), P0's `Timestamps are absolute instants defaulting to now` (DM-6), P1's
  `An archived rabbit is read-only everywhere` (DM-4) and P2's
  `A health tag catalog belongs to exactly one household` (DM-2) without restating or altering any
  of them.
- **MED-2's reminder clauses.** §6.7 MED-2 says a finished medication "SHALL NOT generate reminders"
  and that ending a course "cancel[s] pending reminders". That is reminder behaviour and is P4's
  (MED-9, MED-11). MED-2 is specified here only as the observable lifecycle: active while the end
  date is absent or in the future, finished once passed, retained with its full dose history, and
  finished immediately when the end date is set to today. The reminder consequences are realised in
  P4 against the same derived status.
- **US-MED-09** traces **both** MED-3 and MED-11. Split: the PRN *configuration* (no time slots, an
  optional maximum-per-day hint) is MED-3 and belongs here; MED-11's "PRN schedules no reminders" is
  P4's. This change therefore covers US-MED-09 only as far as MED-3 goes.
- **US-MED-23 (offline dose logging)** is **dropped** by D1. It is not specified here, not partially
  specified, and no placeholder is left for it: dose logging is a save like any other and requires a
  reachable shared service and a valid account.

### Settled decisions this change honours

- **D1 — connectivity-required saves.** Every medication and dose write goes through P0's
  reachability-gated save path. There is no local write queue, no optimistic write and no
  reconciliation, and **US-MED-23 is dropped** (IMP-1 closes). A refused dose save is surfaced with
  its cause and retried by the person.
- **D2 — owner-dependent sharing.** Medications and doses are household-scoped content counted
  against the household owner's allowance (HS-12). Nothing here needs ownership transfer.
- **D3 — one unified reminder mechanism.** Deliberately realised in P4. This change builds the
  schedule model that mechanism consumes and adds no reminder configuration, no reminder preference
  and no notification surface of its own.
- **D4 — fresh start.** No migration and no import: nothing reads or converts medication data from
  anywhere except the household's shared collection.
- **D5 — no chart baseline band.** Weight charts only; this change adds no chart at all.
- **D6 — technology-agnostic specs.** No vendor or integration is named.
- **The medication-editing decision (product-owner addition, MED-13).** The source documents never say
  whether a medication's own details are editable or whether a medication can be removed. The product
  owner settled both: an **active** course's details are correctable, a **finished** course's **end
  date** stays correctable so a mistyped end date is never frozen, and a medication is **never
  deleted** — full CRUD is explicitly rejected, because deleting a course would take its recorded dose
  history out of the clinical record. MED-13 states this, and MED-2 and MED-12 are marked in place with
  the one clause each takes from it (design D5.11).

## Impact

- **New Rust surface:** `src-tauri/src/ipc/` (P5 commands, events, shared types), the medication,
  schedule and dose domain, the derived active/finished status, the medication-detail correction path,
  and the write-boundary refusal that makes a finished medication (its end date excepted) and an
  archived rabbit's medications read-only.
- **New frontend surface:** `src/routes/` (medication form with schedule editor, medication detail with
  its correction affordance, give-dose confirmation, dose list and dose edit, the rabbit dashboard's
  medications section), `src/features/medications/`, and regenerated `src/bindings.ts` (never
  hand-edited).
- **Security surface: unchanged.** This change adds no capability, CSP entry, scope, entitlement or
  permission string. It opens no notification, photo, file or network surface — raising and
  handling reminders is P4's, and that is where the notification permission belongs. There is
  therefore **no `tauri-security` task** in `tasks.md`; `architect` confirms and records the finding
  (task 1.3).
- **Dependencies added:** none. This change uses the mechanism P0 selected and justified.
- **Data written:** medications and doses into the household's shared collection through P0's
  boundary. No medication or dose data is stored per device.
- **Risk carried forward:** the double-dosing guard is only as good as the data it compares against
  (see `design.md`, R1, and `questions.md` Q1/Q2), and whether a dose actually given after a course's
  end date may be recorded is still open (R2, Q4). The risk that a wrongly set end date was frozen in
  the UI is **resolved** by MED-13's correction path, not carried (R2, design D5.11).
- **Not in this change:** medication reminders and their per-device configuration (MED-9 … MED-11,
  P4), the missed-dose follow-up (P4), offline dose logging (dropped by D1), export (P4), language
  selection (P4), read-path performance (P6), and any change to P0's, P1's or P2's requirements.

## Dependencies

This change **requires `add-shared-household-foundation` (P0), `add-rabbit-weight-core-loop` (P1)
and `add-health-and-photo-history` (P2) to be archived first.**

- **P0** is required for the household's one shared collection and its scoping, the
  reachability-gated save path and the single cause-carrying failure surface (`CommandError`,
  `SaveCause`), absolute-instant timestamps (DM-6, which every dose and medication date uses), and
  the member identity that DM-5 attributes a dose to.
- **P1** is required for rabbit identity (a medication belongs to a rabbit), the rabbit dashboard
  whose History section and layout this change extends with a medications section, and the
  archived-rabbit read-only boundary (DM-4) that MED-12 references.
- **P2** is required because it opens the `dashboard` capability file this change also adds to, and
  because the rabbit dashboard's composed layout is settled there. This change adds no health
  behaviour and restates none of P2's requirements.

| Phase | change-id | Dependency |
|---|---|---|
| P0 | `add-shared-household-foundation` | **MUST be archived first** — collection identity, household scoping, save/read semantics, `CommandError`/`SaveCause`, `Instant`, member identity |
| P1 | `add-rabbit-weight-core-loop` | **MUST be archived first** — rabbits, the rabbit dashboard this change extends, the archived read-only gate (DM-4) |
| P2 | `add-health-and-photo-history` | **MUST be archived first** — the `dashboard` capability file and the composed rabbit dashboard |
| P3 | `add-household-sharing-hardening` | precedes P5 in the archive order but P5 depends on none of its content (`household-sharing`, `sync-and-offline`, `product-quality` are untouched here) |
| P4 | `add-vet-export-and-reminders` | **depends on this change, not the reverse** — it consumes the `Schedule` value type for slot reminders (MED-9 … MED-11) and for export (EXP-1, EXP-2, EXP-4), and it prints the dose history including the giver (DM-5) |
| P6 | `add-read-path-performance` | later; P5 must not break the read-path boundaries P6 optimises |

**Archive order.** Because this change creates a capability file and adds to two capability files
opened by earlier changes, archives MUST run strictly in order
**P0 → P1 → P2 → P3 → P5 → P4 → P6**. Archiving P5 before P2 leaves the `dashboard` delta applied
against text that does not yet exist; archiving P4 before P5 leaves `medication-tracking` with three
reminder requirements whose schedule model does not yet exist.

**Open items.** P0's `questions.md` Q1/Q2/Q3 are **settled**, not open: they are recorded as P0 design
decisions **D0.12, D0.13 and D0.14**, and this change consumes them as fixed behaviour — allowance
exhaustion is reported as its own condition, distinct from a generic write error; the bounded times
(15 s save attempt, 5 s reachability probe) and the offline threshold (3 consecutive failures) are named
constants; and every state-changing command carries a client-generated `request_id` the service
deduplicates on, so a retry is idempotent by construction and its deduplication criterion is inherited
through the P0 contract. P2's Q1/Q2/Q3 remain their own. This change's own open items are recorded in
`questions.md`.
