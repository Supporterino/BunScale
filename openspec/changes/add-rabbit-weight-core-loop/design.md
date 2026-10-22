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

## IPC contract (frozen by task 1.1)

**Status: frozen, and shipped with its generated bindings.** `architect` owns this section and the
typed stubs in `src-tauri/src/ipc/` (tasks 1.1–1.3). The thirteen commands, the two events and the
shared types below are the P1 surface; section 2 implements the bodies behind the stubs, section 3
wraps them in `api.ts`. **Task 1.1 regenerated `src/bindings.ts` from this builder** (the gate's
stage 5), so the generated file the frontend compiles against already exports all thirteen commands,
both events and every P1 type. Task 3.1 therefore **consumes** those names rather than creating them:
re-running the generator there is a drift check that must leave no diff, and no frontend work waits on
it. `src/bindings.ts` is never hand-edited — the names in this section are what it contains, and
renaming one afterwards is a deliberate bindings diff, not a tidy-up.

Naming style extends P0's contract: snake_case commands, `noun:changed` events, shared types in
PascalCase, and P0's `CommandError`, `SaveCause`, `Instant`, `WeightUnit`, `HouseholdId` reused
unchanged rather than redeclared. **Case is fixed in two places at once, and they differ:** command
**arguments** are exported camelCased (`householdId`, `includeArchived`) while **struct fields and
enum variants** stay `snake_case` (`household_id`, `days_7`), exactly as P0's do (`created_at`). No
struct carries `rename_all`, and "tidying" either half to match the other is a wire-shape change.

P1 introduces **no new cause code**: `SaveCause` is a frozen,
exhaustive list (D0.6), so every refusal in this phase — RAB-2's blank name, WT-4's non-positive value
and unconfirmed outlier, a write to an archived rabbit, an unknown or blank id — is `validation`, and
`validation`'s "the input was refused" is exactly what each of those is. No new cause is justified:
the archived case is a defence-in-depth path the UI already withholds (D1.5), not a normal flow that
needs cause-specific copy of its own.

Notation below: **write-path set** = `connectivity`, `no_account`, `allowance_exceeded`,
`rate_limited`, `timeout`, `unknown`, the causes P0's reachability-gated write path reports (D1.1);
every P1 write is such a write, so all six are reachable behind it. **local set** = `unknown`, a local
collection or device store that cannot answer. Reads never report a service cause: unreachability is a
status fact, not a read failure (SYNC-3).

### Commands

| Command | Arguments | Returns | Errors | Requirements |
|---|---|---|---|---|
| `create_rabbit` | `household_id: HouseholdId`, `profile: RabbitProfile`, `request_id: RequestId` | `Rabbit` | `validation`; write-path set | RAB-1, RAB-2, RAB-3 |
| `list_rabbits` | `household_id: HouseholdId`, `include_archived: bool` — `false` → the **active list only**, `true` → the **archived list only** (never a union of the two) | `RabbitSummary[]` | local set | DASH-1, RAB-6 |
| `get_rabbit` | `rabbit_id: RabbitId` | `Rabbit` | `validation` (unknown or blank id); local set | DASH-6, RAB-5 |
| `update_rabbit` | `rabbit_id: RabbitId`, `profile: RabbitProfile`, `request_id: RequestId` | `Rabbit` | `validation` (blank name, archived rabbit); write-path set | RAB-5, WT-7 |
| `archive_rabbit` | `rabbit_id: RabbitId`, `request_id: RequestId` | `Rabbit` | `validation` (unknown or already archived); write-path set | RAB-6, RAB-9 |
| `restore_rabbit` | `rabbit_id: RabbitId`, `request_id: RequestId` | `Rabbit` | `validation` (unknown or not archived); write-path set | RAB-7 |
| `delete_rabbit` | `rabbit_id: RabbitId`, `request_id: RequestId` | `()` | `validation` (**active** rabbit, unknown rabbit); write-path set | RAB-10 |
| `get_dashboard` | `household_id: HouseholdId` | `DashboardRow[]` | local set | DASH-1, DASH-2, DASH-3, DASH-4 |
| `create_weight_entry` | `rabbit_id: RabbitId`, `entry: WeightEntryDraft`, `request_id: RequestId` | `WeightEntry` | `validation` (≤ 0 g, unconfirmed outlier, archived rabbit); write-path set | WT-1, WT-3, WT-4, DM-1 |
| `get_weight_entries` | `rabbit_id: RabbitId`, `range: WeightRange` | `WeightTrend` | `validation` (unknown or blank id); local set | WT-6, WT-7 |
| `get_weight_entry` | `entry_id: WeightEntryId` | `WeightEntry` | `validation` (unknown or blank id); local set | WT-8 |
| `update_weight_entry` | `entry_id: WeightEntryId`, `entry: WeightEntryDraft`, `request_id: RequestId` | `WeightEntry` | `validation` (≤ 0 g, unconfirmed outlier, archived rabbit); write-path set | WT-5, WT-7, WT-8 |
| `delete_weight_entry` | `entry_id: WeightEntryId`, `request_id: RequestId` | `()` | `validation` (archived rabbit, unknown entry); write-path set | WT-5, WT-8 |

