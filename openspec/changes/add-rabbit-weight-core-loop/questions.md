# Questions — add-rabbit-weight-core-loop (P1)

**No open questions remain.** Q1–Q3 were settled by the product owner before implementation and are
recorded as decisions in `design.md` (D1.12 … D1.14); Q4 was raised during implementation and settled
the same day as `design.md` D1.16. Each is reflected in the scenarios it touches and carried by the
tasks that consume it. No task in this change is blocked on an answer.

Values the source documents already state are still not questions, and never were: the 5% default drop
threshold (WT-7), the 10 kg plausibility bound (WT-4) and the ≈15 minute medication dose window (MED-7,
not this phase). The same applies to the settled rule that only an **archived** rabbit can be
permanently deleted (see `design.md` D1.8 and R7): it was an earlier question in this list, it carries
no number here any more, and it is not an open question. Settled decisions (D1–D6) are **not** reopened
here.

| Former question | Topic | Decision |
|---|---|---|
| Q1 | which entry is "the previous entry" when two weight entries share a timestamp — timestamp order, earlier-created entry first on a tie | `design.md` D1.12 |
| Q2 | the drop alert as a pure derivation over the current entry set, re-evaluated when an entry is edited or deleted | `design.md` D1.13 |
| Q3 | the value sets for a rabbit's sex (`male`/`female`/`unknown`) and neuter/spay status (`intact`/`neutered`/`spayed`/`unknown`) | `design.md` D1.14 |

New ambiguity discovered while implementing is appended here as a new question, per project hard
rule 7; Q4–Q10 have been added since Q3, each settled in the same session that raised it.


## Q4 — A missing payload channel in the local item model (found in task 2.5, settled 2026-10-22)

**Status: settled by the product owner.** The additive payload path was chosen; the decision is
`design.md` D1.16, implemented by task 2.0, which unblocks 2.5, 2.6, 2.7, 2.8 and 2.11–2.15.

`rust-backend` stopped at task 2.5's first stop condition, correctly. The finding, verified by the
orchestrator:

- The device's whole local model of a collection row was `StoredItem { item_id, updated_at, deleted }`
  (`src-tauri/src/storage/collection.rs`), converted from the equally payload-free `CollectionItem`
  (`src-tauri/src/ipc/types.rs`). `HouseholdCollection` carried only those rows, the change feed
  deserialised exactly that shape (`src-tauri/src/sync/read.rs`), and persistence snapshotted the same
  structure.
- `save_item` stores the incoming payload **as the item's version** (archived P0 design). So a write
  carrying only `is_archived: true` would have replaced the profile with it — the data loss RAB-9
  forbids.
- `archive_rabbit(rabbit_id, request_id)` has no profile to submit and there is no partial-update RPC.
- `list_rabbits` was blocked by the same gap, not by task 2.14: `RabbitSummary` needs `name` and
  `thumbnail_ref`, which the local set did not hold.

This was not a design conflict — P0 anticipated it. `storage/collection.rs` recorded the intended fix
as an **additive** step: "the item's *content* arrives with the first domain type (P1's rabbits and
weight entries) and is an additive change to this type." The plan simply had no task for it.

**Options put to the product owner:** (a) extend the local item model with the item payload — chosen;
(b) keep the model payload-free and re-plan the archive/list/read tasks around a service read, rejected
because it contradicts P0's offline-read design and would rewrite several tasks; (c) pause P1's
backend and finish the frontend first, rejected as it leaves the change half-built.

**Resolution:** `design.md` D1.16 (one internally tagged `ItemPayload`, `payload: Option<ItemPayload>`
where `None` means content unknown, version marker and tombstone unchanged), implemented by task 2.0.
Because the type and its storage fallout cannot be split without leaving the crate uncompilable, task
1.4 records the decision and 2.0 makes the change — `architect` owns `src-tauri/src/ipc/*` only, and
`rust-backend` owns the `storage/`/`sync/` plumbing.

## Q5 — Nothing refreshes the local copy after a confirmed write (found in task 2.5)

**Status: settled by adding a task.** Raised and resolved 2026-10-22.

`design.md`'s event section says a listener invalidates and refetches through the read commands, and
P0 shipped exactly that for `household:changed` (closing the same class of gap in its task 8.1). But the
device's copy is written **only** by the read path — the write gate holds a transport and nothing else —
and no task in this change owns a `rabbit:changed`/`weight:changed` subscriber. A mutation's own
invalidation therefore re-reads a stale copy: an archived rabbit stays in the active list, and a saved
threshold edit leaves the old flags, until the next foreground refresh.

**Resolution:** task 3.9 added (frontend-logic), subscribing to both P1 events and refreshing through
the read commands, following P0's `household:changed` pattern. No design change was needed — the
design already required it; the plan was missing the task.

## Q6 — A failed P1 read has nowhere to name its cause (found reviewing task 4.1)

**Status: settled by adding two tasks.** Raised and resolved 2026-10-23.

Reviewing 4.1 showed that a failed dashboard read renders as P0's "nothing loaded" intro: the list
conflates *pending*, *failed* and *empty*, so the cause never reaches the person. Two separate gaps
produced that:

- `describeSaveFailure` deliberately discards `CommandError.message_key`, so D1.16's
  `error.household_content_unloaded` — already in the catalog from 3.8 — could not be rendered even if
  the surface tried. Only `error.unknown`'s operation-neutral copy was reachable.
