# add-vet-export-and-reminders

> Phase **P4 — Handoff & preferences** of the BunScale reimplementation.
> Source of truth: `docs/BunScale-Reimplementation-Requirements.md` and
> `docs/BunScale-Reimplementation-User-Stories.md`.
> This change owns **13 requirement IDs and no others**: EXP-1 … EXP-4, REM-1 … REM-3,
> MED-9, MED-10, MED-11, SET-3, NFR-4, NFR-5 — plus the one unified-reminder-mechanism
> requirement it names itself (the expression of decision **D3**), which has no source ID.

## Why

BunScale's promise is a shared record of care that a household can *act* on. By the end of
P5 the product holds rabbits, weights, health observations, photos, medication courses and
doses. Two things are still missing before the product is usable in the real world:

1. **The record cannot leave the product.** G-6 is vet handoff: a rabbit's weight, health and
   medication history must be handable to a vet as a formatted PDF, and exportable as raw CSV for
   the household's own archiving. Without it the household's care record stays trapped in an app,
   which is the "kept in prose" failure the product exists to correct.
2. **The record only works if someone remembers to use it.** A trend is only visible if weights
   are taken regularly, and a medication course is only safe if slots are not missed. Reminders
   are what close that loop, and they are deliberately **one** mechanism (D3) — not two
   subsystems with two follow-up logics (IMP-5).

