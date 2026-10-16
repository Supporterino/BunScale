# add-household-sharing-hardening

> Phase **P3 — Sharing hardening** of the BunScale reimplementation.
> Source of truth: `docs/BunScale-Reimplementation-Requirements.md` and
> `docs/BunScale-Reimplementation-User-Stories.md` (EP-1, EP-2, EP-12).
> This change owns requirement IDs **HS-5, HS-6, HS-7, HS-9, HS-10, HS-11, HS-12, SYNC-6, SYNC-7,
> NFR-6** and no others.
> **HS-13 carries no requirement here** — under decision D2 there is no ownership-transfer flow, so
> HS-13 is recorded as a carried risk in `design.md` instead of being restated as behaviour.

## Why

BunScale's product promise is that several people with their own accounts edit **one** authoritative
record of care. Historically that promise is exactly what broke: IMP-2 records recurring multi-member
failures — **rabbits disappearing, invitations not acceptable, and the owner missing from the member
list**. P0 built the foundation those failures sit on (one logical collection per household,
connectivity-required saves, loaded-data reads, last-write-wins), but P0 deliberately ships no sharing
flow: no invitation, no member management, no household deletion, no propagation.

P3 is the phase that makes the promise observable end to end. Its subject is not new feature surface
but **convergence**: two identities, one household, one set of data, and a user-visible outcome for
every path that can fail. Convergence is therefore stated as the primary risk of this phase (R1), and
every requirement here is written so that it can be tested by taking two accounts through the same
household rather than by inspecting internals.

Three requirements in this phase close the historical failures directly:

- **invitations become acceptable** (HS-5, HS-6): availability is verified before an attempt, the
  existing invitation is reused instead of duplicated, a stalled attempt resolves with a stated cause
  within a bounded time, and accepting on a fresh setup lands the recipient on the shared dashboard
  without a restart and without being dumped back into onboarding;
- **the owner is always present in the member list** (HS-9): the current person is listed locally
  before any participant refresh, with a localized "You"-style label and a fallback chain, and a failed
  refresh keeps the locally known members instead of blanking the list;
- **changes actually reach the others** (HS-11, SYNC-6, SYNC-7): propagation without a manual refresh,
  live updates for a household created or joined mid-session, background delivery, and recovery of the
  household when the product is set up again on the same account.

The remaining requirements close the destructive and accounting edges of sharing: the six-member cap
(HS-7), owner-gated irreversible household deletion with its full cascade (HS-10), storage attribution
to the owner's allowance (HS-12), and honest service status so that a single blip never claims the
product is offline (NFR-6).

**Why now:** P0's `design.md` records that "offline" may only be shown after a *fresh* outcome and leaves
the repeated-failure rule to this phase (NFR-6); that rule is now settled — three consecutive failures —
and is consumed here rather than re-derived, as are the bounded times. P0 also records the owner field
precisely so that P3 can gate deletion on it (HS-10). P1 and P2 add equal read/write access to the
household and photo/local-copy semantics that this phase's acceptance and deletion paths traverse.
Hardening sharing before medications and export avoids hardening the flows twice.

## What Changes

Specs stay technology-agnostic (D6): "the shared service" and "the account" are the only names used for
the sharing mechanism.

| Capability | Requirements added | Substance |
|---|---|---|
| `household-sharing` | HS-5, HS-6, HS-7, HS-9, HS-10, HS-11, HS-12 | Invitation creation with reuse and bounded-time resolution; invitation acceptance that adopts the household without a restart or an onboarding fallback; the six-member cap; a member list that always includes the current person and survives refresh failures; owner-gated irreversible deletion with cascade, access withdrawal and post-deletion landing; propagation to every member; shared content charged to the owner's allowance |
| `sync-and-offline` | SYNC-6, SYNC-7 | Live updates for a household created or joined mid-session, including via an accepted invitation; recovery of a household's data when the product is set up again on the same account, the shared service being the source of truth |
| `product-quality` | NFR-6 | Honest service status: no "offline" from a single transient failure, offline only after three consecutive failures or a direct check, no carried-over status shown as current, one stable "syncing" state instead of flicker |

Supporting work: extending the P0 IPC contract with the invitation lifecycle, member-list resolution,
owner-gated deletion, live-update subscription and the bounded-time enforcement point; the native
surfaces this phase cannot avoid (the standard sharing options for an invitation and background
delivery of shared-data changes) on both iOS and macOS; the capability, entitlement, background-mode
and notification-permission changes those need; the frontend data layer for invitations, members,
deletion and status; the P3 screens (invite, joining, member list, household row with its delete
control, status indicator); and tests at unit, component and E2E level, including end-to-end runs with
two identities sharing one household.

### Settled decisions this change implements

- **D1 — connectivity-required saves (no queue, no optimistic write).** There is no local write queue
  and no offline acceptance: an invitation cannot be accepted while the shared service is unreachable,
  and a stalled invitation or acceptance **resolves with a timeout failure message** rather than
  completing later by itself, consistent with P0's `SaveCause.timeout`. Every attempt is a single
  attempt with a deadline — 15 s, the same bound a save attempt is allowed; the pre-attempt reachability
  probe has its own 5 s bound (see `design.md` D-H12).
