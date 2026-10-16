# data-integrity — delta for add-rabbit-weight-core-loop

Traces requirements **DM-1, DM-4**.
Stories: US-QUAL-09 (the DM-1 and DM-4 clauses of it), US-WT-04, US-WT-05 (DM-1), US-RAB-09 (DM-4).

P0 (`add-shared-household-foundation`) owns NFR-10, the generic boundary rule that a canonical value
is stored only in its canonical form and that a write to an entity the boundary marks as immutable is
refused, together with DM-6. The two requirements below state the weight-specific and
archive-specific observable behaviour; they do not restate NFR-10 and do not restate DM-6.

## ADDED Requirements

### Requirement: Weight is stored in grams as the single canonical unit

**DM-1.** A weight value SHALL be stored in grams, and grams SHALL be the only stored representation
of a weight. The display unit SHALL be a presentation concern only: conversion SHALL happen at the
point where a weight is entered or displayed and SHALL NOT be persisted, and no weight entry SHALL
carry a unit of its own.

#### Scenario: Every stored weight is in grams

- **WHEN** a weight value is recorded by any path
- **THEN** the stored value is expressed in grams

#### Scenario: A value entered in kilograms is stored as grams

- **WHEN** the display unit is kilograms and a person enters a value in that unit and saves
- **THEN** the entered value is converted to grams
- **AND** the stored value is the gram equivalent

#### Scenario: A weight entry carries no unit of its own

- **WHEN** a weight entry is stored
- **THEN** it carries a value in the canonical unit and no per-entry unit
- **AND** no entry can disagree with another entry about the unit of its value

#### Scenario: Changing the display unit leaves stored weights unchanged

- **WHEN** the person changes the display unit
- **THEN** the visual presentation of weights changes
- **AND** every stored weight value is unchanged

### Requirement: An archived rabbit is read-only everywhere

**DM-4.** An archived rabbit SHALL be read-only: while it is archived, its weight, health and
medication data SHALL NOT be created, edited or deleted, wherever that data is reached from. The
archive SHALL be retrievable, and archived data SHALL be retained by default. Restoring the rabbit
SHALL return it to the active state with its history intact (RAB-7, RAB-9). (The generic refusal rule
for entities the storage boundary marks as immutable is NFR-10, owned by P0; the read-only
presentation of the timeline is RAB-8. Health and medication data, and the verification of their
read-only behaviour, belong to the phases that own those capabilities.)

#### Scenario: No weight data of an archived rabbit is mutable

- **WHEN** an archived rabbit's weight data is reached, from any entry point in the product
- **THEN** no weight entry of that rabbit can be created, edited or deleted

#### Scenario: Archived health and medication data are not mutable

- **WHEN** an archived rabbit's health or medication data is reached
- **THEN** none of it can be created, edited or deleted

#### Scenario: The archive is retrievable and its data is retained by default

- **WHEN** a rabbit is archived
- **THEN** the rabbit remains retrievable from the archived list
- **AND** none of its weight data is discarded by archiving

#### Scenario: Restoring returns the rabbit with its history intact

- **WHEN** an archived rabbit is restored
- **THEN** the rabbit is active again
- **AND** its weight history is intact and writable again
