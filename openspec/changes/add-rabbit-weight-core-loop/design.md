# Design — add-rabbit-weight-core-loop (P1)

## Context

P0 (`add-shared-household-foundation`) established the contract surface, the storage boundary, the
reachability-gated write path, the loaded-data read path and the single failure surface. P1 is the
first change to put product data through them, so it decides how rabbit identity, the canonical
weight value, the derived drop alert and the archived state behave. No code exists yet; P0's typed
stubs in `src-tauri/src/ipc/` are the starting point.

Platforms: iOS (primary) and macOS (Phase 1). Baselines: 390×844 first, 1280×800 second. Android,
Windows and Linux are later, so everything stays `cfg`-clean.

Stack: Tauri v2, Rust in `src-tauri/`, React + TypeScript + Vite in `src/`, TanStack Router +
Query, generated bindings in `src/bindings.ts` (never hand-edited), bun as package manager.

Scope held: **30 requirement IDs — RAB-1 … RAB-10 (10), WT-1 … WT-8 (8), DASH-1/2/3/6/8/9 with the
weight-scoped parts of DASH-4 and DASH-7 (8), HS-8 (1), NFR-9 (1), DM-1 and DM-4 (2).** Nothing else
is implemented here.

## Decisions

### D1.1 — Writes stay connectivity-required (implementation of D1, unchanged from P0)

Every rabbit and weight write goes through P0's reachability gate. There is **no local write queue,
no optimistic write, no deferred reconciliation, and no "pending" entry state**. A refused write is
surfaced with its cause from P0's `CommandError`/`SaveCause` and may be retried by the person.

Consequences accepted: weighing while offline is impossible, so the weight form must not let a person
invest effort and then fail silently — the failure is visible before the form is trusted again
(NFR-8, P0). Because nothing is written optimistically, the trend, the drop alert and the dashboard
are always computed from data the account has actually accepted; no derived value is ever computed
from an unaccepted write. The offline dose-logging contradiction (IMP-1, US-MED-23) stays dropped.

### D1.2 — Grams are canonical; the display unit exists only at the boundary (DM-1, WT-3)

A stored weight is grams, always, and a weight entry records no unit of its own. Conversion happens
exactly twice in the product's life: parsing what the person typed into grams when a weight is
entered, and formatting grams into the display unit when a weight is shown. Nothing else may convert.

The preference itself is not re-decided here: it is P0's SET-4, stored per device outside the
household collection and defaulting from the device's regional settings. WT-3 adds only the
entry-side behaviour: the form uses the preferred unit and offers **no unit picker**. `architect`
decides in task 1.1 whether the parse/format lives in `api.ts` or in the command, but the stored
shape is grams-only either way, and no command accepts a per-entry unit.

Consequence worth stating: a displayed value is never stored, and a display-unit change can never
rewrite history. This is what makes DM-1's "changing the display unit leaves stored weights unchanged"
scenario true by construction rather than by care.

### D1.3 — The drop alert is derived, never stored (WT-7)

The alert on an entry is computed on read from the rabbit's stored threshold and the entry's drop
against its predecessor, not persisted as a flag on the entry. This is chosen precisely because WT-7
requires the flag to be **recomputed when the threshold changes**: a stored flag would have to be
found and rewritten on every threshold edit, would need a migration for existing entries, and would
be able to disagree with the threshold it was computed from. A derived alert cannot disagree.

The threshold is a field of the rabbit (RAB-1/RAB-5, default 5%) so it is shared with the household
like any other profile field, and a member's threshold edit reaches everyone through the ordinary
change event. The dashboard flag (DASH-4 weight part) is the same derivation surfaced as a row
indicator, so the dashboard flag and the entry flag can never disagree. The derivation is over the
current entry set rather than a snapshot taken when an entry was saved, so an edit or a deletion also
re-evaluates the alerts of the surrounding entries (D1.13) — a recomputation is a re-read, never a
repair pass over stored entries.

### D1.4 — Range selection is a read-side concern; the two ranges are independent (WT-6)