Every command is `async` and returns `Result<_, CommandError>`, P0's single failure surface. Writes
take a `RequestId` and carry it on a failure; reads take none and report `request_id: None`, because
they are idempotent and a retry is a fresh call rather than a re-issued one (D0.14). `delete_*`
returns `()` — the removal is the answer, and a deleted entity has no value to return.

Two command-level shapes are worth calling out because section 2 must not renegotiate them:

- **`update_rabbit` / `update_weight_entry` replace the whole editable set** through `RabbitProfile` /
  `WeightEntryDraft`, not a partial patch. A patch would need a double-optional per field to tell
  "unchanged" from "cleared", which the frozen wire style does not carry; both forms are pre-filled
  with the current values anyway, so they submit the full set. What the earlier draft of this table
  called `patch` is therefore `profile` / `entry`.
- **`get_weight_entries` serves both ranged views.** The trend chart and the weight-only history call
  it with their own `WeightRange`, each held in the route's typed search parameters (D1.4). The backend
  stores no selected range, so one view cannot influence the other (WT-6).

**The empty state is keyed on the active list, not on the household's rabbit count** (DASH-8, RAB-6): a
household whose rabbits are all archived has zero active rows from `list_rabbits(household_id, false)`
and `get_dashboard`, so it presents DASH-8's empty state, while the archived list stays reachable as its
own screen from that dashboard rather than as a row in the active list.

### Events

| Event | Payload type | Payload | Emitted by |
|---|---|---|---|
| `rabbit:changed` | `RabbitChanged` — `kind: RabbitChangeKind` = `created \| updated \| archived \| restored \| deleted` | `{ household_id: HouseholdId, rabbit_id: RabbitId, kind }` | every successful rabbit write |
| `weight:changed` | `WeightChanged` — `kind: WeightChangeKind` = `created \| updated \| deleted` | `{ household_id: HouseholdId, rabbit_id: RabbitId, entry_id: WeightEntryId, kind }` | every successful weight write |

The four names `RabbitChanged`, `RabbitChangeKind`, `WeightChanged` and `WeightChangeKind` are frozen
here too: they are the frontend's types, exported because `collect_events!` registers the two payloads,
and the subscriber's `switch` is over the two `kind` unions. They are reached as
`events.rabbitChanged` and `events.weightChanged`, never by a hand-built string.

Both mirror `household:changed` — a household-scoped payload naming the affected entity and the kind
of change — so P0's one subscription pattern and one invalidation policy serve them too, and the only
new shapes are the two `kind` enums and the extra ids. `weight:changed` names `rabbit_id` beside
`entry_id` because a weight change invalidates that rabbit's trend, history, detail and **dashboard
row** (the alert is derived, D1.13), and the household id is there because the frontend's cache is
keyed by household. Neither event carries data to apply: the listener invalidates and refetches through
the read commands, so an event can never become a second, stale copy of the truth (HS-8, P6 optimises
later).

### Shared types

