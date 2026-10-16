# reminders — delta for add-vet-export-and-reminders

Traces requirements **REM-1, REM-2, REM-3** plus the unified-mechanism requirement named in this
delta (the expression of decision **D3**, resolving IMP-5).
Stories: US-REM-01, US-REM-02, US-REM-03, US-REM-04; the mechanism requirement is exercised by
US-MED-17, US-MED-18 and US-MED-21 as well.
Related, referenced and not restated: this capability is created by this change; the medication
slots a medication reminder is raised for are MED-9 … MED-11 (specified in this same change under
`medication-tracking`); that a per-device preference never enters the household's shared
collection is P0's `Household management from Settings & Profile` boundary rule and its
`Display weight unit is per-device and applied at the boundary` requirement; and the background
execution window the follow-up reconciliation runs in is **consumed from
`add-household-sharing-hardening` (P3)** — the background mode/backing-task declaration, the native
plugin, its registration and the Rust handoff that P3's HS-11 requirement establishes for background
delivery. This delta declares no second background mode, entitlement or plugin, and P3 archives before
this change consumes it.

## ADDED Requirements

### Requirement: One reminder mechanism serves weigh reminders and medication reminders

**D3 (unified mechanism).** A single scheduling and follow-up mechanism SHALL serve both the
recurring weigh reminder and medication slot reminders. Reminders of both kinds SHALL be created,
held, scheduled, delivered and followed up through that one mechanism; the two kinds SHALL differ
only in their configuration and never in a separate system, a separate persistence shape or a
separate follow-up path. A reminder SHALL record its kind, its target (a rabbit or the household
for a weigh reminder; a medication for a medication reminder), the person and device that
configured it, its schedule, whether its follow-up is enabled, and whether the reminder itself is
enabled. Reminders SHALL be held per person and per device, SHALL survive a relaunch on that
device, and SHALL NOT be part of any household's shared data.

#### Scenario: Both kinds of reminder are created and listed through one mechanism

- **WHEN** a person configures a recurring weigh reminder and a medication reminder on the same device
- **THEN** both appear in the one list of reminders held for that device
- **AND** neither is held, scheduled or followed up by a second mechanism

#### Scenario: Both kinds carry the same configuration and the same day-to-day operations

- **WHEN** the reminders held for a device are presented
- **THEN** each reminder of either kind exposes its target, its schedule, whether its follow-up is enabled and whether it is enabled
- **AND** each reminder of either kind can be changed, disabled and deleted in the same way

#### Scenario: Reminders live on the device that created them and survive a relaunch

- **WHEN** a reminder is configured on a device and the product is started again on that device
- **THEN** the reminder is still configured
- **AND** it is not part of the data any household member loads

#### Scenario: A reminder fires while the product is not open [ios]

- **WHEN** a configured reminder falls due on an iOS device while the product is not open
- **THEN** the reminder is raised by the device without the product being open

#### Scenario: A reminder fires while the product stays running [macos]

- **WHEN** a configured reminder falls due on a macOS device while the product remains running
- **THEN** the reminder is raised without the product being relaunched

### Requirement: A recurring weigh reminder is configurable per rabbit or household-wide

**REM-1.** A recurring weigh reminder SHALL be configurable with a user-defined recurring schedule
(for example "every Sunday 10:00"), either for one rabbit or for the whole household. A
household-wide reminder SHALL cover the household's rabbits rather than a single rabbit. A
configured reminder SHALL be changeable, disableable and deletable, and its schedule SHALL be
honoured without the person re-creating it.

#### Scenario: A reminder configured for a recurring day and time fires on schedule

- **WHEN** a person configures a weigh reminder for Sundays at 10:00
- **THEN** the reminder is raised on that schedule, repeatedly, without being re-created

#### Scenario: A household-wide reminder covers the household rather than one rabbit

- **WHEN** a person configures a household-wide reminder
- **THEN** the reminder applies to the household rather than to a single rabbit
- **AND** the reminder is not attached to one rabbit's data

#### Scenario: An edited reminder is honoured with its new schedule

