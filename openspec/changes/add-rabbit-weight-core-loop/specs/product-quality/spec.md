# product-quality — delta for add-rabbit-weight-core-loop

Traces requirement **NFR-9**.
Stories: US-QUAL-08.

P0 (`add-shared-household-foundation`) owns NFR-1, NFR-2, NFR-3 and NFR-8 in this capability. NFR-4
… NFR-7 belong to later phases. This change adds NFR-9 only and does not restate any of them.

## ADDED Requirements

### Requirement: Primary and destructive actions are visible and confirmed

**NFR-9.** Primary and destructive actions — including editing a rabbit profile, deleting a weight
entry and deleting a rabbit — SHALL be presented as directly visible controls. They SHALL NOT be
reachable only through an undiscoverable gesture such as a long press or a hidden context menu. A
destructive action SHALL require a confirmation that states its consequence before it takes effect.

#### Scenario: A primary action is a directly visible control

- **WHEN** a rabbit's detail or a weight entry's detail is presented
- **THEN** its edit action is a directly visible control
- **AND** no undiscoverable gesture is required to reach it

#### Scenario: A destructive action is not hidden behind a gesture

- **WHEN** a rabbit or a weight entry is presented
- **THEN** its delete action is a directly visible control
- **AND** it is not reachable only through a long press or another undiscoverable gesture

#### Scenario: A destructive action requires a confirmation stating the consequence

- **WHEN** a person triggers a destructive action such as deleting a rabbit or deleting a weight entry
- **THEN** a confirmation is presented before anything is deleted
- **AND** the confirmation states the consequence of the action

#### Scenario: Declining the confirmation changes nothing

- **WHEN** a person declines a destructive action's confirmation
- **THEN** the data remains unchanged
