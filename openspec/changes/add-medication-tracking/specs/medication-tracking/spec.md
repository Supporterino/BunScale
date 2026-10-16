# medication-tracking — delta for add-medication-tracking

Traces requirements **MED-1, MED-2, MED-3, MED-4, MED-5, MED-6, MED-7, MED-8, MED-12, MED-13**.
Stories: US-MED-01 … US-MED-16, US-MED-22.

**MED-13 has no counterpart in either source document.** This must be read plainly:
`docs/BunScale-Reimplementation-Requirements.md` §6.7 states MED-1 … MED-12 and
`docs/BunScale-Reimplementation-User-Stories.md` (EP-7) states no story for it. The product owner
therefore decided, as a deliberate addition to §6.7's medication entity, that a medication's own details
are correctable while its course is active and that a medication is **never deletable**: full CRUD is
explicitly rejected, because deleting a course would take its recorded dose history out of the clinical
record, and a mistyped end date — which MED-2 finishes with and MED-12 then freezes — must have a
correction path. MED-13 adds no entity, changes no MED-1 … MED-12 requirement's identity, and states the
one amendment it makes to the read-only boundary of MED-12 and to the lifecycle of MED-2 (both marked in
place below).

**MED-12 carries two bounded exceptions, and no others.** The end-date correction is MED-13's amendment
(the mistyped end date MED-2 finishes and MED-12 would otherwise freeze). The second is the
product-owner decision that a dose **actually given** stays recordable against a finished course, so a
course ended in the product while doses continued in reality, and a last-day dose logged after midnight,
are not lost (design D5.15). Neither exception lets an **existing** dose be edited, deleted or
retargeted — the dose history stays frozen — and an **archived rabbit** gains neither of them, because
DM-4 freezes it everywhere.

`medication-tracking` is **created by this change**. It states ten requirements and no others. Three
further requirements belong to this capability but are **`add-vet-export-and-reminders` (P4)**'s,
where D3's one unified reminder mechanism is specified:

| Requirement part | Owner |
|---|---|
| `Medication reminders are raised per device with record and snooze actions` (MED-9) | `add-vet-export-and-reminders` (P4) |
| `A missed dose window raises a single follow-up reminder` (MED-10) | `add-vet-export-and-reminders` (P4) |
| `PRN medications schedule no reminders` (MED-11) | `add-vet-export-and-reminders` (P4) |

Those three are deliberately **not specified, not reserved and not forward-referenced** by any
requirement below. Where §6.7 states reminder behaviour inside a requirement this delta owns — MED-2's
"SHALL NOT generate reminders" and "cancel pending reminders" — the requirement is scoped here to its
observable medication lifecycle, and the reminder consequence is P4's against the same derived status.

`US-MED-23` (offline dose logging) is **dropped** by decision D1 and is specified nowhere. Dosing is a
save like any other and requires a reachable shared service and a valid account.

`US-MED-09` traces both MED-3 and MED-11: the as-needed **configuration** (no time slots, an optional
maximum-per-day hint) is MED-3 and is stated below; MED-11's "no reminders" is P4's and is not stated
here.

## ADDED Requirements

### Requirement: A rabbit carries one or more medications with their treatment details

**MED-1.** A rabbit SHALL be able to carry one or more medications. A medication SHALL record a name, an
optional reason, a start date that defaults to today and is editable, an optional end date, an amount
per serving expressed as a numeric value together with a unit drawn from exactly **{mg, ml, drops,
tablets}**, and an optional note. The name, the start date and the amount per serving SHALL be
required; the reason, the end date and the note SHALL be optional. Several medications SHALL be able to
be active for one rabbit at the same time. An amount per serving SHALL be **positive**: an amount of
zero or a negative amount SHALL be refused and SHALL never be stored. No upper bound and no
implausible-value confirmation SHALL be applied to an amount per serving, because no single plausibility
threshold holds across mg, ml, drops and tablets; this is deliberately unlike the implausible-value
confirmation WT-4 states for weight (P1).

#### Scenario: A medication is recorded with its treatment details

