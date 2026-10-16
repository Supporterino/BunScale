# export — delta for add-vet-export-and-reminders

Traces requirements **EXP-1, EXP-2, EXP-3, EXP-4**.
Stories: US-EXP-01, US-EXP-02, US-EXP-03, US-EXP-04, US-EXP-05, US-EXP-06.
Related, referenced and not restated: RAB-8's export clause (P1) is realised here; the
cause-specific reporting of a failed export is P0's `Every failure path states its cause`
(NFR-8); the medication schedule summary EXP-4 presents is MED-4 (P5); the medication and
`Schedule` model EXP-1/EXP-2/EXP-4 export is MED-1 … MED-3 (P5).

PDF and CSV are named because the source requirements name the format the person receives —
they are what a vet can open and what the household can archive. No library, tool or vendor is
named anywhere in this delta.

## ADDED Requirements

### Requirement: A rabbit's history exports as a formatted, vet-readable PDF for a selected date range

**EXP-1.** A rabbit's weight, health and medication history SHALL be exportable for a date range
the person selects, as a formatted **PDF** readable by a vet. The export SHALL present the weight
entries, the health entries and the medication courses that the selected range covers. Medication
export SHALL include **finished as well as active courses**, and each course SHALL be presented
with its **full dose history** — the timestamp, amount and giver of every dose recorded for it —
so that no dose of a presented course is hidden by the range selection. The range control SHALL open
on **everything the rabbit has** by default, and SHALL offer the weight chart's 7-day, 30-day,
90-day and all-history presets alongside the rabbit's full span. The selected range SHALL decide
**which entries and which medication courses are presented**, and a course that is presented SHALL
report **every** dose recorded for it, so that a presented course's "full" dose history is not
narrowed by the range. Export SHALL remain available for an archived rabbit. An export that cannot be
produced SHALL present no file.

#### Scenario: A selected range produces a formatted PDF covering weight, health and medications

- **WHEN** a rabbit has weight, health and medication history and a range is selected for export
- **THEN** a formatted PDF is produced covering that rabbit's weight entries, health entries and medication courses
- **AND** the PDF is presented for the person to review before it is handed over

#### Scenario: The selected range determines which entries and courses are presented

- **WHEN** a range is selected that excludes some of the rabbit's entries
- **THEN** entries outside the selected range are not presented
- **AND** entries inside the selected range are presented
- **AND** a medication course whose history the range does not cover is not presented

#### Scenario: The range control opens on everything the rabbit has

- **WHEN** the export range control is opened for a rabbit
- **THEN** the range opens on everything the rabbit has, from its earliest history to its latest
- **AND** the weight chart's 7-day, 30-day, 90-day and all-history presets are offered alongside the
  rabbit's full span

#### Scenario: Finished courses are presented alongside active ones

- **WHEN** the rabbit has both an active course and a finished course
- **THEN** both are presented in the medication part of the PDF
- **AND** the finished course is not omitted for being finished

#### Scenario: Each medication is presented with its full dose history

- **WHEN** a course in the PDF has recorded doses
- **THEN** every dose recorded for that course is presented with its timestamp, its amount and the giver
- **AND** no dose of a presented course is omitted because it falls outside the selected range
- **AND** a presented course reports every dose it has, so its dose history is not narrowed by the range

#### Scenario: Export remains available for an archived rabbit

- **WHEN** a rabbit is archived
- **THEN** its history remains exportable
- **AND** the export presents the same history it would present for an active rabbit

#### Scenario: An export that cannot be produced presents no file

- **WHEN** an export cannot be produced
- **THEN** no file is presented to the person
- **AND** the failure is reported with its specific cause rather than as a success

### Requirement: The same history exports as raw CSV with dedicated medication and dose files

**EXP-2.** The same history SHALL be exportable, for the same selected date range, as raw **CSV**
rather than as a formatted PDF. Medications and doses SHALL be written to **dedicated files** — one
for medications and one for doses — separate from each other and from the files holding the
rabbit's weight and health entries. The medication file SHALL carry each course's fields, including
its schedule and its active or finished state; the dose file SHALL carry every dose's timestamp,
amount and giver. Values SHALL be exported as the **canonical stored values** — the grams, amounts
and units exactly as they were recorded — without presentation formatting and without conversion to
the person's display unit. The formatted PDF (EXP-1) is unaffected: it presents values in the
person's display unit, as any screen would.