- **D2 — owner-dependent sharing, no ownership transfer and no backup/restore.** Ownership exists only
  to gate household deletion (HS-10). **HS-13 remains an accepted risk and is not implemented**; the
  product offers no transfer flow and no restore flow.
- **D3 — one unified reminder mechanism.** Not touched here; reminders belong to
  `add-vet-export-and-reminders`.
- **D6 — technology-agnostic specs.** The concrete shared service was selected by P0's `architect`;
  this phase extends that contract and names no vendor. Where HS-12's attribution depends on how far the
  mechanism reports the owner's allowance, the open point is recorded rather than guessed (see
  `design.md` D-H7 and `questions.md` Q4).

### HS-13 — accepted risk, not a requirement

Under D2 (and non-goal NG-3) the product provides **no ownership-transition flow**: household data is
owned by the account that created it, and if that account is removed, other members may lose access.
That is the absence of a capability, so this change states no requirement and adds no scenario for it;
it adds no scenario asserting its absence either, because an absence of behaviour is not observable
end to end in a way a test could fail. HS-13 is recorded in `design.md` as **risk R9 (carried,
unmitigated)** and IMP-8 stays uncommitted backlog. Adding transfer or backup/restore later is a new
change, not a deferred task of this one.

## Impact

- **Rust surface:** `src-tauri/src/ipc/` (contract extensions and typed stubs), plus sharing/invitation,
  member-list resolution, owner-gated deletion and post-deletion election, live-update subscription and
  the background-delivery handoff into the existing sync layer, owner-attributed storage accounting, and
  the honest-status rules. Existing P0 modules (`sync/`, `settings/`, `diagnostics/`, storage boundary)
  are extended, not replaced.
- **Native surface:** the standard sharing options for an invitation link and background delivery of
  shared-data changes are platform work on **both iOS and macOS** — the two most platform-sensitive
  parts of this phase. Handled by `apple-native`, `cfg`-clean so Android and desktop targets stay
  possible.
- **Frontend surface:** `src/features/sharing/`, `src/features/members/`, extensions to
  `src/features/households/` (delete flow) and `src/features/sync/` (status rules), new/updated routes
  (invite, joining, member list, household row, status indicator), and regenerated `src/bindings.ts`
  (never hand-edited).
- **Security surface:** background modes/`BackgroundTasks`, notification permission strings and the
  related entitlements, plus any scope needed to present an invitation link — **`tauri-security` only**,
  least-privilege, keeping iOS and macOS configurations in step.
- **Dependencies added:** none beyond what P0 justified; if the share sheet or background delivery needs
  a dependency, it must be justified in one line in `design.md` and must support both iOS and macOS.
- **Risk carried forward:** sharing correctness (R1, IMP-2) is the reason this phase exists and is
  tested with two identities end to end.
- **Depends on P0:** reporting the owner's storage allowance distinctly is settled, not open — P0's
  design (D0.12) requires an exhausted owner allowance to be reportable as its **own** condition,
  distinct from a generic write error, and makes that a **hard selection criterion** for the mechanism in
  task 1.1, so `SaveCause.allowance_exceeded` is reachable by contract. What this change still asks is
  the separate, downstream attribution question — whether a **non-owner's** contribution is charged to
  the owner's allowance end to end, and how that is observed — recorded as `questions.md` Q4. The
  concrete bounded times — a 15 s attempt bound and a 5 s reachability-probe bound — and the
  consecutive-failure count for "offline" (3) are settled too: they are settled by P0's design (D0.13)
  and consumed here as configuration, not re-derived (`design.md` D-H12). Retry is likewise settled: every state-changing command carries a client-generated
  `request_id` that the shared service deduplicates on (D0.14), so retrying invitation creation or
  household deletion is idempotent by construction.
- **Not in this change:** rabbits, weight, health, medications, photos, export, reminders, language
  override, any ownership transfer or backup/restore, any local write queue, and read-path optimisation.

## Dependencies

**P0 is required.** This change extends the contract, the household/member model, the owner field, the
storage boundary, the read/write model and the failure surface that `add-shared-household-foundation`
defines; it can start only after P0's contract is frozen (P0 task 1.3) and the shared-service mechanism
is selected (P0 task 1.1). P1 (`add-rabbit-weight-core-loop`) and P2 (`add-health-and-photo-history`)
are required because HS-6's adoption path, HS-10's cascade and HS-12's accounting are stated over
rabbits, entries and photos.

**Archive order.** Because every change after P0 modifies capability specs P0 opened, archives MUST run
strictly in order:

```
P0 → P1 → P2 → P3 → P5 → P4 → P6
```

Archiving out of order (or in parallel) applies a delta against spec text that does not yet exist.
The P4/P5 ordering is resolved, not open: `add-medication-tracking` (P5) archives **before**
`add-vet-export-and-reminders` (P4), because the export consumes the medication model while medication
reminders consume the reminder mechanism. P6 stays last because it optimises the read path this phase
traverses.