- **WHEN** a person registers a medication for a rabbit with a name, a reason, a start date, an end date,
  an amount per serving and a note
- **THEN** the medication is saved for that rabbit
- **AND** it is presented as one of that rabbit's medications with those details

#### Scenario: Several medications are active at the same time

- **WHEN** a rabbit already has an active medication and a person adds another medication without an end
  date
- **THEN** both medications are active at the same time
- **AND** neither replaces, hides or ends the other

#### Scenario: Only the required fields are needed

- **WHEN** a person supplies only a name, a start date and an amount per serving
- **THEN** the medication is saved
- **AND** no reason, end date or note is required of them

#### Scenario: The start date defaults to today and is editable

- **WHEN** a person opens the form for a new medication
- **THEN** the start date is already filled with today's date
- **AND** the person can replace it before saving

#### Scenario: The serving unit is one of the four supported units

- **WHEN** a person chooses the unit for an amount per serving
- **THEN** mg, ml, drops and tablets are offered
- **AND** no other unit can be recorded for a medication

#### Scenario: A non-positive serving amount is refused

- **WHEN** a person supplies an amount per serving of zero or a negative value
- **THEN** the medication is not saved
- **AND** the refusal states that the amount per serving must be positive

#### Scenario: No plausibility confirmation is applied to a serving amount

- **WHEN** a person supplies an amount per serving far outside the range of amounts they usually record
- **THEN** the medication is saved without an implausible-value confirmation
- **AND** the amount is stored as supplied

### Requirement: A medication is active until its end date passes, then finished and its record retained

**MED-2.** A medication SHALL be **active** while it has no end date or its end date lies in the future,
and SHALL be **finished** once its end date has passed. A finished medication SHALL remain visible with
its full dose history and SHALL NOT be removed, hidden or truncated by finishing. Setting a medication's
end date to today SHALL finish the course immediately. Finishing a course SHALL NOT by itself make the
medication's end date uncorrectable: while a medication is finished its end date remains correctable, so
a mistyped end date is never frozen (MED-13).

(The reminder consequences §6.7 attaches to these states belong to
`add-vet-export-and-reminders` (P4) and are not stated here.)

#### Scenario: No end date keeps the medication active

- **WHEN** a medication has no end date
- **THEN** it is active

#### Scenario: A passed end date finishes the medication

- **WHEN** a medication's end date has passed
- **THEN** it is finished
- **AND** it remains visible with its full dose history intact

#### Scenario: A future end date keeps the medication active

- **WHEN** a medication's end date lies in the future
- **THEN** it is active

#### Scenario: Setting the end date to today finishes the course immediately

- **WHEN** an active medication's end date is set to today
- **THEN** it is finished immediately
- **AND** its dose history is unchanged

#### Scenario: A finished medication is still reachable with its dose history

- **WHEN** a person opens a finished medication's detail
- **THEN** the medication's details are presented
- **AND** every dose recorded during the course is still listed

### Requirement: Medications support times-per-day, every-X-days, weekly and as-needed schedules

**MED-3.** A medication's schedule SHALL be exactly one of four types:

- **times per day** — a number of daily slots between **1 and 4**, each slot carrying its own explicit
  time; the slots SHALL be pre-filled with sensible defaults that the person can edit;
- **every X days** — an interval in days **counted fixed from the medication's start date**, at a chosen
  time;
- **weekly** — **one** weekday together with a time;
- **as needed** — no fixed times, with an optional maximum-per-day hint.

An as-needed schedule SHALL require no time slot.

#### Scenario: Times per day offers one to four pre-filled editable slots

- **WHEN** a person chooses a times-per-day schedule with two slots
- **THEN** two daily slots are presented
- **AND** each is pre-filled with a default time that the person can edit

#### Scenario: The times-per-day slot count is limited to one to four

- **WHEN** a person chooses a times-per-day schedule
- **THEN** a count of one and a count of four are accepted
- **AND** no schedule with zero slots or with more than four slots can be recorded

