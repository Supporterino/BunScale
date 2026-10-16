# dashboard — delta for add-medication-tracking

Traces requirement **DASH-5** and the medications part of **DASH-7**.
Stories: US-DASH-06 (DASH-5), US-DASH-08 (the medications part of DASH-7).

This delta adds two requirements and no others, under the exact names fixed for this phase:

- `Rabbits with active medications carry an informational badge` (**DASH-5**)
- `Medications section on the rabbit dashboard` (**DASH-7**, medications part)

`dashboard` is opened by three phases. The requirements below are referenced, not restated or altered:

| Requirement | Owner |
|---|---|
| `Dashboard lists non-archived rabbits with name, thumbnail and last-weighed date` (DASH-1), `A never-weighed rabbit shows a never-weighed indication` (DASH-2), `Rabbits are sorted by most recent weigh-in with never-weighed last` (DASH-3), `Rabbits with a weight-drop alert are flagged on the dashboard` (DASH-4 weight clause), `Selecting a rabbit opens that rabbit's dashboard` (DASH-6), `Rabbit dashboard with weight trend and history entry points` (DASH-7 weight part), `Zero rabbits shows an empty state prompting to add the first rabbit` (DASH-8), `Each dashboard row carries a per-rabbit quick-weight control` (DASH-9) | `add-rabbit-weight-core-loop` (P1) |
| `Rabbits with a recent severe health entry are flagged on the dashboard` (DASH-4 severe-health clause), `Health-only and full combined history views` (DASH-7 health part) | `add-health-and-photo-history` (P2) |

The medication badge and the warning flag coexist on one row and must stay distinguishable, so the
requirement below states the informational/informational-vs-warning distinction explicitly rather than
relying on it being inferred from P1's and P2's flag requirements.

## ADDED Requirements

### Requirement: Rabbits with active medications carry an informational badge

**DASH-5.** A rabbit that has **at least one active medication** SHALL carry a badge on the dashboard.
The badge SHALL be **informational**: it SHALL be visually distinct from the dashboard warning flag that
a weight-drop alert (P1) or a recent severe health entry (P2) raises, and it SHALL **never** use warning
styling. The badge SHALL be **absent** when none of the rabbit's medications is active, including when
every medication of that rabbit is finished. The badge SHALL be a presentation of the rabbit's derived
medication status and SHALL NOT be storable or editable on its own.

#### Scenario: A rabbit with an active medication carries the informational badge

- **WHEN** a rabbit has at least one active medication
- **THEN** that rabbit's dashboard row carries a badge
- **AND** the badge is presented in informational styling

#### Scenario: The badge is absent when every medication is finished

- **WHEN** all of a rabbit's medications are finished
- **THEN** that rabbit's dashboard row carries no medication badge

#### Scenario: A rabbit with no medications carries no badge

- **WHEN** a rabbit has no medications at all
- **THEN** that rabbit's dashboard row carries no medication badge

#### Scenario: The badge is distinct from the warning flag

- **WHEN** a rabbit carries both an active medication and a warning flag from a weight-drop alert or a
  recent severe health entry
- **THEN** both are presented on the row
- **AND** each is visually distinguishable from the other
- **AND** the medication badge does not use warning styling

#### Scenario: The badge clears when the last course finishes

- **WHEN** the only active medication of a rabbit is finished, for example because its end date is set
  to today
- **THEN** the rabbit's dashboard row no longer carries the medication badge

#### Scenario: The badge is visible without a hover on a phone [mobile]

- **WHEN** the dashboard is presented at a 390×844 viewport
- **THEN** the badge is presented on the row without requiring a hover or any other fine-pointer
  interaction

### Requirement: Medications section on the rabbit dashboard

**DASH-7 (medications part).** A rabbit's dashboard SHALL present a **medications section** listing that
rabbit's medications, each showing whether it is **active or finished** together with its **human-readable
schedule summary** (MED-4), and each offering an entry point to that medication's detail and its dose
history. A finished medication SHALL be listed in the section rather than omitted from it. For an
**archived rabbit** the section SHALL be presented **read-only**, offering no action that adds, edits or
deletes a medication or a dose. (The weight and health parts of DASH-7 are P1's and P2's and are not
restated here; the read-only rule for an archived rabbit is DM-4, P1's `An archived rabbit is read-only
everywhere`.)

#### Scenario: The section lists the rabbit's medications with their status and summary

- **WHEN** a rabbit's dashboard is opened and the rabbit has medications
- **THEN** a medications section is presented
- **AND** each of the rabbit's medications is listed with its active or finished status
- **AND** each is listed with its schedule summary

#### Scenario: A finished medication is listed and not omitted

- **WHEN** a rabbit has one active medication and one finished medication
- **THEN** the medications section lists both
- **AND** the finished one is presented as finished

#### Scenario: Each medication offers an entry point to its detail and dose history

- **WHEN** a person selects a medication in the medications section
- **THEN** that medication's detail is presented
- **AND** its dose history is reachable from there

#### Scenario: A rabbit with no medications presents the section without failing

- **WHEN** a rabbit has no medications
- **THEN** the medications section is presented with no medication listed
- **AND** it does not fail or replace the rest of the rabbit dashboard

#### Scenario: An archived rabbit's medications section is read-only

- **WHEN** an archived rabbit's dashboard is opened
- **THEN** its medications section lists its medications and their statuses
- **AND** no control that adds, edits or deletes a medication or a dose is available
