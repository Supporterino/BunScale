# product-quality — delta for add-household-sharing-hardening

Traces requirement **NFR-6**.
Stories: US-QUAL-04.

Not in this delta: NFR-1, NFR-2, NFR-3 and NFR-8, owned by `add-shared-household-foundation`. This
delta is **ADDED only** and does not restate the connectivity model of NFR-1: it states only what the
product is allowed to claim about the service's status.

The threshold asserted below is the settled product value — **three consecutive failures** — owned by
P0's design and consumed here rather than re-derived (see this change's `design.md`, D-H12).

## ADDED Requirements

### Requirement: Honest service status

**NFR-6.** A single transient failure SHALL NOT cause the product to present the shared service as
offline or the account as unavailable. "Offline" SHALL be presented only after three consecutive
failures, or after a direct check that the shared service or the account is unavailable. A status
carried over from a previous session SHALL NOT be presented as the current status before a fresh
outcome exists. Rapid consecutive updates SHALL be presented as a single stable "syncing" state rather
than flickering between states.

#### Scenario: A single transient failure does not show offline

- **WHEN** connectivity is working and a single update fails
- **THEN** the product does not present the shared service as offline

#### Scenario: Three consecutive failures show offline

- **WHEN** three consecutive updates fail
- **THEN** the product presents the shared service as offline
- **AND** the product does not present the shared service as offline before the third consecutive
  failure

#### Scenario: A direct check that the service or account is unavailable shows offline

- **WHEN** a direct check establishes that the shared service or the account is unavailable
- **THEN** the product presents that outcome, without waiting for the consecutive-failure threshold

#### Scenario: A status carried over from a previous session is not presented as current

- **WHEN** the product starts after a session in which the service was presented as offline
- **THEN** that previous outcome is not presented as the current status before a fresh outcome exists
- **AND** the status is presented as not yet determined until that fresh outcome is observed

#### Scenario: Rapid consecutive updates show one stable syncing state

- **WHEN** updates are in flight in rapid succession
- **THEN** a single stable "syncing" state is presented for the duration
- **AND** the presented status does not flicker between states for each update