- No task owned a read-failure surface at all. D1.16 says the unknown-content refusal "renders on the
  affected screen's failure state, which offers a refresh", and NFR-8 requires cause-specific copy;
  the plan added the string but never the surface.

**Resolution:** task 3.10 (frontend-logic) adds the read-side cause mapper that keeps the surface key;
task 4.15 (frontend-ui) renders the failure state with a refresh on every P1 screen a failed read can
reach. No design change — D1.16 already required the behaviour.

## Q7 — WT-7 orders tied entries by creation, which nothing stored (found in task 2.12)

**Status: settled — the orchestrator approved one additive field.** Raised 2026-10-23.

WT-7's scenario "Entries sharing a timestamp are ordered by creation" needs a creation order. Nothing
stored it: `WeightEntry` carries the *reading* timestamp (editable into the past, so it cannot double as
creation), `updated_at` is the LWW version marker and moves on every correction, and the entry's item id
is 128 random bits, unordered. Task 2.12 therefore broke ties by id — deterministic but arbitrary, which
would let 5.5's scenario pass while testing nothing.

**Options:** (a) store creation order on the entry, chosen; (b) derive it from an existing field, rejected
because a single WT-5 correction falsifies every candidate; (c) restate WT-7 in terms of what is
observable, rejected because the requirement's purpose is cross-device determinism.

**Resolution:** `design.md` D1.12 now adds `created_at: Instant` to `WeightEntry`, required (no serde
default — a default would invent an order), set once at create from the memo that already existed,
preserved across edits like `id`/`rabbit_id`/`photo_refs`. Total order is `(timestamp, created_at, id)`.
This is an **additive change to a type frozen in task 1.1**; the orchestrator approved it because the
spec wins over the frozen snapshot and no data exists yet (pre-release), and recorded the ripple: the
bindings change deliberately, 2.12 adopts one comparator, 2.13 orders the full set with it, and 5.5 must
assert the discriminating case (earlier-created entry edited *after* the later one).

## Q8 — The rabbit profile read had no owning task (found in task 2.15)

**Status: settled by adding a task.** Raised and resolved 2026-10-23.

Implementing the last read (2.15) left exactly one P1 command still returning `not_implemented`:
`get_rabbit`. The frozen contract has it, `src/features/rabbits/api.ts` wraps it, and 4.4's profile
detail reads through it — but no task in section 2 named it. Left alone, the profile screen could never
load, and 4.4's "edit control gated on the profile read" would be permanently empty.

**Resolution:** task 2.16 added (rust-backend), implementing `get_rabbit` with the same read shape as
`summaries`/`dashboard`/`detail` (tombstone before the content check, unknown content refuses, archived
readable, blank or non-rabbit id refused as `validation`). No design change — the contract already
required the command; the plan was missing the task.

## Q9 — The rabbit dashboard had no route for its range state (found in task 4.16)

**Status: settled by adding a task.** Raised and resolved 2026-10-24.

4.11 built `RabbitDashboard` to take the trend and history ranges as props, because D1.4 requires the
two selections to live in **typed route search parameters** rather than screen state — but no task
owned `src/routes/**`, and `frontend-ui` cannot write it. Left alone the ranges would have been
`useState`, which the decision forbids and which drops both on reload.

**Resolution:** task 4.16 added (frontend-logic): the `$rabbitId` route with
`validateSearch: validateWeightRangeSearch`, passing `trend`/`history` through and removing the
screen's state, with the row control opening it.

## Q10 — An archived rabbit's read-only dashboard had no way in (found in task 4.6)

**Status: settled by adding a task.** Raised 2026-10-25.

Task 4.6 built the archived rabbit's read-only presentation on the `$rabbitId` route, but nothing in
the product navigates there for an archived rabbit: `ArchivedRabbitList` (4.5) offers restore and
permanent delete only, and the dashboard's row control (4.7, unbuilt) is defined for the active list.
RAB-6 says an archived rabbit's history is retrievable from the archived list and DASH-7's archived
scenario assumes it is opened, so as it stood the read-only surfaces were reachable by deep link only —
the same "built but unmounted" shape Q49 recorded in P0.

**Resolution:** task 4.17 added (frontend-ui): the archived row opens the same read-only dashboard
route; lifecycle controls stay where 4.5 put them and no weight-write control appears on the archived
surfaces.

## Q11 — The weight form had no route, and 4.7 mounted it inline (found reviewing task 4.7)

**Status: settled by adding a task.** Raised 2026-10-25.

D1.10 is explicit: both quick paths "open the same weight form route with the rabbit bound through
typed search parameters", which is what keeps range, unit and binding behaviour in one place. Task 4.7
could not create a route (`src/routes/**` is `frontend-logic`'s), so it mounted 4.9's `WeightForm`
inline in `RabbitList` instead. The review flagged that as a design contradiction with a concrete cost
— 4.8 would duplicate the household read, the mounting state and the focus return, and the inline
mount is a dead end when 4.9's display-unit read is pending or failed, because that card has no cancel
control.

**Resolution:** task 4.18 added (frontend-logic): one weight-form route with typed search parameters,
cancel returning to the opener and a successful save returning to the rabbit's context. 4.7 keeps its
directly visible row control but navigates to that route instead of mounting the form inline; 4.8
opens the same route. The Q9/4.16 precedent applies: when a decided design needs a route, the plan
gains the task rather than the implementation choosing an alternative.






