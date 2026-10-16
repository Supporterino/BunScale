# medication-tracking — delta for add-vet-export-and-reminders

Traces requirements **MED-9, MED-10, MED-11** — **ADDED only**.
Stories: US-MED-17, US-MED-18, US-MED-19, US-MED-20, US-MED-21.

**Capability ownership.** This capability is created by `add-medication-tracking` (P5), which owns
**MED-1 … MED-8 and MED-12**: the medication entity, the four schedule types (`timesPerDay`,
`everyXDays`, `weekly`, `prn`), the human-readable schedule summary, and dose recording, editing,
deleting and the near-simultaneous-dose guard. P5 specifies no reminder behaviour. This delta adds
exactly MED-9, MED-10 and MED-11 on top of that model; **no P5 requirement is restated, modified
or removed here**, and no `## MODIFIED Requirements` section appears in this file.

Related, referenced and not restated:
- the reminder mechanism these three requirements are configurations of is
  `One reminder mechanism serves weigh reminders and medication reminders` in `reminders`
  (this same change, decision D3), which also owns the general rule that reminders are held per
  device and never enter a household's shared collection;
- which medications are active and which are finished is MED-2, stated by
  `add-medication-tracking` (P5), whose scope note hands the reminder consequences of that status
  here; because this change owns the single reminder mechanism (D3), the reminder consequence
  §6.7 attaches to finishing a course — that setting an end date of today finishes the course and
  cancels pending reminders, and that a finished medication generates no reminders — is stated
  **here**, in MED-9's requirement
  `Medication reminders are raised per device with record and snooze actions` below, as the
  cancellation of that medication's pending and scheduled reminder occurrences. P5 deliberately
  states the medication lifecycle only and does not state this;
- the pre-filled dose confirmation a reminder's "record the dose as given" action opens is MED-8,
  and the recorded dose's fields are MED-5 (P5);
- the human-readable schedule summary export presents is MED-4 (P5).

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
afterwards, while its recorded doses SHALL remain available.

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

- **WHEN** a medication's course is finished and a moment at which it was scheduled arrives afterwards
- **THEN** no reminder is raised for that medication at that moment or later

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
- **AND** the slot remains unsatisfied rather than being reported as given
- **AND** the reminder is raised again rather than being silently dropped

#### Scenario: Only the device that configured the reminder is reminded

- **WHEN** two members share a medication and one member configures a reminder for it
- **THEN** only the configuring member's device raises medication reminders for that medication
- **AND** the other member's device raises none and is not notified

### Requirement: A missed dose window raises a single follow-up reminder

**MED-10.** A medication reminder SHALL have an optional follow-up for a missed dose window, which
SHALL be **enabled by default**. When the follow-up is enabled and a slot has passed with no dose
recorded for it, exactly **one** follow-up reminder SHALL be raised for that window. When a dose
has been recorded for the window, no follow-up SHALL be raised. When the follow-up is disabled,
none SHALL be raised.

#### Scenario: A missed dose window raises a single follow-up

- **WHEN** a slot passes with no dose recorded for it and the follow-up is enabled
- **THEN** exactly one follow-up reminder is raised for that window

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
