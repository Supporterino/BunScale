# Questions — add-vet-export-and-reminders (P4)

Genuine unknowns only. Settled decisions (**D1–D6**) are **not** reopened here, and values the
source documents do state are not turned into questions: the medication follow-up being
**enabled by default** (MED-10), the sharing options being the **device's standard** ones (EXP-3),
the language set being **System / Deutsch / English** (SET-3), metric units being the **default**
(NFR-5) and reminders being **one** mechanism (D3).

One question previously recorded here has been **answered** and is no longer open. It asked whether a
follow-up can be guaranteed when the device does not run the product again. The product owner has
settled it: a **background mode is requested**, so the product reconciles and cancels pending
follow-ups while it is suspended, consuming and extending the background mechanism
`add-household-sharing-hardening` (P3) already establishes for HS-11. The decision — including that
it deliberately reverses this change's earlier no-background-mode choice, and that the platform may
never run a granted window, so the gap is narrowed and not closed — is recorded in `design.md`
(D4.3/D4.5) and carried as risk R3.

**No open questions remain.** The former Q1–Q6 were settled by the product owner; each is recorded as
a decision in `design.md` (D4.10 … D4.15) and reflected in the requirements, scenarios and tasks that
consume it. No task in this change is blocked on an answer, and settled decisions (D1–D6) were never
reopened here.

| Former question | Topic | Decision |
|---|---|---|
| Q1 | what a snooze defers, whether it repeats, and what a snooze past the window leaves | `design.md` D4.10 |
| Q2 | the export range control's default and its presets | `design.md` D4.11 |
| Q3 | whether the selected range narrows a presented course's dose history | `design.md` D4.12 |
| Q4 | whether the raw files carry stored values or display-formatted ones | `design.md` D4.13 |
| Q5 | the definition of trend direction and the no-value/one-entry wording | `design.md` D4.14 |
| Q6 | the product's behaviour when notification authorization is denied or revoked | `design.md` D4.15 |

New ambiguity discovered while implementing is appended here as a new question, per project hard
rule 7; nothing has been added since Q6.
