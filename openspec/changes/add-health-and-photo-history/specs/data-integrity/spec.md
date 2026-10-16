# data-integrity — delta for add-health-and-photo-history

Traces requirement **DM-2** (and, through it, the tag clause of US-QUAL-09).
Stories: US-HL-09, US-QUAL-09 (tag clause).

The generic household-isolation rule is stated by `add-shared-household-foundation` (P0) as
`Household-scoped isolation and canonical storage at the boundary` (NFR-10) and is not restated here.
This requirement is the tag-catalog-specific statement of it, and it states **HL-6** once for the
whole change.

## ADDED Requirements

### Requirement: A health tag catalog belongs to exactly one household

**DM-2, HL-6.** Each household SHALL own exactly one copy of the canonical health categories, so that
establishing the catalog again — including by a second member or on a second device — creates no
duplicate category and every member of that household resolves the same single catalog. A category tag
SHALL NEVER be referenced by an entry of another household, and no household's catalog SHALL be
readable from another household. This is the tag-catalog statement of the household-scoping rule held
by `Household-scoped isolation and canonical storage at the boundary` (NFR-10).

#### Scenario: Each household owns exactly one copy of the canonical categories

- **WHEN** a household's health tag catalog is established
- **THEN** that household has one copy of each canonical category
- **AND** the catalog belongs to that household alone

#### Scenario: Establishing the catalog again creates no duplicate category

- **WHEN** the catalog is established again for a household that already has it — after reopening health logging, or from a second member or device
- **THEN** no second copy of a category is created
- **AND** the household still has exactly one copy of each canonical category

#### Scenario: A tag is never referenced by another household's entries

- **WHEN** two households each hold entries that carry category tags
- **THEN** no tag of one household is referenced by an entry of the other household
- **AND** no tag of one household is readable from the other household

#### Scenario: All members of a household resolve the same catalog

- **WHEN** two members of the same household each view the categories available to health logging
- **THEN** both resolve the same single set of categories for that household
