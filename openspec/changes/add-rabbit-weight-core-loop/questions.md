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
rule 7; only Q4 has been added since Q3.

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


