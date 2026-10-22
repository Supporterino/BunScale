# data-integrity — delta for add-shared-household-foundation

Traces requirements **NFR-10, DM-6**.
Stories: US-QUAL-09 (NFR-10).

## ADDED Requirements

### Requirement: Household-scoped isolation and canonical storage at the boundary

**NFR-10.** The storage boundary introduced by this change SHALL uphold the product's data-integrity
invariants: every household-scoped item SHALL belong to exactly one household and SHALL never be
readable from another household; a value that has a single canonical representation SHALL be stored
only in that representation, with any presentation conversion applied outside stored data; and a
write to an entity the boundary marks as immutable SHALL be refused.

#### Scenario: No item is readable from another household

- **WHEN** two households each hold data
- **THEN** no item of one household is readable from the other
- **AND** no household-scoped identifier of one household is referenced by the other

#### Scenario: Canonical representation survives presentation changes

- **WHEN** a stored value is later displayed under a different presentation setting
- **THEN** the stored value is unchanged
- **AND** no stored value is rewritten into a presentation unit

#### Scenario: Writes to immutable entities are refused

- **WHEN** a write targets an entity the storage boundary marks as immutable
- **THEN** the write is refused
- **AND** no stored data changes

### Requirement: Timestamps are absolute instants defaulting to now

**DM-6.** Timestamps SHALL be stored as absolute instants, never as offsets or local-time strings,
and SHALL default to the moment of creation when the person does not supply one. Where a capability
accepts a person-supplied timestamp, the supplied instant SHALL be stored unchanged, including
instants in the past.

#### Scenario: A timestamp defaults to now

- **WHEN** a timestamped item is created without a person-supplied time
- **THEN** the item records the moment of its creation

#### Scenario: A supplied instant is stored unchanged

- **WHEN** the storage boundary is given an explicit instant, including one in the past
- **THEN** that instant is stored and returned unchanged

#### Scenario: Stored instants do not shift with the device offset

- **WHEN** the device's timezone or UTC offset changes
- **THEN** the meaning of a stored timestamp is unchanged
- **AND** it is not re-interpreted as a local time
