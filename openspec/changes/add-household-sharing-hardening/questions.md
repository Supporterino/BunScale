# Questions — add-household-sharing-hardening (P3)

Genuine unknowns only. Settled decisions (D1–D6) are **not** reopened here, and values the source
documents already give are not turned into questions — the six-member cap (6), the deletion cascade list,
the reuse-don't-duplicate rule and the localized "You"-style label are settled and only need to be
implemented.

Two questions that were previously recorded here have been **answered** and are no longer open. Both were
settled by the product owner and are consumed as values rather than re-derived: the concrete bounded
times for a sharing attempt, and the number of consecutive failures after which the service is presented
as offline. The settled values — a **15 s** attempt bound, a **5 s** reachability-probe bound and a
**3-consecutive-failure** offline threshold — and their ownership are recorded in `design.md` D-H12.
Nothing replaces them here.

---

**No open questions remain.** The former Q1–Q4 were settled by the product owner; each is recorded as a
decision in `design.md` (D-H13 … D-H16) and reflected in the requirements, scenarios and tasks that
consume it. No task in this change is blocked on an answer, and settled decisions (D1–D6) were never
reopened here.

| Former question | Topic | Decision |
|---|---|---|
| Q1 | the member cap is enforced at membership level, so acceptance into a full household is refused with the same explanation as creation beyond the cap | `design.md` D-H13 (HS-7) |
| Q2 | background-delivered changes are **visible**: a local notification naming the household, whose authorization surface (`request_notification_authorization()` / `get_notification_authorization()`, `NotificationAuthorization`) is declared here and consumed by P4 | `design.md` D-H14 (HS-11) |
| Q3 | a member offline at deletion keeps reading the cached household until the device next reaches the service, when the copy is dropped and access is gone | `design.md` D-H15 (HS-10) |
| Q4 | a non-owner's contribution is attributed to the owner's allowance end to end, and that is a hard selection criterion for the mechanism | `design.md` D-H16 (HS-12) |

New ambiguity discovered while implementing is appended here as a new question, per project hard
rule 7; nothing has been added since Q4.