| Type | Shape | Notes |
|---|---|---|
| `RabbitId`, `WeightEntryId` | opaque, stable strings | transparent newtypes, same style as `HouseholdId` |
| `Grams` | a number, integer and canonical | `i32` under a transparent newtype — see I7; the generated binding is `type Grams = number` |
| `Sex` | `male \| female \| unknown` | fixed value set, D1.14 |
| `NeuterStatus` | `intact \| neutered \| spayed \| unknown` | fixed value set, D1.14; `neutered` and `spayed` stay distinct |
| `Rabbit` | `{ id, household_id, name, photo_ref?, breed?, dob?, dob_unknown: bool, sex, neuter_status, adoption_date?, is_archived: bool, weight_drop_threshold_percent }` | no baseline/healthy-range field (D5, R5); `dob`/`adoption_date` are calendar dates and `dob_unknown` excludes `dob` (I8) |
| `RabbitProfile` | the editable field set of `Rabbit`: everything except `id`, `household_id`, `is_archived` | input to `create_rabbit` and `update_rabbit` |
| `RabbitSummary` | `{ id, name, thumbnail_ref?, is_archived }` | dashboard rows and the quick-entry picker |
| `DashboardRow` | `{ rabbit: RabbitSummary, last_weighed_at?, never_weighed: bool, has_weight_drop_alert: bool }` | both flags derived, never stored (D1.3) |
| `WeightEntry` | `{ id, rabbit_id, value_grams: Grams, timestamp, note?, photo_refs, has_weight_drop_alert: bool }` | **no unit field** (DM-1); the flag is derived (see I2) |
| `WeightEntryDraft` | `{ value_grams: Grams, timestamp?, note?, outlier_confirmed: bool }` | input to create and update; grams only (see I4). `timestamp: None` means "now" on create and **"keep the entry's existing timestamp" on update** — an edit never restamps a back-dated entry |
| `WeightEntrySummary` | `{ id, value_grams: Grams, timestamp, has_weight_drop_alert }` | trend points and history rows |
| `WeightRange` | `days_7 \| days_30 \| days_90 \| all` | WT-6; one enum serves both independent ranges; spelled per variant so serde cannot produce `days7` |
| `WeightTrend` | `{ range: WeightRange, points: WeightEntrySummary[], drop_threshold_percent }` | oldest first; threshold travels with the series (D1.3, R2) |
| `PLAUSIBILITY_BOUND_GRAMS` | `Grams(10_000)` (Rust const) | WT-4's bound, D1.9; a constant, so it is in no binding |
| reuse | `CommandError`, `SaveCause`, `Instant`, `WeightUnit`, `HouseholdId` | from P0, unchanged |

`Sex` and `neuter_status` are always present on `Rabbit` and `RabbitProfile` rather than optional:
D1.14 permits `unknown` as a recorded value, so "not stated" has a representation that is not an
absence, and RAB-1's "optional fields may be left unset" scenario is carried by the fields that are
genuinely `Option<_>`. `dob_unknown` is the one flag, because a date cannot express "unknown" any
other way and RAB-1 distinguishes it from "not supplied"; I8 fixes what it forbids.

### Decisions recorded in this section

**I1 — Display-unit conversion happens in `api.ts`, not in the commands** (D1.2, WT-3, DM-1). The
weight commands accept and return grams only; `src/features/weight/api.ts` parses what the person typed
into grams before the call and formats grams into the preferred unit after it, reading the unit from
P0's `get_display_weight_unit`. This is what keeps DM-1 true by construction: nothing on the wire
carries a unit, so no command can store a display-unit value and no change of preference can rewrite a
stored one. It also puts the parse next to the form that has the typed text and the locale, and keeps
the backend free of a per-device preference it would otherwise have to resolve for every write. The
alternative — passing the typed value plus a unit to the command — was rejected because it would put a
unit back on the wire and make the same command mean two different values depending on a preference
read at a different moment.

**I2 — `WeightEntry` carries the derived alert flag** (WT-8, D1.13). The table above already asked
`get_weight_entry` for the alert flag, but the earlier `WeightEntry` shape had no field for it. Adding
`has_weight_drop_alert` resolves that gap in the smallest way: the flag is derived on read like
`WeightEntrySummary`'s and `DashboardRow`'s, and the three surfaces therefore share one derivation
instead of the detail view re-deriving a rule the contract does not state.

**I3 — Range limits the points, never the derivation** (WT-6, WT-7, D1.13). `get_weight_entries`
returns the entries inside the window, but each flag is computed from the rabbit's full entry set, so
the earliest point of a window may be flagged on the basis of an entry outside it. A derivation that
re-based at the window edge would make one entry disagree with the dashboard flag as soon as a range
was selected — exactly R2.

**I4 — The plausibility bound is enforced by the command, from one constant** (WT-4, D1.9).
`WeightEntryDraft.outlier_confirmed` is how the person's confirmation reaches the boundary: an
unconfirmed value above `PLAUSIBILITY_BOUND_GRAMS` is refused with `validation`, and the same value is
saved once confirmed, so the bound stays a confirmation threshold rather than a rejection. The bound is
one `pub const` in `src-tauri/src/ipc/types.rs`; `api.ts` mirrors the number only to decide *when* to
ask, never what is stored, so a drift could change when the prompt appears but never the saved value.

