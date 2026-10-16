# sync-and-offline — delta for add-shared-household-foundation

Traces requirements **SYNC-1, SYNC-2, SYNC-3, SYNC-5**.
Stories: US-SYNC-01, US-SYNC-02, US-SYNC-03, US-SYNC-04, US-SYNC-05, US-SYNC-06, US-SYNC-09.

## ADDED Requirements

### Requirement: One logical collection per household

**SYNC-1.** All of a household's data SHALL be held as one logical collection per household and
SHALL be shared across that household's members, with exactly one copy of each item for every
member. Loading or refreshing the collection SHALL NOT produce a second copy of an item that is
already present.

#### Scenario: Every member resolves the same items

- **WHEN** two members of the same household each view the household's data
- **THEN** both see the same set of items
- **AND** each item is presented exactly once for each of them

#### Scenario: Refreshing does not duplicate an item

- **WHEN** a household's collection is loaded again after it was already loaded, or a write the
  service already accepted is re-issued with the same client-supplied request identity
- **THEN** items that were already present appear once, not twice
- **AND** the collection holds exactly one copy of every item

### Requirement: Saves require the shared service and never pretend to succeed

**SYNC-2.** New or modified data SHALL be saved only while the shared service is reachable and the
account is valid. A save that cannot complete SHALL NOT be silently discarded and SHALL NOT be
presented as saved. The person SHALL be told the specific cause — connectivity unavailable, no
account, storage allowance exceeded, temporarily rate limited — and SHALL be offered a retry.

#### Scenario: Save refused without connectivity

- **WHEN** the shared service is unreachable and a person attempts to save new or modified data
- **THEN** nothing is written to the shared collection
- **AND** the person is told that connectivity is required
- **AND** the attempted data is not presented as saved

#### Scenario: Save succeeds when the service is reachable

- **WHEN** the shared service is reachable and the account is valid and a person saves new or modified data
- **THEN** the save completes
- **AND** the change is part of the household's shared collection for every member

#### Scenario: Save failure names its cause and offers a retry

- **WHEN** a save fails because the account is unavailable, the storage allowance is exceeded, or the service is temporarily rate limiting
- **THEN** the message identifies that specific cause
- **AND** an exhausted owner storage allowance is named as its own cause, never as a generic write failure
- **AND** a retry of the save is offered

### Requirement: Reads use previously loaded data when the service is unreachable

**SYNC-3.** Reads SHALL be served from previously loaded data while the shared service is
unreachable. A start with connectivity SHALL refresh that previously loaded data. When no data was
ever loaded and the service is unreachable, the product SHALL show an explicit
connectivity-required state and SHALL NOT present stale or empty data as current.

#### Scenario: Previously loaded data is shown while unreachable

- **WHEN** the product starts while the shared service is unreachable and the household's data was loaded before
- **THEN** the previously loaded data is shown

#### Scenario: A start with connectivity refreshes loaded data

- **WHEN** the product starts while the shared service is reachable
- **THEN** the household's data is refreshed from the shared service
- **AND** the refreshed data replaces the previously loaded data

#### Scenario: Nothing loaded and unreachable

- **WHEN** the product starts while the shared service is unreachable and no household data was ever loaded on that device
- **THEN** an explicit state stating that connectivity is required is shown
- **AND** no empty or stale data is presented as the household's current data

### Requirement: Concurrent edits converge last-write-wins

**SYNC-5.** Concurrent edits to the same item SHALL converge on a single version of that item
using last-write-wins, without creating a duplicate and without a member's edit failing because of
the conflict.

#### Scenario: Two members edit the same item

- **WHEN** two members save a change to the same item and both changes reach the shared service
- **THEN** exactly one version of the item remains
- **AND** it reflects the change that reached the shared service last

#### Scenario: Convergence does not duplicate or fail

- **WHEN** concurrent edits to the same item are resolved
- **THEN** no second copy of the item is created
- **AND** neither member is shown a failure caused by the conflict
