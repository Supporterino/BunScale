# Questions — add-read-path-performance (P6)

Genuine unknowns only. Settled decisions (D1–D6) are **not** reopened here, and neither are P0's
open questions Q1–Q3 in `add-shared-household-foundation/questions.md`.

---

## Q1 — What is the concrete fixed request bound for a cold dashboard load?

**Blocks:** task 1.2 (bound definition), task 2.2 (concurrent cold load), task 3.4 (prefetch
grouping), and every test task that asserts a count (5.1, 5.8).
**Requirement affected:** NFR-7 ("a small, fixed number of remote requests", first clause).

NFR-7 mandates a fixed request bound for a cold dashboard load but gives no number, and the source
documents do not state how many data areas a dashboard load actually covers — that depends on what
P1, P2, P4 and P5 end up presenting. So the magnitude cannot be derived from the docs.

**Needed:** `architect` to state, in task 1.2, the concrete number and what counts toward it
(which data areas of the cold load are separate remote requests), so tests assert a value rather
than a shape. Until then the spec deliberately asserts only the shape — fixed, and independent of
the number of rabbits — which task 1.2 must not weaken.

---

## Q2 — What is the freshness window duration, and is it global or per data area?

**Blocks:** task 1.3 (window policy), task 2.1 (serve from the local copy inside the window),
task 3.3 (window in the query cache), tasks 5.2 and 5.5.
**Requirement affected:** NFR-7 (load-once reuse, "SHALL NOT force a full cold reload on the next
visit"); direction from IMP-3 ("serve-from-loaded-data-first with a freshness window").

IMP-3 names the policy but no duration, and P0 D0.2 says only that a start or foreground return
with connectivity refreshes the loaded copy. Two sub-questions follow from that: how long a loaded
value may be served without refreshing, and whether the window is one global value or one per data
area (a rabbit list and a weight history plausibly age differently).

**Needed:** a decision, not a spec invention. The window must be a single named, configurable value
so tests can drive it to zero and to "never" deterministically, and it must not suppress a change
signal or be readable as a service status (design D6.2). Also worth deciding here: whether P0's
start/foreground refresh may override the window, or whether the window is evaluated first.

---

## Q3 — Does "bounded" include a wall-clock target, or only the request count?

**Blocks:** the assertion strategy of tasks 5.1 and 5.8 (whether a time budget is asserted
alongside the call count).
**Requirement affected:** NFR-7.

NFR-7 defines boundedness as a request count ("target: a small, fixed number of remote requests"),
while US-QUAL-07's first acceptance criterion says the dashboard "loads in a bounded time rather
than a long serialized chain". No latency target appears anywhere in the requirements.

**Needed:** confirmation that the observable proxy is the request count and request shape, not a
wall-clock budget — otherwise a time budget must be stated, because a simulator-based latency
assertion would be non-deterministic and would need a documented tolerance (design risk R6). This
change asserts counts and call order only until the question is answered.
