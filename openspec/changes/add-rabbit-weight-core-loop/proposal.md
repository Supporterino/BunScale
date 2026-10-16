# add-rabbit-weight-core-loop

> Phase **P1 — MVP core loop** of the BunScale reimplementation.
> Source of truth: `docs/BunScale-Reimplementation-Requirements.md` and
> `docs/BunScale-Reimplementation-User-Stories.md`.
> This change owns 30 requirement IDs and no others (RAB-1 … RAB-10, WT-1 … WT-8, DASH-1/2/3/6/8/9
> plus the weight-scoped parts of DASH-4 and DASH-7, HS-8, NFR-9, DM-1, DM-4).

## Why

P0 proved that a household exists, that its data is one shared collection, and that reads and writes
are honest about connectivity. It deliberately stopped before the product did anything: there were no
rabbits and no weights.

P1 is the change that makes BunScale worth opening. It delivers the smallest complete care loop the
product's two motivating failure modes depend on:

- a person can record their rabbits and keep those records accurate (RAB-1 … RAB-10);
- a weight reading is captured in seconds, from the dashboard, without friction (WT-1, WT-2, DASH-9);
- the reading becomes a *trend* rather than a memory, over selectable ranges and with its own
  history listing (WT-6, DASH-7);
- a decline beyond the rabbit's own threshold surfaces as a warning on the entry and on the dashboard,
  and re-evaluates when the threshold is changed (WT-7, DASH-4);
- the whole household gets the same record with the same rights (HS-8).

Everything the product does later — health logging, photos, medication, reminders, export,
performance — hangs off the rabbit identity and the weight timeline built here. They must therefore
be shaped correctly once: canonical grams, a derived alert, honest failure, and an archived state
that is genuinely immutable rather than merely unrendered.

**Why here and not later:** the drop alert, the trend ranges and the weight detail are the product's
answer to "missed trends" (G-2). Deferring them would leave P1 as a data-entry app with no signal,
which is precisely the failure the reimplementation exists to correct.

## What changes

Specs remain technology-agnostic (D6): "the shared service" and "the account" are the only names used
for the sharing mechanism.

| Capability | Requirements added | Substance |
|---|---|---|
| `rabbit-profiles` | RAB-1 … RAB-10 | Profile fields with "unknown" date of birth, non-blank name, unlimited rabbits, always-available creation, fully editable active profiles including the drop threshold, archive / restore / permanent delete, read-only archived timeline, indefinite retention |
| `weight-tracking` | WT-1 … WT-8 | Weight entries with editable timestamps and notes, two quick paths (row control + picker), entry in the preferred unit with no unit picker, non-positive rejection and outlier confirmation, edit/delete, 7d/30d/90d/all trend ranges with an independently ranged weight history, the drop alert and its recomputation, the entry detail view |
| `dashboard` | DASH-1, DASH-2, DASH-3, DASH-6, DASH-8, DASH-9, plus the weight-scoped parts of DASH-4 and DASH-7 | Rabbit list with name, thumbnail and last-weighed date, never-weighed indication, sort by most recent weigh-in with never-weighed last, opening a rabbit, the empty state, the per-row quick-weight control, the weight-drop dashboard flag, and the rabbit dashboard's weight trend + history entry points |
| `household-sharing` | HS-8 | Every member has equal read/write access; no permission tiers and no per-creator restrictions |
| `product-quality` | NFR-9 | Primary and destructive actions are directly visible controls; destructive ones are confirmed with their consequence stated |
| `data-integrity` | DM-1, DM-4 | Weight stored in grams as the single canonical unit; an archived rabbit is read-only everywhere, retrievable, retained by default and restorable with its history intact |

Supporting work: the P1 IPC contract with typed stubs in `src-tauri/src/ipc/`, rabbit and weight
domain plus the dashboard read model in Rust, the frontend data layer (bindings wrappers, query keys,
range-parameterised queries), the P1 screens (dashboard list and empty state, rabbit creation,
profile, archive list, rabbit dashboard, weight form, weight history, weight detail), and tests at
unit, component and E2E level.

### Cross-phase splits observed by this change

`dashboard`, `data-integrity` and `weight-tracking` are each opened by more than one phase. To keep
archives non-conflicting:

- **`dashboard`** — this change adds the weight trend chart, the weight-only history entry point, and
  the weight-drop dashboard flag under the exact requirement names
  `Rabbit dashboard with weight trend and history entry points` and
  `Rabbits with a weight-drop alert are flagged on the dashboard`. P2 adds the severe-health flag and
  the health-only / full combined history views; P5 adds the medication badge and the medications
  section. None of those are specified here.
