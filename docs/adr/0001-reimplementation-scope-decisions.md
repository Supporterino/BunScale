# ADR 0001 — Reimplementation scope decisions

- **Status:** Accepted
- **Date:** 2026-10-15
- **Sources:** `docs/BunScale-Reimplementation-Requirements.md` (§10), `docs/BunScale-Reimplementation-User-Stories.md` (§13)
- **Deciders:** Product owner (D1–D5, D6 framing); architect (D6 follow-up)

## Context

BunScale is being reimplemented from scratch as a Tauri v2 app: mobile first, iOS primary,
macOS Phase 1, Rust backend with a React + TypeScript frontend.

The two source requirement documents deliberately leave five decisions open (requirements
§10 / user stories §13) and leave one architectural gap: the "shared service" that provides
multi-person data sharing is deliberately unnamed, so capability specs name no vendor or
integration.

The product owner has resolved the five open decisions and fixed the framing of the sixth.
This ADR records those resolutions so specs, contracts and tasks can be written against a
closed scope.

## Decision

### D1 — Write model: connectivity-required saves

SYNC-2, SYNC-3, NFR-1 and NFR-8 stand as written. Saving new or modified data requires a
reachable shared service and a valid account. A failed save is never silently discarded or
faked: the user is told the specific cause (connectivity unavailable, no account, storage
allowance exceeded, temporarily rate limited) and offered a retry. Reads are served from
previously loaded data, refreshed when the service is reachable. Offline dose logging
(US-MED-23, tracking IMP-1) is dropped.

- **Consequences:** no local write queue, no reconciliation or conflict resolution, no
  background sync engine. IMP-1 is closed in favour of connectivity-required saves, and the
  medication capability must be re-read as consistent with SYNC-2: dose logging is a save
  like any other.
- **Rejected:** true offline-first with local retention and later reconciliation — the
  biggest architectural fork, and contrary to SYNC-2/SYNC-3 and NFR-1/NFR-8 as written.

### D2 — Sharing model: owner-dependent sharing is kept

One shared collection per household, owned by the account that created it, is kept. There is
no ownership transfer and no backup/restore flow. HS-13 remains an accepted risk; IMP-8
stays an uncommitted backlog item.

- **Consequences:** NG-3 stays a non-goal — if the creating account disappears, other
  members may lose access and the product does not mitigate this. No transfer UI, no restore
  flow, no extra contract surface for either.
- **Rejected:** designing ownership transfer or backup/restore into the P0 sharing
  foundation (IMP-8).

### D3 — Reminders: one unified mechanism

A single scheduling and follow-up mechanism serves both weigh reminders and medication slot
reminders. This resolves IMP-5.

- **Consequences:** one reminder domain, one persistence shape, one set of IPC commands and
  events, one follow-up model; per-kind behaviour is configuration, not a subsystem.
- **Rejected:** two independent reminder subsystems with separate follow-up logic.

### D4 — Migration: fresh start

A reimplementation starts empty. No import of existing users' data; IMP-7 stays a backlog
item.

- **Consequences:** no import/export-for-migration pipeline, no legacy schema mapping, no
  migration acceptance criteria. Vet-handoff export (G-6) is unaffected — an export feature,
  not a migration path.
- **Rejected:** defining a data migration / export-import path for existing users.

### D5 — Chart baseline / healthy-range band: out

The optional target/healthy-range band on the weight chart is dropped explicitly; IMP-4 is
closed.

- **Consequences:** no per-rabbit baseline value, no band rendering, no legend for it.
  Re-adding it later is a new change, not a deferred task.
- **Rejected:** re-including the band with a per-rabbit baseline.

### D6 — Shared service: mechanism deferred, capability specs stay agnostic

Capability specs stay technology-agnostic: they keep referring to "the shared service" and
never name a vendor. The concrete mechanism is **not yet chosen**; the architect must decide
it inside the P0 change's `design.md` at
`openspec/changes/add-shared-household-foundation/`.

- **Consequences:** the P0 design carries an explicit open architectural question and no
  capability spec may pre-empt it. Until it is resolved, no shared-service dependency is
  added and no IPC contract assumes one implementation.
- **Rejected:** naming the mechanism now, in this ADR.

## Consequences

Cross-cutting effects of D1–D6:

- Scope is closed for EP-2 and EP-7: no offline write path, no second reminder engine.
- The contract surface carries no sync, queue, conflict-resolution or transfer commands.
- IMP-8 (Could) and IMP-7 (Should) stay uncommitted; IMP-1, IMP-4 and IMP-5 are closed here.
- Accepted risk recorded: HS-13 (owner-dependent access).
- D6 remains open and blocks any task that adds a shared-service dependency.
- Each of D1–D6 needs a corresponding delta spec; this ADR is not itself a spec.

### Open follow-up required from the P0 `design.md`

`openspec/changes/add-shared-household-foundation/design.md` must choose the shared service
and record why. It has to satisfy:

1. **Platform support** — works on iOS and macOS; if a Tauri plugin or native module is used,
   verify its iOS and macOS availability before proposing it.
2. **Identity** — it introduces an account/identity requirement (NFR-1: saving needs a valid
   account; HS-5: invitations; SYNC-7: source of truth, recovers a household on reinstall).
3. **Storage accounting** — shared content counts against the household owner's storage
   allowance (HS-12), so per-owner quotas must be expressible.
4. **Failure taxonomy** — it must distinguish and report the SYNC-2 causes: connectivity
   unavailable, no account, storage allowance exceeded, temporarily rate limited.
5. **Privacy** — health/weight data goes only to the product's own shared service; no
   third-party analytics (NFR-3).

No vendor is named or recommended here; that decision is explicitly deferred to that design.

## Alternatives considered

| Alternative | Outcome |
|---|---|
| Offline-first writes with reconciliation (D1) | Rejected — contradicts SYNC-2/SYNC-3, NFR-1/NFR-8 |
| Ownership transfer / backup-restore (D2) | Rejected — HS-13 accepted as risk; IMP-8 stays backlog |
| Two reminder subsystems (D3) | Rejected — duplicate follow-up logic |
| Migration / import path for existing users (D4) | Rejected — fresh start; IMP-7 stays backlog |
| Keep the chart baseline band (D5) | Rejected — dropped explicitly; IMP-4 closed |
| Choose the shared service now (D6) | Rejected — deferred to the P0 `design.md` |
