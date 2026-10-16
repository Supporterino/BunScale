# Design — add-household-sharing-hardening (P3)

> Status of this document: **proposal**. The IPC contract section below is owned by `architect`
> (tasks 1.1–1.2) and is not binding until task 1.1 is reviewed and `bun run gen:bindings`
> regenerates `src/bindings.ts` with no diff.

## Context

P0 (`add-shared-household-foundation`) established the shapes every phase inherits: the IPC contract
and typed bindings, the storage boundary with household isolation and canonical representation, the
connectivity-required write model (D1), the loaded-data read model, last-write-wins convergence, the
single failure surface with cause codes, and the household/member model **including the owner member**
(P0 design D0.3, added precisely so this phase can gate deletion on it).

P3 adds the sharing flows that sit on that foundation. It is the phase that must make multi-person
correctness observable, because IMP-2 records the historical failures there: rabbits disappearing,
invitations not acceptable, the owner missing from the member list.

Platforms: iOS (primary) and macOS (Phase 1). Baselines 390×844 first, 1280×800 second. Android,
Windows and Linux are later, so all new code — Rust, frontend and native — stays `cfg`-clean. This
phase is the most platform-sensitive of the reimplementation because two of its requirements depend on
platform behaviour: the standard sharing options (HS-5) and background delivery (HS-11).

Stack: Tauri v2, Rust in `src-tauri/`, React + TypeScript + Vite in `src/`, TanStack Router + Query,
generated bindings in `src/bindings.ts` (never hand-edited), bun as package manager.

## Decisions

### D-H1 — One bounded-time, single-attempt rule for sharing operations (implementation of D1)

Invitation creation and invitation acceptance are each a **single attempt with a deadline**. The
attempt resolves as success or as a failure carrying `SaveCause.timeout`; nothing is queued, nothing is
retried in the background, and a stalled attempt never completes on its own after the deadline. This is
D1 applied to sharing: no local write queue, no optimistic acceptance, no reconciliation, and therefore
**no offline acceptance**.

Consequences accepted: a person cannot invite or accept while the shared service is unreachable; the
alternative — queueing an acceptance — would create exactly the ambiguous state D1 was chosen to
eliminate (an invitation that "half happened"). The bounds are per-device configuration values —
`SharingPolicy.attempt_timeout_ms` for a state-changing attempt and
`SharingPolicy.reachability_timeout_ms` for the pre-attempt reachability probe — so changing them is a
configuration change, not a redesign. Their values are **owned by P0's design and consumed here, not
re-derived** (D-H12).

### D-H2 — One invitation per household, resolved by reuse

A household has at most one invitation. Creation resolves in this order: (1) availability is verified
before any attempt (HS-5); (2) if an invitation for the household already exists, it is reused and its
standard sharing options are presented, with no second invitation created (HS-5); (3) otherwise a new
invitation is requested. This makes re-inviting idempotent in the user-visible sense that killed the
historical "already shared" failure (US-HS-07).

Reuse is decided on the product's own record of the household's invitation, then confirmed against the
shared service; a create that reached the service but whose outcome never returned is a duplicate risk
that is eliminated by construction rather than tolerated: every state-changing command carries a
client-generated `request_id` that the shared service deduplicates on (settled decision, D-H12).
Invitation creation therefore carries one, and a retry re-invokes the same command with the same
`request_id` instead of a naive second attempt.

### D-H3 — Acceptance is an adoption, not an onboarding branch

Acceptance has three obligations that must hold together: the recipient becomes a member with
read/write access **even on a fresh setup with no household** (HS-6); the adopted household's dashboard
appears within the bounded time with **no further action** by the recipient; and the product **never
falls back to the creation onboarding merely because the data had not yet arrived** (HS-6).

