# data-integrity — delta for add-medication-tracking

Traces requirement **DM-5** (and, through it, the dose-giver clause of US-QUAL-09).
Stories: US-MED-12, US-MED-14, US-QUAL-09 (DM-5 clause).

This delta is **ADDED only**. It declares no `## MODIFIED Requirements`, `## REMOVED Requirements` or
`## RENAMED Requirements` section, and it states exactly one requirement. The requirements below are
referenced by it and are not restated or altered:

| Requirement | Owner |
|---|---|
| `Household-scoped isolation and canonical storage at the boundary` (NFR-10) | `add-shared-household-foundation` (P0) |
| `Timestamps are absolute instants defaulting to now` (DM-6) | `add-shared-household-foundation` (P0) |
| `Weight is stored in grams as the single canonical unit` (DM-1) | `add-rabbit-weight-core-loop` (P1) |
| `An archived rabbit is read-only everywhere` (DM-4) | `add-rabbit-weight-core-loop` (P1) |
| `A health tag catalog belongs to exactly one household` (DM-2) | `add-health-and-photo-history` (P2) |

## ADDED Requirements

### Requirement: A dose records the giver as a member of the household

**DM-5.** A dose SHALL record **exactly one giver**, and that giver SHALL be a **member of the same
household** as the dose's rabbit. A dose SHALL NOT be stored without a giver, and SHALL NOT record a
member of another household or a person who is not a member of that household. The giver SHALL be
attributed from **the member who recorded the dose** (MED-5) rather than chosen from a list of the
household's members, and SHALL be held as that member's identity rather than as a copied display name,
so that the dose can always be resolved to a member of the household. A dose's giver SHALL NOT be
alterable through the edits MED-6 permits, which are the dose's amount, its note and its timestamp.
(The household-scoping rule this relies on is NFR-10, owned by P0; the instant a dose carries is DM-6,
also P0's.)

#### Scenario: Every dose names a household member as its giver

- **WHEN** a member records a dose for a medication of their household
- **THEN** the dose records that member as its giver
- **AND** the giver is a member of the same household as the dose's rabbit

#### Scenario: A dose is never stored without a giver

- **WHEN** a dose is recorded, by any path
- **THEN** the stored dose carries exactly one giver
- **AND** no dose exists that names no member

#### Scenario: A giver is never a member of another household

- **WHEN** two households each hold doses recorded by their own members
- **THEN** no dose of one household names a member of the other household
- **AND** no dose names a person who is not a member of its own household

#### Scenario: The giver is not changed by the edits a dose permits

- **WHEN** a dose's amount, note or timestamp is edited
- **THEN** the dose's giver is unchanged
- **AND** the recorded attribution remains the member who recorded it

#### Scenario: A renamed member does not rewrite past doses

- **WHEN** a member's display name changes or cannot be resolved
- **THEN** every dose that member recorded still resolves to that member as its giver
- **AND** no dose's stored attribution changes
