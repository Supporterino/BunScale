# product-quality — delta for add-read-path-performance

Traces requirement **NFR-7** and no other.
Stories: US-QUAL-07. Direction adopted from IMP-3.
This is an ADDED-only delta; the existing `product-quality` requirements are neither modified nor
restated. Specs stay technology-agnostic: "the shared service", "the product" and its data only.

## ADDED Requirements

### Requirement: Bounded cold load and load-once reuse

**NFR-7.** A cold dashboard load SHALL be bounded: the data the dashboard presents SHALL be
obtained with a fixed number of remote requests that does NOT grow with the amount of data in the
household, and those requests SHALL NOT be arranged as a chain in which each one waits for the
previous one to finish. Opening a rabbit SHALL NOT re-download data the dashboard has already
loaded. A remote change SHALL invalidate only the data it affects, and SHALL NOT force a full cold
reload on the next visit.

#### Scenario: Cold dashboard load stays within a fixed request bound

- **WHEN** a person opens the dashboard for a household whose data has not been loaded and the shared service is reachable
- **THEN** the data the dashboard presents is obtained with a fixed number of remote requests
- **AND** that number is the same for a household holding one rabbit and a household holding many

#### Scenario: A cold load is not a serialised chain

- **WHEN** a cold dashboard load needs data from more than one area
- **THEN** those remote requests are in flight together
- **AND** no request waits for an earlier one to complete before it starts

#### Scenario: Already-loaded data is reused inside the freshness window

- **WHEN** the dashboard is opened again inside the freshness window
- **THEN** the already-loaded data is presented
- **AND** no remote request is made for data that is already loaded

#### Scenario: Data is refreshed once the freshness window has elapsed

- **WHEN** the freshness window has elapsed and the shared service is reachable
- **THEN** the loaded data is refreshed
- **AND** the refresh stays within the same fixed request bound as a cold load

#### Scenario: Already-loaded data is served while a refresh is outstanding

- **WHEN** data has already been loaded and the shared service is reachable but slow
- **THEN** the already-loaded data is presented without waiting for the refresh to complete
- **AND** the refresh replaces the presented data once it completes

#### Scenario: Opening a rabbit re-downloads nothing the dashboard already loaded

- **WHEN** the dashboard has loaded a rabbit's data and the person opens that rabbit
- **THEN** no remote request is made for data that is already loaded
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
