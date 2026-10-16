# Design — add-health-and-photo-history (P2)

## Context

P0 (`add-shared-household-foundation`) is archived; P1 (`add-rabbit-weight-core-loop`) is archived.
Both must be, because this change extends the P0 contract and the P1 dashboard/rabbit/weight specs
rather than redefining them. No code for P2 exists yet.

Platforms: iOS (primary) and macOS (Phase 1). Baselines: 390×844 first, 1280×800 second. Android,
Windows and Linux remain later, so every P2 addition stays `cfg`-clean.

Inherited from P0 and taken as given here:

- connectivity-required saves, one failure surface (`CommandError` / `SaveCause`), no local write
  queue (D1);
- loads served from the previously loaded household collection, refreshed when reachable (SYNC-3);
- absolute-instant timestamps normalised to UTC (DM-6);
- household isolation and canonical storage at the storage boundary (NFR-10);
- the generated `src/bindings.ts`; the frontend never calls `invoke` directly.

Inherited from P1 and taken as given here: the rabbit record, weight entries, the rabbit dashboard and
its History section, the archived-rabbit read-only gate (RAB-8, DM-4), and the local copy of the loaded
collection that the offline photo path reuses.

## Decisions

### D2.1 — Health entries and photos are saves, not local records (implementation of D1)

Creating, editing and deleting a health entry, establishing a household's tag catalog, and adding or
removing a photo all go through the P0 reachability gate and return the P0 failure surface. Nothing is
written locally "to be sent later": a refused save is surfaced with its cause and never shown as
saved. Consequently the P2 UI must make the connectivity requirement visible **before** a person
invests effort in the health-entry form, exactly as P0 requires for its own writes.

### D2.2 — Photo bytes boundary: a reference in the collection, a local read copy on the device

A photo has two parts and they live in different places:

- the **photo reference and its owner** (the rabbit profile, a weight entry, or a health entry) are
  part of the household's shared collection, so every member resolves the same photo for the same
  entry (PHOTO-1, PHOTO-4, SYNC-1);
- the **photo content** is fetched for display and kept as a device-local **read** copy, keyed by the
  reference, exactly as P0's loaded-data copy is a cache of the collection and never a competing
  record (D0.2).

Adding or removing a photo is a save (D2.1); the local copy is only ever a read optimisation and never
a pending write. No photo content, and no rabbit, note or weight content, enters diagnostics (NFR-3,
owned by P0).

### D2.3 — The offline placeholder decision (implementation of SYNC-4)

Resolving a photo for display produces one of a small set of explicit availability states:

| State | When | Presentation |
|---|---|---|
| available locally | the device already holds the local copy | the photo is displayed, without connectivity |
| fetched now | not held locally, and the shared service is reachable | the photo is displayed and the local copy is kept |
| unavailable offline | never displayed, so no local copy exists, and the shared service is unreachable | the "cannot load without connectivity" placeholder |
| missing | the reference does not resolve in the shared collection | a distinct "no longer available" state |