Range selection (7 days / 30 days / 90 days / full history) is a query window, not a stored value: it
lives in the route's typed search parameters, so the trend on the rabbit dashboard and the weight-only
history listing carry their own independent range and neither writes to the other. The server-side
query takes the range and returns the entries inside the window plus the rabbit's threshold, which is
what the alert derivation needs (D1.3). "Full history" is the absence of a lower bound, not a
computed period, so a rabbit's whole record stays reachable without a magic number.

### D1.5 — Archived means immutable at the boundary, not merely unrendered (DM-4, RAB-8, RAB-9)

Archived state is enforced where writes are accepted, not only by hiding controls. Every rabbit,
weight (and later health and medication) write path refuses to touch an archived rabbit and reports
the failure through P0's single failure surface; the UI additionally presents no create/edit/delete
control for archived data, so the two agree. This is the P1 instance of P0's NFR-10 rule that a write
to an entity the boundary marks as immutable is refused. Retention is indefinite — nothing purges
archived data, and the only removal path in P1 is RAB-10's confirmed permanent deletion, which RAB-10
restricts to an archived rabbit (D1.8).

### D1.6 — The dashboard is composed by requirement, split across phases by name

P1 renders only the weight-scoped parts of DASH-4 and DASH-7. The History section is built as an
extensible list of entry points and P1 contributes exactly the weight-only one; the severe-health flag
and the health-only / full combined views are added by P2 under their own requirement names, and the
medications section by P5. No placeholder for health or medication content is rendered in P1 — an
empty section that belongs to another phase is worse than no section, and would collide at archive
time.

### D1.7 — No chart baseline or healthy-range band (D5)

The weight trend chart in this change has **ranges, a time axis, the entry series and the drop-alert
markers, and nothing else**. There is deliberately no target band, no healthy-range band, no
per-rabbit baseline value, no baseline input field in the profile and no legend, format or contract
field for one. Decision D5 closed IMP-4; re-introducing a band is a new change, not a deferred task,
and this design records its absence explicitly so that it cannot be re-added silently during review
or implementation.

### D1.8 — Archived, restored and deleted are three distinct states

`restore` returns the rabbit to the active list with history intact; `delete` is permanent, cascades
to the rabbit's entries, and is irreversible. **Permanent deletion is available only for an archived
rabbit and is refused for an active one**, which is what keeps archiving a genuine step rather than a
decorative flag: it is the recoverable stage that must precede the irreversible one, so a mistaken tap
can always be undone before the point of no return. The delete affordance therefore lives on the
archived list and the archived rabbit's read-only surface (RAB-6, RAB-10) and is absent from an active
rabbit's profile.

This is a deliberate **narrowing** of the source RAB-10, which permits permanent deletion without
stating a precondition. US-RAB-10's only case is an archived rabbit, and the product owner chose the
archived-only reading; a later reader must not "correct" it back to an unqualified delete.

Archiving is a soft delete, is reversible, and is therefore **not** treated as a destructive action
under NFR-9 — it needs no consequence confirmation. Permanent deletion does, and its confirmation
states the consequence. This keeps NFR-9 meaningful instead of cheapening it into a confirmation on
every lifecycle action.

### D1.9 — The plausibility bound is the one the requirements state