The design consequence is that "the person holds no household" and "the person is in an unfinished
acceptance" must be distinguishable states. Acceptance therefore records an **adoption in progress**
before any household record exists locally, and the route guard treats a pending adoption as "do not
show onboarding". A household the person creates while an invitation is still resolving wins: it
becomes and stays the active household, and the arrival of the invited household's data does not
displace it (HS-6). An acceptance that cannot complete within the bound resolves as a timeout failure
and leaves no pending adoption behind (D-H1).

Nothing delivered is discarded without a trace: a non-completed acceptance writes a redacted diagnostic
record — identifiers, counts and outcomes only, no names or content (NFR-3, HS-6).

### D-H4 — Member-list resolution is local-first and never blank

The member list is built as: the current person **from local data, unconditionally** → locally known
members for the household → remote participants as they resolve. The current person is therefore always
present, even before any participant resolves and even when a refresh fails, which is the direct fix
for "the owner missing from the member list" (IMP-2, HS-9).

The display-name chain is account display name → locally known member name → localized
"You"-style/placeholder label (HS-9). The label is resolved through an i18n key so no user-facing string
is hard-coded (NFR-5 alignment; the full German/English pass stays in P4). A failed participant refresh
keeps the locally known list and surfaces the cause through the one failure surface (P0 D0.6, NFR-8)
rather than replacing the list with an error or an empty state.

### D-H5 — Ownership gates exactly one operation, and that operation cascades in one step

Ownership's only use is gating household deletion (HS-10); there are no permission tiers (HS-8 is P1's
requirement and this phase does not weaken it — a member has the same read/write access as the owner by
design). The owner gate is enforced **at the storage boundary**, not in the UI: a deletion request from
a non-owner is refused with a cause, so hiding the control is presentation, not protection.

Deletion is one operation that removes the household and its rabbits, weight and health entries, photos,
medications, doses, members and tags, and withdraws access from every member (HS-10). It is irreversible
by design: **no undo, no trash, no restore path** — the product provides no flow that recovers a deleted
household (in step with D2/D4). The post-deletion landing is deterministic: the newest remaining
household becomes active, or onboarding is presented when none remains (HS-10, and consistent with DM-3
as P0 stated it).

### D-H6 — Live updates are bound to the household, and background delivery is a native handoff

Live updates are a per-household subscription in the Rust sync layer, not a global stream. The
subscription is established or re-established whenever a household becomes available to the person —
created, joined by acceptance (SYNC-6), or loaded — which is the "without a restart" clause of both
SYNC-6 and HS-11.

Background delivery is deliberately split so that platform code stays thin and `cfg`-clean:

```
native layer (Swift, cfg ios / cfg macos)   →  delivery of a change while the product is not open
rust sync layer (src-tauri/src/sync/)       →  applies it to the loaded collection, emits the change event
frontend (TanStack Query)                   →  invalidates only the affected household queries
```

The skill `testing-strategy` records that native lifecycle and plugin behaviour cannot be driven by
`tauri-driver` on iOS/macOS, so background delivery carries a manual simulator verification task (5.15)
in addition to Rust-level tests of the handoff (5.6). The presentation question — whether a delivered
change may raise a user-visible notification or must be silent until the product is next shown — is a
genuine unknown recorded as `questions.md` Q2; HS-11 says delivery MAY happen in the background and does
not decide this.

### D-H7 — Storage is attributed to the owner, and that attribution must be observable end to end

HS-12 charges shared content to the **owner's** allowance, whichever member saved it. Two parts are
distinct and must not be conflated:

1. **Attribution** — the product attributes a saved item's consumption to the household owner. This is
   product behaviour and is stated observably (HS-12 scenario 1).
2. **Reportability of allowance exhaustion** — the shared service reporting an exhausted owner allowance
   as its **own** condition, distinct from a generic write error. This is **settled upstream, not open**:
   P0's D0.12 makes it a hard selection criterion for the shared-service mechanism (task 1.1), so a
   candidate that cannot is rejected and `SaveCause.allowance_exceeded` is reachable by contract. It is a
   precondition of mapping an exhausted allowance to the storage-allowance cause (HS-12 scenario 2,
   SYNC-2) and is therefore guaranteed rather than hoped for.