#### Scenario: The every-X-days interval is counted from the start date

- **WHEN** a medication's start date is a Monday and a person chooses an interval of every 3 days at
  09:00
- **THEN** the medication is due on the start date and every third day after it, counted from the start
  date
- **AND** its due days are Monday, Thursday, Sunday and so on, at 09:00

#### Scenario: A weekly schedule is one weekday plus a time

- **WHEN** a person chooses a weekly schedule for Monday at 10:00
- **THEN** the medication is due every Monday at 10:00
- **AND** exactly one weekday is carried by the schedule

#### Scenario: An as-needed schedule needs no time and accepts a maximum per day

- **WHEN** a person chooses an as-needed schedule
- **THEN** no time slot is required
- **AND** the person can set an optional maximum-per-day hint

### Requirement: A human-readable schedule summary renders for every schedule type

**MED-4.** For every medication the product SHALL present a human-readable summary of that medication's
schedule. The summary SHALL render for **all four** schedule types, SHALL include the parameters of the
type it describes, and SHALL render **whether or not** any dose has been recorded — in every case
without failing, without presenting an empty or error state in place of the schedule, and without
omitting a parameter the person configured.

#### Scenario: A times-per-day summary shows the daily slot count and their times

- **WHEN** a medication's schedule is times per day with two slots at 08:00 and 20:00
- **THEN** the summary states that the medication is given twice a day
- **AND** it states both slot times

#### Scenario: An every-X-days summary shows the interval

- **WHEN** a medication's schedule is every 3 days at 09:00
- **THEN** the summary states the three-day interval
- **AND** it states the time
- **AND** it renders without failing

#### Scenario: A weekly summary shows the weekday and the time

- **WHEN** a medication's schedule is weekly on Monday at 10:00
- **THEN** the summary states Monday and the time

#### Scenario: An as-needed summary shows the as-needed nature and the maximum-per-day hint

- **WHEN** a medication's schedule is as needed with a maximum of 3 per day
- **THEN** the summary states that the medication is given as needed
- **AND** it states the maximum of 3 per day
- **AND** it renders without failing

#### Scenario: An as-needed summary renders when no maximum-per-day hint is set

- **WHEN** a medication's schedule is as needed with no maximum-per-day hint
- **THEN** the summary states that the medication is given as needed
- **AND** no maximum is presented

#### Scenario: Every schedule type renders a summary with no recorded doses

- **WHEN** a medication has no recorded doses, for each of the four schedule types
- **THEN** the summary of its schedule is presented
- **AND** no summary fails, and none presents an empty or error state in place of the schedule

#### Scenario: Every schedule type renders a summary with recorded doses

- **WHEN** a medication has recorded doses, for each of the four schedule types
- **THEN** the summary of its schedule is presented
- **AND** the summary is unchanged by the presence of doses

#### Scenario: The summary is fully readable on a phone [mobile]

- **WHEN** a medication detail is presented at a 390×844 viewport
- **THEN** the schedule summary presents the type and all of its parameters in full
- **AND** no parameter is hidden behind a gesture or a further control

#### Scenario: The summary is fully readable on a desktop viewport [desktop]

- **WHEN** a medication detail is presented at a 1280×800 viewport
- **THEN** the schedule summary presents the type and all of its parameters in full

### Requirement: A dose is recorded with a timestamp, an amount and the member who recorded it

**MED-5.** Recording a dose SHALL capture the medication it belongs to, a timestamp that defaults to
**now** and is editable, an amount that is **pre-filled from the medication's amount per serving** and
is editable, an optional note, and the identity of the **member who recorded it**. A dose SHALL carry
**its own unit together with its own value**, and that unit **MAY be a different unit** from the
medication's amount per serving: the dose, not the course, is what the person declares was given, so a
dose counted in drops against a serving recorded in ml SHALL be recordable, and the product SHALL
perform **no conversion** between units. A dose's amount SHALL be **positive**: an amount of zero or a
negative amount SHALL be refused and SHALL never be stored, and no upper bound and no
implausible-value confirmation SHALL be applied to it. A recorded dose SHALL be part of the household's
shared collection, so every member of the household resolves the same dose. (The giver itself is stated
as an invariant by `A dose records the giver as a member of the household` in `data-integrity`.)