- **WHEN** a person changes a reminder's schedule
- **THEN** the reminder is raised on the new schedule
- **AND** it is no longer raised on the previous one

#### Scenario: A disabled or deleted reminder raises nothing

- **WHEN** a person disables or deletes a reminder
- **THEN** no further reminder is raised for it

### Requirement: Reminders are scoped to the person and device that created them

**REM-2.** A reminder SHALL be scoped to the person and the device that created it. It SHALL NOT
be shared with other members, SHALL NOT notify another member's device, and SHALL NOT be affected
by another member's reminders or by that member's changes to their own. Every member SHALL be able
to configure their own reminders independently.

#### Scenario: Only the creating person's device is reminded

- **WHEN** a person configures a reminder and it falls due
- **THEN** that person's device raises it
- **AND** no other person's device raises it

#### Scenario: Another member is never notified by a reminder they did not create

- **WHEN** members share a household and one member has configured a reminder
- **THEN** the other member receives no notification from it at any time

#### Scenario: A second member configures an independent reminder

- **WHEN** a second member configures a reminder while the first member's reminder exists
- **THEN** both reminders exist independently
- **AND** neither replaces nor suppresses the other

#### Scenario: One person's reminder change leaves another member's reminder untouched

- **WHEN** a person edits or deletes their own reminder
- **THEN** the reminders of every other member are unchanged

#### Scenario: A reminder does not follow the person to another device

- **WHEN** a person configures a reminder on one of their devices
- **THEN** their other device raises no reminder unless a reminder is configured on that device

### Requirement: An optional missed-reminder follow-up fires the next day when no weigh-in was logged

**REM-3.** A weigh reminder SHALL have an optional follow-up. When the follow-up is enabled and no
weight entry has been logged for the reminder's target since the reminder fired, exactly **one**
follow-up reminder SHALL be raised on the following day — including when the product has not been
opened since the reminder fired, subject to the platform granting the product background execution on
that device. The product SHALL reconcile outstanding follow-ups while it is suspended when the platform
grants it a background execution window, so that a follow-up whose target has since been weighed on
another device is cancelled rather than raised. When a weight entry has been logged for that target
since the reminder fired, no follow-up SHALL be raised. When the follow-up is not enabled, none SHALL
be raised.

#### Scenario: A missed weigh reminder raises one follow-up the next day

- **WHEN** a weigh reminder fires, its follow-up is enabled, and no weight entry was logged for its target since it fired
- **THEN** exactly one follow-up reminder is raised on the following day

#### Scenario: The follow-up is raised while the product is not open [ios]

- **WHEN** a weigh reminder fires with its follow-up enabled, no weight entry was logged for its target
  since it fired, and the product has not been opened since it fired
- **THEN** exactly one follow-up reminder is raised on the following day without the product having
  been opened, subject to the platform granting the product background execution

#### Scenario: A weigh-in logged elsewhere cancels the follow-up while the product is not open [ios]

- **WHEN** a weigh reminder fires with its follow-up enabled, a weight entry for its target is logged on
  another device, and the platform grants this device background execution without the product being opened
- **THEN** the product reconciles in that background execution window and cancels the follow-up for that occurrence
- **AND** no follow-up reminder is raised

#### Scenario: A weigh-in logged elsewhere is reconciled while the product stays running [macos]

- **WHEN** a weigh reminder fires with its follow-up enabled, a weight entry for its target is logged on
  another device, and the product keeps running on this device
- **THEN** the follow-up for that occurrence is cancelled rather than raised

#### Scenario: A weigh-in after the reminder suppresses the follow-up

- **WHEN** a weigh reminder fires and a weight entry is logged for its target before the following day
- **THEN** no follow-up reminder is raised

#### Scenario: No follow-up is raised when the follow-up is not enabled

- **WHEN** a weigh reminder fires and its follow-up is not enabled
- **THEN** no follow-up is raised, whatever is logged afterwards

#### Scenario: The follow-up is raised once, not repeatedly

- **WHEN** a follow-up reminder has been raised for a missed weigh reminder
- **THEN** no further follow-up is raised for that occurrence
