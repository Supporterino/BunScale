# BunScale — Reimplementation Requirements

> **Status:** Draft for reimplementation planning
> **Scope:** All shipped capabilities **except** developer-only demo tooling.
> **Fidelity:** Describes the product **as it behaves today**, plus a clearly flagged
> *Improvement Candidates* section for known pain points.
> **Stance:** **Technology-agnostic.** These requirements state behaviour and outcomes
> only. No implementation choice, vendor, or integration is mandated or implied. Terms such as "the shared service" refer to *whatever* mechanism provides
> multi-person data sharing — they are deliberately not a product name.

---

## 1. How to read this document

- **SHALL** = mandatory. **SHOULD** = strongly expected, deviation needs justification.
  **MAY** = optional.
- Requirements carry stable IDs (`HS-1`, `WT-4`, …) grouped per capability. The companion
  document `BunScale-Reimplementation-User-Stories.md` implements these as user stories and
  maps every story back to these IDs.
- A "capability" here mirrors a coherent product area, not a code module. Nothing in this
  document prescribes how a capability is built.

---

## 2. Product vision

BunScale is a multi-person rabbit health companion. It lets a household track each
rabbit's **weight over time** and **structured health observations**, so early signs of
illness are caught from trends rather than from memory, and it lets a household share
one authoritative record of care — including **medication courses and doses** — across
several people who each use their own account. A rabbit's full history can be exported
for a vet visit.

The product exists because rabbits hide illness until it is advanced; a gradual weight
decline or a cluster of subtle symptoms is often the earliest signal. Two failure modes
motivate the design:

1. **Missed trends** — weight and symptoms are forgotten or kept in prose, so patterns
   never become visible.
2. **Coordinated multi-person care** — more than one person cares for the same rabbits;
   care information (especially medication) must be shared, and double-dosing must be
   prevented.

---

## 3. Goals and non-goals

### 3.1 Goals

- **G-1** Fast, low-friction weight capture (per rabbit, ideally ≤3 taps from launch).
- **G-2** Trend visualization that surfaces gradual or sudden decline.
- **G-3** Structured, filterable health/abnormality logging (not free-form prose).
- **G-4** True multi-person household access — several people, each with their own
  account, editing one shared set of data.
- **G-5** Shared medication tracking with dose logging to prevent double dosing.
- **G-6** Vet-handoff: weight, health, and medication history exportable.
- **G-7** Sensible behaviour when connectivity is unavailable (show what was already
  loaded; never misrepresent a failed save).

### 3.2 Non-goals (as shipped)

- **NG-1** Species beyond rabbits.
- **NG-2** Clinical decision support or dosage calculation — the product records doses,
  it does not compute or advise them.
- **NG-3** Ownership transfer of a shared household when the creating account disappears
  (accepted risk, see HS-13).
- **NG-4** Third-party analytics of health data.
- **NG-5** Developer-only demo tooling (excluded from this document by scope decision).

---

## 4. Personas and roles

| Persona | Description | What they do |
|---|---|---|
| **Primary owner** | Creates the household; primary caregiver. | Sets up household + rabbits, invites others, logs weight/health, configures reminders. |
| **Co-owner / partner** | Second caregiver with their **own account**. | Equal read/write on all data; logs their own weight/health/doses; sets up **their own** reminders independently. |

**Roles are flat.** The only privilege asymmetry in the product is *ownership of a
household* (HS-10), used solely to gate destructive household deletion.

---

## 5. Domain model