What remains open is downstream of both: whether a contribution made by a **non-owner** is attributed by
the service to the owner's allowance end to end, and how that attribution is observed. That is this
change's own question and is recorded as `questions.md` Q4 and as risk R8 rather than papered over with
a generic message (NFR-8).

### D-H8 — Status is derived from fresh outcomes only

`ServiceStatus.state` starts as `unknown` (P0) and stays there until a fresh outcome exists. The rules
implemented here are exactly NFR-6: a single transient failure does not present "offline"; "offline"
requires three consecutive failures **or** a direct check that the service or account is unavailable; a
status carried over from a previous session is never presented as current; and rapid consecutive
updates collapse into one stable "syncing" state.

The threshold is `SharingPolicy.offline_failure_threshold`, settled at **three consecutive failures**
and owned by P0's design (D-H12); the carried-over-state rule stays `observed_at`-based and is
unaffected by the count. "Syncing" is a presentation state with a single owner (the status store) so
that two in-flight updates cannot produce two different visible states.

### D-H9 — Set-up-again recovery reads the shared service as the source of truth

SYNC-7 is the read side of D2: the device holds a cache, the household collection is the record. On a
start on the same account with connectivity, recovery resolves the household from the shared service
and presents it, without an empty-household or onboarding state standing in for recoverable data. With
nothing ever loaded and the service unreachable, the explicit connectivity-required state of P0 SYNC-3
applies — recovery does not invent data (D1). No import, no migration, no local backup: D4 stands.

### D-H10 — Why HS-13 adds no requirement

D2 keeps owner-dependent sharing with **no ownership transfer and no backup/restore**. HS-13 is the
statement of that dependency as an accepted risk (NG-3). It is not behaviour anyone can perform, so it
cannot be expressed as a scenario with an observable outcome, and this change does not invent one. It is
carried as risk R9, unmitigated, and IMP-8 stays uncommitted backlog. If the product later wants a
transfer or restore path, that is a new change with its own requirements — not a deferred task here.

### D-H11 — Scope discipline: what this phase deliberately does not touch

No local write queue and no optimistic write (D1). No ownership transfer or restore flow (D2). No
second reminder mechanism (D3). No data import or migration (D4). No read-path optimisation: this phase
uses P0's loaded-data boundaries as they are, and only invalidates the affected queries (NFR-7 belongs
to P6). No language-override work (SET-3 belongs to P4). No export (P4), no medications (P5).

### D-H12 — The bounded times, the failure threshold and retry idempotency are consumed, not re-derived

These are settled project decisions. They are **owned by P0's `design.md`**
(`add-shared-household-foundation`) and consumed here as values — none of them is re-derived, guessed
or modelled as an open policy. This change records them in one place so its requirements, scenarios,
contract and tasks agree on one number each.

| Constant | Settled value | What it bounds | Where this change consumes it |
|---|---|---|---|
| Attempt bound | **15 s** | one state-changing attempt with a deadline — an invitation create or accept, at the same bound a save attempt is allowed | HS-5's stalled invitation resolves at this bound (no separate sharing number); HS-6's acceptance timeout; `SharingPolicy.attempt_timeout_ms` |
| Reachability-probe bound | **5 s** | a reachability probe of the shared service, including the pre-attempt check | HS-5's pre-attempt verification via `check_service_reachability()`; `SharingPolicy.reachability_timeout_ms` |
| Offline failure threshold | **3 consecutive failures** | how many consecutive failures precede presenting the service as offline | NFR-6; `ServiceStatus.consecutive_failures`; `SharingPolicy.offline_failure_threshold` |

Two clauses are independent of the count and are not weakened by it: **a single transient failure never
presents "offline"**, and **a status carried over from a previous session is never presented as
current** before a fresh outcome exists (NFR-6). A direct check that the service or the account is
unavailable presents that outcome without waiting for the threshold.

