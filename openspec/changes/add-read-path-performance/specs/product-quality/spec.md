# product-quality — delta for add-read-path-performance

Traces requirement **NFR-7** and no other.
Stories: US-QUAL-07. Direction adopted from IMP-3.
This is an ADDED-only delta; the existing `product-quality` requirements are neither modified nor
restated. Specs stay technology-agnostic: "the shared service", "the product" and its data only.

Boundedness is asserted as a **request count** — deterministic and environment-independent, so the
automated gate asserts it (tasks 5.1 and 5.8). The wall-clock figure is measured once on the
reference device and recorded as evidence, never as a CI assertion (design D6.7).

## ADDED Requirements

### Requirement: Bounded cold load and load-once reuse

**NFR-7.** A cold dashboard load SHALL be bounded: the data the dashboard presents SHALL be
obtained with a fixed set of remote requests — the household's collection and one check of the
shared service's status — whose number does NOT grow with the amount of data in the household or
with the number of rabbits it holds, and those requests SHALL NOT be arranged as a chain in which
each one waits for the previous one to finish. Already-loaded data SHALL be presented from the
already-loaded copy without a remote request while it is inside a freshness window of 30 seconds,
and that window SHALL be a single value applying to all loaded data alike. Opening a rabbit SHALL
NOT re-download data the dashboard has already loaded. A remote change SHALL invalidate only the
data it affects, and SHALL NOT force a full cold reload on the next visit.

#### Scenario: Cold dashboard load stays within a fixed request bound

- **WHEN** a person opens the dashboard for a household whose data has not been loaded and the shared service is reachable
- **THEN** the data the dashboard presents is obtained with exactly two remote requests: the household's collection and one check of the service status
- **AND** the number of remote requests is the same for a household holding one rabbit and a household holding many

#### Scenario: A cold load is not a serialised chain

- **WHEN** a cold dashboard load needs data from more than one area
- **THEN** those remote requests are in flight together
- **AND** no request waits for an earlier one to complete before it starts

#### Scenario: Already-loaded data is reused inside the freshness window

- **WHEN** the dashboard is opened again less than 30 seconds after the data was loaded
- **THEN** the already-loaded data is presented
- **AND** no remote request is made for data that is already loaded

#### Scenario: One freshness window applies to all loaded data

- **WHEN** several parts of the household's data were loaded together and any one of them is opened again inside the 30 seconds
- **THEN** that part is served from the already-loaded copy without a remote request
- **AND** the same 30 seconds applies to every part, so no part ages differently from another

#### Scenario: Data is refreshed once the freshness window has elapsed

- **WHEN** 30 seconds have elapsed since the data was loaded and the shared service is reachable
- **THEN** the loaded data is refreshed
- **AND** the refresh stays within the same fixed request bound as a cold load

#### Scenario: Already-loaded data is served while a refresh is outstanding

- **WHEN** data has already been loaded and the shared service is reachable but slow
- **THEN** the already-loaded data is presented without waiting for the refresh to complete
- **AND** the refresh replaces the presented data once it completes

#### Scenario: Opening a rabbit re-downloads nothing the dashboard already loaded

- **WHEN** the dashboard has loaded a rabbit's data and the person opens that rabbit inside the 30 seconds
- **THEN** no remote request is made for data that is already loaded
- **AND** the household's collection is not requested again
- **AND** the rabbit's screen is presented from the already-loaded data

#### Scenario: Opening a rabbit loads only the data that was not already loaded

- **WHEN** the person opens a rabbit whose data the dashboard did not load
- **THEN** only the data missing for that rabbit is requested
- **AND** data already loaded for the rest of the household is not requested again

#### Scenario: A remote change invalidates only the affected data

- **WHEN** a change made by another member affects one part of the household's data
- **THEN** only the data covering that part is invalidated
- **AND** the rest of the already-loaded data is reused without a remote request

#### Scenario: A remote change does not force a cold reload on the next dashboard visit [ios]

- **WHEN** a change from another member has already been applied, the product has been suspended, and the person returns to the dashboard
- **THEN** the already-loaded data is presented
- **AND** no full cold load is performed

#### Scenario: A remote change does not force a cold reload while the product stays open [macos]

- **WHEN** a change from another member is applied while the product remains running and the person returns to the dashboard
- **THEN** the already-loaded data is reused
- **AND** no full cold load is performed
