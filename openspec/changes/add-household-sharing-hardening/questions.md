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
Nothing replaces them here; what follows is what remains genuinely open.

---

## Q1 — Does the member cap block only invitation creation, or also acceptance of an outstanding invitation?

**Blocks:** task 2.1 (cap check on creation), task 2.5 (cap check at membership level), task 5.3.
**Requirements affected:** HS-7.

HS-7 says the household is capped at six members and that "invitations beyond the cap SHALL be blocked
with an explanation". That is unambiguous for creating a new invitation. It does not say what happens
when an invitation was issued *before* the household reached six members and is accepted *after* — for
example because a seventh person was added in between. Two readings are possible: the cap is checked
when the invitation is created and an already-issued invitation remains acceptable, or the cap is
enforced at membership level and acceptance is refused with the same explanation.

**Needed:** a decision. This change's delta currently states the membership-level reading (the household
never exceeds six), because the cap is stated as a property of the household; if the other reading is
intended, HS-7's second scenario must change.

---

## Q2 — May a background-delivered change be presented as a notification, or must it stay silent until the product is next shown?

**Blocks:** task 1.3 (notification permission string), tasks 1.6–1.7 (background delivery), task 5.15.
**Requirements affected:** HS-11 (updates MAY be delivered in the background), NFR-3, NFR-6.

HS-11 says a change reaches every member "without requiring the user to open the product manually" and
that updates MAY be delivered in the background. It does not say whether that delivery is allowed to be
visible to the person — a notification naming the household — or must remain silent so that the change
is simply already there when the product is next opened. Besides being a UX decision, this decides
whether a notification permission string and its usage description are needed at all, and whether the
permission may be requested.

**Needed:** a decision before `tauri-security` declares the notification permission. Until then task 1.3
declares only what the chosen delivery mechanism requires and no user-visible notification is raised.

---

## Q3 — What does "access is withdrawn" mean for a member who is offline when the household is deleted?

**Blocks:** task 2.7 (deletion and access withdrawal), task 5.12 (E2E).
**Requirements affected:** HS-10 (deletion withdraws access from all members and is shared with them),
SYNC-3 (reads are served from previously loaded data while the service is unreachable).

HS-10 requires deletion to withdraw access from every member. SYNC-3 requires reads to be served from
previously loaded data while the shared service is unreachable. A member who is offline at the moment of
deletion has a local copy and no way to learn about the deletion until their next successful
refresh — so for that window the two requirements point in different directions, and the source
documents do not say which wins. Deleting the local copy on a push is also not possible while that
device is unreachable.

**Needed:** a decision on the observable rule — for example whether the local copy of a deleted
household is dropped at the next refresh, and whether a member may keep reading it until then. This
change is recorded as risk R11 and its scenarios currently assert withdrawal of access generally, not
the erasure of another device's cache.

---

## Q4 — How is allowance attribution observable end to end?

**Blocks:** task 2.11 (attribution and cause mapping), task 5.7.
**Requirements affected:** HS-12, SYNC-2.

HS-12 says shared content counts against the household owner's allowance, and this phase states two
observable scenarios from that: content saved by a non-owner is charged to the owner, and an exhausted
owner allowance refuses a member's save with the storage-allowance cause. The first is product
behaviour and needs no external reporting.

The upstream point is **settled** and is not this question: whether the shared service can report
allowance exhaustion at all was P0's, and P0 has decided it (D0.12). An exhausted owner allowance must
be reportable as **its own condition**, distinct from a generic write error, and that is a **hard
selection criterion** for the shared-service mechanism in task 1.1 — a candidate that cannot is
rejected. `SaveCause.allowance_exceeded` is therefore reachable by contract, and HS-12's second scenario
no longer rests on an open answer.

What remains open here is different and downstream of that: whether content contributed by a
**non-owner** is attributed to the household **owner's** allowance, end to end — that is, whether the
service's own accounting charges a member's contribution to the owner rather than to that member's own
account — and how that attribution is observed, from one member's device to another and in the product.

**Needed:** confirmation of whether the mechanism attributes a non-owner's contribution to the owner's
allowance as an observable fact, and of the surface that makes it observable end to end. Until it is
answered, task 2.11 implements attribution at the product's own boundary and maps the cause the
mechanism can actually produce; any part the mechanism does not state is recorded rather than replaced
by a generic failure message (NFR-8).
