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
entry-side behaviour: the form uses the preferred unit and offers **no unit picker**. Both
conversions live in `src/features/weight/api.ts`, not in the commands: every weight command accepts
and returns grams only, so the wrapper parses the typed text into grams before the call and formats
the stored grams after it, taking the unit from P0's `get_display_weight_unit` (I1). No command
takes a per-entry unit, and no command resolves the device's preference.

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
change event. The read that carries the alert carries the threshold with it — `WeightTrend` is
`range`, `points` and `drop_threshold_percent`, as frozen in 1.1 — so a point is judged against the
same number that produced it, never against a threshold read separately and possibly changed in
between. The dashboard flag (DASH-4 weight part) is the same derivation surfaced as a row
indicator, so the dashboard flag and the entry flag can never disagree. The derivation is over the
current entry set rather than a snapshot taken when an entry was saved, so an edit or a deletion also
re-evaluates the alerts of the surrounding entries (D1.13) — a recomputation is a re-read, never a
repair pass over stored entries.

### D1.4 — Range selection is a read-side concern; the two ranges are independent (WT-6)

Range selection (7 days / 30 days / 90 days / full history) is a query window, not a stored value: it
lives in the route's typed search parameters, so the trend on the rabbit dashboard and the weight-only
history listing carry their own independent range and neither writes to the other. The server-side
query takes the range and returns the entries inside the window plus the rabbit's threshold, which is
what the alert derivation needs (D1.3). **A bounded range is `[now - N days, now]`, both edges
inclusive; full history is unbounded on both sides**, so a rabbit's whole record stays reachable
without a magic number. A timestamp from a device whose clock is ahead is therefore outside every
bounded range until the reading device's clock passes it — the window is recomputed per call and `all`
stays complete, so the entry appears as the clock catches up rather than being lost.

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

### D1.12 — Creation order is stored with the entry, because nothing else records it (WT-7)

WT-7 compares an entry against "the previous entry", and WT-1 lets a person set a timestamp, including
one in the past, so two entries sharing a timestamp are reachable through entirely ordinary use (two
readings logged for the same moment, or two entries back-dated to the same day). The alert basis is
ordered by **timestamp, and where two entries share a timestamp the earlier-created entry is the
previous entry** for the later one.

**Nothing this change stores records creation order today, and no existing fact can stand in for it.**
Checked fact by fact:

- `timestamp` is the tie key itself, and WT-1 makes it editable, so it cannot also carry creation
  order.
- The item's `updated_at` is the LWW **version marker** (`src-tauri/src/ipc/types.rs:341-344`). A
  create mints it from the clock once per operation and memoizes it for retries
  (`AppState::operation_marker`, `src-tauri/src/state.rs:1388-1417`), so on a create it *coincides*
  with the creation moment — but an edit is a new operation with a new marker
  (`update_weight_entry`, `state.rs:793`), so the instant an entry was created is overwritten by its
  first correction and cannot be recovered. A rule reading it would order an edited entry as though
  it had been created when it was last corrected, so WT-5's ordinary edit would silently re-order the
  journal.
- The item id is opaque and unordered: an entry's `item_id` **is** its create operation's id
  (`src-tauri/src/domain/weight.rs:89-96`), and the frontend mints that from 128 random bits —
  "WebCrypto is the source rather than a counter or a timestamp" (`src/lib/request-id.ts:71-89`). It
  is deterministic and shared, so it is a sound *last resort*, but it carries no order.
- The operation memo already holds a creation instant (`OperationMemo::entry_created_at`,
  `state.rs:101-113,1419-1443`) — but only when the draft omitted the timestamp, only in memory, and
  never in the payload, so no read and no other device can see it.
- The row carries identity, version and tombstone only (`CollectionItem`, `types.rs:337-362`), the
  feed decodes exactly those three plus the payload (`FeedRow`, `src-tauri/src/sync/read.rs:271-282`),
  and the local copy is a `HashMap` keyed by item id (`src-tauri/src/storage/collection.rs:140-162`),
  so neither arrival order nor a service sequence number is available.

