# weight-tracking — delta for add-rabbit-weight-core-loop

Traces requirements **WT-1, WT-2, WT-3, WT-4, WT-5, WT-6, WT-7, WT-8**.
Stories: US-WT-01 … US-WT-15, US-DASH-10 (the quick weigh path of WT-2), US-SET-06 and US-WT-04
(the entry-side half of WT-3; the preference itself is SET-4, owned by P0).

## ADDED Requirements

### Requirement: A weight entry carries a value, a timestamp and an optional note

**WT-1.** A weight entry SHALL consist of a weight value, a timestamp and an optional note. The
timestamp SHALL default to the moment the entry is created and SHALL be editable by the person,
including to a time in the past.

#### Scenario: An entry is recorded with the current time

- **WHEN** a person saves a weight entry without changing the timestamp
- **THEN** the entry's timestamp is the moment of its creation

#### Scenario: An entry records an edited timestamp

- **WHEN** a person sets the timestamp of a weight entry to a time in the past and saves
- **THEN** the entry is stored with that timestamp

#### Scenario: The note is optional

- **WHEN** a person saves a weight entry without a note
- **THEN** the entry is saved
- **AND** it carries no note

### Requirement: Two quick paths reach a saved weight entry

**WT-2.** Two quick paths to a saved weight entry SHALL exist: a per-rabbit weigh control, and a
quick-entry control that offers a rabbit picker. From a freshly launched product, the per-rabbit
path SHALL reach a saved entry within three taps.

#### Scenario: The per-rabbit weigh control opens the form bound to that rabbit

- **WHEN** a person uses the weigh control belonging to a rabbit
- **THEN** the weight entry form opens bound to that rabbit
- **AND** saving records the entry for that rabbit

#### Scenario: The quick-entry control offers a rabbit picker

- **WHEN** a person uses the quick-entry control
- **THEN** a rabbit picker is offered
- **AND** choosing a rabbit opens the weight entry form bound to it

#### Scenario: A rabbit is weighed in at most three taps from launch [mobile]

- **WHEN** the product is freshly launched on a 390×844 viewport with the rabbit present on the dashboard
- **THEN** using that rabbit's weigh control, entering a value and saving records the entry in no more than three taps

### Requirement: Weight is entered and presented in the preferred unit without a unit picker

**WT-3.** A weight value SHALL be entered and presented in the person's preferred display unit and
SHALL be interpreted into the canonical stored unit when it is saved. The preference SHALL default
from the person's regional settings when they have not chosen one. The weight entry form SHALL NOT
offer a unit picker of its own. (Where the preference is chosen and how it is stored is SET-4,
owned by P0; it is not restated here.)

#### Scenario: A value entered in the preferred unit is saved in the canonical unit

- **WHEN** the preferred display unit is kilograms and a person enters a value in the weight entry form and saves
- **THEN** the entered value is interpreted from kilograms
- **AND** the entry is stored in the canonical unit

#### Scenario: Stored values are presented in the preferred unit

- **WHEN** weight values are presented anywhere in the product
- **THEN** each is shown in the person's preferred display unit

#### Scenario: The entry form offers no unit picker

- **WHEN** the weight entry form is presented
- **THEN** it offers no control for choosing the unit of the entered value

#### Scenario: The form uses the regional default when no unit was chosen

- **WHEN** a person has never chosen a display unit and opens the weight entry form
- **THEN** the form uses the unit derived from the device's regional settings

### Requirement: Non-positive values are rejected and implausible outliers are confirmed

**WT-4.** A weight value of zero or less SHALL be rejected and SHALL NOT be saved. A weight value
above the plausibility bound stated by WT-4 (10 kg) SHALL require an explicit confirmation from the
person before it is saved; until that confirmation is given, the entry SHALL NOT be saved.

#### Scenario: A zero value is rejected

- **WHEN** a person submits a weight value of zero
- **THEN** the entry is not saved
- **AND** the person is prompted for a valid value

#### Scenario: A negative value is rejected

- **WHEN** a person submits a negative weight value
- **THEN** the entry is not saved
- **AND** the person is prompted for a valid value

#### Scenario: A value above the plausibility bound requires confirmation

- **WHEN** a person submits a weight value above the plausibility bound
- **THEN** the value is not saved until the person confirms it
- **AND** the person is asked to confirm that the value is intended

#### Scenario: A confirmed outlier is saved

- **WHEN** a person confirms a value above the plausibility bound
- **THEN** the entry is saved with that value

### Requirement: A past weight entry is editable and deletable and the change is shared

**WT-5.** Any past weight entry SHALL be editable and deletable. A saved edit or deletion SHALL be
part of the household's shared data for every member. An entry of an archived rabbit SHALL NOT be
editable or deletable.

#### Scenario: A past entry is corrected and the change is shared

- **WHEN** a person edits the value of a past weight entry and saves
- **THEN** the entry is stored with the edited value
- **AND** every member of the household resolves the edited entry

#### Scenario: A past entry is deleted and the change is shared

- **WHEN** a person deletes a past weight entry and confirms
- **THEN** the entry is no longer present for any member of the household

### Requirement: Weight trend ranges and an independently ranged weight history

**WT-6.** The weight trend SHALL be presentable over selectable ranges of 7 days, 30 days, 90 days
and the full history, and a selected range SHALL limit the presented weight data to the entries
within that window. The weight-only history listing SHALL have its own range selection that is
independent of the trend's, so that changing one SHALL NOT change the other.

#### Scenario: Selecting a range limits the trend to that window

