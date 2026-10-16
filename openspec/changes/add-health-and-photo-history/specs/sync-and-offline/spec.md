# sync-and-offline — delta for add-health-and-photo-history

Traces requirement **SYNC-4**.
Stories: US-SYNC-07, US-SYNC-08.

SYNC-1, SYNC-2, SYNC-3 and SYNC-5 are owned by `add-shared-household-foundation` (P0) and are not
restated here. In particular, the rule that saves require the shared service and are never faked
(SYNC-2) already governs attaching and removing a photo, and the rule that reads use previously loaded
data while the service is unreachable (SYNC-3) already governs every non-photo read this change adds.

## ADDED Requirements

### Requirement: Viewed photos remain available offline

**SYNC-4.** A photo that has been displayed at least once SHALL be kept locally on the device and
indefinitely, until the household or the photo itself is removed, so that it can be displayed again
while the shared service is unreachable. That retained copy SHALL NOT be bounded by a size budget and
SHALL NOT be evicted to reclaim space, so the guarantee is unconditional: a photo that was displayed
once is available offline for as long as it exists. A photo that has never been displayed SHALL NOT be
available while the shared service is unreachable, and SHALL be presented as a "cannot load without
connectivity" placeholder instead; a photo that has never been displayed is the only case in which that
placeholder is reachable. This retained photo is a read copy of what the household's shared collection
holds; it SHALL NOT be treated as a local write, and changing which photos an entry has remains a save
governed by SYNC-2.

#### Scenario: A photo that was displayed once is shown again offline

- **WHEN** a photo was displayed at least once while the shared service was reachable
- **AND** the same photo is subsequently displayed while the shared service is unreachable
- **THEN** the photo is shown from the copy held on the device
- **AND** it is still shown no matter how much time has passed or how much other content the device has since stored, because that copy is never evicted

#### Scenario: A never-displayed photo shows a cannot-load placeholder offline

- **WHEN** a photo has never been displayed and the shared service is unreachable
- **THEN** a placeholder stating that the photo cannot load without connectivity is shown in its place
- **AND** this is the only case in which that placeholder is reachable: a photo that was displayed once is never replaced by it

#### Scenario: An entry whose photos cannot load is not presented as having no photos

- **WHEN** an entry has photos, none of which has ever been displayed, and the shared service is unreachable
- **THEN** the entry's photos are represented by the cannot-load placeholder
- **AND** the entry is not presented as if it had no photos