Entities (conceptual; identifiers are opaque and stable across everyone's copies):

```
Household
 ├─ id, name, createdAt
 ├─ members: [Member]              (max 6, incl. creator)
 ├─ rabbits: [Rabbit]
 └─ healthTagCatalog: [HealthTag]  (household-scoped)

Member
 ├─ id, accountUserID, displayName
 └─ (identity of the account that joined; used to attribute doses and label the member list)

Rabbit
 ├─ id, name, photo?, breed?, dob? (may be "unknown"), sex, neuterStatus,
 │   adoptionDate?, isArchived, weightDropThresholdPercent (default 5%)
 ├─ weightEntries: [WeightEntry]
 ├─ healthEntries: [HealthEntry]
 └─ medications: [Medication]

WeightEntry
 ├─ id, rabbitID, valueGrams, timestamp, note?, photos: [Photo]

HealthEntry
 ├─ id, rabbitID, tags: [HealthTag], severity (Mild|Moderate|Severe), timestamp,
 │   note?, photos: [Photo]

HealthTag
 ├─ id, householdID, name (one of 9 canonical categories)

Medication
 ├─ id, rabbitID, name, reason?, startDate, endDate?, amountValue, amountUnit
 │   (mg|ml|drops|tablets), schedule: Schedule, note?

Schedule (value type)
 ├─ type: timesPerDay | everyXDays | weekly | prn
 ├─ slots: [timeOfDay]           (timesPerDay: 1–4; weekly: 1 weekday + time)
 ├─ intervalDays: Int?           (everyXDays)
 ├─ weekday: Weekday?            (weekly)
 └─ maxPerDay: Int?              (prn hint)

Dose
 ├─ id, medicationID, timestamp, amountValue, amountUnit, note?, giverMemberID

Photo
 ├─ id, owner (rabbit profile | weight entry | health entry), photoRef, createdAt
```

**Per-device configuration (deliberately NOT shared between people):**

```
ReminderConfig           — recurring weigh reminder; scope = household or one rabbit;
                           schedule; rePromptEnabled; owned by the member who created it.
MedicationReminderConfig — per-medication reminder + rePromptEnabled (default on).
DisplayPreference        — g/kg display unit (per person, per device).
LanguagePreference       — System | DE | EN (per person, per device).
```

### 5.1 Domain invariants

- **DM-1** Weight is stored in **one canonical unit (grams)**. Display unit is a
  presentation concern only.
- **DM-2** A `HealthTag` belongs to exactly **one** household and is never referenced by
  another household's entries.
- **DM-3** At most one household is *active* at a time for a given person.
- **DM-4** An archived rabbit is **read-only** everywhere (no create/edit/delete of its
  weight, health, or medication data).
- **DM-5** A `Dose` records the **giver** (a named member of the household).
- **DM-6** Timestamps default to "now" and are editable; they are stored as absolute
  instants, not as offsets.

---

## 6. Functional requirements

### 6.1 Household sharing (HS)

| ID | Requirement |
|---|---|
| **HS-1** | A person SHALL be able to create a Household with a non-blank name; the creator SHALL become its first member and it SHALL become the active household. |
| **HS-2** | A household-creation onboarding flow SHALL be shown when no household exists; it SHALL NOT be repeated once a household exists. |
| **HS-3** | A person SHALL be able to hold **more than one** household, each with its own rabbits/members, without mixing data; exactly **one** SHALL be active. |
| **HS-4** | The Settings & Profile screen SHALL allow listing all households, switching the active household, and creating an additional household, without re-running first-run onboarding. |
| **HS-5** | A member SHALL be able to invite another person via an invitation link. Before attempting, the system SHALL verify the shared service is available; it SHALL give progress feedback; it SHALL present the standard sharing options on success; on failure it SHALL state the **specific cause**; a stalled attempt SHALL resolve within a bounded time with feedback; and an existing invitation SHALL be **reused** rather than duplicated. |
| **HS-6** | Accepting an invitation SHALL make the recipient a member with read/write access, including on a **fresh setup** with no household: once the household's data becomes available to them, the system SHALL show that household's dashboard **within a bounded time and without requiring the recipient to take further action**, and SHALL NOT fall back to onboarding merely because data had not yet arrived. A household the person creates while an invitation is still loading SHALL remain active. |
| **HS-7** | A household SHALL be capped at **6 members**; invitations beyond the cap SHALL be blocked with an explanation. |
| **HS-8** | All members SHALL have equal read/write access; no permission tiers SHALL exist. |
| **HS-9** | The member list SHALL always list the current person (labeled with a localized "You"-style label), showing their **account display name** with a local/placeholder fallback when it cannot be resolved. It SHALL keep showing locally known members when a participant refresh fails (surfacing the cause), and SHALL render before participants resolve, refreshing once they do. |
| **HS-10** | Only the **household owner** SHALL be able to permanently delete a household. Deletion SHALL be irreversible, SHALL cascade to all rabbits/entries/photos/medications/doses/members/tags, SHALL withdraw access from all members, and SHALL be shared with them. On the Settings row it SHALL be a **directly visible control** (not a hidden gesture), and confirmation SHALL state the full consequence. After deletion the newest remaining household SHALL become active, or the system SHALL fall back to onboarding when none remain. Deletion failure SHALL leave the household intact and be surfaced. |
| **HS-11** | Changes by any member SHALL reach all members within normal shared-service propagation time, without requiring the user to open the product manually (updates MAY be delivered in the background). |
| **HS-12** | Shared content SHALL count against the **household owner's** storage allowance (accepted shared responsibility). |
| **HS-13** | *(Accepted risk)* Household data is owned by the account that created it; if that account is removed, other members may lose access. No ownership-transfer flow exists. |

### 6.2 Synchronization & persistence (SYNC)

| ID | Requirement |
|---|---|
| **SYNC-1** | All household data (households, members, rabbits, weight/health entries, tags, medications, doses, photos) SHALL be shared across members as one logical collection per household, with exactly one copy of each item for everyone (no duplicates). |
| **SYNC-2** | New or modified data SHALL only be saved when the shared service is reachable. A save that cannot complete SHALL NOT be silently discarded or faked; the user SHALL be told the **cause** (connectivity unavailable, no account, storage allowance exceeded, temporarily rate limited) and offered a retry. |
| **SYNC-3** | Reads SHALL be served from previously loaded data when the service is unreachable; a launch with connectivity SHALL refresh that data. With nothing previously loaded and no connectivity, the product SHALL show an explicit "connectivity required" state — never stale or empty data presented as current. |
| **SYNC-4** | A photo SHALL be available locally after it is first displayed so it can be viewed again without connectivity; a never-viewed photo while offline SHALL render a "cannot load" placeholder. |
| **SYNC-5** | Concurrent edits to the same item SHALL converge on a single version using **last-write-wins**, without duplication or loss. |
| **SYNC-6** | A household created or joined mid-session SHALL receive live updates immediately (without restarting the product), including households joined via an accepted invitation. |
| **SYNC-7** | Setting the product up again on the same account SHALL recover the household's data from the shared service (the shared service is the source of truth). |

### 6.3 Dashboard (DASH)

| ID | Requirement |
|---|---|
| **DASH-1** | On launch the product SHALL show all **non-archived** rabbits of the active household, each with name, thumbnail, and last-weighed date. |
| **DASH-2** | A rabbit with no weight entries SHALL show a "never weighed" indication instead of a date. |
| **DASH-3** | The list SHALL be sorted by most recent weigh-in (newest first); never-weighed rabbits SHALL sort last. |
| **DASH-4** | A rabbit SHALL be visually flagged when it has an **active weight-drop alert** or a **severe health entry within the last 7 days**. The flag SHALL clear when the drop alert no longer applies (e.g. threshold raised). |
| **DASH-5** | A rabbit with at least one **active medication** SHALL show an **informational** badge, distinct from the warning flag and never using warning styling. |
| **DASH-6** | Selecting a rabbit SHALL open that rabbit's dashboard. |
| **DASH-7** | The rabbit dashboard SHALL show the weight trend chart, a medications section, and a History section linking to weight-only, health-only, and full combined history; an archived rabbit's dashboard SHALL be read-only. |
| **DASH-8** | With zero rabbits, the dashboard SHALL show an empty state prompting the user to add their first rabbit. |
| **DASH-9** | Each dashboard row SHALL carry a per-rabbit quick-weight control opening the weight form pre-bound to that rabbit. |

### 6.4 Rabbit profiles & lifecycle (RAB)

| ID | Requirement |
|---|---|
| **RAB-1** | A rabbit profile SHALL record name, photo, breed (optional), date of birth (optional, supports "unknown"), sex, neuter/spay status, and adoption date. |
| **RAB-2** | A profile SHALL require a non-blank name; whitespace-only SHALL be treated as blank and rejected. |
| **RAB-3** | A household SHALL support multiple rabbits with no fixed upper limit. |
| **RAB-4** | The rabbit-creation form SHALL be reachable from the dashboard at all times, and the empty state SHALL include an add-first-rabbit prompt. |
| **RAB-5** | Every field of an active rabbit's profile — including the weight-drop threshold — SHALL be editable and the change SHALL be shared with all members. The edit action SHALL be a **directly visible control** on the rabbit's detail, not hidden in a menu. Archived rabbits SHALL NOT be editable. |
| **RAB-6** | A rabbit SHALL be archivable (soft delete): hidden from active lists, history retained, retrievable from an archived list. |
| **RAB-7** | An archived rabbit SHALL be restorable to the active list with history intact. |
| **RAB-8** | An archived rabbit's timeline SHALL be read-only (no new entries, no edit/delete of existing entries); export and photo-gallery actions SHALL remain available. |
| **RAB-9** | Archived data SHALL be retained indefinitely by default. |
| **RAB-10** | A rabbit and all associated data SHALL be permanently deletable on confirmation, and the deletion SHALL be shared with all members. |

### 6.5 Weight tracking (WT)

| ID | Requirement |
|---|---|
| **WT-1** | A weight entry SHALL consist of a value, a timestamp (default now, editable), and an optional note. |
| **WT-2** | Two quick paths SHALL exist: a per-rabbit weigh control, and a quick-entry control with a rabbit picker. The per-row path SHALL reach a saved entry in **≤3 taps from launch**. |
| **WT-3** | Values SHALL be stored in grams. The display-unit preference (g/kg) SHALL be per person and per device, default from the person's regional settings, settable in Settings, applied only at the entry/display boundary, and the entry form SHALL NOT offer its own unit picker. |
| **WT-4** | Non-positive values SHALL be rejected; implausible outliers (e.g. >10 kg) SHALL require confirmation before saving. |
| **WT-5** | Any past entry SHALL be editable or deletable, and the change SHALL be shared with all members. |
| **WT-6** | The rabbit dashboard SHALL show a weight trend chart with selectable ranges **7d/30d/90d/all**; a dedicated weight-only history sub-view SHALL exist with its **own independent** range. |
| **WT-7** | A visual warning SHALL be shown when an entry represents a drop exceeding the rabbit's configurable threshold (default **5%**) versus the previous entry; the warning SHALL be **recomputed** when the threshold changes (appearing or disappearing accordingly) and SHALL also flag the rabbit on the dashboard (DASH-4). |
| **WT-8** | A weight entry detail view SHALL show the value (display unit), timestamp, full untruncated note, the drop-alert flag when applicable, and the entry's photos as a swipeable pager, with Edit (pre-filled form) and Delete (after confirmation). Archived rabbits' entry details SHALL be read-only. |

### 6.6 Health logging (HL)

| ID | Requirement |
|---|---|
| **HL-1** | A health entry SHALL consist of zero or more category tags (Appetite, Stool/Droppings, Activity/Behavior, Breathing, Eyes/Nose, Coat/Grooming, Teeth, Posture/Gait, Other), a severity (Mild/Moderate/Severe), an optional note, an editable timestamp (default now), and optional photos. |
| **HL-2** | An entry with **no tags** SHALL be savable (free-text-only entries are valid). |
| **HL-3** | Weight and health entries SHALL appear interleaved by timestamp in a single full-history timeline. |
| **HL-4** | Health entries SHALL be filterable by tag, severity, and date range in the health-history sub-view. |
| **HL-5** | Any past health entry SHALL be editable or deletable, and the change SHALL be shared with all members. |
| **HL-6** | The tag catalog SHALL be **household-scoped**: each household owns exactly one copy of the canonical tags, populated idempotently, and a tag SHALL NEVER be referenced by another household. |
| **HL-7** | A health entry detail view SHALL show severity, all tags, full untruncated note, timestamp, and a swipeable photo pager, with Edit and Delete. Archived rabbits' detail SHALL be read-only. |

### 6.7 Medication tracking (MED)

| ID | Requirement |
|---|---|
| **MED-1** | A rabbit SHALL be able to have one or more medications, each with a name, optional reason, start date (default today, editable), optional end date, amount per serving (numeric + unit from **{mg, ml, drops, tablets}**), and optional note. Multiple medications SHALL be active concurrently. |
| **MED-2** | A medication SHALL be **active** while its end date is absent or in the future and **finished** once passed. Finished medications SHALL remain visible with full dose history and SHALL NOT generate reminders. Setting the end date to today SHALL immediately finish the course and cancel pending reminders. |
| **MED-3** | Supported schedules SHALL cover: `timesPerDay` (1–4 slots with explicit times), `everyXDays` (interval counted fixed from the start date, at a chosen time), `weekly` (one weekday + time), and `prn` (as needed, optional max-per-day hint). `timesPerDay` slots SHALL be pre-filled with sensible editable defaults. |
| **MED-4** | A human-readable schedule summary SHALL render for **every** schedule type, with and without recorded doses, without failing, including the type's parameters. |
| **MED-5** | A user SHALL be able to record a dose with an editable timestamp (default now), an amount pre-filled from the serving and editable, an optional note, and the identity of the **member who recorded it**. |
| **MED-6** | A chronological dose list SHALL show each dose's timestamp, amount, and giver; a dose SHALL be deletable and its amount/note/timestamp editable, shared with all members. |
| **MED-7** | Before confirming a dose, the system SHALL warn if a dose for the same medication was already logged within a short recent window (**≈15 minutes**), and SHALL let the user continue or cancel. |
| **MED-8** | The medication detail SHALL offer a primary "give dose" action opening a confirmation with the amount pre-filled. |
| **MED-9** | For each upcoming slot of an **active** medication, a reminder SHALL be raised on the device that configured it; it SHALL offer actions to record the dose as given (opening the pre-filled confirmation) or to snooze. Medication reminders SHALL be per-device and SHALL NOT notify other members. |
| **MED-10** | A missed dose window SHALL optionally trigger a **single** follow-up reminder; this SHALL default to **enabled**. |
| **MED-11** | PRN medications SHALL NOT schedule reminders. |
| **MED-12** | Finished medications' records, and all medications of an archived rabbit, SHALL be viewable but read-only. |

### 6.8 Photos (PHOTO)

| ID | Requirement |
|---|---|
| **PHOTO-1** | One or more photos SHALL be attachable to a weight entry or a health entry. |
| **PHOTO-2** | A profile photo SHALL be settable for a rabbit and SHALL appear as its thumbnail across the product. |
| **PHOTO-3** | A per-rabbit gallery SHALL show all attached photos (entries + profile), ordered by entry date. |
| **PHOTO-4** | Photos SHALL be shared with all household members through the household's shared collection. |
| **PHOTO-5** | An entry's photos SHALL be viewable in its detail view as a swipeable full-width presentation; selecting a photo thumbnail in a timeline row SHALL navigate to the **detail view** (not a separate full-screen viewer); an entry with no photos SHALL show no pager. |

### 6.9 Reminders (REM)

| ID | Requirement |
|---|---|
| **REM-1** | A recurring weigh reminder SHALL be configurable per rabbit or household-wide with a user-defined schedule (e.g. "every Sunday 10:00"). |
| **REM-2** | Reminders SHALL be scoped to the person/device that created them and SHALL NOT be shared with or notify other members. |
| **REM-3** | An optional missed-reminder follow-up SHALL fire the next day when no weight entry was logged since the reminder fired. |

### 6.10 Export (EXP)

| ID | Requirement |
|---|---|
| **EXP-1** | A rabbit's weight, health, and medication history SHALL be exportable for a selectable date range as a formatted, vet-readable **PDF**. Medication export SHALL include finished as well as active courses, each with its **full dose history** (timestamp, amount, giver). |
| **EXP-2** | The same history SHALL be exportable as raw **CSV**, with **dedicated files** for medications and for doses. |
| **EXP-3** | Exported files SHALL be presented through the device's standard sharing options. |
| **EXP-4** | Export SHALL succeed without failing for **any** supported schedule type, including `everyXDays` and `prn`, with or without recorded doses. |

### 6.11 Settings & profile (SET)

| ID | Requirement |
|---|---|
| **SET-1** | A Settings & Profile screen SHALL be reachable from a gear-shaped control on the dashboard and SHALL open as its own screen. |
| **SET-2** | The current person's name SHALL be shown **read-only**, resolved from their account identity, falling back to the locally known member name and then a localized "You" placeholder; it SHALL NOT disrupt the screen when unresolved. |
| **SET-3** | The product language SHALL be selectable as System / Deutsch / English, stored **per device**, taking effect on the **next launch** (the current session keeps its language and the user is informed). |
| **SET-4** | The display weight unit (grams/kilograms) SHALL be selectable, stored **per device**, and applied only at the entry/display boundary. |

---

## 7. Non-functional requirements (NFR)

| ID | Requirement |
|---|---|
| **NFR-1 (Connectivity model)** | Saving new/modified data SHALL require the shared service to be reachable and a valid account; failures SHALL be surfaced with cause and never faked. Reading SHALL use previously loaded data when offline, and refresh when the service is available. |
| **NFR-2 (Conflict resolution)** | Concurrent edits SHALL resolve **last-write-wins** without failing or duplicating. |
| **NFR-3 (Privacy)** | Health/weight data SHALL NOT be sent to third-party analytics. Data SHALL be limited to the product's own shared service and locally kept copies. Diagnostic logs SHALL contain identifiers/counts/outcomes only — **never** rabbit names, note text, photo content, or weight values. |
| **NFR-4 (Accessibility)** | The product SHALL respect the user's preferred text size and SHALL provide screen-reader labels; charts SHALL expose an accessible summary (e.g. latest value + trend direction). |
| **NFR-5 (Localization)** | German and English SHALL be supported; metric units SHALL be the default. All user-facing text (including system-supplied labels like "You" / "never weighed") SHALL be localized. |
| **NFR-6 (Honest service status)** | A single transient failure SHALL NOT show "offline"; offline SHALL be shown only after repeated consecutive failures or a direct check that the service/account is unavailable. A status carried over from a previous session SHALL NOT be shown as current before a fresh outcome. Rapid consecutive updates SHALL show a single stable "syncing" state, not flicker. |
| **NFR-7 (Read performance)** | A cold dashboard load SHALL be bounded (target: a small, fixed number of remote requests), not serialized into many; opening a rabbit SHALL NOT re-download data the dashboard already loaded; a remote change SHALL invalidate only the affected data and SHALL NOT force a full cold reload on the next visit. |
| **NFR-8 (Write semantics)** | Saves SHALL never silently fail or appear to succeed when they did not. Every failure path (save, invitation creation, deletion, export) SHALL produce a user-visible, cause-specific message. |
| **NFR-9 (Interaction conventions)** | Primary/destructive actions (delete household, edit profile, delete entry) SHALL be directly visible controls, not hidden behind undiscoverable gestures (e.g. long-press). Destructive actions SHALL require confirmation stating the consequence. |
| **NFR-10 (Data integrity)** | Weight SHALL be stored in a single canonical unit. A household's tags SHALL never leak to another household. Archived entities SHALL be immutable. |

---

## 8. Improvement candidates (flagged, non-binding)

These are **known pain points or drifts** observed in the current product. They are not
requirements; they are candidates for the reimplementation to resolve deliberately.

| ID | Issue | Suggested direction |
|---|---|---|
| **IMP-1** | **Spec conflict — offline writes.** The medication capability states doses/medications can be created/edited **offline** and shared later, while the sync capability states **saves require connectivity** and nothing is kept locally. Both cannot hold. | Decide one write model and apply it uniformly; true offline-first requires keeping changes locally and reconciling later, which the current design deliberately abandoned. |
| **IMP-2** | **Sharing reliability history.** Earlier versions produced recurring multi-member failures (rabbits disappearing, invitations not acceptable, owner missing from the member list). The reimplementation should treat correctness of sharing as the primary risk. | Treat multi-person convergence as a first-class, testable concern; make invitation acceptance observable end-to-end. |
| **IMP-3** | **Read-path performance.** Dashboard loads were slow (many sequential remote requests), re-fetching data loaded moments earlier. | Adopt load-once, parallel loading, serve-from-loaded-data-first with a freshness window, and invalidate only affected data. |
| **IMP-4** | **Dropped feature.** The original requirements included an optional target/healthy-range band on the weight chart. It has no spec and no implementation. | Either re-include (with a per-rabbit baseline) or drop explicitly. |
| **IMP-5** | **Cross-cutting duplication.** Two independent reminder sub-systems exist (weigh reminders, medication reminders) with separate follow-up logic. | Unify into one scheduling/follow-up mechanism. |
| **IMP-6** | **Discoverability conventions.** Several fix cycles were needed to make delete/edit/invite actions visible rather than gesture-hidden. | Codify NFR-9 as a design-review checklist item. |
| **IMP-7** | **No migration path.** The current design treats existing local data as non-portable (a fresh setup starts empty). | Define an explicit migration/export-import story for a reimplementation. |
| **IMP-8** | **Ownership dependency.** No ownership transfer if the creating account disappears (HS-13). | Evaluate a transfer or backup/restore path. |

---

## 9. Out of scope (as shipped)

- Species beyond rabbits.
- Clinical dosing advice / drug interaction checks.
- A bespoke, product-owned data service beyond what the shared mechanism already provides.
- Third-party analytics on health data.
- Ownership transfer of a shared household.
- Developer-only demo tooling (excluded from this document by scope decision).

---

## 10. Open questions / decisions needed for the reimplementation

1. **Write model** (see IMP-1): connectivity-required saves, or true offline-first with
   local retention and reconciliation? This is the single biggest architectural fork.
2. **Sharing model**: keep the "one shared collection per household, owned by its
   creator, owner-dependent" model, or design something with ownership transfer (IMP-8)?
3. **Reminder unification** (IMP-5): one mechanism or two?
4. **Migration** (IMP-7): must a reimplementation import existing users' data, or is a
   fresh start acceptable again?
5. **Chart baseline band** (IMP-4): in or out?

---

## 11. Appendix — traceability to product capabilities

Every requirement in §6–§7 derives from one of the product's capability specifications.
This mapping is for planning only and names capabilities (product areas), not
implementation choices.

| §This doc | Capability |
|---|---|
| HS | `household-sharing` |
| SYNC | Synchronization & offline behaviour |
| DASH | `dashboard` |
| RAB | `rabbit-profiles` |
| WT | `weight-tracking` |
| HL | `health-logging` |
| MED | `medication-tracking` |
| PHOTO | `photos` |
| REM | `reminders` |
| EXP | `export` |
| SET | `settings-profile` |
| NFR | Cross-cutting product quality |
