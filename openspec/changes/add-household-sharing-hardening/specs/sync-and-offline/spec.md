# sync-and-offline — delta for add-household-sharing-hardening

Traces requirements **SYNC-6, SYNC-7**.
Stories: US-SYNC-10, US-SYNC-11.

Not in this delta: SYNC-1, SYNC-2, SYNC-3 and SYNC-5, owned by `add-shared-household-foundation`;
SYNC-4, owned by `add-health-and-photo-history`. This delta is **ADDED only**.

## ADDED Requirements

### Requirement: A household created or joined mid-session receives live updates without a restart

**SYNC-6.** A household that becomes available to a person while the product is running — because they
created it, or because they joined it by accepting an invitation — SHALL receive live updates for that
household immediately, without the product being restarted and without the person taking an additional
action to begin receiving them.

#### Scenario: A household created mid-session receives live updates without a restart

- **WHEN** a person creates a household while the product is running
- **THEN** live updates for that household are received immediately
- **AND** the product was not restarted to receive them
- **AND** a change another member makes afterwards reaches the person without any further action

#### Scenario: A household joined by accepting an invitation receives live updates without a restart

- **WHEN** a person joins a household by accepting an invitation while the product is running
- **THEN** live updates for that household are received immediately
- **AND** the product was not restarted to receive them
- **AND** a change another member makes afterwards reaches the person without any further action

### Requirement: Setting the product up again on the same account recovers the household's data

**SYNC-7.** When the product is set up again on the same account, the household's data SHALL be
recovered from the shared service — which is the source of truth — and presented as that household's
current data. Recovery SHALL NOT present an empty household, or the household-creation onboarding, in
place of data the shared service holds. When the shared service is unreachable and nothing was
previously loaded on that device, the explicit connectivity-required state SHALL be shown instead.

#### Scenario: Setting the product up again on the same account recovers the household

- **WHEN** the product is set up again on the same account that holds a household
- **THEN** the household is recovered from the shared service
- **AND** its data is presented as that household's current data

#### Scenario: Recovery does not present an empty household in place of the recovered data

- **WHEN** the product is set up again on the same account while the shared service is reachable
- **THEN** no empty household state is presented in place of the recovered household's data
- **AND** the household-creation onboarding is not presented while the household can be recovered

#### Scenario: Nothing is recovered without the shared service

- **WHEN** the product is set up again on the same account while the shared service is unreachable and
  no household data was ever loaded on that device
- **THEN** an explicit state stating that connectivity is required is shown
- **AND** no empty household is presented as the household's current data
