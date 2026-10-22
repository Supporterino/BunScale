# household-sharing — delta for add-shared-household-foundation

Traces requirements **HS-1, HS-2, HS-3, HS-4, DM-3**.
Stories: US-HS-01, US-HS-02, US-HS-03, US-HS-04, US-HS-05.

## ADDED Requirements

### Requirement: Create a household with a non-blank name

**HS-1.** A person SHALL be able to create a household by supplying a non-blank name. The creator
SHALL become the household's first member, and the created household SHALL become the active
household. A name that is empty or that contains only whitespace SHALL be treated as blank and
rejected, and no household SHALL be created.

#### Scenario: Create the first household

- **WHEN** a person with no household submits a valid, non-blank name during first-run onboarding
- **THEN** a household is created with that name
- **AND** the person is recorded as its first member
- **AND** that household is the active household

#### Scenario: Blank name is rejected

- **WHEN** a person submits an empty name, or a name consisting only of whitespace characters
- **THEN** no household is created
- **AND** the person is prompted to supply a name

### Requirement: First-run household creation onboarding runs once

**HS-2.** A household-creation onboarding flow SHALL be presented when the person has no
household. Once at least one household exists for the person, the onboarding flow SHALL NOT be
presented again.

#### Scenario: Onboarding shown when no household exists

- **WHEN** the product starts and the person has no household
- **THEN** the household-creation onboarding flow is presented

#### Scenario: Onboarding not repeated once a household exists

- **WHEN** the product starts and at least one household exists for the person
- **THEN** the active household's dashboard is presented
- **AND** no household-creation onboarding flow is presented

### Requirement: Multiple households with exactly one active

**HS-3.** A person SHALL be able to hold more than one household, each with its own members and
its own data, and SHALL be able to choose which household is active. Data belonging to one
household SHALL NOT be presented as data of another household.

#### Scenario: A second household coexists without mixing data

- **WHEN** a person who already holds a household creates another household
- **THEN** both households exist for that person, each with its own members
- **AND** no member or item of one household appears as a member or item of the other

#### Scenario: Switching the active household changes the presented data

- **WHEN** a person holding two households selects the one that is not active
- **THEN** the selected household becomes the active household
- **AND** the data presented as active is that household's data and no longer the previously active household's data

### Requirement: Household management from Settings & Profile

**HS-4.** Settings & Profile SHALL list every household the person holds, SHALL indicate which of
them is active, SHALL allow switching the active household, and SHALL allow creating an additional
household. Creating an additional household this way SHALL NOT re-run first-run onboarding.

#### Scenario: All households are listed with the active one indicated

- **WHEN** a person holding two households opens Settings & Profile
- **THEN** both households are listed
- **AND** the active household is indicated as active

#### Scenario: Create an additional household without onboarding

- **WHEN** a person creates a household from Settings & Profile with a non-blank name
- **THEN** the household is created with that person as its first member and becomes active
- **AND** the first-run household-creation onboarding flow is not presented

### Requirement: Exactly one active household per person

**DM-3.** At most one household SHALL be active for a given person at any time. When the set of
households a person holds changes such that the previously active household is no longer held, the
person SHALL again have exactly one active household, or none when no household remains.

#### Scenario: Selecting a household leaves exactly one active

- **WHEN** a person holding more than one household selects a different household as active
- **THEN** exactly one household is the active household afterwards
- **AND** the household that was active before is no longer indicated as active

#### Scenario: Active household is replaced when it is no longer held

- **WHEN** the household that was active is no longer among the households the person holds
- **THEN** exactly one of the remaining households becomes active
- **AND** when no household remains, no household is active and the creation onboarding is presented
