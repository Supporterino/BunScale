# household-sharing — delta for add-household-sharing-hardening

Traces requirements **HS-5, HS-6, HS-7, HS-9, HS-10, HS-11, HS-12**.
Stories: US-HS-06, US-HS-07, US-HS-08, US-HS-09, US-HS-10, US-HS-11, US-HS-12, US-HS-13, US-HS-15,
US-HS-16, US-HS-17, US-HS-18, US-HS-19, US-HS-20, US-HS-21, US-HS-22, US-HS-23, US-HS-24, US-HS-25.

Not in this delta: HS-1 … HS-4 and DM-3, owned by `add-shared-household-foundation`; HS-8 (equal
read/write access for all members), owned by `add-rabbit-weight-core-loop`; HS-13, an accepted risk
with no behaviour (see this change's `design.md`, risk R9). This delta is **ADDED only** — it
redefines no requirement another change owns.

## ADDED Requirements

### Requirement: Inviting a person is availability-checked, observable and never duplicated

**HS-5.** A member SHALL be able to invite another person to the household through an invitation.
Before an invitation is attempted, the product SHALL verify that the shared service is available and
SHALL NOT attempt the invitation when it is not. While an attempt is in progress the product SHALL give
progress feedback. On success it SHALL present the standard sharing options for the invitation. On
failure it SHALL state the specific cause. An attempt that does not complete promptly SHALL resolve
within a bounded time with feedback rather than remain unresolved, and SHALL NOT complete on its own
later. When an invitation for the household already exists, an attempt SHALL reuse it and SHALL NOT
create a second one.

#### Scenario: Inviting is refused before an attempt when the shared service is unavailable

- **WHEN** the shared service is unavailable and a member chooses to invite another person
- **THEN** no invitation attempt is made and no sharing options are presented
- **AND** the person is told that the shared service is unavailable

#### Scenario: An invitation attempt gives progress feedback

- **WHEN** a member chooses to invite another person while the shared service is available
- **THEN** progress feedback is shown while the attempt is in progress
- **AND** the product remains usable while the attempt runs

#### Scenario: A successful invitation presents the standard sharing options [ios]

- **WHEN** invitation creation succeeds on an iOS device
- **THEN** the device's standard sharing options are presented for the invitation
- **AND** the invitation can be delivered to the other person from there

#### Scenario: A successful invitation presents the standard sharing options [macos]

- **WHEN** invitation creation succeeds on a macOS device
- **THEN** the device's standard sharing options are presented for the invitation
- **AND** the invitation can be delivered to the other person from there

#### Scenario: An invitation failure states its specific cause

- **WHEN** invitation creation fails because connectivity is unavailable, because there is no valid
  account, because the owner's storage allowance is exceeded, or because the shared service is
  temporarily rate limiting
- **THEN** the message identifies that specific cause
- **AND** the person is offered a retry of the invitation

#### Scenario: A stalled invitation attempt resolves with a timeout failure

- **WHEN** an invitation attempt does not complete promptly
- **THEN** the attempt resolves with a failure message naming a timeout within 15 seconds, the same
  bounded time a save attempt is allowed
- **AND** the person is left with a stated outcome rather than an attempt that appears to hang
- **AND** the attempt does not complete on its own later without the person asking again

#### Scenario: An existing invitation is reused rather than duplicated

- **WHEN** the household already has an invitation and a member chooses to invite another person again
- **THEN** the existing invitation's sharing options are presented
- **AND** no second invitation is created for that household

### Requirement: Accepting an invitation adopts the household without a restart or an onboarding fallback

**HS-6.** Accepting an invitation SHALL make the recipient a member of that household with read/write
access to that household's data, including when the recipient has no household of their own. Once the
household's data becomes available to the recipient, the product SHALL present that household's
dashboard within a bounded time and without the recipient taking any further action, and SHALL NOT
present the household-creation onboarding merely because the data had not yet arrived. A household the
recipient creates while an invitation is still being resolved SHALL remain the active household. An
acceptance that cannot complete within the bounded time SHALL resolve with a failure naming its cause
and SHALL NOT be treated as accepted. Delivered invitation details SHALL NOT be discarded without a
trace: an acceptance that does not complete SHALL be recorded in diagnostics carrying identifiers and
outcomes only.

#### Scenario: Accepting an invitation on a fresh setup grants membership with read/write access

- **WHEN** a person who holds no household accepts an invitation with their own account
- **THEN** they are a member of the inviting household
- **AND** they can read and write that household's data to the same extent as its other members

#### Scenario: The adopted household's dashboard appears without further action

- **WHEN** acceptance has completed and the household's data becomes available to the recipient
- **THEN** that household's dashboard is presented within the bounded time
- **AND** the recipient is not required to take any further action to get it
- **AND** the product was not restarted to reach it

#### Scenario: Acceptance does not fall back to onboarding while data is still arriving

- **WHEN** acceptance completes while the household's data has not yet arrived
- **THEN** the household-creation onboarding flow is not presented
- **AND** the adopted household's dashboard is presented once its data arrives within the bounded time

#### Scenario: An acceptance that cannot complete resolves with a timeout failure

- **WHEN** an acceptance does not complete promptly — for example because the shared service becomes
  unreachable while it is being attempted
- **THEN** it resolves within the bounded time with a failure message naming a timeout
- **AND** the person is not treated as a member of that household
- **AND** nothing is written locally to be accepted later

#### Scenario: A household created while an invitation is still resolving stays active

- **WHEN** a person creates a household while an invitation they were given is still being resolved
- **THEN** the household they created is the active household
- **AND** it remains active when the invited household's data arrives

#### Scenario: A delivered invitation is never discarded without a trace

- **WHEN** invitation details have been delivered and acceptance does not complete
- **THEN** the non-completion and its cause are recorded in diagnostics using identifiers and outcomes
  only
- **AND** the delivered details are not discarded without such a record

### Requirement: A household holds at most six members

**HS-7.** A household SHALL hold at most six members, counted including the member who created it. An
invitation that would take a household beyond six members SHALL be blocked and the limit SHALL be
explained to the person attempting it.

#### Scenario: An invitation beyond the member cap is blocked with an explanation

- **WHEN** a household already holds six members and a member chooses to invite another person
- **THEN** no invitation for a further member is created
- **AND** the six-member limit is explained

#### Scenario: Accepting an outstanding invitation cannot take the household beyond six members

- **WHEN** a household holds six members and a person accepts an invitation that was issued earlier
- **THEN** the household still holds six members afterwards
- **AND** the person accepting is given an explanation of the limit instead of being admitted

### Requirement: The member list always includes the current person and survives resolution failures

**HS-9.** The member list SHALL always include the current person, labelled with a localized
"You"-style label. Each member SHALL be shown by their account display name when that resolves, falling
back to the locally known name and then to a localized placeholder when it does not. The list SHALL be
rendered before participants resolve, using the current person and any locally known members, and SHALL
refresh once participants resolve. When a participant refresh fails, the locally known members SHALL
remain listed and the failure SHALL be surfaced with its cause.

#### Scenario: The current person is listed with a localized You-style label

- **WHEN** a person opens the member list of a household that has never been shared with anyone else
- **THEN** that person is listed as a member of the household
- **AND** their entry is labelled with a localized "You"-style label rather than an unresolved name

#### Scenario: The account display name is shown when it resolves

- **WHEN** the current person's account identity resolves and they open the member list
- **THEN** their entry shows the account display name

#### Scenario: An unresolved display name falls back without breaking the list

- **WHEN** a member's account display name cannot be resolved
- **THEN** that member still appears in the list using the locally known name
- **AND** when no locally known name exists either, a localized placeholder is shown instead
- **AND** the list is still presented rather than failing to render

#### Scenario: The list renders before participants resolve and refreshes once they do

- **WHEN** a person opens the member list before the household's participants have resolved
- **THEN** the current person and any locally known members are listed immediately
- **AND** the list is refreshed once participants resolve

#### Scenario: A failed participant refresh keeps locally known members and states the cause

- **WHEN** a participant refresh fails while locally known members exist
- **THEN** those members remain listed
- **AND** the failure is surfaced together with its cause

### Requirement: Only the owner can permanently delete a household, and the consequence is stated

**HS-10.** Only the household owner SHALL be able to delete a household, and only through a directly
visible control on that household's row in Settings & Profile. The control SHALL NOT be offered for a
household the person joined. Deletion SHALL require a confirmation that states the full consequence,
including that the household's members lose access and that the data cannot be recovered. Confirmed
deletion SHALL be irreversible and SHALL remove the household together with its rabbits, its weight and
health entries, its photos, its medications, its doses, its members and its tags, SHALL withdraw access
from every member, and SHALL reach the household's members. After the active household is deleted, the
newest remaining household SHALL become active, or the household-creation onboarding SHALL be presented
when none remains. A deletion that does not take effect SHALL leave the household and its data intact
and SHALL state its cause.

#### Scenario: A joined household offers no delete control to a non-owner

- **WHEN** a person who joined a household through an accepted invitation views that household in
  Settings & Profile
- **THEN** no control to delete that household is offered

#### Scenario: The owner's delete control is directly visible on the household's Settings row

- **WHEN** the owner views their own household in Settings & Profile
- **THEN** a directly visible control to delete that household is present on the household's row
- **AND** it is reachable without a hidden gesture such as a long press

#### Scenario: The confirmation states the full consequence

- **WHEN** the owner selects the delete control for a household
- **THEN** a confirmation is presented before anything is deleted
- **AND** it states that the household's members lose access and that the data cannot be recovered

#### Scenario: Cancelling the confirmation deletes nothing

- **WHEN** the owner dismisses the deletion confirmation without confirming
- **THEN** the household and all of its data remain unchanged

#### Scenario: Confirmed deletion removes the household and all of its data

- **WHEN** the owner confirms deletion of a household that has rabbits, weight and health entries,
  photos, medications, doses, members and tags
- **THEN** the household and all of those are removed
- **AND** none of them is recoverable through the product afterwards

#### Scenario: Confirmed deletion withdraws access from every member and reaches them

- **WHEN** the owner confirms deletion of a shared household
- **THEN** no member of that household retains access to it
- **AND** the deletion reaches the household's members

#### Scenario: The newest remaining household becomes active after deletion

- **WHEN** the active household is deleted and the person still holds others
- **THEN** the newest remaining household is the active household
- **AND** its rabbits are the ones presented

#### Scenario: Onboarding is presented after the only household is deleted

- **WHEN** the only household a person holds is deleted
- **THEN** no household is active
- **AND** the household-creation onboarding is presented

#### Scenario: A failed deletion leaves the household intact and states its cause

- **WHEN** a deletion does not take effect
- **THEN** the household and all of its data remain intact
- **AND** the failure is surfaced with its specific cause

### Requirement: Changes reach every member without the product being opened manually

**HS-11.** A change saved by any member SHALL reach that household's other members within normal
shared-service propagation time, without those members refreshing manually and without them having to
open the product to fetch it. A change MAY be delivered in the background while the product is not
open, and a change delivered that way SHALL already be present when the product is next shown.

#### Scenario: A change made by one member appears for another without a manual refresh

- **WHEN** two members have the same household and one of them saves a change
- **THEN** the change appears for the other member
- **AND** the other member did not have to refresh or reload manually

#### Scenario: A change is delivered while the product is not open [ios]

- **WHEN** a member saves a change while the product is not open on another member's iOS device
- **THEN** the change is delivered to that device in the background without the member opening the
  product
- **AND** it is present when that member next opens the product

#### Scenario: A change is delivered while the product is not open [macos]

- **WHEN** a member saves a change while the product is not open on another member's macOS device
- **THEN** the change is delivered to that device in the background without the member opening the
  product
- **AND** it is present when that member next opens the product

#### Scenario: Every member of the household ends up with the same change

- **WHEN** three members hold the same household and one of them saves a change
- **THEN** each of the other two ends up with that change
- **AND** neither of them had to refresh or reload manually

### Requirement: Shared content counts against the household owner's storage allowance

**HS-12.** Content saved into a shared household SHALL count against the storage allowance of that
household's owner, whichever member saved it. A save that cannot complete because that allowance is
exhausted SHALL be refused and SHALL name the storage allowance as its cause rather than appearing to
have succeeded.

#### Scenario: Content saved by a non-owner counts against the owner's allowance

- **WHEN** a member who is not the household's owner saves content into a shared household
- **THEN** the storage that content consumes counts against the household owner's allowance
- **AND** not against the saving member's own allowance

#### Scenario: An exhausted owner allowance refuses a member's save with that cause

- **WHEN** the household owner's storage allowance is exhausted and a member saves new content
- **THEN** the save is refused
- **AND** the message names the storage allowance as the cause
- **AND** the content is not presented as saved
