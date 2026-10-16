# household-sharing — delta for add-rabbit-weight-core-loop

Traces requirement **HS-8**.
Stories: US-HS-14.

P0 (`add-shared-household-foundation`) owns HS-1 … HS-4 and DM-3 in this capability; P3
(`add-household-sharing-hardening`) owns HS-5 … HS-7 and HS-9 … HS-13. This change adds HS-8 only
and does not restate any of them.

## ADDED Requirements

### Requirement: All members have equal read/write access

**HS-8.** Every member of a household SHALL have equal read and write access to that household's
data. No permission tiers SHALL exist, and data SHALL NOT be restricted by which member created it:
a rabbit or a weight entry created by one member SHALL be readable, editable and deletable by any
other member. The single privilege asymmetry permitted by the product is ownership of the household,
which only gates permanent deletion of the household itself (HS-10, P3).

#### Scenario: A member edits data created by another member

- **WHEN** two members share a household and one of them edits a rabbit or a weight entry created by the other and saves
- **THEN** the edit is saved
- **AND** every member of the household resolves the edited data

#### Scenario: A joining member writes into the same shared data

- **WHEN** a member who joined the household rather than created it creates a rabbit or records a weight entry
- **THEN** the created data belongs to the same shared household data as that of every other member
- **AND** no permission tier hides it from or restricts it for the other members

#### Scenario: No member is offered a reduced view of household data

- **WHEN** any member of a household opens a rabbit or a weight entry
- **THEN** the data is presented with the same read and write capabilities as for every other member
- **AND** no capability is withheld because of the member's role in the household