The decision is made from the *local copy's presence plus a fresh reachability outcome*, never from a
status carried over from a previous session (P0's honesty rule). The unavailable-offline state is
reachable **only** for a photo that has never been displayed: retention is local and indefinite (D2.9),
so a displayed photo is never evicted and never falls back into this state. Two rules follow that must
not be short-circuited for convenience:

- an entry that **has** photos which cannot be loaded SHALL NOT present as an entry with no photos —
  the placeholder occupies the photo's place, so the person can tell "not loaded" from "nothing here";
- this is a **read** path only. It never queues an upload, and it never contradicts D1: the write model
  is untouched.

### D2.4 — One canonical category catalog per household, established lazily and idempotently

The nine canonical categories (Appetite, Stool/Droppings, Activity/Behavior, Breathing, Eyes/Nose,
Coat/Grooming, Teeth, Posture/Gait, Other) are held **per household**, not per device and not globally
(DM-2, HL-6). The catalog belongs to a household record that P0 created, so P2 cannot populate it at
household-creation time; instead it is established **on first need** and is **idempotent on the
canonical category**, so two members establishing it concurrently still converge on exactly one copy of
each category (SYNC-1, SYNC-5). A health entry references the household's tags by their opaque
identifiers, never by a shared global identifier, which is what keeps DM-2 true.

### D2.5 — The dashboard health flag is derived, never stored

Whether a rabbit carries the severe-health flag is computed on read from its health entries, never
persisted onto the rabbit. The flag applies when the rabbit has a health entry of severity Severe whose
timestamp falls inside a rolling window of 7 × 24 hours measured back from the present instant,
independent of the device's timezone and of any calendar day boundary (D2.11). This keeps
the flag automatically correct when an entry is edited to a lower severity, deleted, or ages out of
the window, and it means a threshold-like recomputation (the weight-drop analogue owned by P1) needs no
stored state. The flag is presented as the same dashboard warning flag that the weight-drop clause
raises; the medication badge is a different, informational indicator and belongs to P5.

### D2.6 — One interleaved history model

Health-only history and full combined history are the same read with different filters, not two
pipelines: one timeline item type, ordered by timestamp, where a combined read merges weight entries
(P1) and health entries (P2) into a single list. The health-only view narrows that list to health
entries and applies the tag, severity and date-range filters of HL-4. The history read resolves from
the already-loaded household collection; it MUST NOT introduce a second cold load of data the
dashboard already holds (NFR-7 stays P6's subject, but P2 must not violate its boundary).

### D2.7 — Photo selection is platform-mediated, specifications stay neutral (D6)

Nothing in the specs names a picker product. Platform behaviour differs only where the platform
genuinely differs: on iOS the photo source is the device's photo library and access to it is a
permission; on macOS the photo source is the platform's photo source. The capability spec tags those
scenarios; every other P2 scenario is platform-neutral. The permission, scope, capability and
entitlement changes for both platforms are declared only by `tauri-security`, and the picker bridges
belong to `apple-native`.

### D2.8 — Archived rabbits are read-only through one gate

DM-4 / RAB-8 make an archived rabbit's data immutable. P2 surfaces archived data in three places —
the health-entry detail, the history views, and the per-rabbit gallery — and all three consult the
single archived gate introduced in P1 rather than each re-deriving it. Read access stays available
(the gallery in particular MUST remain reachable from an archived rabbit's timeline).

### D2.9 — A displayed photo is kept locally and indefinitely (settles Q1)

Local retention is not a cache with a budget: a photo that has been displayed at least once is kept
**locally and indefinitely**, until the household or the photo itself is removed. It is **not
size-bounded and not evictable**, not by the app and not as a reclaimable cache. Two consequences are
made explicit because they are what the tests can hold onto:

- SYNC-4's guarantee becomes **unconditional** — "available locally after first display" has no
  "unless it was evicted" clause, so a photo that was displayed once is offline-available for as long
  as it exists;
- the "cannot load without connectivity" placeholder is reachable **only** for a photo that was never
  displayed (D2.3). A displayed photo never falls back into the unavailable-offline state.

The retained copy remains a **read** copy of the shared collection, so D2.1/D1 are untouched: nothing
about it is a write, and no retention-related local record competes with the collection.

### D2.10 — The gallery shows the profile photo first, then entry photos by entry date (settles Q2)

A profile photo belongs to no entry and therefore has no entry date to sort by. PHOTO-3 orders the
gallery explicitly: the rabbit's **profile photo appears first**, before all entry photos; the photos
attached to entries **follow it, ordered by the date of the entry each belongs to**. When the rabbit
has no profile photo, the entry photos simply start the order. Nothing is ordered by upload time, and
the rule is the same for an active and an archived rabbit (D2.8).

### D2.11 — The severe-health window is 7 × 24 hours rolling back from the present instant (settles Q3)

DASH-4's severe-health clause uses a **rolling window of 7 × 24 hours measured back from the present
instant**, not a count of calendar days. It is therefore **timezone-independent** and free of any
calendar day boundary, which is consistent with P0's rule that timestamps are absolute instants (DM-6):
the window is decided from the stored entry instant alone. A test can assert it against stored instants
without a device calendar — instants stated relative to the entry's stored timestamp and to the present
instant. The weight-drop clause of DASH-4 is P1's and is not redefined here.

## State and boundaries

```
Rust (src-tauri/)                                      Frontend (src/)
  src-tauri/src/ipc/             extended contract: health, catalog, history, photos  → src/bindings.ts (generated)
  src-tauri/src/domain/health/   health entry, severity, canonical categories, household catalog
  src-tauri/src/domain/history/  timeline item, ordering, per-view narrowing and filters
  src-tauri/src/photos/          photo owner, save/upload path, local read copy, availability
  src-tauri/src/storage/         (P0) household isolation, canonical form, immutability
  src-tauri/src/archive/         (P1) is_archived gate, reused by every P2 read and write
  src-tauri/capabilities/        photo-source permissions (tauri-security owns)
                                                 → src/features/health/  api.ts + hooks + form state
                                                 → src/features/photos/  api.ts + hooks + availability state
                                                 → src/features/history/ hooks + view state
                                                 → src/routes/ health form, entry detail, health-only
                                                    history, combined history, gallery
```

Rules held here:

- The frontend reaches IPC only through `src/features/*/api.ts` over generated bindings (project hard
  rule 2).
- A health entry is a household-scoped item: it obeys P0's storage boundary, P0's timestamps and P0's
  save path. P2 adds no second write path and no second error surface.
- A photo reference is shared data; photo content on the device is a read cache. `photo_ref` values
  are opaque and stable, shared verbatim across members.
- The tag catalog is household-scoped data, never per-device. Nothing about health logging is stored
  on the device only.
- Archived-ness is read from the rabbit, never inferred from the entry.

## IPC contract — to be defined by architect

**Status: proposal only. `architect` owns this section and the typed stubs in `src-tauri/src/ipc/`
(tasks 1.1–1.2). Names follow the P0 style (snake_case, opaque string ids, one `CommandError`). Nothing
may be implemented against them until the contract is frozen and `bun run gen:bindings` regenerates
`src/bindings.ts`.**

### Commands the frontend needs in this phase

| Command (proposed) | Purpose | Requirements |
|---|---|---|
| `create_health_entry(rabbit_id, tag_ids, severity, note?, timestamp?, photo_refs?)` | Create a health entry; zero tags allowed; timestamp defaults to now | HL-1, HL-2, HL-3, PHOTO-1 |
| `list_health_entries(rabbit_id, filter?)` | The health-only list with tag / severity / date-range narrowing | HL-4, HL-7 |
| `get_health_entry(entry_id)` | Detail payload: severity, all tags, full untruncated note, timestamp, photos, archived state | HL-7 |
| `update_health_entry(entry_id, …)` | Edit a past entry; shared with members | HL-5 |
| `delete_health_entry(entry_id)` | Delete a past entry; shared with members | HL-5 |
| `list_rabbit_history(rabbit_id, view, filter?, range?)` | `view ∈ {weight_only, health_only, combined}`; `combined` interleaves by timestamp. `weight_only` is P1's; `architect` reconciles both phases into one command | HL-3, DASH-7 |
| `ensure_health_tag_catalog(household_id)` | Idempotently establish and return the household's canonical categories | HL-6, DM-2 |
| `upload_photo(source, owner)` | Save photo content into the household's collection; reachability-gated; returns a reference | PHOTO-1, PHOTO-4 |
| `resolve_photo(photo_ref)` | Resolve a photo for display: local copy, fresh fetch, or an explicit unavailable-offline state | SYNC-4, PHOTO-5 |
| `list_rabbit_photos(rabbit_id, range?)` | Gallery: the rabbit's photos with the entry each belongs to — profile photo first, then entry photos ordered by entry date (D2.10) | PHOTO-3 |
| `remove_photo(photo_ref)` | Remove a photo from its owner; a save | PHOTO-1, PHOTO-4 |
| `set_rabbit_profile_photo(rabbit_id, photo_ref)` | Set the profile photo; the rabbit record itself is P1's | PHOTO-2 |
| *(type extension, not a new command)* | The P1 dashboard list row gains the derived health-flag fields; `architect` reconciles the row type with P1 rather than adding a second dashboard command | DASH-4 |

Photo content transfer is a contract decision for `architect`: bytes over IPC versus a device-local
file reference the UI resolves. Whatever is chosen must (a) keep large content off the JS heap where
possible, and (b) make the local copy reusable without a second fetch.

### Events the frontend needs in this phase

| Event (proposed) | Payload | Purpose |
|---|---|---|
| `health:changed` | `{ household_id, rabbit_id, kind: created \| updated \| deleted \| catalog_ready }` | Invalidate only the affected health, history and dashboard queries |
| `photos:changed` | `{ household_id, rabbit_id?, photo_ref, kind: added \| removed \| cached }` | Invalidate gallery/entry photo queries; `cached` marks a photo newly available locally (SYNC-4) |

P0's `household:changed` and `service:status-changed` are reused unchanged; no P2 event replaces them.

### Shared types

| Type (proposed) | Shape | Notes |
|---|---|---|
| `HealthEntryId`, `HealthTagId`, `PhotoRef` | opaque, stable strings | shared verbatim across members |
| `CanonicalHealthCategory` | `appetite \| stool_droppings \| activity_behavior \| breathing \| eyes_nose \| coat_grooming \| teeth \| posture_gait \| other` | the nine canonical categories (HL-1) |
| `HealthSeverity` | `mild \| moderate \| severe` | exactly three levels (HL-1) |
| `HealthTag` | `{ id, household_id, canonical_category }` | exactly one copy per household (DM-2) |
| `HealthEntry` | `{ id, rabbit_id, tag_ids, severity, note?, timestamp, photo_refs }` | `timestamp` is P0's `Instant` (DM-6) |
| `HealthEntryFilter` | `{ tag_ids?, severities?, from?, to? }` | HL-4 |
| `HistoryView` | `weight_only \| health_only \| combined` | `weight_only` is P1's; the other two are this change's |
| `HistoryItem` | `{ kind: weight \| health, id, timestamp, payload, photo_refs }` | one interleaved timeline model (HL-3) |
| `Photo` | `{ id, owner, created_at }` | PHOTO-1 |
| `PhotoOwner` | `{ kind: rabbit_profile \| weight_entry \| health_entry, id }` | PHOTO-1, PHOTO-2 |
| `PhotoSource` | `{ kind: photo_library \| file, handle }` | platform-mediated selection (D2.7) |
| `PhotoAvailability` | `available_local \| fetched \| unavailable_offline \| missing` | drives the SYNC-4 placeholder decision |
| `RabbitPhoto` | `{ photo_ref, source: PhotoOwner, entry_timestamp? }` | gallery ordering: profile photo first, then entry photos by entry date (PHOTO-3, D2.10) |
| `DashboardHealthFlag` | `{ has_recent_severe_health: bool, latest_severe_at? }` | DASH-4 severe clause; reconciled into P1's dashboard row |

P0's `CommandError`, `SaveCause` and `Instant` are reused unchanged; P1's `RabbitId`, `WeightEntry` and
`WeightUnit` are reused unchanged. No new failure cause is invented: a photo save that exceeds the
owner's storage allowance reports P0's `allowance_exceeded`.

## Traceability: user stories → scenarios

| Story | Requirement(s) | Capability / scenario |
|---|---|---|
| US-HL-01 | HL-1 | health-logging: "A structured health entry is created with tags and a severity", "Severity is one of three levels" |
| US-HL-02 | HL-2 | health-logging: "A note-only entry is accepted", "A tagless entry displays without an error state" |
| US-HL-03 | HL-1 | health-logging: "The timestamp defaults to now and is editable" |
| US-HL-04 | HL-3, DASH-7 | dashboard: "The full history interleaves weight and health entries by timestamp", "A rabbit with only health entries shows no empty-state prompt" |
| US-HL-05 | HL-4 | health-logging: "Filtering by tag shows only entries carrying it" |
| US-HL-06 | HL-4 | health-logging: "Filtering by severity shows only that severity" |
| US-HL-07 | HL-4 | health-logging: "Filtering by date range restricts to that range" |
| US-HL-08 | HL-5 | health-logging: "Editing a health entry is shared with members", "Deleting a health entry removes it for every member" |
| US-HL-09 | HL-6, DM-2 | data-integrity: "Each household owns exactly one copy of the canonical categories", "Establishing the catalog again creates no duplicate category", "A tag is never referenced by another household's entries" |
| US-HL-10 | HL-7 | health-logging: "The detail shows every recorded field", "The detail shows the entry's photos" |
| US-HL-11 | HL-7 | health-logging: "An archived rabbit's health detail is read-only" |
| US-DASH-04 (severe-health half) | DASH-4 | dashboard: "A Severe health entry within the last 7 days flags the rabbit", "A Severe health entry older than 7 days does not flag the rabbit", "A Mild or Moderate entry does not flag the rabbit" |
| US-DASH-08 | DASH-7, HL-3 | dashboard: "The health-only history shows only health entries", "An archived rabbit's history views are available without add, edit or delete actions" |
| US-PHOTO-01 | PHOTO-1 | photos: "Photos are attached to a weight entry" |
| US-PHOTO-02 | PHOTO-1 | photos: "Photos are attached to a health entry" |
| US-PHOTO-03 | PHOTO-2 | photos: "A profile photo appears as the rabbit's thumbnail across the product" |
| US-PHOTO-04 | PHOTO-3 | photos: "The gallery shows the rabbit's photos", "Entry photos are ordered by entry date", "The gallery remains available for an archived rabbit" |
| US-PHOTO-05 | PHOTO-4 | photos: "A photo attached by one member is seen by another" |
| US-PHOTO-06 | PHOTO-5 | photos: "The detail shows a swipeable full-width pager", "Selecting a timeline thumbnail opens the detail view", "An entry with no photos shows no pager" |
| US-SYNC-07 | SYNC-4 | sync-and-offline: "A photo that was displayed once is shown again offline" |
| US-SYNC-08 | SYNC-4 | sync-and-offline: "A never-displayed photo shows a cannot-load placeholder offline" |
| US-QUAL-09 (tag clause) | DM-2 (NFR-10 via P0) | data-integrity: "A tag is never referenced by another household's entries" |
| US-DASH-04 (weight half), US-DASH-05 | WT-7, DASH-4 weight clause | **P1** — not restated here |

Stories with no P2 scenario: **none** among the stories whose requirements this change owns. Two
requirement IDs sit in a sibling capability by design and are counted once:
**HL-3** is the `dashboard` requirement `Health-only and full combined history views`, and **HL-6** is
the `data-integrity` requirement `A health tag catalog belongs to exactly one household`.

## Risks

| # | Risk | Mitigation in this change |
|---|---|---|
| R1 | **Photo content is the largest consumer of the household owner's storage allowance** (HS-12, P3), so photo saves are where allowance failures will actually appear. | No photo-specific failure is invented: an upload failure maps onto P0's `SaveCause` (`allowance_exceeded`), which is guaranteed reachable by contract — D0.12 makes distinct allowance reporting a hard selection criterion for the shared service (P0 task 1.1), so the cause is not an unresolved observability question. The residual risk here is only that photo uploads are the surface where it will most often be hit. |
| R2 | **The SYNC-4 guarantee depends on a cache nobody has bounded.** | Settled by D2.9: the local copy of a displayed photo is kept locally and indefinitely, is not size-bounded and is never evicted, so the promise in SYNC-4 is unconditional and the placeholder is reachable only for a never-displayed photo. The spec asserts exactly that split and no retention duration. |
| R3 | **Orphaned photo after a partial save.** Content uploaded but the entry save refused (or the reverse) leaves a photo with no owner or an owner with a missing photo. | The entry save carries its photos as one operation (D2.1); `architect` (task 1.1) reuses P0's settled idempotent-request model (D0.14 — every state-changing command carries a client-generated `request_id` deduplicated by the service, so a retry is idempotent by construction) and no task may create an orphan path. |
| R4 | **Two members each establish the tag catalog**, producing duplicate categories and violating DM-2/SYNC-1. | Establishment is idempotent on the canonical category within the household (D2.4) and is tested with a concurrent case; no device-local catalog exists. |
| R5 | **The 7-day severe window is not reproducible** if "last 7 days" is read as calendar days while tests use instants. | Settled by D2.11: the window is a rolling 7 × 24 hours measured back from the present instant and is timezone-independent, so tests assert it against stored instants with instants stated relative to the entry's stored timestamp and to the present instant, and no device calendar is involved. |
| R6 | **Interleaved history re-downloads what P1 already loaded** (NFR-7). | The history read resolves from the already-loaded household collection (D2.6); read-path optimisation itself stays P6. |
| R7 | **Archived read-only (DM-4 / RAB-8) is surfaced in three P2 places** — health detail, history views, gallery — and could drift. | One shared archived gate (D2.8) plus a scenario in each of the three capabilities. |
| R8 | **Photo-source access widens capability/CSP/entitlements on two platforms.** | `tauri-security` (task 1.3) declares the least-privilege permission and scope for iOS and macOS; `apple-native` owns the picker bridges; specs stay platform-neutral except where behaviour genuinely differs. |
| R9 | **Many photos in a gallery or pager on a phone**: memory and jank. | `architect` decides the byte-transfer shape (device-local file reference preferred over inline bytes); thumbnails are not a requirement, so no thumbnail pipeline is invented here. |
| R10 | **Scope creep into P1/P5.** The rabbit dashboard and the medication badge are adjacent. | P2 touches the dashboard only through the two named requirements; the medication badge (DASH-5) and the weight-drop flag (DASH-4 weight clause) are explicitly out. |

## Verification

- Specs: `openspec validate add-health-and-photo-history --strict` (could not be run in the authoring
  session — the shell was unavailable; structural checks were done by hand).
- Gate (per `definition-of-done`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` (no diff),
  `git diff --exit-code src/routeTree.gen.ts`, `bun run typecheck`, `bun run lint`, `bun run test`,
  `bun run test:e2e`.
- Every requirement above is named by at least one test task in `tasks.md`.
- UI tasks capture 390×844 and 1280×800 screenshots into `.artifacts/screens/`.
- Cross-check at review: the delta contains exactly HL-1, HL-2, HL-4, HL-5, HL-7, PHOTO-1 … PHOTO-5,
  the DASH-4 severe-health clause, the DASH-7 health clause, HL-3, SYNC-4 and DM-2 — no more, no less —
  and no P1 or P5 requirement name is restated or altered.