- **`data-integrity`** — DM-1 is scoped to weight (`Weight is stored in grams as the single canonical
  unit`) and DM-4 to the observable archive behaviour (`An archived rabbit is read-only everywhere`).
  Both reference P0's NFR-10 requirement `Household-scoped isolation and canonical storage at the
  boundary` rather than restating it.
- **`weight-tracking`** — WT-3 is scoped to weight *entry* behaviour (preferred unit, no unit picker,
  regional default, interpreted to grams on save). The preference itself stays P0's SET-4
  `Display weight unit is per-device and applied at the boundary`.
- **RAB-8** — the read-only clauses are expressed fully; its export clause is realised by EXP-1 …
  EXP-4 (P4) and its photo-gallery clause by PHOTO-3 (P2) and is not implemented here.
- **RAB-1** — the photo *field* and the thumbnail are P1; choosing a photo from the device's photo
  library is PHOTO-2 (P2), so this change alters no capability, CSP or scope.

### Settled decisions this change honours

- **D1 — connectivity-required saves.** Weight and rabbit writes go through P0's reachability-gated
  save path. There is no local write queue, no optimistic write and no reconciliation. A refused
  write is surfaced with its cause and retried by the person.
- **D5 — no chart baseline / healthy-range band.** The weight chart has ranges and the drop alert,
  and deliberately no target or healthy-range band, no per-rabbit baseline and no legend for one.
  IMP-4 is closed and re-adding it is a new change, not a deferred task.
- **D2 — owner-dependent sharing.** P1 adds no ownership transfer and nothing that would require one.
- **D6 — technology-agnostic specs.** No vendor or integration is named.
- **D3 (unified reminders) and D4 (fresh start)** do not affect P1; they are recorded in the changes
  that own them.

## Impact

- **New Rust surface:** `src-tauri/src/ipc/` (P1 commands, events, shared types), the rabbit and
  weight domain, the dashboard read model, and the boundary rule that archived data is immutable.
- **New frontend surface:** `src/routes/` (dashboard, rabbit creation, rabbit profile, archived list,
  rabbit dashboard, weight form, weight history, weight detail),
  `src/features/rabbits/`, `src/features/weight/`, `src/features/dashboard/`, and regenerated
  `src/bindings.ts` (never hand-edited).
- **Security surface:** unchanged. P1 adds no new capability, CSP entry, scope or entitlement — in
  particular no photo-library access, which arrives with PHOTO-2 in P2.
- **Dependencies added:** none. P1 uses the mechanism P0 selected and justified.
- **Data written:** rabbit profiles and weight entries into the household's shared collection through
  P0's boundary; the display unit stays a per-device preference outside the collection.
- **Risk carried forward:** the drop alert is derived and therefore truthful only if it is computed
  from the same canonical grams that the trend uses (see `design.md`, R2).
- **Not in this change:** health logging, photos (attachment, gallery, pager supply), medication,
  reminders, export, invitations and member management, language override, read-path performance
  work, and any data import.

## Dependencies

**Predecessor: `add-shared-household-foundation` (P0), which MUST be archived before this change.**
P1 depends on P0 for the storage boundary and household collection identity, the reachability-gated
save path and the single cause-carrying failure surface, the loaded-data read path, the per-device
display weight unit (SET-4) and the absolute-instant timestamp rule (DM-6). P1 extends the P0
contract and does not reopen any P0 requirement.

| Phase | change-id | Depends on P1 for |
|---|---|---|
| P2 | `add-health-and-photo-history` | Rabbit identity and the interleaved timeline position, weight entry detail as the pattern for entry detail views, the archived read-only boundary, the entry-photo pager slot in the weight detail |
| P3 | `add-household-sharing-hardening` | The rabbit and weight data that members share, and the archived/immutable rules their flows must respect |
| P4 | `add-vet-export-and-reminders` | The weight history and its canonical grams to export, the read-only archived timeline whose export action it supplies, the weigh-reminder target (a rabbit) |
| P5 | `add-medication-tracking` | The rabbit dashboard the medications section extends, and the archived read-only boundary for medication records |
| P6 | `add-read-path-performance` | The dashboard, trend and history queries whose read path it optimises |

**Archive order.** Because each later change modifies capability specs opened here, archives MUST run
strictly in order **P0 → P1 → P2 → P3 → P5 → P4 → P6**.
Archiving out of order, or in parallel, applies deltas against text that does not yet exist.

**Open items.** The three former P0 questions consumed here — allowance observability, bounded-time
values and retry idempotency — are **settled by P0** as decisions **D0.12/D0.13/D0.14** and are
consumed through the P0 contract as fixed behaviour: `SaveCause.allowance_exceeded` is reachable,
because D0.12 makes distinct allowance reporting a hard selection criterion for the shared service (P0
task 1.1); the bounds are D0.13's named constants — a 15 s save attempt, a 5 s reachability probe and
"offline" only after 3 consecutive failures; and every state-changing command carries a
client-generated `request_id` deduplicated by the service (D0.14), so a retry is idempotent by
construction. This change's own open items are recorded in `questions.md`; none remain open in P1 —
this change's former Q1–Q3 are settled as D1.12 … D1.14.
