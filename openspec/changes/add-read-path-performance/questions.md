# Questions — add-read-path-performance (P6)

**No open questions remain.** The former Q1–Q3 were settled by the product owner; each is recorded
as a decision in `design.md` (D6.1, D6.2, D6.7) and reflected in the tasks and scenarios that consume
it. No task in this change is blocked on an answer, and settled decisions (D1–D6) were never reopened
here. P0's former Q1–Q3 are settled in `add-shared-household-foundation/design.md` (D0.12–D0.20) and
are likewise not reopened.

| Former question | Topic | Decision |
|---|---|---|
| Q1 | the concrete fixed request bound for a cold dashboard load | `design.md` D6.1 — **two** remote requests (the household's collection plus one service-status check), independent of rabbit count |
| Q2 | the freshness window: duration, and whether it is global or per data area | `design.md` D6.2 — **30 seconds**, one global value for all loaded data |
| Q3 | whether "bounded" means a wall-clock target or only the request count | `design.md` D6.7 — the request count is the CI-asserted requirement; the wall-clock figure is measured on the reference device and recorded as evidence |

New ambiguity discovered while implementing is appended here as a new question, per project hard
rule 7; nothing has been added since Q3.
