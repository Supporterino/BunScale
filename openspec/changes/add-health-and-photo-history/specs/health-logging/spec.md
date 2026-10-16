# health-logging — delta for add-health-and-photo-history

Traces requirements **HL-1, HL-2, HL-4, HL-5, HL-7**.
Stories: US-HL-01, US-HL-02, US-HL-03, US-HL-05, US-HL-06, US-HL-07, US-HL-08, US-HL-10, US-HL-11.

**Stated once, in a sibling capability of this change:** HL-3 (weight and health entries interleaved
by timestamp) is the `dashboard` requirement `Health-only and full combined history views`; HL-6 (the
household-scoped tag catalog) is the `data-integrity` requirement `A health tag catalog belongs to
exactly one household`. Neither is restated here.

## ADDED Requirements

### Requirement: A health entry records tags, severity, a note, a timestamp and optional photos

**HL-1.** A health entry SHALL consist of zero or more category tags, a severity, an optional note, an
editable timestamp that defaults to now, and zero or more photos. The category tags SHALL be drawn from
the canonical categories Appetite, Stool/Droppings, Activity/Behavior, Breathing, Eyes/Nose,
Coat/Grooming, Teeth, Posture/Gait and Other, and SHALL be the categories held by the household. The
severity SHALL be exactly one of Mild, Moderate or Severe. An entry SHALL be creatable with no note and
with no photo.

#### Scenario: A structured health entry is created with tags and a severity

- **WHEN** a person records a health entry for a rabbit, selects one or more category tags, sets the severity to Moderate, and saves
- **THEN** the entry is recorded for that rabbit with those tags
- **AND** its severity is Moderate

#### Scenario: An entry can be created with no note and no photo

- **WHEN** a person saves a health entry that has tags and a severity but no note and no photo
- **THEN** the entry is accepted
- **AND** the absent note and the absent photo are not presented as errors

#### Scenario: Severity is one of the three defined levels

- **WHEN** a person records a health entry
- **THEN** the severity recorded is Mild, Moderate or Severe
- **AND** no severity other than those three can be recorded

#### Scenario: The timestamp defaults to now and is editable

- **WHEN** a person opens the form for a new health entry
- **THEN** the timestamp is pre-filled with the present moment
- **AND** when the person changes it to a past instant and saves, the entry records that instant

### Requirement: A health entry with no tags is savable

**HL-2.** A health entry SHALL be savable with no category tags attached, so that a note alone is a
complete health entry. The absence of tags SHALL NOT prevent saving and SHALL NOT be presented as a
problem after saving.

#### Scenario: A note-only entry is accepted

- **WHEN** a person enters a note, selects no category tags, and saves
- **THEN** the entry is saved
- **AND** no tag is required for the save to complete

#### Scenario: A tagless entry displays without an error state

- **WHEN** a saved entry that has no tags is shown in a history view or in its detail view
- **THEN** it is shown as a valid health entry
- **AND** no missing-tag error is presented

### Requirement: Health entries are filterable by tag, severity and date range

**HL-4.** The health-only history view SHALL allow a rabbit's health entries to be restricted by
category tag, by severity and by date range, alone or in combination. Applying a filter SHALL only
narrow which entries are listed and SHALL NOT alter any stored entry. Removing a filter SHALL restore
the entries it had hidden.

#### Scenario: Filtering by tag shows only entries carrying it

- **WHEN** a rabbit has health entries carrying different category tags and the person filters by one of those tags
- **THEN** only the entries carrying that tag are listed

#### Scenario: Filtering by severity shows only that severity

- **WHEN** a rabbit has entries of more than one severity and the person filters by Severe
- **THEN** only the Severe entries are listed

#### Scenario: Filtering by date range restricts to that range

- **WHEN** a rabbit has entries spread over several months and the person sets a date range
- **THEN** only the entries whose timestamp falls inside that range are listed

#### Scenario: Filters can be combined

- **WHEN** a person applies a tag, a severity and a date range together
- **THEN** only the entries matching all three are listed

#### Scenario: Removing a filter restores the entries it hid

- **WHEN** a person removes a filter that had been applied
- **THEN** the entries that filter had removed from the list are listed again

### Requirement: A past health entry is editable and deletable, and the change is shared

**HL-5.** Any past health entry SHALL be editable and deletable by any member of the household, and the
change SHALL be shared with all members of the household through its shared collection.

#### Scenario: Editing a health entry is shared with members

- **WHEN** one member edits a past health entry and saves the change
- **THEN** the edited entry is what every member of the household sees

#### Scenario: Deleting a health entry removes it for every member

- **WHEN** a member deletes a past health entry
- **THEN** the entry is no longer listed for that member
- **AND** it is no longer listed for any other member of the household

#### Scenario: An archived rabbit's entry cannot be edited or deleted

- **WHEN** a write is attempted against a health entry that belongs to an archived rabbit
- **THEN** the write is refused
- **AND** the stored entry is unchanged

### Requirement: A health entry detail shows severity, all tags, the full note and its photos

**HL-7.** The detail view of a health entry SHALL show its severity, all of its category tags, its full
untruncated note, its timestamp and its photos. For a health entry that belongs to an archived rabbit,
the same information SHALL be presented read-only: no edit and no delete action SHALL be available, and
the entry SHALL remain viewable.

#### Scenario: The detail shows every recorded field

- **WHEN** a person opens the detail of a health entry that has a severity, several tags, a long note and a timestamp
- **THEN** the severity, all of the tags and the timestamp are shown
- **AND** the note is shown in full, without truncation

#### Scenario: The detail shows the entry's photos

- **WHEN** a person opens the detail of a health entry that has one or more photos
- **THEN** the entry's photos are shown in that detail

#### Scenario: An archived rabbit's health detail is read-only

- **WHEN** a person opens the detail of a health entry that belongs to an archived rabbit
- **THEN** the recorded information is shown
- **AND** no edit action and no delete action is available