**Decision: the entry payload carries its creation instant.** `WeightEntry` gains
`created_at: Instant` (shared-types table below). It is fixed when the entry is created, is **not** in
`WeightEntryDraft`, and is copied from the stored entry on every edit — exactly as `id`, `rabbit_id`
and `photo_refs` already are (`domain::weight::revise`, `weight.rs:245-254`) — so a correction changes
the value, the note and the timestamp and leaves the entry's recorded creation order unchanged. It is
stored rather than derived **because it is not derivable**: this is the one value in the change that must be
recorded at creation or lost forever, so the "no derived value is persisted" rule of *State and
boundaries* does not reach it — it is primary data like the timestamp, not a function of other stored
data. The alert itself stays derived (D1.3, D1.13); only its ordering input is stored.

The value is minted once per create operation by the memo that already exists
(`AppState::entry_created_at`), which is what keeps a retried create byte-identical (D0.14,
`state.rs:1419-1443`): a supplied timestamp keeps the operation's instant for `created_at` while
`timestamp` stays the person's value, and the two coincide when the person leaves the timestamp alone.
The write path is the enforcement point in both directions — the draft has no creation field, so no
form can set or move it, and `revise` / `save_body` (`weight.rs:419-442`) carry the stored value into
the row unchanged.

**The total order is `(timestamp, created_at, item_id)`, one comparator.** Two entries can still be
created within the same recorded instant (the clock is read once per operation, at whatever resolution
the platform gives), so the item id closes the order — that is the entry's `id`, which is the item id
it is stored under (`weight.rs:89-96`) — and since it is shared through the collection and unique, even
the last-resort comparison is deterministic across devices. Where `created_at` differs the id is never
consulted.

Consequences:

- **2.13** orders the rabbit's **full** entry set (I3) by this one comparator and takes the
  predecessor from it — never from `updated_at`, never from the id alone, and never from the window a
  caller happened to select. `created_at` is an ordering key only, never a displayed value.
- **2.12**'s `trend` sorts by `(timestamp, id)` today (`weight.rs:326-330`). It changes to the same
  shared comparator, so the order a person sees and the predecessor an alert is derived from cannot
  disagree; `WeightEntrySummary` still exposes no `created_at`.
- **5.5** can assert the scenario as written: two entries at one timestamp with different
  `created_at`, the later-created one lower, flagged against the earlier-created one — plus the
  discriminating case, the earlier-created entry edited *after* the later one, which an `updated_at`
  rule would flip and `created_at` does not. A create-path test asserts `created_at` survives an edit
  and is re-sent unchanged by a retry.
- **Bindings and stored data.** `WeightEntry` is exported, so `src/bindings.ts` gains one field on
  regeneration — a deliberate bindings diff, not drift; every command, event and other type keeps its
  shape, and every literal `WeightEntry` in Rust and in the frontend gains the field (the compiler
  finds them). The field is required, with no `#[serde(default)]`: a default would invent a creation
  order, and a payload without it is content this version cannot describe (D1.16's unknown content,
  answered under `error.household_content_unloaded`). That is acceptable only because **P1 has not
  shipped** — no household holds an entry payload — and it is the one accepted consequence: a
  pre-release entry payload written before this field existed reads as unknown content and cannot be
  repaired by an edit (a write needs the content first), so a pre-release household is re-created.
  This amends D1.16's sentence that `WeightEntry` keeps its frozen shape.