#### Scenario: A dose is recorded with the serving amount, the current time and the recording member

- **WHEN** a medication's amount per serving is 0.2 ml and a member confirms giving a dose without
  changing anything
- **THEN** a dose of 0.2 ml is recorded for that medication
- **AND** it carries the moment of confirmation as its timestamp
- **AND** it records that member as the one who recorded it

#### Scenario: The pre-filled amount can be changed

- **WHEN** a medication's amount per serving is 0.4 ml and a person changes the amount to 0.2 ml before
  confirming
- **THEN** the recorded dose carries 0.2 ml
- **AND** the medication's amount per serving is unchanged

#### Scenario: A dose may carry a different unit from the serving

- **WHEN** a medication's amount per serving is 0.2 ml and a person records a dose of 3 drops
- **THEN** the dose is recorded carrying the value 3 and the unit drops
- **AND** the medication's amount per serving and its unit are unchanged
- **AND** no conversion between ml and drops is attempted

#### Scenario: A non-positive dose amount is refused

- **WHEN** a person confirms a dose with an amount of zero or a negative value
- **THEN** no dose is recorded
- **AND** the refusal states that the dose amount must be positive

#### Scenario: No plausibility confirmation is applied to a dose amount

- **WHEN** a person records a dose whose amount is far larger than the medication's amount per serving
- **THEN** the dose is recorded without an implausible-value confirmation

#### Scenario: A dose can be back-dated

- **WHEN** a person records a dose for a time earlier than now, including earlier the same day
- **THEN** the dose carries that earlier instant
- **AND** the instant is the one the person supplied

#### Scenario: The note is optional

- **WHEN** a dose is recorded without a note
- **THEN** the dose is recorded
- **AND** no note is required of the person

#### Scenario: A dose recorded by one member is resolved by every member

- **WHEN** one member records a dose and another member of the same household opens that medication
- **THEN** the second member resolves the same dose
- **AND** there is exactly one copy of it for the household

### Requirement: Doses are listed chronologically and remain editable and deletable

**MED-6.** The product SHALL present a medication's doses in **chronological order**, each dose showing
its timestamp, its amount and its **giver**. A recorded dose SHALL be editable in its amount, its note
and its timestamp, and SHALL be deletable. Every such change SHALL be shared with all members of the
household. This holds while the medication is not read-only: a finished medication's **existing** dose
history is frozen (MED-12) and an archived rabbit's medications are read-only everywhere (DM-4, P1).
(Two exceptions leave the existing history frozen nonetheless: a dose that was actually given may still
be **recorded** against a finished medication, and its end date stays correctable — both MED-12; an
archived rabbit gets neither. The giver is not among the editable attributes; see
`A dose records the giver as a member of the household` in `data-integrity`.)

#### Scenario: The dose list is chronological and shows timestamp, amount and giver

- **WHEN** a medication has several recorded doses and a person opens its detail
- **THEN** the doses are listed in chronological order
- **AND** each entry shows that dose's timestamp, its amount and its giver

#### Scenario: A dose's amount is edited and the change is shared

- **WHEN** a member edits a dose's amount and the change is saved
- **THEN** the dose carries the new amount
- **AND** another member of the household resolves the changed dose

#### Scenario: A dose's note and timestamp are edited and the change is shared

- **WHEN** a member edits a dose's note or its timestamp and the change is saved
- **THEN** the dose carries the new note or timestamp
- **AND** another member of the household resolves the changed dose

#### Scenario: A dose is deleted and the deletion is shared

- **WHEN** a member deletes a dose and confirms
- **THEN** the dose is removed from the medication's dose list
- **AND** it is no longer resolved by any member of the household
- **AND** the medication itself is unchanged

### Requirement: A near-simultaneous dose of the same medication is warned about before it is recorded

