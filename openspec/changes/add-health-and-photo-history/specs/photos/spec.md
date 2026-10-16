# photos — delta for add-health-and-photo-history

Traces requirements **PHOTO-1, PHOTO-2, PHOTO-3, PHOTO-4, PHOTO-5**.
Stories: US-PHOTO-01, US-PHOTO-02, US-PHOTO-03, US-PHOTO-04, US-PHOTO-05, US-PHOTO-06.

The weight entry a photo can attach to is defined by the weight-tracking capability
(`add-rabbit-weight-core-loop`, P1); this change adds the photo attachment itself, not the entry.

## ADDED Requirements

### Requirement: One or more photos can be attached to a weight entry or a health entry

**PHOTO-1.** One or more photos SHALL be attachable to a weight entry and to a health entry, and an
entry SHALL be savable without any photo. Attaching a photo SHALL add it to that entry, so that the
entry's photos are part of the same saved record as the entry's other fields.

#### Scenario: Photos are attached to a health entry

- **WHEN** a person records or edits a health entry and attaches one or more photos
- **THEN** those photos are part of that health entry

#### Scenario: Photos are attached to a weight entry

- **WHEN** a person records or edits a weight entry and attaches one or more photos
- **THEN** those photos are part of that weight entry

#### Scenario: An entry is savable without photos

- **WHEN** a person saves a health entry or a weight entry without attaching any photo
- **THEN** the entry is saved
- **AND** no photo is required for the save to complete

#### Scenario: Several photos can be attached to one entry

- **WHEN** a person attaches more than one photo to the same entry and saves
- **THEN** all of the attached photos are part of that entry

#### Scenario: The photo source is the device photo library on iOS [ios]

- **WHEN** a person attaching a photo on an iOS device opens the photo source
- **THEN** photos are chosen from the device's photo library
- **AND** when access to that library has not been granted, no photo is attached, the reason is shown, and the entry can still be saved without photos

### Requirement: A rabbit profile photo appears as its thumbnail across the product

**PHOTO-2.** A profile photo SHALL be settable for a rabbit and SHALL be shown as that rabbit's
thumbnail wherever the rabbit is presented, including where a rabbit with no profile photo shows a
non-photo placeholder instead. Replacing the profile photo SHALL change the thumbnail everywhere the
rabbit appears.

#### Scenario: A profile photo appears as the rabbit's thumbnail across the product

- **WHEN** a rabbit has a profile photo
- **THEN** that photo is shown as its thumbnail in the rabbit list and on the rabbit's own dashboard

#### Scenario: A rabbit without a profile photo shows a placeholder

- **WHEN** a rabbit has no profile photo
- **THEN** a non-photo placeholder is shown in place of its thumbnail
- **AND** no broken or empty photo presentation is shown

#### Scenario: Replacing a profile photo changes the thumbnail

- **WHEN** a rabbit's profile photo is replaced with another photo
- **THEN** the new photo is the rabbit's thumbnail wherever the rabbit is presented

### Requirement: A per-rabbit gallery shows the rabbit's photos ordered by entry date

**PHOTO-3.** A per-rabbit gallery SHALL show all of the rabbit's photos — the photos attached to its
entries and its profile photo — with the photos attached to entries ordered by the date of the entry
they belong to. When the rabbit has a profile photo, that photo SHALL appear first, before every photo
attached to an entry, and the entry photos SHALL follow it. The gallery SHALL remain reachable and
available for an archived rabbit.

#### Scenario: The gallery shows the rabbit's photos

- **WHEN** a person opens a rabbit's gallery and that rabbit has a profile photo and photos attached to several entries
- **THEN** all of those photos are shown

#### Scenario: Entry photos are ordered by entry date

- **WHEN** the gallery lists a rabbit's photos and that rabbit has a profile photo
- **THEN** the profile photo is shown first, before every photo attached to an entry
- **AND** the photos attached to entries follow it, ordered by the date of the entry each photo belongs to

#### Scenario: The gallery remains available for an archived rabbit

- **WHEN** a person opens the gallery of an archived rabbit from its timeline
- **THEN** the gallery is available
- **AND** the rabbit's photos are shown

### Requirement: Photos are shared with all household members through the shared collection

**PHOTO-4.** A photo attached to an entry, and a rabbit's profile photo, SHALL be part of the
household's shared collection, so that every member of the household sees the same photos for the same
entry and the same rabbit. A photo SHALL NOT be held only on the device that attached it.

#### Scenario: A photo attached by one member is seen by another

- **WHEN** one member attaches a photo to an entry and the change reaches the household's shared collection
- **THEN** every other member of the household sees that photo on that entry

#### Scenario: Members see the same profile photo

- **WHEN** a member sets a rabbit's profile photo
- **THEN** every member of the household sees that photo as the rabbit's thumbnail

### Requirement: An entry's photos are shown as a swipeable pager in its detail view

**PHOTO-5.** The detail view of an entry that has photos SHALL present them as a swipeable, full-width
pager. Selecting a photo thumbnail in a timeline row SHALL open that entry's detail view rather than a
separate photo viewer. An entry that has no photos SHALL present no pager.

#### Scenario: The detail shows a swipeable full-width pager

- **WHEN** a person opens the detail view of an entry that has more than one photo
- **THEN** the photos are shown as a full-width pager
- **AND** the person can move from one photo to the next within the same detail view

#### Scenario: Selecting a timeline thumbnail opens the detail view

- **WHEN** a person selects a photo thumbnail shown in a timeline row
- **THEN** that entry's detail view opens
- **AND** no separate full-screen photo viewer is opened in its place

#### Scenario: An entry with no photos shows no pager

- **WHEN** a person opens the detail view of an entry that has no photos
- **THEN** no photo pager is shown

#### Scenario: The pager is operable without a swipe gesture [desktop]

- **WHEN** the pager is shown on a device with a fine pointer
- **THEN** a directly visible control moves to the next and previous photo
- **AND** the photos can be paged through without performing a swipe gesture
