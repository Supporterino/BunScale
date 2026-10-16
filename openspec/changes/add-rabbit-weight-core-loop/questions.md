# Questions — add-rabbit-weight-core-loop (P1)

**No open questions remain.** The former Q1–Q3 were settled by the product owner; each is recorded as
a decision in `design.md` (D1.12 … D1.14), reflected in the scenarios of RAB-1 and WT-7, and carried
by the tasks that consume it. No task in this change is blocked on an answer.

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
rule 7; nothing has been added since Q3.