**MED-7.** Before a dose is confirmed, the product SHALL determine whether a dose of **the same
medication** was already logged within a short recent window of approximately **15 minutes**. The
window SHALL be measured from **the timestamp entered for the new dose** — the instant that will be
recorded, not the moment of confirmation — and SHALL cover doses of that medication **both before and
after** that instant, so a back-dated dose colliding with a later one is caught as well as one logged
minutes earlier. The determination SHALL be made against **that medication's doses read afresh** at
the moment of confirmation, and not against a copy the device may already hold, so a dose another
member recorded minutes ago is not missed. When one or more doses of that medication fall inside the
window, the product SHALL present a warning that names the existing dose or doses and SHALL let the
person either **continue**, in which case the dose is recorded, or **cancel**, in which case no dose is
recorded. When no dose of that medication falls within the window, no warning SHALL be presented and
the dose SHALL be recorded.

#### Scenario: A dose inside the window raises a warning

- **WHEN** another member logged a dose of the same medication approximately 15 minutes before the dose
  being confirmed
- **THEN** a warning is presented before the dose is confirmed
- **AND** it names the existing dose

#### Scenario: Continuing past the warning records the dose

- **WHEN** the warning is presented and the person continues
- **THEN** the dose is recorded
- **AND** the dose list shows both doses

#### Scenario: Cancelling at the warning records nothing

- **WHEN** the warning is presented and the person cancels
- **THEN** no dose is recorded
- **AND** the dose list is unchanged

#### Scenario: A dose outside the window records without a warning

- **WHEN** no dose of the same medication falls inside the window, on either side of the timestamp
  entered for the new dose
- **THEN** no warning is presented
- **AND** the dose is recorded

#### Scenario: The person's own recent dose raises the warning too

- **WHEN** the person confirming the dose recorded a dose of the same medication approximately 15 minutes
  earlier themselves
- **THEN** a warning is presented before the dose is confirmed
- **AND** the person may continue or cancel

#### Scenario: A dose of a different medication does not raise the warning

- **WHEN** a dose of a different medication of the same rabbit was logged within the window
- **THEN** no warning is presented for the medication being dosed
- **AND** the dose is recorded

#### Scenario: A medication with no earlier dose raises no warning

- **WHEN** the medication has no recorded dose at all
- **THEN** no warning is presented
- **AND** the dose is recorded

#### Scenario: The window is measured from the timestamp entered for the new dose

- **WHEN** a person records a dose stamped 09:00, hours after the moment of confirmation, and another
  member's dose of the same medication was recorded at 08:50
- **THEN** the warning is presented, because 08:50 falls inside the window measured from 09:00
- **AND** the moment of confirmation is not the anchor of the window

#### Scenario: A dose after the entered timestamp inside the window raises the warning too

- **WHEN** a person records a dose stamped 09:00 and another member's dose of the same medication was
  recorded at 09:05
- **THEN** a warning is presented before the dose is confirmed
- **AND** it names that dose, which lies after the timestamp entered
- **AND** the person may continue or cancel

#### Scenario: The check reads the medication's doses afresh before it is evaluated

- **WHEN** another member recorded a dose of the same medication minutes ago on another device, and the
  confirming device's local copy does not yet hold it
- **THEN** confirming a dose reads that medication's doses from the household's shared collection before
  the window is evaluated
- **AND** the warning is presented for that dose
- **AND** the check itself records nothing

#### Scenario: The warning does not depend on the device [mobile]

- **WHEN** a dose inside the window is confirmed from a device at a 390×844 viewport
- **THEN** the warning is presented before the dose is recorded
- **AND** no gesture is required to see it

### Requirement: The medication detail offers a primary give-dose action

**MED-8.** A medication's detail SHALL offer a primary action to give a dose. That action SHALL open a
confirmation whose amount is pre-filled from the medication's amount per serving, and confirming it SHALL
record the dose and update the presented dose list.

#### Scenario: Give dose opens the pre-filled confirmation and records on confirm

- **WHEN** a medication's detail is open and a person chooses to give a dose and confirms the pre-filled
  amount
