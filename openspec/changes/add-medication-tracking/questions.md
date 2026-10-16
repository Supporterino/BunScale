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

Four open items are recorded below. Each names the tasks it blocks and the requirements it affects.

---

## Q1 — From which point is the approximately 15-minute window measured, and in which direction?

**Blocks:** task 2.9 (the near-simultaneous check), task 3.5 (the warning state), task 4.4 (the
warning presentation), task 5.7 (the guard tests), and design D5.5.
**Requirements affected:** MED-7, with MED-5 and DM-6.

MED-7 warns when "a dose for the same medication was already logged within a short recent window
(≈15 minutes)". Two things are unspecified:

1. **The anchor.** "Already logged" is measured relative to *what* — the moment the person confirms, or
   the timestamp entered for the new dose? MED-5 and DM-6 let that timestamp be edited, including to
   the past, so the two anchors disagree exactly when a person is correcting a forgotten dose: a dose
   recorded now but stamped for this morning should not be compared against what was logged in the last
   fifteen minutes of wall-clock time.
2. **The direction.** Only *preceding* doses, or any dose inside a window around the new timestamp? If a
   person back-dates a dose to 09:00 and another member's dose was logged at 09:05, a purely backward
   reading sees no conflict while the record ends up with two doses five minutes apart.

**Needed (a product decision):** the anchor and the direction. Until it is answered, the backend
compares preceding doses against the timestamp being recorded — the narrowest reading of "already
logged" — and the specs assert only the inside/outside behaviour the requirement states, so no test
depends on the undecided part. The approx. 15-minute duration itself is given and is not in question.

---

## Q2 — Must the guard be checked against freshly fetched data?

**Blocks:** task 2.9 (the check), task 3.5 (the warning state), task 5.7 (the guard tests), and
design D5.5 and R1.
**Requirements affected:** MED-7, with SYNC-3 (P0) and D1.

Reads are served from the household's locally kept copy and refreshed when the shared service is
reachable (P0's D0.2). A dose another member recorded four minutes ago may therefore not be in the
local copy when this device confirms a dose — which is precisely the situation the guard exists to
prevent, and the reason MED-7 exists at all. The requirement says the person SHALL be warned if a dose
"was already logged" without saying whether the product must go and look.

**Needed (a product decision):** whether confirming a dose forces a fresh read of that medication's
doses before the check is evaluated (slower, truthful), or whether the check is evaluated against the
data the device currently holds (fast, and can miss a very recent dose by another member). Until it is
answered, task 2.9 implements the check as a distinct read — so that forcing a fetch later is a
one-line change rather than a redesign — and the specs promise only that the person is warned when such
a dose is *known*, without claiming a guaranteed freshness.

---

## Q3 — What amount rules apply to a serving and to a recorded dose?

**Blocks:** task 1.1 (the dose payload's unit field), task 3.3 and 3.4 (the form state), task 4.1 and
4.3 (the forms), task 5.5 (the dose tests).
**Requirements affected:** MED-1, MED-5.

Two gaps:

1. **May a recorded dose carry a different unit from the medication's serving?** The domain model gives
   `Dose` both an `amountValue` and an `amountUnit`, which implies a dose is self-describing, and a dose
   can plausibly be given in another unit from the one the course is recorded in (a drop count against a
   serving in ml). MED-5 says only that the amount is pre-filled from the serving and editable; the
   story that traces it (US-MED-13) changes the value and never the unit. If doses may differ, the dose
   form needs a unit control; if they may not, the unit is fixed by the serving and the stored dose unit
   is redundant.
2. **What validity applies to an amount?** No rule is stated for a serving or a dose — unlike weight,
   where WT-4 rejects non-positive values and confirms implausible ones (P1). Whether a zero, negative
   or arbitrarily large amount is refused, accepted, or accepted after confirmation is undecided, and
   the form cannot be built without knowing.

**Needed (a product decision):** whether a dose may use a unit other than the serving's, and what
validity applies to both amounts. Until it is answered, no unit conversion is implemented (design
D5.7 — a unit is a label with its value, never converted), the dose unit is carried on the dose as the
domain model requires, and no amount-validation rule is invented into the specs or the tests.

---

## Q4 — May a dose that was actually given after the end date still be recorded?

**Blocks:** task 2.10 (read-only enforcement), task 4.8 (the read-only presentation), task 5.9 (the
read-only tests).
**Requirements affected:** MED-2 and MED-12, with MED-5.

MED-12 makes a finished medication's records read-only, and a new dose is a write, so under the strict
reading a course that was ended — including by setting the end date to today (MED-2) — can never
receive another dose, even one that was genuinely given. Two ordinary situations collide with that:

- a course ended at the wrong moment, or ended in the product while doses continued in reality;
- a dose given on the last day of a course and logged after midnight, when the end date has already
  passed.

The requirements never describe the give-dose action for a finished medication, and the medication
detail is required to offer "a primary give-dose action" (MED-8) without qualifying the medication's
status.

**Needed (a product decision):** whether a given dose after the end date is recordable (and if so,
whether it is an exception in the guard or a general "record a dose against a finished course" path),
or whether MED-12's read-only reading stands without exception. Until it is answered, this change
implements MED-12 as written: a finished medication offers no action that records a new dose, and task
5.9 asserts that state.