**Retry is idempotent by construction.** Every state-changing command carries a client-generated
`request_id` that the shared service deduplicates on. The retryable operations in this phase are
**invitation creation** (HS-5) and **household deletion** (HS-10): a retry re-invokes the same command
with the same `request_id`, so it cannot produce a second invitation or a second deletion. Retry stays
a re-invocation of the original command rather than a separate retry command, and it does not make a
stalled attempt complete on its own (D-H1).

## State and boundaries

```
Rust (src-tauri/)                                   Frontend (src/)
  src-tauri/src/ipc/            contract: extended commands, events, shared types
                                                        → src/bindings.ts (generated)
  src-tauri/src/sharing/        invitation lifecycle: availability check, create,
                                reuse, cap check, bounded attempt, acceptance/adoption
  src-tauri/src/members/        member-list resolution, display-name fallback chain
  src-tauri/src/domain/         owner gate, household deletion cascade, post-deletion
                                election (extends P0's household domain)
  src-tauri/src/sync/           live-update subscription per household, background
                                delivery handoff, set-up-again recovery (extends P0)
  src-tauri/src/storage/        only extended: the deletion cascade runs through it, and
                                a deleted household stops being readable
  src-tauri/src/settings/       sharing policy values (attempt and probe bounds,
                                failure threshold)
  src-tauri/src/diagnostics/    non-completed acceptance records (identifiers only)
  src-tauri/plugins/            Swift: standard sharing options, background delivery
  src-tauri/capabilities/       background modes, notification permissions, entitlements
                                                        → src/features/sharing/{api.ts,hooks}
                                                        → src/features/members/{api.ts,hooks}
                                                        → src/features/households/ (delete flow)
                                                        → src/features/sync/ (status store rules)
                                                        → src/routes/ (invite, joining, settings rows, status)
```

Rules held here:

- The frontend never calls `invoke`; only `src/features/*/api.ts` wrappers over generated bindings
  (project hard rule 2).
- The household collection remains the only source of truth for household data; the local copy is a
  cache. Acceptance and deletion change the **set of households the person holds**, never the record's
  ownership semantics.
- Sharing policy values (the attempt bound, the reachability-probe bound and the offline threshold) are
  per-device configuration, not household data, exactly like P0's display unit. They must not enter the
  household collection.
- No sharing operation writes locally before the service outcome is known (D1), with the single
  exception of the transient "adoption in progress" marker, which is a local UI-state record and is
  cleared when the attempt resolves.
- Per-platform behaviour is tagged in the spec (`[ios]`, `[macos]`) and gated in code with `cfg`
  (Rust/Swift) or the platform helper (TS).

## IPC contract — to be defined by architect

**Status: proposal only.** `architect` owns this section and the typed stubs in `src-tauri/src/ipc/`
(tasks 1.1–1.2). Nothing may be implemented against these names until 1.1 lands and `bun run
gen:bindings` regenerates `src/bindings.ts`. Naming follows P0's style: `<verb>_<noun>` commands,
`domain:event` events, `snake_case` fields.

### Commands the frontend needs in this phase

| Command (proposed) | Purpose | Requirements |
|---|---|---|
| `create_invitation(household_id)` | Availability-checked invitation creation; reuses the household's existing invitation instead of creating a second; resolves within the attempt bound (15 s) or fails with a cause; retryable — a retry re-invokes it with the same `request_id` | HS-5, HS-7 |
| `get_invitation(household_id)` | The household's existing invitation (or none), so a re-invite presents the same sharing options | HS-5 |
| `accept_invitation(invitation_code)` | Accept an invitation with the current account; resolves as membership or fails with a cause within the attempt bound (15 s); never queues | HS-6, SYNC-6 |
| `get_pending_adoption()` | The adoption in progress, so the route guard does not present onboarding while it is unresolved | HS-6 |
| `get_member_list(household_id)` | Local-first member list: current person first, resolved participants merged in, plus whether participants are resolved and a cause when a refresh failed | HS-9 |
| `delete_household(household_id)` | Owner-gated irreversible deletion with the full cascade and access withdrawal; returns the post-deletion active household or none; retryable — a retry re-invokes it with the same `request_id` | HS-10 |
| `subscribe_household_updates(household_id)` | Start live updates for a household created or joined mid-session | SYNC-6, HS-11 |
| `unsubscribe_household_updates(household_id)` | Stop live updates when a household stops being held | SYNC-6 |
| `get_storage_usage(household_id)` *(optional)* | Owner-attributed usage, so HS-12 is observable; omitted or degraded when the mechanism cannot report it (R8) | HS-12 |

