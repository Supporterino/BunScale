# Questions — add-rabbit-weight-core-loop (P1)

Genuine unknowns only. Settled decisions (D1–D6) are **not** reopened here, and values the source
documents already state — the 5% default drop threshold (WT-7), the 10 kg plausibility bound (WT-4)
and the ≈15 minute medication dose window (MED-7, not this phase) — are applied as given rather than
turned into questions.

---

## Q1 — Can an active rabbit be permanently deleted, or only an archived one?

**Blocks:** task 2.7 (permanent deletion), task 4.5 (the delete control), task 5.2 (lifecycle tests).
**Requirements affected:** RAB-10 against RAB-6, RAB-7 and RAB-8.

RAB-10 states that a rabbit and all associated data are permanently deletable on confirmation, with no
condition on the rabbit's state. US-RAB-10, which traces it, gives its acceptance criterion from an
archived rabbit ("GIVEN an archived rabbit, WHEN I confirm permanent deletion …"), and RAB-8 makes an
archived rabbit's timeline read-only. The documents never say whether deletion is a two-step flow
(archive first, then delete) or available directly from an active rabbit's profile.

**Needed:** a product decision on whether permanent deletion is offered for active rabbits, archived
rabbits only, or both. Until it is answered, task 2.7 implements the archived case (the only case a
story describes) and the active case is left out rather than invented.

---

## Q2 — Which entry is "the previous entry" when two weight entries share a timestamp?

**Blocks:** task 2.13 (drop-alert derivation), task 5.5 (alert tests).
**Requirements affected:** WT-7, with WT-1 and DM-6.

WT-7 defines the drop as the change against "the previous entry", and WT-1 lets a person edit a
timestamp, including to a past time, so entries with identical timestamps are reachable through
entirely ordinary use (two readings logged for the same moment, or two entries back-dated to the same
day). The documents state neither an ordering nor a tie-break.

**Needed:** the ordering rule for the alert basis — timestamp order with a defined tie-break (for
example by the order in which the entries were created), or a rule that an entry with a tied
timestamp is not compared at all. The rule must be deterministic, because otherwise the entry flag and
the dashboard flag (DASH-4) can disagree and the same data can produce two different alerts on two
devices.

---

## Q3 — Does the drop alert recompute when a weight entry is edited or deleted, not only when the threshold changes?

**Blocks:** task 2.13 (drop-alert derivation), task 2.11 (entry edit and delete), task 5.5 (alert tests).
**Requirements affected:** WT-7 against WT-5.

WT-7 requires the alert to be recomputed when the rabbit's threshold changes, and says nothing about
what happens to the alert when the entries themselves change. WT-5 allows any past entry to be edited
(value or timestamp) or deleted, and deleting an entry changes which entry is "the previous entry" for
the one that follows it, so the following entry's alert basis changes.

**Needed:** confirmation that the alert is a pure derivation over the current entry set — that is,
that editing or deleting an entry re-evaluates the alerts of the entries around it — rather than a
value fixed when the entry was saved. `design.md` D1.3 assumes the pure derivation because it is the
only reading consistent with WT-5 and WT-7 having to agree; the assumption should be confirmed rather
than left implicit, since it also determines what the dashboard flag means after a deletion.

---

## Q4 — What are the value sets for a rabbit's sex and neuter/spay status?

**Blocks:** task 1.1 and 1.2 (contract types and their recorded value set), task 4.3 (creation form),
task 5.1 (profile tests).
**Requirements affected:** RAB-1.

RAB-1 requires a rabbit profile to record "sex" and "neuter/spay status", and the domain model (§5)
names the fields as `sex` and `neuterStatus`. Neither document enumerates the permitted values, and
the staging of a neuter/spay status is clinically meaningful in a way that a free-text field or a
guessed two-value enumeration would both get wrong.

**Needed:** the permitted values for both fields. `architect` records the value set with the contract
in task 1.2 when the answer is available; until then the types are declared and the task is blocked
rather than the values being invented.