The phase also lands the last two cross-cutting quality bars: full German/English coverage with
metric defaults (NFR-5, which P0's i18n scaffold consciously left to this phase) and
accessibility — preferred text size, screen-reader labels and an accessible chart summary
(NFR-4) — plus the per-device language override (SET-3).

**Why here and not earlier:** export needs the medication model (P5) to include courses and
dose histories, and reminders need the medication schedule (P5) to derive medication slots.
Everything this change reads and writes already exists by the time it starts.

### Why P4 delivers after P5, and why MED-9 … MED-11 live here

The source user-stories document proposes "P4 export/reminders, then P5 medications". That
ordering is circular once the two phases are read closely: EXP-1/EXP-2/EXP-4 export medication
courses and dose histories that only exist once P5's medication model does, and MED-9's reminder
slots can only be derived from P5's schedule types. The product owner therefore resolved the
order to **P0 → P1 → P2 → P3 → P5 → P4 → P6**. As part of that resolution, the medication
reminders **MED-9, MED-10 and MED-11 moved into this change**, which is where the reminder
mechanism is specified: decision D3 says reminders are **one** unified mechanism, so medication
reminders are written here, beside the weigh reminders, as *configurations* of that one mechanism.
Their relocation is therefore deliberate and not a duplication: `add-medication-tracking` (P5)
owns MED-1 … MED-8 and MED-12 (the medication entity, the four schedule types, the schedule
summary, dose recording/editing/deleting and the near-simultaneous-dose guard) and specifies no
reminder behaviour at all; this change ADDS MED-9 … MED-11 to the same `medication-tracking`
capability on top of P5's model and restates none of it.

## What changes

Specs remain technology-agnostic (D6): "the shared service" and "the account" are the only names
used for the sharing mechanism, and the reminder and export delivery surfaces are described as
the device's own facilities, never by vendor or API name.

| Capability | Requirements added | Substance |
|---|---|---|
| `export` | EXP-1 … EXP-4 | A rabbit's weight, health and medication history exported for a selectable date range as a formatted, vet-readable PDF, including finished courses with their full dose histories; the same history as raw CSV with dedicated medication and dose files; the files handed to the device's standard sharing options; export completing for every schedule type with or without recorded doses |
| `reminders` | REM-1, REM-2, REM-3, plus the unified-mechanism requirement named here | One scheduling and follow-up mechanism for both reminder kinds (D3); a recurring weigh reminder per rabbit or household-wide with a user-defined schedule; reminders scoped to the person and device that created them and never notifying other members; an optional missed-reminder follow-up the next day when no weight entry was logged |
| `medication-tracking` | MED-9, MED-10, MED-11 | A reminder at each upcoming slot of an active medication on the device that configured it, offering record-the-dose and snooze; a single follow-up for a missed dose window, enabled by default; no reminder schedule for PRN medications |
| `settings-profile` | SET-3 | Product language selectable as System / Deutsch / English, stored per device, taking effect on the next launch with the running session keeping its language |
| `product-quality` | NFR-4, NFR-5 | Preferred text size respected, screen-reader labels, an accessible weight-chart summary; complete German and English coverage including system-supplied labels, with metric units as the default |

Supporting work: the P4 IPC contract with typed stubs in `src-tauri/src/ipc/`, the device-local
reminder store, the PDF and raw-CSV generation path, the native notification and
file-handoff bridges, the language preference, the frontend data layer and the P4 screens
(reminders, export flow, language control, accessibility surfaces), and tests at unit, component,
screen-reader and E2E level.

### Settled decisions this change implements

- **D3 — one unified reminder mechanism.** This change *is* the implementation of D3. One
  device-local store, one schedule form, one occurrence model, one follow-up path; weigh and
  medication reminders differ only in configuration (target, occurrences, follow-up default).
  IMP-5 is closed. A second reminder subsystem is explicitly out of scope and is a review
  finding, not an option.
- **D1 — connectivity-required saves.** Reminders and the language preference are **device-local
  configuration**, not household data, so they are not saves to the shared service and are not
  subject to the reachability gate; P0's rule that per-device preferences never enter the
  household collection stands. Export is a **read**, so it is served from already-loaded data
  through P0's read path (SYNC-3) and adds no write and no local write queue.
- **D4 — fresh start, no migration.** Export is vet handoff (G-6), not a migration or
  import/restore path; nothing here makes existing users' data portable.
- **D5 — no chart baseline band.** The accessible chart summary reports the latest value and the
  trend direction from the chart's own series. It adds no target band, no baseline value and no
  legend for one.
- **D6 — technology-agnostic specs.** No vendor, integration or API name appears in the specs.
  The notification delivery and the sharing handoff are described as the device's standard
  facilities; the concrete bridges are named only in `design.md` and `tasks.md`.
- **D2 — owner-dependent sharing.** Unaffected; this change adds nothing that would require
  ownership transfer.

## Impact

- **New Rust surface:** `src-tauri/src/ipc/` (reminder, export and language commands, events,
  shared types), `src-tauri/src/reminders/` (the one device-local store, occurrence model,
  follow-up), `src-tauri/src/export/` (selection, document generation, raw-file generation),
  `src-tauri/src/settings/` (the language preference beside P0's display unit), and the
  platform-gated bridges for notification delivery and file handoff.
- **New frontend surface:** `src/routes/` (reminders screen, weigh-reminder form, export flow,
  the reminder-action landing), `src/features/reminders/`, `src/features/export/`, the
  language control in `src/features/settings/`, the accessible chart summary and screen-reader
  labels in the chart and control components, the completed German and English string tables,
  and regenerated `src/bindings.ts` (never hand-edited).
- **Security surface:** notification authorization and the reminder actions, and writing the
  exported files to a device-local location, change `src-tauri/capabilities/`, scopes, and
  possibly entitlements — via `tauri-security` only. No CSP widening is expected, because no
  remote content is loaded. A background mode is deliberately **not** requested (see
  `design.md`, D4.3).
- **Dependencies added:** one document-generation dependency with a one-line justification in
  `design.md`, plus the native notification and sharing bridges. Any dependency must support
  both iOS and macOS (project hard rule 6).
- **Data written:** nothing new into the household's shared collection. Reminders and the
  language preference stay on the device; export reads and produces files locally. This is a
  property the review checks.
- **Risk carried forward:** reminder delivery depends on the device's authorization state and on
  the product actually running (follow-up evaluation), so honesty about both is a first-class
  concern (see `design.md`, D4.3/D4.7 and risks R1/R3).
- **Not in this change:** the medication entity, the four schedule types, the schedule summary
  and dose recording/editing/deleting (MED-1 … MED-8, MED-12 — P5); the active-medication
  dashboard badge and medications section (DASH-5, DASH-7 medication clause — P5); the display
  weight unit (SET-4 — P0); the account name and Settings entry point (SET-1, SET-2 — P0);
  honest service status (NFR-6 — P3); bounded cold load (NFR-7 — P6); visible actions and
  confirmations (NFR-9 — P1); the write model, conflict resolution and privacy diagnostics
  (NFR-1/2/3/8 — P0); any data import or migration.

## Dependencies

**Predecessors, all of which MUST be archived before this change:**
`add-shared-household-foundation` (P0), `add-rabbit-weight-core-loop` (P1),
`add-health-and-photo-history` (P2), `add-household-sharing-hardening` (P3) and
**`add-medication-tracking` (P5)**.

| Phase | change-id | What P4 needs from it |
|---|---|---|
| P0 | `add-shared-household-foundation` | the read path (already-loaded data) that export reads through, the cause-carrying failure surface export failures are reported with, the household collection boundary that keeps reminders and the language preference out of shared data, the per-device preference storage and the i18n scaffold |
| P1 | `add-rabbit-weight-core-loop` | rabbit identity and the weight history export covers, the weight trend series the accessible chart summary is derived from, the archived state whose export action RAB-8 keeps available |
| P2 | `add-health-and-photo-history` | the health entry model and the interleaved history the export presents |
| P3 | `add-household-sharing-hardening` | the member identity a reminder's creating person resolves to, and the shared/device-local split restated there |
| **P5** | **`add-medication-tracking`** | **the medication and `Schedule` model, the human-readable schedule summary (MED-4) export reuses, the record-a-dose flow and its pre-filled confirmation (MED-5, MED-8) that MED-9's "record as given" action opens, and the active/finished lifecycle (MED-2) that decides which medications raise reminders** |

| Phase | change-id | Depends on P4 for |
|---|---|---|
| P6 | `add-read-path-performance` | nothing functionally; it optimises the read path P4 also reads through, and it archives after P4 |

**Archive order.** Because each later change modifies capability specs opened earlier, archives
MUST run strictly in order **P0 → P1 → P2 → P3 → P5 → P4 → P6**. Archiving out of order leaves
`openspec/specs/` with deltas applied against text that does not yet exist — in this case, most
visibly, a `medication-tracking` capability holding MED-9 … MED-11 while the medication entity
they are reminders *for* is missing. This is recorded as an open coordination item in
`questions.md`.

**Open items.** P0's `questions.md` Q1 … Q3 are **settled**, not open: they are recorded as P0 design
decisions **D0.12, D0.13 and D0.14**, and this change consumes them as fixed behaviour — allowance
exhaustion is reported as its own condition, distinct from a generic write error; the bounded times
(15 s save attempt, 5 s reachability probe) and the offline threshold (3 consecutive failures) are named
constants; and every state-changing command carries a client-generated `request_id` the service
deduplicates on, so a retry is idempotent by construction. Every other change's open items remain
theirs. This change's own open items are `questions.md` Q1 … Q7.