Reused from P0 without change: `get_active_household()`, `set_active_household(household_id)`,
`list_households()`, `load_household(household_id)`, `check_service_reachability()` (used as HS-5's
pre-attempt verification, bounded by the 5 s reachability-probe bound), `get_service_status()`,
`get_identity()`, `get_diagnostics_summary()`.

Retry (HS-5, NFR-8) is a re-invocation of the failed command, not a separate retry command. Every
state-changing command carries a client-generated `request_id` that the shared service deduplicates on,
so the re-invocation reuses the same `request_id` and is idempotent by construction rather than a
second attempt (settled decision; D-H2, D-H12).

### Events the frontend needs in this phase

| Event (proposed) | Payload | Purpose |
|---|---|---|
| `invitation:changed` | `{ household_id, kind: created \| reused \| accepted \| failed }` | Drive the invite/joining surfaces from the outcome instead of polling |
| `household:members-changed` | `{ household_id, reason: refresh \| joined \| left, participants_resolved }` | Refresh the member list, keeping local entries until participants resolve |
| `household:deleted` | `{ household_id, deleted: bool, cause? }` | Move the product to the post-deletion landing on every member's device (HS-10) |
| `service:status-changed` | `ServiceStatus` | P0 event, extended with the `syncing` state and the consecutive-failure count (NFR-6) |

Reused from P0: `household:changed` with `kind: created | switched | refreshed | removed` — the
`removed` kind carries deletion propagation, and `refreshed` is what a background-delivered change
produces when the product is next foregrounded. No new event is defined for background delivery:
the delivery itself is a native-to-Rust handoff (D-H6), not an IPC event.

### Shared types (extending P0's set)

| Type (proposed) | Shape | Notes |
|---|---|---|
| `Invitation` | `{ household_id, code?, created_at, reusable: bool }` | `code` is an opaque token; never logged with content |
| `PendingAdoption` | `{ household_id, started_at, cause? }` | Backs the "never fall back to onboarding" rule (HS-6); cleared when the attempt resolves |
| `InvitationOutcome` | `{ household_id, accepted: bool, cause? }` | One failure surface shape for accept |
| `MemberView` | `{ member_id, account_user_id?, display_name?, is_current_person, is_owner, label_key? }` | `label_key` carries the localized "You"/placeholder fallback (HS-9) |
| `MemberList` | `{ household_id, members: [MemberView], participants_resolved: bool, cause? }` | `cause` set only when a participant refresh failed; `members` still populated |
| `HouseholdDeletionOutcome` | `{ household_id, deleted: bool, cascade_counts?, next_active_household_id?, cause? }` | `cascade_counts` are **counts only** — no names or content (NFR-3) |
| `SharingPolicy` | `{ attempt_timeout_ms: 15000, reachability_timeout_ms: 5000, offline_failure_threshold: 3 }` | Per-device configuration whose values are settled by P0's design and consumed here, not re-derived (D-H12) |
| `StorageUsage` *(optional)* | `{ owner_account_user_id?, used?, allowance? }` | Degrades when the mechanism cannot report allowance state (R8) |

