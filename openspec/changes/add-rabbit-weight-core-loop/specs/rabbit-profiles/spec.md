# rabbit-profiles — delta for add-rabbit-weight-core-loop

Traces requirements **RAB-1, RAB-2, RAB-3, RAB-4, RAB-5, RAB-6, RAB-7, RAB-8, RAB-9, RAB-10**.
Stories: US-RAB-01 … US-RAB-10, US-DASH-09 (the add-first-rabbit entry part of RAB-4).

## ADDED Requirements

### Requirement: A rabbit profile records identifying details

**RAB-1.** A rabbit profile SHALL record the rabbit's name, an optional photo, an optional breed,
an optional date of birth that can be explicitly marked unknown, a sex, a neuter/spay status, and an
optional adoption date. The sex SHALL be recorded as one of `male`, `female` or `unknown`, and the
neuter/spay status SHALL be recorded as one of `intact`, `neutered`, `spayed` or `unknown`: a neutered
rabbit and a spayed rabbit SHALL be recorded distinctly rather than as one "fixed" value, and `unknown`
SHALL be permitted for both, as an unknown date of birth is. Fields the person does not supply SHALL
be recorded as absent rather than filled with invented values, and a profile SHALL be savable without
them. Where a profile carries a photo, that photo SHALL be presented as the rabbit's thumbnail wherever
the rabbit is presented; where it does not, a placeholder SHALL be presented instead. (Choosing a photo
from the device's photo library is PHOTO-2, owned by `add-health-and-photo-history`, P2; it is not
implemented here.)

#### Scenario: A profile is created with the supplied identifying details

- **WHEN** a person creates a rabbit profile supplying a name, a breed, a date of birth, a sex, a neuter status and an adoption date
- **THEN** the profile records each of those values
- **AND** the rabbit is afterwards presented with that name

#### Scenario: Optional fields may be left unset

- **WHEN** a person creates a rabbit profile supplying only a name and a sex
- **THEN** the profile is saved
- **AND** the fields that were not supplied are recorded as absent

#### Scenario: Date of birth may be marked unknown

- **WHEN** a person marks a rabbit's date of birth as unknown
- **THEN** the profile is saved without a birth date
- **AND** the profile presents the date of birth as unknown rather than as absent or as a placeholder date

#### Scenario: A neutered and a spayed rabbit are recorded distinctly

- **WHEN** one rabbit profile is created with the neuter status `neutered` and another with the neuter status `spayed`
- **THEN** each profile records its own value
- **AND** the two statuses are not recorded as the same value

#### Scenario: Sex and neuter status may each be recorded as unknown

- **WHEN** a person creates a rabbit profile with the sex `unknown` and the neuter status `unknown`
- **THEN** the profile is saved
- **AND** it records those values rather than an invented sex or neuter status

#### Scenario: A profile photo is presented as the rabbit's thumbnail

- **WHEN** a rabbit profile carries a photo and the rabbit is presented in a list or as a detail
- **THEN** that photo is presented as the rabbit's thumbnail

#### Scenario: A rabbit without a photo is presented with a placeholder

- **WHEN** a rabbit profile carries no photo
- **THEN** a placeholder is presented in place of a thumbnail
- **AND** no other rabbit's photo is presented in its place

### Requirement: A rabbit profile requires a non-blank name

**RAB-2.** Creating or updating a rabbit profile SHALL require a name containing at least one
non-whitespace character. A name that is empty or that consists only of whitespace SHALL be treated
as blank, SHALL be rejected, and SHALL NOT create or update a profile.

#### Scenario: An empty name is rejected

- **WHEN** a person submits a rabbit profile with an empty name
- **THEN** no profile is created or updated
- **AND** the person is prompted to supply a name

#### Scenario: A whitespace-only name is treated as blank

- **WHEN** a person submits a rabbit name consisting only of whitespace characters
- **THEN** the name is treated as blank and rejected
- **AND** no profile is created or updated

### Requirement: A household supports multiple rabbits with no fixed upper limit

**RAB-3.** A household SHALL support more than one rabbit and SHALL NOT impose a fixed upper limit
on the number of rabbits. Every rabbit SHALL belong to exactly one household, and a rabbit of one
household SHALL never be presented as a rabbit of another.

#### Scenario: Additional rabbits are created without restriction

- **WHEN** a household already holds rabbits and a person adds another rabbit with a valid name
- **THEN** the new rabbit is created
- **AND** no fixed upper limit prevents its creation

#### Scenario: Rabbits of different households are never mixed

- **WHEN** two households each hold rabbits
- **THEN** the dashboard of the active household presents only that household's rabbits
- **AND** no rabbit of the other household appears in it

### Requirement: Rabbit creation is reachable from the dashboard at all times

**RAB-4.** The rabbit-creation form SHALL be reachable from the dashboard whenever the dashboard is
presented. The dashboard's empty state SHALL include a prompt to add the first rabbit.

#### Scenario: The add control is available whenever the dashboard is shown

- **WHEN** the dashboard is presented, whether the household holds rabbits or none
- **THEN** a control that opens the rabbit-creation form is present

#### Scenario: The empty state includes an add-first-rabbit prompt

- **WHEN** the dashboard is presented and the household holds no rabbits
- **THEN** a prompt to add the first rabbit is presented
- **AND** it leads to the rabbit-creation form

### Requirement: Every field of an active rabbit's profile is editable and shared

