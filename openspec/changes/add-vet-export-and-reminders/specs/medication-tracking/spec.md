# medication-tracking — delta for add-vet-export-and-reminders

Traces requirements **MED-9, MED-10, MED-11** — **ADDED only**.
Stories: US-MED-17, US-MED-18, US-MED-19, US-MED-20, US-MED-21.

**Capability ownership.** This capability is created by `add-medication-tracking` (P5), which owns
**MED-1 … MED-8, MED-12 and MED-13**: the medication entity, the four schedule types (`timesPerDay`,
`everyXDays`, `weekly`, `prn`), the human-readable schedule summary, and dose recording, editing,
deleting and the near-simultaneous-dose guard, plus MED-13's correction path (referenced by id, not
restated), through which a course's status can change back from finished to active. P5 specifies no
reminder behaviour. This delta adds exactly MED-9, MED-10 and MED-11 on top of that model; **no P5
requirement is restated, modified or removed here**, and no `## MODIFIED Requirements` section
appears in this file.

Related, referenced and not restated:
- the reminder mechanism these three requirements are configurations of is
  `One reminder mechanism serves weigh reminders and medication reminders` in `reminders`
  (this same change, decision D3), which also owns the general rule that reminders are held per
  device and never enter a household's shared collection;
- which medications are active and which are finished is MED-2, stated by
  `add-medication-tracking` (P5), whose scope note hands the reminder consequences of that status
  here; because this change owns the single reminder mechanism (D3), the reminder consequence
  §6.7 attaches to a course's status — that setting an end date of today finishes the course and
  cancels pending reminders, that a finished medication generates no reminders **for as long as the
  course remains finished**, and that a finished course returned to active by correcting its end
  date (MED-13) has its upcoming occurrences scheduled again — is stated
  **here**, in MED-9's requirement
  `Medication reminders are raised per device with record and snooze actions` below, as the
  cancellation of that medication's pending and scheduled reminder occurrences while it stays
  finished, and their rescheduling once it is active again. P5 deliberately states the medication
  lifecycle only and does not state this, and MED-13 is referenced by id rather than restated;
- the pre-filled dose confirmation a reminder's "record the dose as given" action opens is MED-8,
  and the recorded dose's fields are MED-5 (P5);
- the human-readable schedule summary export presents is MED-4 (P5);
- the background execution window MED-10's follow-up reconciliation runs in is **consumed from
  `add-household-sharing-hardening` (P3)**: the same background mode/backing-task declaration and
  entitlement, the same native plugin and registration, and the same Rust handoff its HS-11
  requirement ("changes reach every member without the product being opened manually") establishes.
  Nothing here declares a second background mode, entitlement or plugin, and P3 archives before this
  change consumes it.

## ADDED Requirements

### Requirement: Medication reminders are raised per device with record and snooze actions

**MED-9.** For each upcoming slot of an **active** medication, a reminder SHALL be raised on the
device that configured it. The reminder SHALL offer an action to **record the dose as given**,
which opens the dose confirmation pre-filled from the medication's serving, and an action to
**snooze**. Recording the dose through that confirmation SHALL satisfy that medication's slot;
snoozing SHALL NOT record a dose and SHALL leave the slot unsatisfied. Upcoming slots SHALL be
derived from the medication's schedule for every schedule type that schedules doses. Medication
reminders SHALL be per device and SHALL NOT notify other members. When a medication's course is
**finished** — including when its end date is set to today — every occurrence of its reminder that
is still pending or scheduled SHALL be cancelled, so that no reminder is raised for that medication
for **as long as the course remains finished**, while its recorded doses SHALL remain available.
Conversely, when a finished course is returned to **active** by correcting its end date (MED-13),
its reminder occurrences SHALL be scheduled again for its upcoming slots, so that an active
medication raises reminders again at the slots its schedule defines.
A **snooze** SHALL defer the occurrence by a fixed **10 minutes** and MAY be repeated; a snooze
SHALL NOT record a dose and SHALL NOT satisfy the slot, and an occurrence whose snooze runs past the
slot's dose window SHALL leave that slot **missed**, so that MED-10's missed-dose follow-up still
applies to it.
Where the device's notification **authorization** is not granted — denied, not yet determined or
revoked — the product SHALL present that state honestly, SHALL keep the reminder configuration
usable, and SHALL NOT present a configured reminder as one that will fire. This is the same honesty
the product applies to service status, **extended from the service to notifications**: the state is
shown rather than assumed, and the configuration is never silently left looking as though it works.

