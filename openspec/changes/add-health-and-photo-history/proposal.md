# add-health-and-photo-history

> Phase **P2 — Health & evidence** of the BunScale reimplementation.
> Source of truth: `docs/BunScale-Reimplementation-Requirements.md` and
> `docs/BunScale-Reimplementation-User-Stories.md`.
> Decision record: `docs/adr/0001-reimplementation-scope-decisions.md` (D1–D6).
> This change owns the requirement IDs listed under "What changes" and no others.

## Why

BunScale exists because rabbits hide illness until it is advanced: a gradual weight decline or a
cluster of subtle symptoms is the earliest signal, and prose notes never become a pattern anyone can
query (G-3). The weight half of that signal lands in P1; this change adds the other half — health
observations recorded **structured**, not as free text — and the **evidence** that makes an
observation trustworthy to a second person: photos attached to the entry, viewable in its detail,
and shared with the household.

Two things make P2 the right next slice:

- **Structured health logging only pays off next to weight.** Once weight entries exist, a
  full combined history can interleave weight and health by timestamp, so a symptom cluster can be
  read against the weight curve. Without interleaving, the two capabilities stay two separate
  notebooks (HL-3, DASH-7).
- **Photos attach to both entry kinds.** A weight entry and a health entry are the only two places a
  photo is meaningful today; building the photo boundary once, right after weight entries exist,
  avoids retrofitting it later (PHOTO-1 … PHOTO-5).

The evidence surface must also stay honest when connectivity is not there: a photographed wound is
exactly the thing a person wants to re-open at the vet's, so a photo that was seen once has to remain
viewable offline, and one that was never seen has to say why it cannot load rather than pretending
the entry has no photos (SYNC-4).

**Why here:** P0 fixes the write model, the collection identity, the failure surface and the timestamp
rule; P1 adds rabbits, weight entries and the rabbit dashboard. Health entries and photos are then a
straight extension of the same contract and the same storage boundary.

## What changes

| Capability | Requirements added | Substance |
|---|---|---|
| `health-logging` | HL-1, HL-2, HL-4, HL-5, HL-7 | A health entry of zero-or-more canonical category tags, a severity, an optional note, an editable timestamp and optional photos; a tagless entry is savable; health entries filterable by tag, severity and date range; editable and deletable by any member; a detail view showing severity, all tags, the full untruncated note, the timestamp and the entry's photos, read-only for an archived rabbit |
| `photos` | PHOTO-1 … PHOTO-5 | Photos attachable to a weight entry or a health entry; a rabbit profile photo used as its thumbnail across the product; a per-rabbit gallery ordered by entry date; photos shared through the household's shared collection; an entry's photos shown as a swipeable full-width pager in its detail view |
| `dashboard` | DASH-4 (severe-health clause), DASH-7 (health part), HL-3 | Two requirements: a rabbit with a Severe health entry within the last 7 days is flagged on the dashboard; a health-only history view and a full combined history view that interleaves weight and health entries by timestamp |
| `sync-and-offline` | SYNC-4 | A photo is available locally after it has been displayed once; a never-displayed photo while unreachable renders a "cannot load" placeholder |
| `data-integrity` | DM-2 | The health tag catalog: exactly one copy of the canonical categories per household, established idempotently, never referenced by another household's entries |

**Stated once, not twice.** The weight-and-health interleaving (HL-3) is stated in the `dashboard`
requirement `Health-only and full combined history views`. The household-scoped tag catalog (HL-6) is
stated in the `data-integrity` requirement `A health tag catalog belongs to exactly one household`.
Neither is restated in `health-logging`.

Supporting work: the P2 extension of the P0 IPC contract with typed stubs, the health domain (entries,
severities, canonical categories, the household catalog), the interleaved history read, the photo
storage/local-copy boundary, the derived dashboard health flag, the frontend data layer (bindings
wrappers, query keys, filter and history state, photo availability state), the P2 screens (health-entry
form, health history with filters, combined history, health-entry detail with photo pager, per-rabbit
gallery, photo attach on both entry forms), the platform photo-source access on iOS and macOS, and
tests at Rust, component and E2E level.

### Settled decisions this change implements