**RAB-5.** Every field of an active rabbit's profile — including the rabbit's weight-drop threshold
— SHALL be editable. The edit action SHALL be a directly visible control on the rabbit's detail and
SHALL NOT be reachable only through a hidden gesture. A saved edit SHALL be part of the household's
shared data for every member. An archived rabbit's profile SHALL NOT be editable.

#### Scenario: The edit control is directly visible on the rabbit's detail

- **WHEN** an active rabbit's detail is presented
- **THEN** an edit control is directly visible on it
- **AND** it does not require a hidden gesture such as a long press to reach

#### Scenario: A saved edit including the threshold is shared with every member

- **WHEN** a person edits any field of an active rabbit's profile, including its weight-drop threshold, and saves
- **THEN** the change is saved to the household's shared data
- **AND** every member of the household resolves the edited profile

#### Scenario: An archived rabbit's profile is not editable

- **WHEN** an archived rabbit's detail is presented
- **THEN** no control that edits its profile is available

### Requirement: A rabbit is archivable without losing its history

**RAB-6.** A rabbit SHALL be archivable as a soft delete. An archived rabbit SHALL be hidden from
the active lists, its history SHALL be retained, and it SHALL be retrievable from an archived list.
Archiving SHALL be reversible and SHALL NOT delete any of the rabbit's data.

#### Scenario: Archiving removes the rabbit from the active list

- **WHEN** a person archives a rabbit
- **THEN** the rabbit no longer appears in the active rabbit list
- **AND** the archived list contains it

#### Scenario: An archived rabbit is retrievable with its history

- **WHEN** a person opens the archived list
- **THEN** the archived rabbit is listed
- **AND** its weight history is retrievable from it

### Requirement: An archived rabbit is restorable with its history intact

**RAB-7.** An archived rabbit SHALL be restorable to the active list, and restoring it SHALL return
it with its history unchanged and complete.

#### Scenario: Restoring returns the rabbit to the active list

- **WHEN** a person restores an archived rabbit
- **THEN** the rabbit appears again in the active rabbit list
- **AND** it no longer appears in the archived list

#### Scenario: Restoration leaves the history unchanged

- **WHEN** an archived rabbit that had weight entries is restored
- **THEN** those entries are still present and unchanged

### Requirement: An archived rabbit's timeline is read-only

**RAB-8.** An archived rabbit's timeline SHALL be viewable and read-only: no new entry SHALL be
addable to it, and its existing entries SHALL NOT be edited or deleted. Export actions (EXP-1 …
EXP-4, owned by `add-vet-export-and-reminders`, P4) and the photo-gallery action (PHOTO-3, owned by
`add-health-and-photo-history`, P2) SHALL remain available for an archived rabbit; those actions are
realised and verified by the changes that own them and are not implemented here.

#### Scenario: No new entry can be added to an archived rabbit

- **WHEN** an archived rabbit's timeline is presented
- **THEN** no control that creates a new weight entry is available for it

#### Scenario: Archived entries cannot be edited or deleted

- **WHEN** an entry of an archived rabbit is presented
- **THEN** no control that edits or deletes it is available

### Requirement: Archived rabbit data is retained by default

**RAB-9.** An archived rabbit's data SHALL be retained indefinitely by default. Archiving SHALL NOT
discard data, and the product SHALL NOT remove archived data through ageing or an automatic purge.

#### Scenario: Archived history survives an arbitrary interval

- **WHEN** an archived rabbit's history is opened a long time after archiving
- **THEN** the weight entries recorded before archiving are still present

#### Scenario: Archiving discards nothing

- **WHEN** a rabbit with weight entries is archived
- **THEN** every entry it had before archiving is still stored afterwards

### Requirement: An archived rabbit and its data are permanently deletable on confirmation

**RAB-10.** A rabbit that is **archived** — and only an archived rabbit — and all data associated with
it SHALL be permanently deletable after a confirmation that states the consequence. Permanent deletion
SHALL NOT be offered for, or accepted for, an active rabbit: an active rabbit SHALL present no
affordance for it, and such an attempt SHALL be refused. The deletion SHALL be irreversible and SHALL
be part of the household's shared data for every member. Until the confirmation is given, nothing
SHALL be deleted. (This deliberately **narrows** the source requirement RAB-10 in
`docs/BunScale-Reimplementation-Requirements.md`, which permits permanent deletion without stating a
precondition, while US-RAB-10's only case is an archived rabbit. Archival is the recoverable step that
must precede an irreversible action; the narrowing is intentional and MUST NOT be relaxed back to the
unqualified reading.)

#### Scenario: Confirmed deletion removes the rabbit and its data

- **WHEN** a person confirms permanent deletion of an archived rabbit
- **THEN** the rabbit is no longer present in any rabbit list
- **AND** its weight entries are no longer retrievable

#### Scenario: An active rabbit cannot be permanently deleted

- **WHEN** an active rabbit is presented, or permanent deletion of an active rabbit is attempted
- **THEN** no control that permanently deletes it is available
- **AND** the attempt is refused and the rabbit remains active

#### Scenario: Without confirmation nothing is deleted

- **WHEN** a person opens the deletion confirmation for an archived rabbit and does not confirm it
- **THEN** the rabbit and its data remain unchanged

#### Scenario: The deletion is shared with every member

- **WHEN** an archived rabbit is permanently deleted
- **THEN** every member of the household resolves the household without that rabbit