- **Residual risk.** `created_at` is the **writing device's** clock, as `updated_at` and a defaulted
  `timestamp` already are (DM-6, `src-tauri/src/storage/instant.rs:108-118`). A badly set clock can
  order two creations wrongly, but the value is recorded once, so every device agrees on the resulting
  order — which is what WT-7's "same data yields the same flag on every device" requires. A
  server-assigned creation column would move that trust to the service, and the feed carries no such
  field (`read.rs:271-282`), so it would be a service and feed change with nothing in this repository
  able to verify it (R10's class).

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

### D1.15 — P1 opens no capability, CSP entry, scope or entitlement (RAB-1, DASH-1)

Confirmed surface by surface against the shipped configuration, not restated from D1.11.

- **Capability — none.** `src-tauri/capabilities/default.json` is the only capability file, and grants
  the `main` window only `core:event:allow-listen` / `allow-unlisten` on iOS and macOS; no plugin
  directory has content. The thirteen P1 commands are **application** commands mounted through P0's one
  `ipc::builder()`, which the ACL does not gate, and the two events are consumed through the same
  `listen` call as `household:changed`, already covered by that grant. No P1 argument or return is a
  path, URL, picker handle, notification or shell string — only P0's ids and instants, P1's value types,
  and the opaque `photo_ref` / `thumbnail_ref` / `photo_refs` strings — so there is no plugin surface to
  grant.
- **CSP — no entry.** P1's reads and writes all travel the IPC channel the CSP already allows
  (`connect-src: ipc:, http://ipc.localhost`) and the webview opens no connection of its own — which is
  why P0's service origin appears nowhere in `connect-src`; `img-src` stays `'self'`.
- **Scope — none.** No P1 command takes or returns a filesystem path, none opens a picker, and the asset
  protocol is not enabled in `tauri.conf.json`.
- **Entitlement and usage string — none.** The iOS entitlements dictionary is empty and no macOS
  entitlements file exists (`bundle.macOS` sets only `minimumSystemVersion`); P1 touches no sandboxed
  resource, and the iOS `Info.plist`, which carries no usage string today, needs none for a protected
  API P1 does not use.

The photo reference needs nothing of its own: it is carried as data, never resolved by the backend, and
no P1 flow can populate one — selecting a photo is PHOTO-2 in P2 (D1.11) and `WeightEntryDraft` has no
photo field at all — so P1 renders the placeholder. Two limits are recorded so the claim cannot be
widened by accident: a ref populated with any scheme but `'self'` (`asset:`, `data:`, `blob:`, a remote
origin) would need an `img-src` entry, and that scheme belongs to PHOTO-2's capability and scope change
in P2; and the placeholder must be inline or bundled, because a `data:` placeholder is blocked by
`img-src 'self'` and the fix for that is the placeholder, never a CSP widening.

**No shared-service dependency beyond P0's mechanism.** P1 adds no client, transport, endpoint or
dependency: every command is a local collection read or a reachability-gated write through P0's
`storage/` and `sync/`, every failure is one of P0's own cause sets (D1.1, write-path/local), and both
events reuse P0's one subscription and invalidation pattern.

### D1.16 — A collection row carries its item's payload; "content unknown" is an explicit state (RAB-6, RAB-9, WT-5, DM-4; Q4)

P0's `save_item` stores the submitted `payload` **as the item's version**, and the device's row has
carried only identity, marker and tombstone flag (`StoredItem { item_id, updated_at, deleted }`),
converted from the equally payload-free `CollectionItem`. P1 cannot work through that shape. Archiving,
restoring and deleting a rabbit and correcting a weight entry each submit a whole item, and a write
that carried only the changed field (`{ "is_archived": true }`) would replace the stored version with
that much — the profile and history would be gone, which is exactly the loss RAB-9 forbids (WT-5 on
the entry side, DM-4 on retention). The P1 reads need the content too: `RabbitSummary.name`,
`Rabbit.breed`, an entry's note and photos all live only in the item. P0 anticipated this as an
**additive** step (`src-tauri/src/storage/collection.rs`: the item's content "is an additive change to
this type"); Q4 records that the plan had no task for it.

**The envelope is the write's own envelope, not a second shape.** The payload is one `ItemPayload`,
internally tagged on `kind`:

- `{ "kind": "rabbit", "rabbit": <Rabbit> }` — exactly what `domain::rabbit::save_body` already sends
  for a rabbit item (tasks 2.1/2.4).
- `{ "kind": "weight_entry", "weight_entry": <WeightEntry> }` — what task 2.9's save body sends beside
  it. Both field names are frozen here so an item's write body and its row cannot drift into two
  representations of one value.

The row becomes `CollectionItem { item_id, updated_at, deleted, payload }`, with `payload` typed
`Option<ItemPayload>`, and `StoredItem` carries the same fields. The option is deliberate: a payload is
either one of the two kinds or `None` — an absence, not a third kind of content (it has two
provenances, below).

**One field of a weight entry's payload is not content: `has_weight_drop_alert`.** `WeightEntry`
carries the **derived** flag (I2), so storing the write envelope verbatim would persist a derived value
and let it disagree with the threshold it came from — the snapshot D1.3 and D1.13 forbid, and the
state rule that no derived value is persisted. The field is therefore normalised at the write
boundary: a payload's `has_weight_drop_alert` is always written `false` and **ignored on every read**,
where the flag is re-derived from the current entry set (D1.12, D1.13). The normalised value is a
constant, so a re-issued write stays byte-identical (D0.14), and no stored payload can ever state an
alert the derivation would not.

**Two ways a row can carry no payload, and they are different cases.** They are told apart where the
feed is decoded, and neither is a state of the frozen wire type:

- **Foreign** — the answer carried a payload whose `kind` this version does not know (P2's health
  items will live in this same household collection, so this is expected, not hypothetical). It is
  not content this device failed to read; it is an item of a kind this version does not model. The
  feed **drops** such a row: it never enters the local copy, so no P1 read describes it and no P1
  write can reach it, and a loaded rabbit list is never withheld because a health item shares the
  household. The copy is therefore the items *this version models*, not every item the service holds
  — the shared collection stays the record — and the guarded load's per-id ownership check covers
  those same kinds (the phase that owns a kind applies it to that kind from its first store). The
  phase that models the kind starts storing it from the service when its own type lands.
- **Unknown content** — the row carried no payload, or a payload of a **known** kind that did not
  decode. The item is one this version models and its content is simply not on this device. Every
  rule below is about this case, and only this one.

Neither absence is **archived**: an archived row is a payload that is *present* and carries
`is_archived: true`. Treating an absent payload as archived would make an unread profile look
deliberately archived, and treating it as "live with no content" would let a write erase the profile.

The distinction is therefore the decode's, not the contract's: `payload` stays
`Option<ItemPayload>`, `payload: null` on the wire means "this version cannot describe this row", and
any row that reaches the local copy with no payload is unknown content — a foreign row was dropped
before it could be stored. No contract change.

**What unknown content means to each consumer.** The state is answered, never guessed, and no command
signature changes — `unknown` (the contract's local set) is already in every affected command's error
set:

- **Set reads** — `list_rabbits`, `get_dashboard`, `get_weight_entries` — answer the local-set
  `unknown` cause when any row they would have to describe has unknown content. A partial answer is
  not an answer: SYNC-1 says the collection is every item exactly once, so omitting a row the device
  cannot read would present an incomplete collection as the household's own — the rule SYNC-3 already
  applies to the never-loaded state (omission is indistinguishable from a complete list, and an empty
  state would be a false fact). Concretely, a `None` row leaves `list_rabbits` unable to state
  `name`/`thumbnail_ref`, `get_dashboard` unable to state `is_archived`, `last_weighed_at` or
  `never_weighed`, and `get_weight_entries` unable to derive the points *and* the drop alerts of
  their neighbours (D1.12/D1.13 need the whole entry set).
- **Address-by-id reads** — `get_rabbit`, `get_weight_entry` — answer the same local-set `unknown`
  cause for a row that exists but whose content is unknown; a blank or absent id stays `validation`.
- **Derived values need rows they do not address.** A read that derives an alert or a last-weighed
  value needs the **rabbit row and the whole entry set**, not only the row it names: `get_weight_entry`
  answers `unknown` when the entry is known but its rabbit's payload is not (the flag comes from the
  rabbit's threshold and the entry set, I2/D1.13), `get_weight_entries` answers `unknown` when the
  rabbit row is unknown even though every entry is known — the returned threshold and every derived
  alert come from it — and `get_dashboard`'s `last_weighed_at`/`never_weighed` need the same pair
  (DASH-2, DASH-3). An unknown row among them is a refusal, never a derived value computed from a
  subset.
- **Writes** — every write that submits a whole item (archive, restore, edit, delete and the cascade a
  delete performs) **reads the row's content first** and submits it unchanged except for the field it
  changes. A write that cannot read it **refuses** with the local-set `unknown` cause rather than
  submitting a partial payload: a partial `save_item` replaces the version, which is the data loss
  RAB-9 forbids. So 2.5's archive, 2.6's restore and 2.11's entry edits cannot proceed without the
  content; 2.7's delete needs the rabbit's `is_archived` precondition *and* every cascaded entry's
  content, and refuses rather than deleting a subset or letting a version lose its payload; 2.8's
  immutability check treats "cannot establish that the rabbit is not archived" as a refusal, never as
  "not archived".

**How the refusal reaches the person.** Cause `unknown` and `retryable: false` stay as they are, and
the refusal carries its own **surface key** in the frozen `CommandError.message_key` —
`error.household_content_unloaded`, owned by task 3.8, reading that this device could not read details
of the household data it holds and that they arrive with the next sync — so `error.unknown` keeps its
operation-neutral copy ("That did not work. Nothing was changed.") untouched and no P0 surface or
pinned test changes; a distinct key on a frozen cause is the pattern P0 already ships for the
display-unit store fault (`settings.unit.store_failed`, P0 Q58/Q60) and for the stub placeholder
(`error.not_implemented`, P0 design 1868–1873). It renders on the affected screen's failure state,
which offers a **refresh** (a fresh read through P0's collection read) rather than a retry of the same
command. SYNC-3's connectivity-required state is deliberately **not** reused: that state means
nothing was ever loaded, which is not this condition (rows were loaded, one content was not), and its
copy would name the wrong cause.

**The unknown-content state is transient.** P1's read path asks for the whole collection on every
refresh (`since_change_seq: 0`), so the next successful refresh fills every payload (D0.2, SYNC-3) and
the refusal disappears; until then it is the honest answer. A foreign row is not part of this promise
at all — it is not held, nothing is waiting to be filled, and its own phase reads it when it ships.

**Why the payload travels with the row rather than being fetched per item.** P0's read design answers
every read from the device's own copy while the service is unreachable (D0.2, SYNC-3): a start with no
connectivity shows what was loaded before. A per-item content fetch would make the archived list, a
profile, the dashboard and the weight reads depend on the service, which is the opposite, and would
turn one collection read into N+1 calls. The whole-collection feed already carries every row in one
answer, so carrying the content costs no extra round trip, and the local row is what makes the
archive/restore/delete writes composable at all — they submit the item as it stands.

**The version marker and the tombstone are unchanged.** `updated_at` stays the last-write-wins marker
(the service's tuple stays `(updated_at, writer_id, request_id)`, and the client still orders by its
first element only); `deleted` stays a tombstone that keeps identity and version, and a removal is not
required to blank the payload, so DM-4's retained content is the content of the version that removed
it. The merge's equal-instant tie rule keeps comparing `deleted` — the payload is not a second
ordering input. A tombstoned row's payload is never read *as content*; a delete may only re-submit it,
to retain it on the version it writes.

**Consequences for the implementation** (the types are task 1.4's, the plumbing task 2.0's; this task
writes no behaviour):

- `StoredItem` gains `payload`, and both conversion directions carry it unchanged; the two named
  constructors keep producing a row with unknown content, and 2.0 decides whether a tombstone
  constructor takes content. The feed decode classifies the payload by provenance before the rows
  reach the store — a private row shape, because the frozen `CollectionItem` cannot say "foreign" —
  so the drop is a decode step, not a field on any stored or exported type.
- `CollectionItem`, `HouseholdCollection`, `StoredItem`, `StoredCollection`, `LocalCopy`, `LocalCopies`,
  `GuardedStore` and `DeviceSnapshot` keep `PartialEq` and **lose `Eq`**: a rabbit's payload carries an
  `f64` threshold percentage, so the row cannot be `Eq` without lying. `Eq` is used nowhere these types
  cross (they are not map keys), so only the derives change.
- The field tolerates an omitted value (`#[serde(default)]`), so a feed or a file that omits it reads
  as unknown content rather than as an unreadable answer — a leaner feed must not cost the whole
  refresh (SYNC-3). The decode is lenient **by provenance, not by flattening**: a payload whose `kind`
  this version does not know drops its row (the foreign case above — P2's health items share this
  collection, so that is expected, not hypothetical), while a payload of a known kind that fails to
  decode, like a missing payload, is kept as unknown content. Neither is an error, so one item of a
  later phase can never take a whole collection read down, and a rabbit list is never withheld
  because another kind shares the household. The device never writes a row it could not read (the
  write rule above refuses), so it cannot replace a foreign payload; the shared collection stays the
  record. The field is always present on the wire, as `null`, so "this version cannot describe this
  row" is a visible value rather than an omission, and the generated binding is
  `payload: ItemPayload | null`.
- Nothing else changes: the thirteen commands, both events, `CommandError`, `SaveCause`, `Instant`,
  `Grams`, `Rabbit` and `WeightEntry` keep their frozen names and shapes (D1.12 adds one field to
  `WeightEntry` — its recorded creation instant — for WT-7's tie rule, and nothing else); no
  dependency is added.

### D1.17 — A removal is a tombstone write, and `deleted` is its own top-level argument (RAB-10; D1.8, D1.16)

**What P0 recorded, and what it left open.** P0 froze the collection row with a tombstone flag —
`CollectionItem { item_id, updated_at, deleted }`, "the tombstone flag is part of it because a removal
must stay reportable" (archived P0 `design.md:1971-1973`) — and required the write in one sentence:
"a hard delete must be a tombstone write so the change feed can report the removal"
(archived `design.md:193-194`; the same as an ADR consequence, `docs/adr/0002-shared-service-mechanism.md:56`).
The shipped read path holds that shape: `FeedRow` decodes `item_id`, `updated_at`, `deleted: bool` and
`payload` as four sibling fields (`src-tauri/src/sync/read.rs:272-282`), and a tombstone is stored with
or without content (`read.rs:1133-1186`). What P0 never recorded is how the flag reaches the service on
a write: the only `save_item` definition is six arguments —
`save_item(household_id, item_id, updated_at, payload, writer_id, request_id)`
(archived `design.md:123`) — and ADR 0002 elides the middle of the same call (`0002:26`). The read feed
therefore says a removal is a row-level fact while the write path says nothing about setting it. **The
evidence is silent on the question**, so the choice below is made on one criterion: it must change
nothing about the read feed's shape.

**The flag travels as a seventh top-level argument, `deleted: bool`, defaulting to `false` in
`save_item`.** The RPC body *is* the function's named-argument list: `HttpService` posts
`SaveRequest::body()` to `/rest/v1/rpc/<rpc>` (`src-tauri/src/sync/http.rs:68,229`) and the body is an
opaque `Value` on the client side (`src-tauri/src/sync/service.rs:166-187`), so an argument the function
does not declare is not extra data a service may ignore — it is a call to a function that does not
exist, refused before the write ever happens. The alternatives fail on the same evidence:

- **Inside `payload`.** The flag is a row fact, not content: the answer carries `deleted` beside
  `payload`, never in it (`read.rs:278`). Placing it in the envelope would either extend `ItemPayload`
  past the two kinds D1.16 freezes or make one fact readable from two places, and a reader would have
  to treat a payload-shaped removal as content — the conflation D1.16 exists to prevent.
- **An existing argument.** None is a flag: the ids identify and address, `updated_at` is the version
  marker, `payload` is the item's content. `payload: null` is already spoken for — it means **unknown
  content** on this device, and `StoredItem::tombstone` uses exactly that representation for a tombstone
  whose content was not read (`src-tauri/src/storage/collection.rs:44-69,83-94`). Reading a null payload
  as "removed" would make an unreadable profile and a deleted one the same row, which D1.16 rules out by
  name.
- **A third payload `kind`** (`{"kind": "tombstone"}`) would change the envelope D1.16 freezes, make the
  row's `deleted` a value the service must derive from content, and leave `ItemPayload` unable to carry
  the content the removal has to retain (below).

**The obligations this creates outside this repository.** Two, and both are the deployment's SQL rather
than this client's code.

- **The seventh argument.** `save_item` must declare `deleted boolean default false` and write it to the
  row's own tombstone column, with the LWW tuple, the request ledger and the allowance accounting
  unchanged. The default is what keeps the six-argument body valid — a non-removal write still sends only
  the six (`save_body`, `src-tauri/src/domain/rabbit.rs`) — so the change is additive on the service as
  well as on the client.
- **The replayed verdict.** The ledger returns the outcome the first attempt **stored**; it does not
  re-evaluate a replay against the row as it stands now. A replay of a losing write therefore keeps
  `applied: false`, and a loser must carry the flag rather than omit it, because an absent `applied` is
  read as applied. That is what lets a retry of a cascade see the same verdict on both attempts — and the
  client's stop rule depends on it: a write that lost on version order is never reported as a removal, on
  the first attempt or on a retry. A service that re-evaluated a replay could answer differently once
  another device had written the item, turning a stopped deletion into a reported one.

**What a tombstone retains.** Identity (`item_id`, and the payload's own `Rabbit.id` / `WeightEntry.id`),
the version marker `updated_at`, `writer_id` and `request_id` — the LWW tuple is unchanged, so a removal
is ordered and resumable like any other change (`collection.rs:63-66`) — and the **payload, kept rather
than cleared**: DM-4's retained content is the content of the version that removed it (D1.16's "a
removal is not required to blank the payload"), normalised the way every write is
(`has_weight_drop_alert` written `false`). The row keeps its identity claim for the life of the
collection, so nothing can resurrect what RAB-10 removed: a whole-item write addressed to a tombstoned
row is refused, never re-submitted — the rabbit path already does this
(`AppState::stored_rabbit`, `src-tauri/src/state.rs`), and the entry path (2.11) owes the same rule.

**What a reader does with it: every read skips it, none describes it.** A tombstone stays in the copy —
SYNC-1 counts it as the item — and each read excludes it **before** the unknown-content check, so a
tombstone can never turn a read into `unknown`: it is in neither rabbit list (RAB-10, D1.8), and an entry
tombstone is absent from the trend, the history, the alert derivation, the dashboard and the entry
detail. That ordering is load-bearing, not incidental: a tombstone whose content was never read is
stored as unknown content (`read.rs:294-307`), so a read that checked content first would refuse over a
row it has already been told is removed. The one rabbit read orders it that way today (`summaries` in
`domain::rabbit`; `get_rabbit` is still a stub); the entry reads (2.11–2.15) must do the same, and 2.11
must refuse an edit of a tombstoned entry as an unknown entry (`validation`) rather than re-submitting it.

**Contract impact: none on the IPC surface.** No command signature changes — `delete_rabbit` and
`delete_weight_entry` stay `(id, request_id) -> ()` exactly as the frozen table above states — and the
write body is not an IPC type: it is `SaveRequest`'s opaque `Value` and never crosses the Tauri boundary,
so `src/bindings.ts` does not change and regeneration leaves no diff. Task 2.0's feed decode needs **no**
adjustment either: `deleted` is decoded as its own field and the provenance classification is
independent of it, so a tombstoned row of a known kind keeps its content, one with no payload stays
unknown content on a tombstone (skipped by every read, per above), and a tombstoned row of a foreign kind
is dropped as foreign — the three cases `read.rs:1133-1186` already asserts. What 2.0 must **not** do is
treat `deleted: true` as a reason to drop or blank the row, because the row is the removal's report.

**Residual risk, stated once: the six-parameter reading is unfalsifiable inside this repository.** If the
deployed `save_item` is still the six-argument function, every tombstone write is refused by PostgREST as
an unknown argument — a non-2xx, arriving as cause `unknown` through `cause_of_status`
(`src-tauri/src/sync/gate.rs:184-191`) — and no test here can see it, because the gate mocks the service
(ADR 0002:53) and the scripted double accepts any body it is handed.

**Each item in a cascade has its own ledger key, and that derivation is a rule this change owns.** P0's
ledger is keyed on `(account_user_id, request_id)` and refuses the same key with a different payload
(archived `design.md:154-161`, ADR 0002:54) — D0.14. One `delete_rabbit` submits one write per removed
item under one operation, so it cannot send the operation's id on all of them: the first write claims the
key and every later one would be refused as the same key with a different payload. The rule: **the
rabbit's tombstone carries the operation's `request_id` unchanged**, so the failure surface's retry
re-issues the operation (D0.14), and **each cascaded entry's key is `<operation>/<entry item id>`**,
deterministic so a retry resubmits the same keys. `delete_weight_entry` writes one item and needs no
derivation. This belongs here rather than only in the write path's doc comment because it is a
consequence of the ledger's contract that any later fan-out would meet again, not a private choice of one
caller.

**The derivation is injective only because no id may contain `/`, and that is a rule, not an accident.**
With no `/` in either component the separator is unambiguous, so
`(operation, entry item) -> operation + "/" + item` is injective and a derived key can never equal a raw
operation id. Every id the app mints satisfies it today: `createRequestId` emits 32 lowercase hex
characters (`src/lib/request-id.ts:84-89`), and a rabbit's item id **is** its create operation's id
(`RabbitId(request_id.0.clone())`, `rabbit.rs:298-326`) — the minting rule 2.9 owes a weight entry's
item id as well, since an entry has no id of its own before its create. But `RequestId` is contractually
format-free (`src-tauri/src/ipc/types.rs:31-32`) and `ItemId` is opaque (`types.rs:59-66`), so injectivity
belongs to the minter, not to the contract: **any future minter of either id must keep `/` out**, and one
that cannot must replace this derivation (escaping or length-prefixing) instead of trusting it. The
failure is loud rather than silent — a collision makes the ledger refuse the second write as the same key
with a different payload, so a removal fails visibly instead of overwriting something.

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

**Task 1.4 extends this section additively** (Q4, D1.16): `CollectionItem` gains
`payload: ItemPayload | null` and `ItemPayload` is new, so the regenerated bindings gain one field and
one union. No command signature, event name or existing type name changes — the additive step P0's
storage layer named when it froze the row without content.

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
| `WeightEntry` | `{ id, rabbit_id, value_grams: Grams, timestamp, created_at, note?, photo_refs, has_weight_drop_alert: bool }` | **no unit field** (DM-1); the flag is derived (see I2); `created_at` is the entry's recorded creation instant — the drop derivation's tie key, never editable and never displayed (D1.12) |
| `WeightEntryDraft` | `{ value_grams: Grams, timestamp?, note?, outlier_confirmed: bool }` | input to create and update; grams only (see I4). `timestamp: None` means "now" on create and **"keep the entry's existing timestamp" on update** — an edit never restamps a back-dated entry. It carries **no creation instant**: creation order is recorded, never submitted (D1.12) |
| `WeightEntrySummary` | `{ id, value_grams: Grams, timestamp, has_weight_drop_alert }` | trend points and history rows |
| `WeightRange` | `days_7 \| days_30 \| days_90 \| all` | WT-6; one enum serves both independent ranges; spelled per variant so serde cannot produce `days7` |
| `WeightTrend` | `{ range: WeightRange, points: WeightEntrySummary[], drop_threshold_percent }` | oldest first; threshold travels with the series (D1.3, R2) |
| `PLAUSIBILITY_BOUND_GRAMS` | `Grams(10_000)` (Rust const) | WT-4's bound, D1.9; a constant, so it is in no binding |
| `ItemPayload` | `{ kind: "rabbit", rabbit: Rabbit } \| { kind: "weight_entry", weight_entry: WeightEntry }` | an item's content, in the envelope `save_item` submits (D1.16); internally tagged on `kind`, member names frozen; an unrecognised `kind` is **foreign** and its row is dropped at decode, and a weight entry's `has_weight_drop_alert` is not content — written `false`, ignored on read |
| `CollectionItem` | `{ item_id: ItemId, updated_at: Instant, deleted: bool, payload: ItemPayload \| null }` | one row of `HouseholdCollection`; `payload: null` means this version cannot describe the row — **unknown content**, not archived, not empty; a read or write that needs it (including an alert or last-weighed value derived from the rabbit row and the entry set) answers the local-set `unknown` cause under its own key `error.household_content_unloaded` (task 3.8), never a partial answer (D1.16) |
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
| R10 | **The removal flag is unverified at the wire.** The service is outside this repository and the gate mocks it, so a deployment whose `save_item` lacks the seventh argument fails every delete as `unknown` while every test stays green. | D1.17 records the argument, the default and the service obligation; the deployment change must declare it and test it against a real PostgreSQL before the first release that deletes (ADR 0002). No in-repo test can close this. |

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