**I5 — Stub state.** Every one of the thirteen bodies returns `CommandError::not_implemented` — cause
`unknown`, `retryable: false`, key `error.not_implemented` — with the command's `RequestId` on the
writes and `None` on the reads. A stub is therefore unmistakable in the UI and in a test, and section 2
replaces them without changing a signature or a generated name. `lib.rs` needs no edit for this: the
commands are mounted through the one `ipc::builder()`, whose `collect_commands!` list task 1.1 extended.
The split is asserted rather than documented only:
`ipc::commands::tests::every_p1_write_stub_is_not_implemented_and_carries_the_operations_identity`
covers all eight writes and
`every_p1_read_stub_is_not_implemented_and_carries_no_operation_identity` covers all five reads, each
checking the cause, the withheld retry, the placeholder key and the carried id — so a write that
answered `None`, or a body that named `error.unknown`, fails a test instead of passing every stage.

**I6 — No explicit type registration is needed.** P0 exports `Member` through `Builder::typ` because no
command references it; every P1 **command-reachable** type is exported transitively from the signature
that names it (the two input types from arguments, the rest from results). The four event types are not
command-reachable — `RabbitChanged`, `RabbitChangeKind`, `WeightChanged` and `WeightChangeKind` are
exported because `collect_events!` registers the two payloads — so none of the six needs a second
`Builder::typ` registration, which for any of them would be a duplicate definition rather than a
safeguard.

**I7 — A weight is a `Grams` newtype over `i32`, and no 64-bit integer appears on an exported type**
(DM-1). `bun run gen:bindings` fails at export time on `i64`, `u64`, `usize`, `i128` and `u128`:
`specta-typescript` refuses to export a JavaScript-inaccurate integer rather than silently lose
precision. `i32` is exact as a JavaScript `number`, and its range — over two million kilograms — is far
beyond any weight the product records (the plausibility bound is 10 kg). The newtype is transparent, so
the generated binding is `type Grams = number` with each `value_grams` field typed `Grams`: the unit
stays visible in the contract and a weight cannot be passed where a percentage or a count is expected.
`String` — `Instant`'s representation — was rejected because a weight is arithmetic at the boundary
(the drop derivation divides one value by another) and a string would force a parse on every
comparison. Nothing about DM-1 changes: a weight is still an integer count of grams, has no unit field,
and the display unit still lives only in `api.ts` (I1).

Two export facts follow from the same rule and are recorded so a later reader does not read them as the
contract's intent:

- **A threshold percentage is `f64`, so the generated binding types it `number | null`.** Specta maps
  `f32`/`f64` to `number | null` because JSON can carry `NaN` and `±Infinity` as `null`. The backend
  always sends a number for `weight_drop_threshold_percent` and `drop_threshold_percent`; the nullable
  type is the exporter's honesty about JSON, not a state the domain produces. The frontend treats it as
  a number once read, and the default is the documented 5 (WT-7).
- **`WeightRange`'s variants carry explicit `#[serde(rename)]`s.** Serde's `snake_case` of `Days7` is
  `days7`, while the frozen window set this change states is `days_7`, `days_30`, `days_90`, `all`; the
  per-variant renames are what keep the generated union spelled as this section says it is. A
  `rename_all` attribute cannot express that, because the digit has no boundary to split on.

The contract's exportability is asserted by
`ipc::tests::the_contract_exports_to_typescript`, which exports [`builder`] to a throwaway temp file —
never to `src/bindings.ts`, so it cannot rewrite what the gate's bindings drift check inspects (D0.9).
Without it the first symptom of a non-exportable type is a failing `bun run gen:bindings`, which
`cargo check` cannot see.

**I8 — A birth or adoption date is a calendar date, and "unknown" excludes a date** (RAB-1). Both are
carried as `Instant` because that is P0's one absolute-time type, but they are **dates pinned to a fixed
offset (UTC) and never timezone-shifted**: the day a person entered is the day every member and every
device shows, so a rabbit born on 1 March is not presented as 28 February or 2 March depending on where
the reader is. DM-6's instant rule governs timestamps, where a moment is the point; a date must not be
re-derived from a moment at a different offset. Separate from that, `dob_unknown: true` **requires**
`dob: None` and the pair `true` with a date is refused as `validation`: "unknown" and a date cannot both
be true, and refusing the contradiction is the only answer that cannot disagree with what the form
showed. The rule is stated on `Rabbit::dob` and `RabbitProfile::dob` so section 2 and section 4 read the
same sentence.

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