- **THEN** a dose is recorded
- **AND** the presented dose list includes it without the person leaving and re-opening the detail

#### Scenario: The give-dose action is directly visible on the detail [mobile]

- **WHEN** a medication's detail is presented at a 390×844 viewport
- **THEN** the give-dose action is a directly visible, touchable control
- **AND** it is not reached through a gesture such as a long press or a nested menu

### Requirement: A finished medication's records are read-only while remaining visible

**MED-12.** While a medication is finished, its record and its dose history SHALL remain **viewable**,
and **no existing dose SHALL be edited, deleted or retargeted**. Two deliberate and bounded exceptions
exist, and **no others**; both leave the recorded dose history frozen:

- **the end date stays correctable** while the medication is finished, so a mistyped end date is never
  frozen (MED-13), while every other attribute of the course stays read-only;
- **a dose that was actually given stays recordable**, carrying its own timestamp, so a course that was
  ended in the product while doses continued in reality, and a dose given on the last day and logged
  after midnight, are not lost. Recording it SHALL NOT unfreeze anything else: the course's other
  attributes stay read-only, and its existing doses stay uneditable, undeletable and unretargeted.

For an **archived rabbit**, the medication section of the rabbit dashboard SHALL offer no control that
creates, edits or deletes a medication or a dose — **neither exception applies there**: no correction of
the end date and no recording of a dose — because DM-4 freezes an archived rabbit everywhere. Restoring
an archived rabbit SHALL return write access to its medications, while its finished medications SHALL
remain read-only except for those same two exceptions. (That an archived rabbit is immutable
*everywhere* is `An archived rabbit is read-only everywhere`, DM-4, owned by
`add-rabbit-weight-core-loop` (P1); this requirement states only the observable medication behaviour and
does not restate it.)

#### Scenario: A finished medication's record is visible without edit or delete actions

- **WHEN** a person opens a finished medication
- **THEN** its details are presented
- **AND** no control is available that edits or deletes the medication, other than the correction of its
  end date (MED-13)

#### Scenario: A finished medication's dose history is visible without edit or delete actions

- **WHEN** a person opens the dose list of a finished medication
- **THEN** every dose recorded during the course is presented with its timestamp, amount and giver
- **AND** no control is available that edits or deletes an existing dose

#### Scenario: A dose that was actually given is recordable against a finished course

- **WHEN** a course is finished — because it was ended in the product while doses continued in reality,
  or because a dose given on its last day is being logged after midnight — and a person records a dose
  that was actually given
- **THEN** the give-dose action is available and the dose is recorded
- **AND** the recorded dose carries its own timestamp
- **AND** no attribute of the course other than its end date (MED-13) is editable

#### Scenario: Recording a given dose leaves the frozen dose history frozen

- **WHEN** a dose is recorded against a finished medication
- **THEN** every existing dose keeps its timestamp, its amount and its giver
- **AND** no existing dose is edited, deleted or retargeted

#### Scenario: An archived rabbit's medication section offers no write action

- **WHEN** an archived rabbit's medications are opened, from any entry point in the product
- **THEN** its medications and their dose histories are viewable
- **AND** no action is available that creates, edits or deletes a medication
- **AND** no action is available that creates, edits or deletes a dose

#### Scenario: An archived rabbit gains neither exception

- **WHEN** an archived rabbit's finished medication is opened, from any entry point in the product
- **THEN** no dose can be recorded against it
- **AND** its end date cannot be corrected
- **AND** no existing dose can be edited or deleted

#### Scenario: Restoring the rabbit restores write access, but a finished medication stays read-only

- **WHEN** an archived rabbit is restored and a person opens its medications
- **THEN** its active medications are writable again
- **AND** a medication of that rabbit that is finished remains read-only, except that its end date stays
  correctable (MED-13) and a dose that was actually given stays recordable
- **AND** no existing dose of that finished medication can be edited or deleted

### Requirement: An active course's details are correctable and a medication is never deleted