#### Scenario: A reminder is raised for each upcoming slot of an active medication

- **WHEN** an active medication is scheduled twice a day at 08:00 and 20:00 and a reminder is configured for it on a device
- **THEN** that device raises a reminder at each upcoming slot
- **AND** no reminder is raised at a time the medication is not scheduled for

#### Scenario: Upcoming slots are derived for every schedule type that schedules doses

- **WHEN** a reminder is configured for a medication whose schedule is times-per-day, every-X-days or weekly
- **THEN** the reminder is raised for the slots that schedule defines
- **AND** every one of those schedule types produces its upcoming slots without failing

#### Scenario: No reminder is raised for a medication that is not active

- **WHEN** a medication is not active
- **THEN** no reminder is raised for it
- **AND** its recorded doses remain available

#### Scenario: Setting an end date of today cancels the medication's pending reminder occurrences

- **WHEN** a medication with a reminder configured has its end date set to today
- **THEN** the course is finished
- **AND** no occurrence of that medication's reminder remains pending or scheduled
- **AND** the medication's recorded doses remain available

#### Scenario: A finished course raises no further reminder

- **WHEN** a medication's course is finished and is still finished when a moment at which it was scheduled arrives afterwards
- **THEN** no reminder is raised for that medication at that moment or later
- **AND** its recorded doses remain available

#### Scenario: A course returned to active resumes its upcoming reminders

- **WHEN** a finished course's end date is corrected to a future date, or removed, so that the medication is active again (MED-13)
- **THEN** its reminder occurrences are scheduled again for its upcoming slots
- **AND** a reminder is raised at each of those upcoming slots
- **AND** its recorded doses remain available

#### Scenario: The reminder offers recording the dose as given and snoozing

- **WHEN** a medication reminder is raised
- **THEN** it offers an action to record the dose as given
- **AND** it offers an action to snooze

#### Scenario: Recording from the reminder opens the pre-filled confirmation

- **WHEN** the person chooses to record the dose as given from a medication reminder
- **THEN** the dose confirmation opens with the amount pre-filled from the medication's serving
- **AND** no dose is recorded until that confirmation is confirmed

#### Scenario: The confirmed dose satisfies the slot

- **WHEN** the person confirms the pre-filled confirmation opened from a reminder
- **THEN** the dose is recorded through the product's save path
- **AND** the slot the reminder was raised for is satisfied and raises nothing further

#### Scenario: Snoozing records no dose and leaves the slot unsatisfied

- **WHEN** the person chooses to snooze a medication reminder
- **THEN** no dose is recorded
- **AND** the occurrence is deferred by the fixed 10 minutes
- **AND** the slot remains unsatisfied rather than being reported as given
- **AND** the reminder is raised again rather than being silently dropped

#### Scenario: A snooze may be repeated without satisfying the slot

- **WHEN** the person snoozes a medication reminder and snoozes it again when it is raised
- **THEN** each snooze defers the occurrence by a further 10 minutes
- **AND** no dose is recorded on either occasion
- **AND** the slot remains unsatisfied

#### Scenario: A snooze that runs past the dose window leaves the slot missed

- **WHEN** a medication slot is snoozed and the deferral extends past that slot's dose window
- **THEN** the slot is left missed rather than satisfied
- **AND** the missed-dose follow-up still applies to that slot (MED-10)

#### Scenario: A reminder states honestly that notifications are not authorized

- **WHEN** the device's notification authorization is denied or has not been granted and reminders
  are configured on that device