Extended from P0: `ServiceStatus` gains `syncing` in its state set and a `consecutive_failures` count,
and keeps `observed_at` so a carried-over status can be recognised as stale rather than current.
P0's `CommandError`, `SaveCause`, `Household`, `Member`, `Identity`, `Instant` are reused unchanged;
`SaveCause.timeout` is the cause a stalled sharing attempt resolves with (D-H1).

### How the bounded times are enforced

The Rust sharing layer owns the deadlines and applies them at one place, so no bound exists twice:

- each state-changing attempt — invitation create and invitation accept — is wrapped in
  `SharingPolicy.attempt_timeout_ms` (15 s);
- the pre-attempt reachability check is wrapped in `SharingPolicy.reachability_timeout_ms` (5 s).

At a deadline the operation returns `CommandError { cause: timeout, retryable: true }` and its local
state (including any pending-adoption marker) is cleared. A retry is a fresh re-invocation carrying the
same `request_id`, never a continuation of the expired attempt. The frontend never invents its own
timeout: it renders progress until the command resolves, so each bound has exactly one source of truth
and cannot differ between platforms. Rapid sequences of sharing operations do not stack — the surface is
one attempt at a time, which is also why NFR-6's single "syncing" state is reachable.

## Traceability: user stories → scenarios

| Story | Requirement(s) | Capability / scenario |
|---|---|---|
| US-HS-06 | HS-5, HS-6, HS-11 | household-sharing: "An invitation attempt gives progress feedback", "A successful invitation presents the standard sharing options [ios]", "A successful invitation presents the standard sharing options [macos]" (HS-5), "Accepting an invitation on a fresh setup grants membership with read/write access" (HS-6), "A change made by one member appears for another without a manual refresh" (HS-11) |
| US-HS-07 | HS-5 | household-sharing: "An existing invitation is reused rather than duplicated" |
| US-HS-08 | HS-5 | household-sharing: "Inviting is refused before an attempt when the shared service is unavailable" |
| US-HS-09 | HS-5 | household-sharing: "An invitation failure states its specific cause" |
| US-HS-10 | HS-5 | household-sharing: "A stalled invitation attempt resolves with a timeout failure" |
| US-HS-11 | HS-6 | household-sharing: "Accepting an invitation on a fresh setup grants membership with read/write access", "The adopted household's dashboard appears without further action", "Acceptance does not fall back to onboarding while data is still arriving", "An acceptance that cannot complete resolves with a timeout failure" |
| US-HS-12 | HS-6 | household-sharing: "A household created while an invitation is still resolving stays active", "A delivered invitation is never discarded without a trace" |
| US-HS-13 | HS-7 | household-sharing: "An invitation beyond the member cap is blocked with an explanation", "Accepting an outstanding invitation cannot take the household beyond six members" |
| US-HS-15 | HS-9 | household-sharing: "The current person is listed with a localized You-style label" |
| US-HS-16 | HS-9 | household-sharing: "The account display name is shown when it resolves", "An unresolved display name falls back without breaking the list" |
| US-HS-17 | HS-9 | household-sharing: "A failed participant refresh keeps locally known members and states the cause" |
| US-HS-18 | HS-9 | household-sharing: "The list renders before participants resolve and refreshes once they do" |
| US-HS-19 | HS-10 | household-sharing: "The owner's delete control is directly visible on the household's Settings row", "Cancelling the confirmation deletes nothing", "Confirmed deletion removes the household and all of its data", "Confirmed deletion withdraws access from every member and reaches them" |
| US-HS-20 | HS-10 | household-sharing: "The confirmation states the full consequence" |
| US-HS-21 | HS-10 | household-sharing: "A joined household offers no delete control to a non-owner" |
| US-HS-22 | HS-10 | household-sharing: "The newest remaining household becomes active after deletion", "Onboarding is presented after the only household is deleted" |
| US-HS-23 | HS-10 | household-sharing: "A failed deletion leaves the household intact and states its cause" (NFR-8 clause reuses P0's "No bare failure messages") |
| US-HS-24 | HS-11 | household-sharing: "A change is delivered while the product is not open [ios]", "A change is delivered while the product is not open [macos]", "Every member of the household ends up with the same change" |
| US-HS-25 | HS-12 | household-sharing: "Content saved by a non-owner counts against the owner's allowance", "An exhausted owner allowance refuses a member's save with that cause" |
| US-SYNC-10 | SYNC-6 | sync-and-offline: "A household created mid-session receives live updates without a restart", "A household joined by accepting an invitation receives live updates without a restart" |
| US-SYNC-11 | SYNC-7 | sync-and-offline: "Setting the product up again on the same account recovers the household", "Recovery does not present an empty household in place of the recovered data", "Nothing is recovered without the shared service" |
| US-QUAL-04 | NFR-6 | product-quality: all five "Honest service status" scenarios |
| US-HS-14 | HS-8 (P1) | **Not here.** Listed only to make the boundary explicit: the equal-access requirement is P1's, and this phase's HS-6 scenario states membership plus read/write access without restating HS-8 |
| HS-13 | — | **No story, no scenario.** Accepted risk (D2), carried as R9 and in `design.md` D-H10 |

Stories with no P3 scenario: **none** among the requirements this change owns. Every story in the
coverage matrix rows for HS-5, HS-6, HS-7, HS-9, HS-10, HS-11, HS-12, SYNC-6, SYNC-7 and NFR-6 appears
above.

## Risks

| # | Risk | Mitigation in this change |
|---|---|---|
| **R1** | **Sharing correctness (IMP-2) — the primary risk of this phase.** Two members of one household must converge, or the product's core promise fails: rabbits disappearing, invitations not acceptable, the owner missing from the member list. | Requirements written as observable behaviour and traced to scenarios; an end-to-end test with **two identities sharing one household** (invite → accept → both see the same data → one edits → the other sees it, task 5.10); a second E2E for fresh-setup acceptance (5.11) and a third for deletion reaching the other member (5.12); the member-list local-first rule (D-H4) targeted at the exact historical failure; failure paths carry causes rather than silence (NFR-8). |
| R2 | **Duplicate invitation on retry or concurrent invite.** Two members invite at once, or a create that reached the service is retried, and the household ends up with two invitations. | One invitation per household resolved by reuse before creation (D-H2); the retry path carries the same client-generated `request_id`, so the shared service deduplicates it and the retry is idempotent by construction (task 1.1 fixes the parameter; backend task 2.1 relies on it); covered by a Rust test and an E2E re-invite case. |
| R3 | **Acceptance appears to hang, or dumps the recipient into onboarding.** The historical "invitations not acceptable" failure. | Single-attempt bounded-time rule (D-H1) with a timeout cause; the pending-adoption state separates "no household" from "acceptance unfinished" (D-H3); Rust and E2E tests for the fresh-setup path and for the timeout path. |
| R4 | **The current person is missing from the member list, or the list blanks on a failed refresh.** The third historical failure. | Local-first member-list construction (D-H4): the current person is never dependent on a remote refresh; a failed refresh keeps local members and reports the cause; component and Rust tests per scenario. |
| R5 | **Deletion cascade incomplete, or a member keeps access.** A half-deleted household is worse than an undeleted one. | Cascade runs in one operation at the storage boundary; access withdrawal is asserted separately from data removal; NFR-10's isolation is re-asserted after deletion; a non-owner request is refused (UI hiding is not the protection). |
| R6 | **Background delivery not granted by the platform.** Notification/background permissions may be denied, and macOS/iOS differ in how a change can be delivered while the product is not open — which would make the second clause of HS-11 unobservable. | Platform work is scheduled in the contract/native section (`tauri-security` task 1.3, `apple-native` tasks 1.4–1.8) before the backend work that depends on it; the foreground clause of HS-11 and the "present when next shown" clause stay testable if background delivery is unavailable; manual simulator verification (task 5.15) covers what cannot be automated; the presentation question is recorded as Q2 rather than guessed. |
| R7 | **False or stale "offline" (NFR-6).** Claiming offline after one transient failure, or showing a previous session's status as current, is a truthfulness bug — P0 carried it as R3 and deferred the rule here. | Status starts `unknown` and is derived from fresh outcomes only (D-H8); the threshold is settled at three consecutive failures and consumed, not chosen (D-H12); `observed_at` lets staleness be recognised; a single "syncing" state is owned by one store; E2E covers the transient-failure and relaunch cases. |
| R8 | **Allowance attribution not observable end to end.** Reporting an exhausted owner allowance *distinctly* is settled — D0.12 makes it a hard selection criterion at task 1.1 — so the storage-allowance cause itself is reachable. What is unconfirmed is whether content contributed by a **non-owner** is attributed by the service to the **owner's** allowance, and how that attribution is observed end to end. | Attribution itself is product behaviour and stays testable (scenario 1); the remaining gap is recorded as `questions.md` Q4 and must be stated rather than replaced with a generic failure message (NFR-8). |
| **R9** | **Owner-dependent access (HS-13, D2) — carried, unmitigated, accepted.** If the account that created a household is removed, other members may lose access. No ownership-transfer and no backup/restore exists. | **No mitigation, by decision D2/NG-3.** Recorded here and in D-H10 so it is not silently forgotten; IMP-8 stays uncommitted backlog; the product makes no promise to the contrary in any P3 surface. |
| R10 | **Scope creep** into reminders (D3), migration (D4), export, medications or read-path performance. | D-H11 and the "what changes" table in `proposal.md` bound the phase; reviewer task 6.2 verifies the delta contains exactly this phase's requirement IDs. |
| R11 | **A member's local copy survives a deleted household.** HS-10 requires access to be withdrawn; SYNC-3 serves reads from previously loaded data. A member who is offline at deletion time may keep reading the cached household until a later refresh — a genuine gap in the source documents. | Recorded as `questions.md` Q3. Until answered, the phase's scenarios assert withdrawal of access and reachability of the deletion, not the erasure of another device's cache. |
| R12 | **Dependency risk from task 1.1's mechanism selection.** The selected shared-service mechanism must actually satisfy D0.12 — reporting an exhausted owner allowance as its own condition, distinct from a generic write error — or the inherited `SaveCause.allowance_exceeded` reporting is not reachable in practice. This is a dependency on task 1.1's selection and review, not on an unanswered question: P0 has no remaining open question. | D0.12 makes the reportability a hard selection criterion at task 1.1, which rejects any candidate that cannot meet it; the separate attribution gap is recorded as R8 and `questions.md` Q4, so the phase is not blocked by it. The bounded times, the offline threshold and retry idempotency are settled and consumed as values (D-H12). |

## Verification

- Specs: `openspec validate add-household-sharing-hardening --strict`. **Passes** — re-run when the
  settled values and the retry model were consumed; output:
  `Change 'add-household-sharing-hardening' is valid`. (The original authoring session had no shell
  access, so the deltas were first checked structurally instead — one `## ADDED Requirements` header per
  delta file, at least one `#### Scenario:` per requirement, `- **WHEN**` / `- **THEN**` bullet form, and
  no `## MODIFIED` section.)
- Gate (per `definition-of-done`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` (no diff),
  `git diff --exit-code src/routeTree.gen.ts` after a build, `bun run typecheck`, `bun run lint`,
  `bun run test`, `bun run test:e2e`.
- Every requirement in this change has at least one scenario named by a test task in `tasks.md`;
  native-only behaviour additionally has a manual simulator step (task 5.15).
- Cross-check at review (task 6.2): the delta contains exactly HS-5, HS-6, HS-7, HS-9, HS-10, HS-11,
  HS-12, SYNC-6, SYNC-7 and NFR-6 — no more, no less — and no requirement another change owns is
  redefined or modified.