**MED-13.** *(Deliberate addition — no counterpart in either source document; see the header note.)* A
medication **SHALL NOT be deletable**: the product SHALL offer no affordance that removes a medication
and no path accepts such a request, so a course's recorded dose history always stays part of the
household's record. While a medication is **active**, its details SHALL be **correctable** — its name,
its reason, its start date, its end date, its amount per serving together with its unit, its schedule and
its note — and every correction SHALL be shared with all members of the household through the household's
shared collection. Because finishing a course is what correcting an end date does (MED-2), the **end date
SHALL remain correctable while the medication is finished**, so a mistyped end date can always be
repaired; correcting it to a date in the future, or removing it, SHALL return the medication to active
under MED-2's derivation. Every other attribute of a finished medication, and its entire dose history,
SHALL remain read-only (MED-12). A correction SHALL NOT delete, rewrite or retarget any recorded dose, and
SHALL NOT change a dose's giver (DM-5). MED-12's other exception — recording a dose that was actually
given against a finished course — makes **none** of the course's own attributes correctable: the end
date remains the only attribute MED-13 permits a person to correct on a finished course, and recording
such a dose SHALL NOT become a path to correcting, deleting, rewriting or retargeting anything else. For
an **archived rabbit**, nothing is correctable at all, the end date included, and no such dose can be
recorded (DM-4, MED-12). (No requirement here restates DM-4, MED-6's dose edits or MED-12's
read-only record: this requirement states the medication-detail edit path and the never-deletable rule,
and references the boundaries it must not cross.)

#### Scenario: An active course's details are corrected and the correction is shared

- **WHEN** a member corrects an active medication's name, reason, start date, amount per serving, unit,
  schedule or note and saves
- **THEN** the medication carries the correction
- **AND** every recorded dose of that medication is unchanged
- **AND** another member of the household resolves the corrected medication

#### Scenario: A mistyped end date set to today is corrected and the course is active again

- **WHEN** an active medication's end date is set to today, which immediately finishes the course (MED-2),
  and the person who set it recognises the date as a mistake
- **THEN** the medication is finished but its details remain correctable, its end date included
- **AND** the person can correct the end date
- **AND** once it is corrected to a date in the future, or removed, the medication is active again
- **AND** its dose history is unchanged by the correction

#### Scenario: A mistyped end date in the past is corrected

- **WHEN** a person sets a medication's end date to a past date by mistake, finishing the course, and then
  corrects that end date
- **THEN** the end date is correctable while the medication is finished
- **AND** the course's status follows the corrected end date under MED-2's derivation
- **AND** no recorded dose is deleted, rewritten or retargeted by the correction

#### Scenario: A course is never deleted

- **WHEN** a medication — active or finished — is presented anywhere in the product
- **THEN** no control is available that removes the medication
- **AND** no path accepts a request that deletes it
- **AND** every dose recorded during the course remains reachable

#### Scenario: Correcting a course never rewrites a recorded dose

- **WHEN** any medication detail of an active or finished medication is corrected
- **THEN** every recorded dose keeps its timestamp, its amount and its giver
- **AND** the dose list is unchanged except as the correction of the medication itself requires

#### Scenario: A finished course's end date is correctable while its other details and doses stay read-only

- **WHEN** a person opens a finished medication and chooses to correct it
- **THEN** its end date can be corrected
- **AND** no control is available that changes its name, reason, start date, serving, unit, schedule or
  note
- **AND** no control is available that edits or deletes an existing dose
- **AND** recording a dose that was actually given stays available as MED-12 states, and is not a path to
  any of the changes above

#### Scenario: An archived rabbit's course is not correctable

- **WHEN** a medication of an archived rabbit is opened, from any entry point in the product
- **THEN** its end date cannot be corrected
- **AND** no other attribute of it can be changed

#### Scenario: The end-date correction is reachable without a gesture [mobile]

- **WHEN** a finished medication's detail is presented at a 390×844 viewport
- **THEN** the correction of its end date is reachable as a directly visible, touchable control
- **AND** it is not reached through a gesture such as a long press or a nested menu
