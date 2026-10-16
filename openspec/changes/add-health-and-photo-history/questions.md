# Questions — add-health-and-photo-history (P2)

Genuine unknowns only. Settled decisions (D1–D6) are **not** reopened here, and values the source
documents already give (the nine canonical categories, the three severities, the editable
present-at-creation timestamp, the 7-day flag window itself) are not turned into questions.

---

## Q1 — How long must a viewed photo stay available locally?

**Blocks:** task 2.12 (photo local-copy boundary), task 3.4 (frontend photo availability state),
task 5.9 (offline photo tests) — and, through them, the SYNC-4 guarantee.
**Requirement affected:** SYNC-4.

SYNC-4 promises that a photo is available locally after it has been displayed once. Nothing in the
source documents says whether that retained copy is kept indefinitely, is capped by a size budget, or
may be discarded under storage pressure or by the operating system. If it can be discarded, the
guarantee becomes "available unless it was evicted", and the placeholder case has to be reachable for a
photo that *was* displayed — which changes what the tests can assert.

**Needed (a decision, not a spec invention):** whether the local photo copy is unbounded/indefinite or
evictable, and if evictable, what the product states about it. Until it is decided, the spec promises
only what the local copy actually holds, and task 5.9 asserts the held/unheld cases without asserting a
retention duration.

---

## Q2 — Where does a rabbit's profile photo sit in a gallery ordered by entry date?

**Blocks:** task 2.10 (gallery read), task 4.9 (gallery UI), task 5.8 (gallery ordering tests).
**Requirement affected:** PHOTO-3.

PHOTO-3 shows all of a rabbit's photos — the photos attached to entries *and* its profile photo —
ordered by entry date. A profile photo belongs to no entry, so it has no entry date to sort by. The
source documents give no rule for its position (first, last, by upload date, or outside the ordered
sequence).

**Needed (a decision):** where the profile photo appears in the gallery ordering. The spec currently
asserts only that entry photos are ordered by entry date, so it is testable and truthful in the
meantime.

---

## Q3 — Is the "last 7 days" severe-health window a rolling instant window or calendar days?

**Blocks:** task 2.7 (derived dashboard health flag), task 5.7 (flag tests), task 1.2 (recording the
definition).
**Requirement affected:** DASH-4 (severe-health clause).

DASH-4 flags a rabbit with a Severe entry "within the last 7 days". That is either a rolling window of
seven times twenty-four hours measured from the present instant, or the current plus the six preceding
calendar days in the person's timezone. The two disagree for part of every day, and any
back-dated Severe entry (HL-1 allows an editable past timestamp) lands differently under each reading,
so the flag is not reproducible in tests until one is chosen. The source documents give no timezone
rule for this window either.

**Needed (a decision):** the window definition (rolling duration or calendar days) and the timezone it
is evaluated in. Until then, tests use instants stated relative to the stored entry timestamp rather
than to a device calendar.