- **THEN** the product presents that state where reminders are configured
- **AND** it does not present a configured reminder as one that will fire
- **AND** the reminder configuration remains usable

#### Scenario: Revoking authorization leaves the reminder configuration usable

- **WHEN** the device's notification authorization is revoked while reminders are configured
- **THEN** the product presents the new state honestly and stops presenting the reminders as ones that
  will fire
- **AND** the reminder configuration remains intact and can still be edited, disabled and deleted

#### Scenario: Only the device that configured the reminder is reminded

- **WHEN** two members share a medication and one member configures a reminder for it
- **THEN** only the configuring member's device raises medication reminders for that medication
- **AND** the other member's device raises none and is not notified

### Requirement: A missed dose window raises a single follow-up reminder

**MED-10.** A medication reminder SHALL have an optional follow-up for a missed dose window, which
SHALL be **enabled by default**. When the follow-up is enabled and a slot has passed with no dose
recorded for it, exactly **one** follow-up reminder SHALL be raised for that window — including when
the product has not been opened since the window passed, subject to the platform granting the product
background execution on that device. The product SHALL reconcile outstanding follow-ups while it is
suspended when the platform grants it a background execution window, so that a follow-up whose window
has since been satisfied by a dose recorded elsewhere is cancelled rather than raised. When a dose
has been recorded for the window, no follow-up SHALL be raised. When the follow-up is disabled,
none SHALL be raised.

#### Scenario: A missed dose window raises a single follow-up

- **WHEN** a slot passes with no dose recorded for it and the follow-up is enabled
- **THEN** exactly one follow-up reminder is raised for that window

#### Scenario: A missed dose window raises its follow-up while the product is not open [ios]

- **WHEN** a slot passes with no dose recorded for it, the follow-up is enabled, the product has not
  been opened since the slot passed, and the platform grants the product background execution
- **THEN** exactly one follow-up reminder is raised for that window without the product having been
  opened

#### Scenario: A dose recorded elsewhere cancels a pending follow-up while the product is not open [ios]

- **WHEN** a slot passes with no dose recorded on this device, a dose for that slot is recorded on
  another device, and the platform grants this device background execution without the product being
  opened
- **THEN** the product reconciles in that background execution window and cancels the follow-up for
  that window
- **AND** no follow-up reminder is raised for the satisfied window

#### Scenario: A dose recorded elsewhere cancels a pending follow-up while the product stays running [macos]

- **WHEN** a slot passes with no dose recorded on this device, a dose for that slot is recorded on
  another device, and the product keeps running on this device
- **THEN** the follow-up for that window is cancelled rather than raised

#### Scenario: The follow-up is enabled by default

- **WHEN** a person configures medication reminders for a medication without changing the follow-up setting
- **THEN** the follow-up is enabled, and a missed window raises a follow-up

#### Scenario: No follow-up is raised when the follow-up is disabled

- **WHEN** the person disables the follow-up for a medication reminder and a window passes without a dose
- **THEN** no follow-up reminder is raised

#### Scenario: A recorded dose prevents the follow-up

- **WHEN** a dose is recorded for the window before the follow-up would be raised
- **THEN** no follow-up reminder is raised for that window

### Requirement: PRN medications schedule no reminders

**MED-11.** A medication whose schedule is as-needed SHALL NOT schedule any reminder, whether or
not a maximum-per-day hint is configured, while its doses remain recordable on demand.

#### Scenario: A PRN medication schedules no reminder

- **WHEN** a reminder is configured or would be configured for an as-needed medication
- **THEN** no reminder is scheduled for it and none is raised at any time

#### Scenario: A PRN medication with a maximum-per-day hint schedules no reminder

- **WHEN** an as-needed medication carries a maximum-per-day hint
- **THEN** no reminder is scheduled or raised for it
- **AND** the hint is not treated as a schedule

#### Scenario: Doses for a PRN medication remain recordable on demand

- **WHEN** a person records a dose for an as-needed medication
- **THEN** the dose is recorded as any other dose is
- **AND** recording it depends on no reminder having been raised