WT-4 states the implausible-outlier bound as a value above 10 kg. US-WT-07 illustrates the same rule
with "e.g. 30 kg". The requirements document is the source of truth for requirements, so 10 kg is
adopted and the story's figure is read as an illustrative example rather than a second rule. The
bound is a confirmation threshold, never a rejection: a genuinely heavy rabbit can still be recorded
(WT-4's "a confirmed outlier is saved"). No unit picker exists to bypass it (D1.2).

### D1.10 — The quick loop is a tap budget, not a special screen (WT-2, DASH-9)

The per-rabbit path is the dashboard row's quick-weight control opening the weight form pre-bound to
that rabbit, with saving as the terminal action: control → value → save is the three-tap budget on a
phone. The quick-entry control with a picker is the second path, for "weigh whoever is on the scale".
Neither path is a separate route tree; both open the same weight form route with the rabbit bound
through typed search parameters, which is what keeps the range/unit/binding behaviour in one place.

### D1.11 — Profile photo reference is carried in P1; the picker is P2

RAB-1's photo field and DASH-1's thumbnail are P1 behaviour, so the rabbit carries a photo reference
and the thumbnail renders from it with a placeholder fallback. Choosing a photo from the device's
photo library is PHOTO-2 (`add-health-and-photo-history`, P2) and brings its own capability and scope
change with it. Consequence: **P1 requires no capability, CSP, scope or entitlement change**, and
therefore has no `tauri-security` task; the first photo-library access in the reimplementation happens
in P2, through `tauri-security`, and not here.

### D1.12 — A tied timestamp resolves to the earlier-created entry (WT-7)

WT-7 compares an entry against "the previous entry", and WT-1 lets a person set a timestamp, including
one in the past, so two entries sharing a timestamp are reachable through entirely ordinary use (two
readings logged for the same moment, or two entries back-dated to the same day). The alert basis is
therefore ordered by **timestamp, and where two entries share a timestamp the earlier-created entry is
the previous entry** for the later one. The rule depends only on stored data and is deterministic, so
the entry flag (WT-7) and the dashboard flag (DASH-4) cannot disagree and the same data produces the
same alert on every device; no entry is left without a basis because of a tie.

### D1.13 — The drop alert is a pure derivation over the current entry set (WT-7; confirms D1.3)

The alert is recomputed from the entries as they currently stand, never fixed at the moment an entry
was saved. Editing or deleting an entry therefore re-evaluates the alerts of the entries around it:
after a deletion the entry that follows is compared against the entry that now precedes it, so its
drop — and the flag derived from it — can change although that entry was never itself edited, and the
dashboard flag (DASH-4) reflects the new derivation for the same reason. This is the reading D1.3
depends on, and it means WT-5's edit and delete are alert-affecting operations rather than value
corrections only. Consequences: a recomputation is a re-read of the current entry set and never a
repair pass over stored entries, and no entry carries an alert snapshot that could go stale.

### D1.14 — A rabbit's sex and neuter/spay status have fixed value sets (RAB-1)

`sex` is `male`, `female` or `unknown`; `neuter_status` is `intact`, `neutered`, `spayed` or `unknown`.
The distinction between `neutered` and `spayed` is clinically meaningful, so the two are not collapsed
into a single "fixed" value, and a free-text field — which would accept a misspelt or invented status
— is not an option either. `unknown` is permitted for both, consistent with RAB-1 already permitting an
unknown date of birth: a person is not forced to invent a value they do not have. The sets are frozen
with the contract in task 1.1 and carried in the shared-types table below; the creation and edit forms
offer exactly these options (task 4.3).

## State and boundaries

```
Rust (src-tauri/)                                  Frontend (src/)
  src-tauri/src/ipc/            P0 contract + P1 commands, events, shared types → src/bindings.ts (generated)
  src-tauri/src/domain/rabbit.rs   profile fields, name validation, archived state, drop threshold
  src-tauri/src/domain/weight.rs   entries, canonical grams, drop-alert derivation, trend windows
  src-tauri/src/domain/dashboard.rs  per-household row assembly: sort order, never-weighed, flags
  src-tauri/src/storage/        (P0) boundary: archived immutability, grams canonical, instants
  src-tauri/src/sync/           (P0) reachability gate, loaded copy, change events
                                                → src/features/rabbits|weight|dashboard/ api.ts + hooks
                                                → src/routes/ (dashboard, rabbit form, rabbit profile,
                                                  archived list, rabbit dashboard, weight form,
                                                  weight history, weight detail)
```

Rules held here:

- The frontend never calls `invoke` directly; only `src/features/*/api.ts` wrappers over generated
  bindings (project hard rule 2). Regenerating `src/bindings.ts` leaves no diff.
- Rabbits and weight entries are household-scoped data and live in the household's shared collection.
  Nothing about them is stored per device. The display unit stays the P0 per-device preference.
- Every value that can be derived (the drop alert, the never-weighed indication, the dashboard flag,
  the sort order) is derived from stored canonical data on read; no derived value is persisted.
- Range and rabbit binding are route state (typed search parameters), so back/forward and both
  form factors behave identically without a forked route tree.
- The archived check lives at the write boundary and is mirrored in the UI; neither is allowed to be
  the only one.
- Timestamps are P0's absolute instants (DM-6); P1 supplies them and never re-interprets them.

## IPC contract — to be defined by architect

**Status: proposal only. `architect` owns this section and the typed stubs in `src-tauri/src/ipc/`
(tasks 1.1–1.3). Nothing may be implemented against these names until 1.1 is done and
`bun run gen:bindings` regenerates `src/bindings.ts`.**

Naming style extends P0's contract: snake_case commands, `noun:changed` events, shared types in
PascalCase, and P0's `CommandError` / `SaveCause` / `Instant` / `WeightUnit` reused unchanged rather
than redeclared. `SaveCause.validation` (P0) carries RAB-2's blank-name and WT-4's non-positive-value
refusals; no new cause codes are introduced unless 1.1 justifies one.

### Commands the frontend needs in this phase

| Command (proposed) | Purpose | Requirements |
|---|---|---|
| `create_rabbit(household_id, profile)` | Create a rabbit profile; blank names refused | RAB-1, RAB-2 |
| `list_rabbits(household_id, include_archived)` | Active rabbits for the dashboard, or the archived list | DASH-1, RAB-6 |
| `get_rabbit(rabbit_id)` | One rabbit's profile and archived state | DASH-6, RAB-5 |
| `update_rabbit(rabbit_id, patch)` | Edit any field of an active rabbit, including `weight_drop_threshold_percent` | RAB-5, WT-7 |
| `archive_rabbit(rabbit_id)` | Soft delete; reversible | RAB-6 |
| `restore_rabbit(rabbit_id)` | Return to the active list, history intact | RAB-7 |
| `delete_rabbit(rabbit_id)` | Permanent deletion of an **archived** rabbit with cascade, shared with members; refused for an active rabbit | RAB-10 |
| `get_dashboard(household_id)` | Rows: name, thumbnail ref, last-weighed date or never-weighed, drop flag | DASH-1, DASH-2, DASH-3, DASH-4 |
| `create_weight_entry(rabbit_id, entry)` | Save an entry; grams canonical, timestamp defaults to now | WT-1, DM-1 |
| `get_weight_entries(rabbit_id, range)` | Trend series and the ranged weight history listing, with the alert state | WT-6, WT-7 |
| `get_weight_entry(entry_id)` | Detail: value, timestamp, full note, alert flag, photo references | WT-8 |
| `update_weight_entry(entry_id, patch)` | Edit a past entry, shared with members | WT-5, WT-8 |
| `delete_weight_entry(entry_id)` | Delete a past entry, shared with members | WT-5, WT-8 |

`architect` decides, and records in 1.1, whether the entry-side display conversion is performed in
`api.ts` or inside `create_weight_entry` / `update_weight_entry`. Either is at the boundary; neither
may give an entry a unit or store a non-gram value (DM-1), and every weight command's payload stays
grams-only on the wire except for the value the person is typing.

### Events the frontend needs in this phase

| Event (proposed) | Payload | Purpose |
|---|---|---|
| `rabbit:changed` | `{ household_id, rabbit_id, kind: created \| updated \| archived \| restored \| deleted }` | Invalidate only the affected rabbit, dashboard and trend queries |
| `weight:changed` | `{ household_id, rabbit_id, entry_id, kind: created \| updated \| deleted }` | Invalidate only the affected trend, history and detail queries |

Both are household-scoped like P0's `household:changed`, so a member's weight entry reaches the other
members without a full cold reload (HS-8's equal write access made observable, and the boundary P6
later optimises).

### Shared types

| Type (proposed) | Shape | Notes |
|---|---|---|
| `RabbitId`, `WeightEntryId` | opaque, stable strings | same style as P0's `HouseholdId` |
| `Rabbit` | `{ id, household_id, name, photo_ref?, breed?, dob?: Instant \| null, dob_unknown: bool, sex, neuter_status, adoption_date?: Instant, is_archived: bool, weight_drop_threshold_percent }` | no baseline/healthy-range field (D5) |
| `RabbitSummary` | `{ id, name, thumbnail_ref?, is_archived }` | used by rows and pickers |
| `DashboardRow` | `{ rabbit: RabbitSummary, last_weighed_at?: Instant, never_weighed: bool, has_weight_drop_alert: bool }` | `never_weighed` and the flag are derived, never stored |
| `WeightEntry` | `{ id, rabbit_id, value_grams, timestamp, note?, photo_refs: [] }` | **no unit field** (DM-1) |
| `WeightEntrySummary` | `{ id, value_grams, timestamp, has_weight_drop_alert }` | trend points and history rows |
| `WeightRange` | `days_7 \| days_30 \| days_90 \| all` | WT-6; the same enum serves both independent ranges |
| `WeightTrend` | `{ range, points: [WeightEntrySummary], drop_threshold_percent }` | the threshold travels with the series so the alert is derivable and consistent |
| `Sex`, `NeuterStatus` | `Sex` = `male` \| `female` \| `unknown`; `NeuterStatus` = `intact` \| `neutered` \| `spayed` \| `unknown` | fixed value sets (D1.14); `unknown` is permitted in both, as an unknown date of birth is |
| reuse | `CommandError`, `SaveCause`, `Instant`, `WeightUnit`, `HouseholdId` | from P0, unchanged |

## Traceability: user stories → scenarios

| Story | Requirement(s) | Capability / scenario |
|---|---|---|
| US-DASH-01 | DASH-1 | dashboard: "Both active rabbits are listed with their details", "An archived rabbit is not listed" |
| US-DASH-02 | DASH-2 | dashboard: "A rabbit with no entries is marked as never weighed" |
| US-DASH-03 | DASH-3 | dashboard: "The most recently weighed rabbit sorts first", "A never-weighed rabbit sorts last" |
| US-DASH-04 | DASH-4 (weight part), WT-7 | dashboard: "A rabbit with a flagged drop is flagged"; weight-tracking: "A drop exceeding the threshold is flagged", "A drop within the threshold is not flagged", "A flagged rabbit is flagged on the dashboard" |
| US-DASH-05 | DASH-4 (weight part), WT-7 | dashboard: "The flag clears when the threshold is raised"; weight-tracking: "Raising the threshold clears an existing flag" |
| US-DASH-07 | DASH-6 | dashboard: "Selecting a rabbit opens its dashboard", the `[desktop]` scenario |
| US-DASH-08 | DASH-7 (weight part), WT-6 | dashboard: "The rabbit dashboard presents the weight trend and the history entry point", "The chart's range selection limits the presented trend", "An archived rabbit's dashboard is read-only"; weight-tracking: "The weight history range is independent of the trend range" |
| US-DASH-09 | DASH-8, RAB-4 | dashboard: "The empty state prompts for the first rabbit"; rabbit-profiles: "The empty state includes an add-first-rabbit prompt" |
| US-DASH-10 | DASH-9, WT-2 | dashboard: "The row control opens the form bound to that rabbit"; weight-tracking: "The per-rabbit weigh control opens the form bound to that rabbit" |
| US-RAB-01 | RAB-1 | rabbit-profiles: "A profile is created with the supplied identifying details", "Optional fields may be left unset" |
| US-RAB-02 | RAB-1 | rabbit-profiles: "Date of birth may be marked unknown" |
| US-RAB-03 | RAB-2 | rabbit-profiles: "An empty name is rejected", "A whitespace-only name is treated as blank" |
| US-RAB-04 | RAB-3 | rabbit-profiles: "Additional rabbits are created without restriction" |
| US-RAB-05 | RAB-4 | rabbit-profiles: "The add control is available whenever the dashboard is shown" |
| US-RAB-06 | RAB-5, WT-7 | rabbit-profiles: "The edit control is directly visible on the rabbit's detail", "A saved edit including the threshold is shared with every member", "An archived rabbit's profile is not editable" |
| US-RAB-07 | RAB-6, RAB-9 | rabbit-profiles: "Archiving removes the rabbit from the active list", "Archiving discards nothing", "Archived history survives an arbitrary interval" |
| US-RAB-08 | RAB-7 | rabbit-profiles: "Restoring returns the rabbit to the active list", "Restoration leaves the history unchanged" |
| US-RAB-09 | RAB-8, DM-4 | rabbit-profiles: "No new entry can be added to an archived rabbit", "Archived entries cannot be edited or deleted"; data-integrity: "No weight data of an archived rabbit is mutable". RAB-8's export clause is asserted by P4's EXP-1 … EXP-4 and its photo-gallery clause by P2's PHOTO-3, so P1's delta carries no scenario for them |
| US-RAB-10 | RAB-10 | rabbit-profiles: "Confirmed deletion removes the rabbit and its data", "An active rabbit cannot be permanently deleted", "Without confirmation nothing is deleted", "The deletion is shared with every member" |
| US-WT-01 | WT-1 | weight-tracking: "An entry is recorded with the current time", "An entry records an edited timestamp", "The note is optional" |
| US-WT-02 | WT-2 | weight-tracking: the `[mobile]` three-tap scenario |
| US-WT-03 | WT-2 | weight-tracking: "The quick-entry control offers a rabbit picker" |
| US-WT-04 | WT-3 (entry part), DM-1 | weight-tracking: "Stored values are presented in the preferred unit", "The form uses the regional default when no unit was chosen"; data-integrity: "Changing the display unit leaves stored weights unchanged" |
| US-WT-05 | WT-3 (entry part), DM-1 | weight-tracking: "A value entered in the preferred unit is saved in the canonical unit", "The entry form offers no unit picker"; data-integrity: "A value entered in kilograms is stored as grams" |
| US-WT-06 | WT-4 | weight-tracking: "A zero value is rejected", "A negative value is rejected" |
| US-WT-07 | WT-4 | weight-tracking: "A value above the plausibility bound requires confirmation", "A confirmed outlier is saved" |
| US-WT-08 | WT-5 | weight-tracking: "A past entry is corrected and the change is shared", "A past entry is deleted and the change is shared" |
| US-WT-09 | WT-6 | weight-tracking: "Selecting a range limits the trend to that window", "The full-history range presents every entry" |
| US-WT-10 | WT-6 | weight-tracking: "The weight history range is independent of the trend range" |
| US-WT-11 | WT-7, DASH-4 | weight-tracking: "A drop exceeding the threshold is flagged", "A drop within the threshold is not flagged" |
| US-WT-12 | WT-7, DASH-4 | weight-tracking: "Raising the threshold clears an existing flag", "Lowering the threshold raises a flag" |
| US-WT-13 | WT-8 | weight-tracking: "The detail shows the value in the display unit and the untruncated note", the two pager scenarios, "An entry with no photos shows no pager" |
| US-WT-14 | WT-8 | weight-tracking: "Edit opens the pre-filled form and Delete is confirmed" |
| US-WT-15 | WT-8, DM-4 | weight-tracking: "An archived rabbit's entry detail is read-only" |
| US-HS-14 | HS-8 | household-sharing: all three scenarios |
| US-QUAL-08 | NFR-9 | product-quality: all four scenarios |
| US-QUAL-09 | DM-1, DM-4 | data-integrity: "Every stored weight is in grams", "A weight entry carries no unit of its own", "Restoring returns the rabbit with its history intact", "The archive is retrievable and its data is retained by default" |

Scenarios added from settled decisions rather than from a story: "Entries sharing a timestamp are
ordered by creation", "Deleting an entry re-derives the following entry's alert" and "Editing an entry
re-derives the alerts around it" (WT-7, D1.12/D1.13, named by task 5.5); "A neutered and a spayed
rabbit are recorded distinctly" and "Sex and neuter status may each be recorded as unknown" (RAB-1,
D1.14, named by task 5.1).

