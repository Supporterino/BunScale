# Questions — add-medication-tracking (P5)

Genuine unknowns only. Settled decisions (D1–D6) are **not** reopened here, and values the source
documents already give are applied as given rather than turned into questions — in particular:

- the four serving units {mg, ml, drops, tablets} (§5, MED-1);
- the four schedule types and their parameters (§5, MED-3);
- the times-per-day range of 1 to 4 slots (§5, MED-3);
- the approximately 15-minute near-simultaneous window (MED-7);
- the start date defaulting to today and being editable (MED-1);
- the giver being a named member of the household (DM-5).

`US-MED-23` (offline dose logging) is not a question: it is **dropped** by D1, and the delivery order
(medication before export/reminders) is settled. Medication **reminders** are not a question either:
they are deliberately specified in `add-vet-export-and-reminders` (P4) under D3's one unified
mechanism. Whether a medication's own details are editable, and whether a medication is ever deletable,
is **settled** as well: the product owner decided that an active course's details are correctable and
that a medication is **never deletable**, and it is stated as **MED-13** in
`specs/medication-tracking/spec.md` together with the end-date correction it requires of MED-2 and
MED-12.

**No open questions remain.** The four former questions were settled by the product owner; each is
recorded as a decision in `design.md` (D5.12 … D5.15) and reflected in the requirements, scenarios and
tasks that consume it. No task in this change is blocked on an answer, and settled decisions (D1–D6)
were never reopened here.

| Former question | Topic | Decision |
|---|---|---|
| Q1 | the guard window's anchor and direction | `design.md` D5.12 — measured from the timestamp entered for the new dose, covering doses both before and after that instant |
| Q2 | whether the guard forces a fresh read | `design.md` D5.13 — confirming a dose re-reads that medication's doses before the window is evaluated (resolves R1) |
| Q3 | whether a dose may carry another unit, and the amount rules | `design.md` D5.14 — a dose carries its own unit, which may differ from the serving's; amounts must be positive; no plausibility bound |
| Q4 | recording a dose actually given after the end date | `design.md` D5.15 — it stays recordable, as MED-12's second bounded exception |

New ambiguity discovered while implementing is appended here as a new question, per project hard rule 7;
nothing has been added since Q4.