- **D1 — Write model: connectivity-required saves.** Creating, editing or deleting a health entry,
  establishing the tag catalog, and adding or removing a photo are saves like any other. They require
  the shared service to be reachable and never pretend to succeed. There is no local write queue and
  no optimistic write. The only thing kept for offline use is a **read** copy of a photo that was
  already displayed (SYNC-4), which is the same shape as P0's loaded-data read copy.
- **D4 — Migration: fresh start.** No import path exists, so a household's tag catalog is populated
  fresh for that household rather than migrated. Existing local data is never mapped.
- **D5 — Chart baseline / healthy-range band: out.** The combined history view adds no baseline band
  and no target range to any weight presentation.
- **D6 — Technology-agnostic specs.** The capability specs name only "the shared service", "the
  account" and "the household's shared collection". No vendor, storage provider or picker product is
  named in a spec.
- **D2 — Owner-dependent sharing** is unaffected by P2 beyond obeying it: shared photos count against
  the household owner's storage allowance (HS-12, P3), so a photo save can fail with the allowance
  cause P0 already defines.
- **D3 — Reminders** do not appear in P2.

## Impact

- **New Rust surface:** `src-tauri/src/ipc/` (health, catalog, history and photo commands, events and
  shared types), `src-tauri/src/domain/health/`, `src-tauri/src/domain/history/`,
  `src-tauri/src/photos/` (photo owner, upload/save path, local-copy boundary, availability state),
  the derived dashboard health flag.
- **New frontend surface:** `src/routes/` (health-entry form, health-entry detail, health-only history,
  combined history, per-rabbit gallery), `src/features/health/`, `src/features/photos/`,
  `src/features/history/`, and the regenerated `src/bindings.ts` (regenerated, never hand-edited).
- **Security surface:** photo-library access introduces a permission and a scope on both Phase 1
  platforms — an iOS photo-library permission string and a macOS photo-source scope — declared only by
  `tauri-security`.
- **Dependencies added:** a platform photo-source bridge for iOS (PHPicker / PhotosUI) and macOS
  (Photos library / file source), justified in `design.md` and required to support both platforms.
- **Risk carried forward:** the local photo copy and the SYNC-4 offline guarantee (retention is an
  open question), and tag-catalog duplication if two members establish the catalog independently.
- **Not in this change:** weight entries and the weight-drop dashboard flag (P1), invitations, member
  management and household deletion (P3), export and reminders (P4), medications and their badge (P5),
  read-path performance tuning (P6), and any migration or import of existing data (D4).
- **Screenshots:** every P2 UI task captures 390×844 and 1280×800 into `.artifacts/screens/` for
  review.

## Dependencies

This change **requires P0 and P1 archived first**. It extends the P0 contract (`CommandError`,
`SaveCause`, `Instant`, the household collection and its reachability gate), the P0 capability specs
`sync-and-offline` and `data-integrity`, and the P1 capability specs `dashboard`, `rabbit-profiles` and
`weight-tracking` (weight entries, the archived-rabbit read-only gate, the rabbit profile record and
the rabbit dashboard's History section).

| Phase | change-id | P2 depends on it for |
|---|---|---|
| P0 | `add-shared-household-foundation` | collection identity, household scoping, save/read semantics, `CommandError`/`SaveCause`, absolute-instant timestamps |
| P1 | `add-rabbit-weight-core-loop` | rabbits, weight entries, the rabbit dashboard and its History section, the archived read-only gate, the local copy the offline photo path reuses |
| P3 | `add-household-sharing-hardening` | arrives later; P2 only shares photos through the household collection it assumes |
| P4 / P5 | `add-vet-export-and-reminders` / `add-medication-tracking` | later in the resolved sequence P3 → P5 → P4 → P6; P2 does not depend on either |
| P6 | `add-read-path-performance` | later; P2 must not break the boundaries P6 improves |

**Archive order.** Because P2's deltas apply against capability specs that P0 and P1 open, archives
MUST run strictly in order **P0 → P1 → P2 → P3 → P5 → P4 → P6**. Archiving P2
before P1 leaves the `dashboard` spec with health requirements whose weight counterparts do not yet
exist.
