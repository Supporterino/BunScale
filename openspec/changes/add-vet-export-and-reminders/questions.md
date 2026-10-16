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
(D4.3/D4.5) and carried as risk R3. Nothing replaces it here; what follows is what remains genuinely
open, renumbered contiguously.

---

## Q1 — What does a snooze actually do?

**Blocks:** task 2.8 (snooze handling), task 4.4 (snooze landing), task 5.8 (snooze tests).
**Requirements affected:** MED-9 ("an action to snooze"), and by the mechanism requirement the same
question exists for weigh reminders (no weigh snooze is required today).

MED-9 gives the reminder a snooze action and says nothing else about it: no delay, no limit on
repeated snoozes, and no statement of what happens when a snooze defers a slot past its window or
past the end of the day. The one thing the requirements do decide is that snoozing must not be
mistaken for having given the dose, which is why the requirement asserts that a snoozed slot stays
**unsatisfied** rather than pretending to be given.

**Needed (a decision, not a spec invention):** the snooze delay, whether a snoozed occurrence may be
snoozed repeatedly, and whether a snooze past the medication's window leaves the slot missed (so the
follow-up applies) or simply defers. Until decided, task 2.8 implements "defers, records nothing,
stays unsatisfied" and no delay value is asserted by any test.

---

## Q2 — What date range does the export control start on?

**Blocks:** task 2.10 (`get_export_span`, selection bounds), task 4.5 (range control), task 5.9.
**Requirement affected:** EXP-1 ("a date range the person selects").

EXP-1 requires a selectable range but states no default. The product's chart ranges are
7d/30d/90d/all (WT-6, P1), which is a plausible default set; "everything the rabbit has" is another;
"empty, the person must choose" is a third. The choice is visible to the person and changes the
cost of a large export (R6).

**Needed:** the default selection, and whether the presets are the chart's ranges or an independent
set. Until decided, task 2.10 exposes the span (earliest and latest history instant) and task 4.5
must not invent a preset set.

---

## Q3 — Does the selected range narrow a course's dose history?

**Blocks:** task 2.10 (selection), task 5.9 (assertion).
**Requirement affected:** EXP-1 and its scenarios "The selected range determines which entries are
presented" and "Each medication is presented with its full dose history".

EXP-1 says the history is exportable **for a selectable date range** and, in the same sentence, that
medication export includes each course's **full dose history**. For a course that starts before the
range, or whose doses fall outside it, the two clauses can be read two ways: the range narrows which
courses are presented while a presented course reports every dose it has, or the range narrows the
doses as well, making "full" mean "full within the range".

**The delta takes the first reading** — the range decides which entries and courses are presented,
and a presented course is not stripped of the doses it has, because a vet reading "Baytril, 12 doses"
followed by a dose list that silently omits the out-of-range ones is worse than a longer list. This
reading is recorded here so that it is a confirmed decision rather than an assumption; if the second
reading is intended, EXP-1's prose and scenario are corrected and task 2.10 follows.

---

## Q4 — Do the raw files carry stored values or display-formatted ones?

**Blocks:** task 2.12 (raw generation), task 5.11.
**Requirement affected:** EXP-2 ("raw data"), interacting with P0's SET-4 display-unit boundary.

EXP-2 asks for **raw** data, which argues for the canonical stored values (grams, stored amounts and
units as recorded). The display unit, however, is a per-device preference the product already owns
(SET-4), and presentation of weight values at a display boundary is P0's rule; an export is neither
an entry form nor a screen, so it is not obviously that boundary.

**The delta takes the canonical reading** — values are written as stored, without presentation
formatting — recorded here for confirmation rather than assumed. The PDF (EXP-1) is unaffected: it
is a *formatted* document, so it presents values in the person's display unit as any screen would.

---

## Q5 — What is "trend direction", and what does the summary say when there is nothing to summarise?

**Blocks:** task 3.7 (summary derivation), task 4.8 (chart label), tasks 5.16/5.21.
**Requirement affected:** NFR-4 ("an accessible summary (e.g. latest value + trend direction)").

NFR-4 offers "latest value + trend direction" as an example and defines neither. Open: what counts as
a trend (a comparison with the previous entry, with the range's first entry, with a threshold), how
it is worded, and what is said when a rabbit has one entry or none — where a direction cannot be
honest. NFR-5 does name a "never weighed" label, so the no-value case has vocabulary available.

**Needed:** the definition of trend direction and the wording for the no-value and single-value
cases. The requirement as written deliberately over-claims nothing: it asserts that a summary exists,
that it follows the chart's own values, and that it says so instead of reporting a value when there
is none. Tasks 3.7/4.8 must not invent a direction claim beyond that.

---

## Q6 — What must the product do when notification authorization is denied or revoked?

**Blocks:** task 3.2 (authorization state in the data layer), task 4.1 (reminders screen),
task 5.13 (component assertion).
**Requirements affected:** MED-9 and the mechanism requirement ("a reminder SHALL be raised"), the
consequences of a device state the requirements do not mention.

A reminder cannot be raised without the device's authorization, and the requirements describe only
the device that has it. Open: whether the product must ask when the first reminder is configured,
what it must show when authorization is denied (a configured reminder that will not fire is a
promise the product cannot keep), and what happens to already-scheduled occurrences when
authorization is revoked.

**The design's reading (D4.7)** is to observe and present the state freshly and to keep the
configuration usable, in the same honesty stance P0 fixed for service status — but that stance is
P3's NFR-6 requirement for the service and was never written for notifications. Tasks 3.2/4.1
implement that reading; if the product owner wants a different behaviour (for example refusing to
save a reminder that cannot fire), the requirement text and those tasks change.