#### Scenario: A range produces raw CSV files for the entries

- **WHEN** a rabbit has weight and health entries and a range is selected for raw export
- **THEN** the entries in that range are written to CSV files
- **AND** the values are the ones recorded — the grams, amounts and units as stored — rather than a
  display format or a value converted to the person's display unit

#### Scenario: The raw files carry stored values while the PDF is formatted

- **WHEN** the same rabbit's history is exported both as raw files and as a formatted PDF
- **THEN** the raw files carry the amounts, units and grams exactly as they were recorded
- **AND** the PDF presents those values in the person's display unit, as any screen would

#### Scenario: Medications and doses are written to dedicated files

- **WHEN** raw export runs for a rabbit that has medications with doses
- **THEN** a file dedicated to medications is produced and a separate file dedicated to doses is produced
- **AND** neither medications nor doses are mixed into the files holding the entries

#### Scenario: The dose file carries timestamp, amount and giver for every dose

- **WHEN** the doses file is produced
- **THEN** it contains one row per recorded dose
- **AND** each row carries that dose's timestamp, amount and giver

#### Scenario: Raw files cover finished courses as well as active ones

- **WHEN** raw export runs for a rabbit that has a finished course
- **THEN** the finished course appears in the medications file
- **AND** its doses appear in the doses file

### Requirement: Exported files are handed to the device's standard sharing options

**EXP-3.** Once an export has been produced, the product SHALL present the device's **standard
sharing options** with the exported file or files attached, so that the person can send, save or
hand them over wherever they choose. Every file the export produced SHALL be attached when the
sharing options are presented. The product SHALL NOT send an export to any destination the person
did not choose.

#### Scenario: The device's standard sharing options are presented on iOS [ios]

- **WHEN** an export has been produced on an iOS device
- **THEN** the device's standard sharing options are presented with the export attached

#### Scenario: The device's standard sharing options are presented on macOS [macos]

- **WHEN** an export has been produced on a macOS device
- **THEN** the device's standard sharing options are presented with the export attached

#### Scenario: A multi-file export presents every file it produced

- **WHEN** an export produced more than one file
- **THEN** the sharing options are presented with every produced file attached
- **AND** no produced file is silently dropped

#### Scenario: Dismissing the sharing options sends nothing and leaves the export on the device

- **WHEN** the person dismisses the sharing options without choosing a destination
- **THEN** nothing is sent anywhere
- **AND** the export remains available on the device for a later attempt

### Requirement: Export succeeds for every supported schedule type, with or without recorded doses

**EXP-4.** Export SHALL complete for any medication schedule the product supports — including
`everyXDays` and `prn` — whether or not any dose has been recorded for the course. Export SHALL
NOT fail, omit a course, or abort because of a course's schedule type or because a course has an
empty dose history. Each course SHALL be presented through its human-readable schedule summary.

#### Scenario: A times-per-day course with recorded doses exports

- **WHEN** a rabbit has a times-per-day course with recorded doses and an export is run
- **THEN** the export completes
- **AND** the course is presented with its schedule summary and its dose history

#### Scenario: An every-X-days course with no recorded doses exports

- **WHEN** a rabbit has an every-X-days course with no recorded doses and an export is run
- **THEN** the export completes
- **AND** the course is presented with its schedule summary and an empty dose history

#### Scenario: A weekly course with no recorded doses exports

- **WHEN** a rabbit has a weekly course with no recorded doses and an export is run
- **THEN** the export completes
- **AND** the course is presented with its schedule summary and an empty dose history

#### Scenario: A PRN course with a maximum-per-day hint and no doses exports

- **WHEN** a rabbit has an as-needed course with a maximum-per-day hint and no recorded doses and an export is run
- **THEN** the export completes
- **AND** the course is presented with its schedule summary, including the hint, and an empty dose history

#### Scenario: A course with no doses presents an empty dose history instead of failing

- **WHEN** a course has no recorded doses
- **THEN** its dose history is presented as empty
- **AND** the export does not fail or omit the course

#### Scenario: One export containing all four schedule types completes

- **WHEN** a rabbit has one course of each supported schedule type, some with recorded doses and some without, and an export is run
- **THEN** the export completes for every course
- **AND** each course appears with its own schedule summary