Stories with no P1 scenario: **none** among the requirements this change owns. Clauses deliberately
left to other phases: RAB-8's export and photo-gallery availability (EXP-1 … EXP-4 in P4, PHOTO-3 in
P2), RAB-1's photo selection (PHOTO-2 in P2) and DM-4's health/medication scenarios (P2 and P5).

## Risks

| # | Risk | Mitigation in this change |
|---|---|---|
| R1 | **The three-tap budget is quietly lost.** A "log weight" that first opens a picker, or a form that needs a unit choice, breaks WT-2's promise. | The row control binds the rabbit and the form never asks for a unit (D1.2, D1.10); the budget is asserted as a scenario and exercised in the `[mobile]` E2E flow. |
| R2 | **A derived alert disagrees with the trend.** If the alert is computed from a different window, unit or ordering than the chart, two surfaces contradict each other. | One derivation shape (`WeightTrend`) carrying the threshold with the points; the dashboard flag is the same derivation (D1.3); tests assert the entry flag and the dashboard flag together. |
| R3 | **Archived immutability leaks.** Hiding controls is not enforcement; a stale route or a direct command could still write to an archived rabbit. | The refusal lives at the write boundary as well as in the UI (D1.5) and is tested through the command path, not only through the screen. |
| R4 | **Overlapping capability files collide at archive.** `dashboard`, `data-integrity` and `weight-tracking` are each modified by several phases (and P0 owns the generic NFR-10 text). | Cross-phase requirements use the exact names fixed in `proposal.md`; DM-1/DM-4 and WT-3 are scoped and reference the P0 requirements instead of restating them; the review task checks the delta for exactly the 30 IDs. |
| R5 | **The chart gains a baseline band again.** A healthy-range band is visually obvious "polish" and easy to reintroduce while building the chart. | D1.7 records the exclusion; the delta contains no requirement for it, the contract carries no baseline field, and review checks for it explicitly. |
| R6 | **Back-dated and edited timestamps reorder the list.** WT-1 allows a past timestamp and WT-5 allows edits, so "most recent weigh-in" (DASH-3) is not simply creation order. | Sort on the entry timestamp, not on creation order; the derivation re-runs on every invalidation from `weight:changed` and on every read. Which entry counts as "the previous entry" when two entries share a timestamp is settled: timestamp order, with the earlier-created entry as the previous one on a tie (D1.12). |
| R7 | **Permanent deletion is narrower than the source requirement.** RAB-10 in `docs/BunScale-Reimplementation-Requirements.md` permits permanent deletion with no precondition on the rabbit's state, which would allow an irreversible action straight from an active profile, while US-RAB-10's only case is an archived rabbit. | **Settled, recorded here so it is not reopened:** permanent deletion is available only for an archived rabbit; an active rabbit presents no delete affordance and the operation is refused, so the irreversible action sits behind the recoverable step of archiving (D1.8, task 2.7, task 4.5). This is an intentional **narrowing** of the source RAB-10, not a misreading — do not "restore" the unqualified reading. |
| R8 | **Photo references without a picker.** P1 carries a photo reference whose selection arrives in P2, so the field is never populated in P1's own flows. | Accepted as a deliberate cross-phase split (D1.11): the thumbnail and placeholder behaviour (RAB-1, DASH-1) are testable with a supplied reference, and P2 supplies the picker. No photo-library capability is opened in P1. |
| R9 | **Range state drifting between the two ranges.** A shared range store would silently couple the chart and the history sub-view and violate WT-6. | Ranges are separate typed search parameters, one per view (D1.4); the independence scenario is asserted as a component and as an E2E case. |

## Verification

- Specs: `openspec validate add-rabbit-weight-core-loop --strict`.
- Gate (per `definition-of-done`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` (no diff),
  `git diff --exit-code src/routeTree.gen.ts`, `bun run typecheck`, `bun run lint`, `bun run test`,
  `bun run test:e2e`.
- UI tasks additionally: screenshots at 390×844 and 1280×800 in `.artifacts/screens/`, plus reviewer
  confirmation of touch targets, safe areas and adaptive layout.
- Every requirement above has at least one scenario named by a test task in `tasks.md`.
- Cross-check at review: the delta contains exactly **RAB-1 … RAB-10, WT-1 … WT-8, DASH-1/2/3/6/8/9,
  the two named cross-phase dashboard requirements, HS-8, NFR-9, DM-1, DM-4** — no more, no less;
  and no health-logging, medication, export, reminder or chart-baseline scope.
