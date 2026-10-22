# product-quality Specification

## Purpose
These are the cross-cutting quality promises every capability must keep. Saving requires a reachable shared service and a valid account while reading is served from previously loaded data, with reachability re-evaluated on start, on return to the foreground and when connectivity is restored (NFR-1); concurrent edits converge last-write-wins without failing an edit or duplicating an item (NFR-2); diagnostic records carry identifiers, counts and outcomes only and never rabbit names, note text, photo content or weight values (NFR-3); and every failure path names its specific cause rather than reporting only that something went wrong (NFR-8).

## Requirements

### Requirement: Connectivity model: reachability-gated writes, loaded-data reads

**NFR-1.** Saving new or modified data SHALL require the shared service to be reachable and a valid
account. A save that cannot complete SHALL be refused and surfaced with its cause, and SHALL never
be faked. Reading SHALL use previously loaded data while the shared service is unreachable, and
SHALL refresh once the service becomes available. Reachability SHALL be re-evaluated when the
product starts or returns to the foreground and when connectivity is restored, without the person
having to reinstall or reset the product.

#### Scenario: Writes are accepted only while reachable and identified

- **WHEN** the shared service is reachable and the account is valid
- **THEN** new and modified data can be saved

#### Scenario: Writes are refused while unreachable, reads are not

- **WHEN** the shared service is unreachable
- **THEN** no new or modified data is accepted as saved
- **AND** data that was already loaded remains readable
- **AND** the cause is stated to the person
- **AND** the status is reported to the person as offline only after three consecutive reachability checks fail without an intervening success

#### Scenario: Reachability is re-evaluated after returning to the product [ios]

- **WHEN** a person brings the product back to the foreground after connectivity was restored
- **THEN** reachability is re-evaluated
- **AND** saving is possible again without reinstalling or resetting the product

#### Scenario: Reachability is re-evaluated while the product stays open [macos]

- **WHEN** connectivity is restored while the product remains running
- **THEN** reachability is re-evaluated without a relaunch
- **AND** saving is possible again

### Requirement: Conflict resolution never fails and never duplicates

**NFR-2.** Concurrent edits SHALL resolve last-write-wins. Resolution SHALL NOT fail an edit, SHALL
NOT report a conflict to the person as an error, and SHALL NOT leave more than one copy of the
affected item.

#### Scenario: Concurrent edits resolve without failing

- **WHEN** two members edit the same item without seeing each other's change
- **THEN** one version of the item remains, reflecting the edit saved last
- **AND** neither member's edit is reported as failed because of the conflict

#### Scenario: Resolution leaves no duplicate

- **WHEN** concurrent edits are resolved
- **THEN** the affected item exists exactly once for every member

### Requirement: Health and weight data stay private; diagnostics exclude content

**NFR-3.** Health and weight data SHALL NOT be sent to third-party analytics. Data SHALL be limited
to the product's own shared service and the copies kept locally on the person's devices. Diagnostic
records SHALL contain identifiers, counts and outcomes only, and SHALL NEVER contain rabbit names,
note text, photo content or weight values.

#### Scenario: Saved data reaches no third party

- **WHEN** health or weight data is saved
- **THEN** it is held only by the product's own shared service and the local copy
- **AND** no third-party analytics service receives it

#### Scenario: Diagnostics carry no content

- **WHEN** a diagnostic record is produced
- **THEN** it contains identifiers, counts and outcomes only
- **AND** it contains no rabbit name, no note text, no photo content and no weight value

### Requirement: Every failure path states its cause

**NFR-8.** Saves SHALL never silently fail and SHALL never appear to succeed when they did not.
Every failure path — save, invitation creation, deletion, export — SHALL produce a user-visible
message that names the specific cause rather than reporting only that something went wrong.

#### Scenario: A failed save is visible and specific

- **WHEN** a save fails
- **THEN** a user-visible message names the cause of the failure
- **AND** the change is not presented as applied

#### Scenario: No bare failure messages

- **WHEN** a state-changing operation of the product fails
- **THEN** the message identifies the specific cause
- **AND** it never only states that something went wrong