- **WHEN** a rabbit has weight entries spanning six months and the 30-day range is selected
- **THEN** only the entries of the last 30 days are presented in the trend

#### Scenario: The full-history range presents every entry

- **WHEN** the full-history range is selected
- **THEN** every weight entry of the rabbit is presented in the trend

#### Scenario: The weight history range is independent of the trend range

- **WHEN** the trend is set to 30 days and the weight-only history is set to 7 days
- **THEN** the weight-only history lists only the entries of the last 7 days
- **AND** the trend still presents the last 30 days

### Requirement: A weight drop beyond the threshold raises an alert and flags the rabbit

**WT-7.** A weight entry SHALL be flagged when the drop it represents against the previous entry
exceeds the rabbit's configurable threshold, whose default SHALL be 5%. The entries SHALL be ordered
by their timestamp to determine each entry's previous entry, and where two entries share a timestamp
the earlier-created entry SHALL be the previous entry for the later one, so that the same data yields
the same flag on every device. The flag SHALL be a derivation over the current entry set and SHALL NOT
be fixed when the entry is saved: the flag SHALL be recomputed whenever the rabbit's threshold
changes, so that it appears when the threshold is lowered below an existing drop and disappears when
the threshold is raised above it, and editing or deleting an entry SHALL re-evaluate the flags of the
entries around it — in particular, the entry that follows a deleted entry SHALL afterwards be compared
against the entry that now precedes it. A rabbit carrying a flagged entry SHALL also be flagged on the
dashboard (DASH-4), and that flag SHALL reflect the same derivation.

#### Scenario: A drop exceeding the threshold is flagged

- **WHEN** a rabbit's threshold is 5%, its previous entry is 2000 g and a new entry of 1800 g is saved
- **THEN** the new entry is presented with a visual drop warning

#### Scenario: A drop within the threshold is not flagged

- **WHEN** a rabbit's threshold is 5%, its previous entry is 2000 g and a new entry of 1950 g is saved
- **THEN** the new entry is presented without a drop warning

#### Scenario: Raising the threshold clears an existing flag

- **WHEN** a drop of 10% is flagged at a threshold of 5% and the rabbit's threshold is raised to 15%
- **THEN** the warning is no longer presented for that entry

#### Scenario: Lowering the threshold raises a flag

- **WHEN** a drop of 2.5% carries no warning at a threshold of 5% and the rabbit's threshold is lowered to 2%
- **THEN** the warning is presented for that entry

#### Scenario: Entries sharing a timestamp are ordered by creation

- **WHEN** two weight entries of a rabbit carry the same timestamp and the later-created one is otherwise lower
- **THEN** the earlier-created entry is the previous entry for the later-created one
- **AND** the drop of the later-created entry is measured against it

#### Scenario: Deleting an entry re-derives the following entry's alert

- **WHEN** a rabbit has three entries in timestamp order and the middle one is deleted
- **THEN** the remaining later entry is compared against the entry that now precedes it
- **AND** its flag is presented according to that comparison

#### Scenario: Editing an entry re-derives the alerts around it

- **WHEN** the value of a past weight entry is edited and saved
- **THEN** the flag of the edited entry itself is re-derived
- **AND** the flag of the entry that follows it is re-derived

#### Scenario: A flagged rabbit is flagged on the dashboard

- **WHEN** a rabbit carries an entry flagged by a drop beyond its threshold
- **THEN** the rabbit is flagged on the dashboard
- **AND** the dashboard flag disappears once no entry is flagged any more

### Requirement: Weight entry detail shows the value, timestamp, note, alert and photos

**WT-8.** A weight entry's detail SHALL show the value in the person's display unit, the timestamp,
the entry's full note without truncation, and the drop-alert flag when it applies. When the entry has
photos, its detail SHALL present them as a swipeable full-width pager; when it has none, no pager
SHALL be shown. The detail SHALL offer an Edit action that opens the entry form pre-filled, and a
Delete action that takes effect only after a confirmation. An entry of an archived rabbit SHALL be
presented without Edit and Delete.

#### Scenario: The detail shows the value in the display unit and the untruncated note

- **WHEN** a weight entry with a long note is opened in detail
- **THEN** the value is shown in the person's display unit
- **AND** the timestamp is shown
- **AND** the whole note is shown without truncation

#### Scenario: The drop-alert flag appears in the detail when it applies

- **WHEN** a weight entry that is flagged by a drop beyond the rabbit's threshold is opened in detail
- **THEN** the drop-alert flag is presented in the detail

#### Scenario: An entry's photos are presented as a swipeable pager [mobile]

- **WHEN** a weight entry with photos is opened in detail on a 390×844 viewport
- **THEN** its photos are presented as a full-width pager that can be swiped from one photo to the next

#### Scenario: An entry's photos are presented in the detail on desktop [desktop]

- **WHEN** a weight entry with photos is opened in detail on a 1280×800 viewport
- **THEN** its photos are presented in the detail
- **AND** they can be moved through without a touch gesture

#### Scenario: An entry with no photos shows no pager

- **WHEN** a weight entry with no photos is opened in detail
- **THEN** no photo pager is shown

#### Scenario: Edit opens the pre-filled form and Delete is confirmed

- **WHEN** a weight entry's detail is open and the person chooses Edit
- **THEN** the entry form opens pre-filled with the entry's value, timestamp and note
- **AND** choosing Delete instead takes effect only after a confirmation

#### Scenario: An archived rabbit's entry detail is read-only

- **WHEN** a weight entry of an archived rabbit is opened in detail
- **THEN** the value, timestamp and note are shown
- **AND** no Edit and no Delete action is available
