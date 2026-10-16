# dashboard — delta for add-health-and-photo-history

Traces requirements **DASH-4 (severe-health clause), DASH-7 (health clause) and HL-3**.
Stories: US-DASH-04 (severe-health half), US-DASH-08, US-HL-04.

This delta adds two requirements and no others. The rabbit list, its sort order, the never-weighed
indication, the weight-drop alert clause of DASH-4, the informational active-medication badge (DASH-5)
and the weight-trend presentation of DASH-7 are owned by `add-rabbit-weight-core-loop` (P1) and
`add-medication-tracking` (P5); they are referenced here, not restated or altered.

## ADDED Requirements

### Requirement: Rabbits with a recent severe health entry are flagged on the dashboard

**DASH-4 (severe-health clause).** A rabbit SHALL be flagged on the dashboard when it has a health
entry of severity Severe whose timestamp is within the last 7 days. A Severe entry older than 7 days
SHALL NOT flag the rabbit, and an entry of Mild or Moderate severity SHALL NOT flag it. The flag SHALL
clear as soon as the rabbit no longer has a Severe entry within the last 7 days — for example because
the entry was changed to a lower severity, was deleted, or has aged out of the window. The flag SHALL
be the same dashboard warning flag that the weight-drop clause of this requirement raises.

#### Scenario: A Severe health entry within the last 7 days flags the rabbit

- **WHEN** a rabbit has a health entry of severity Severe whose timestamp is within the last 7 days
- **THEN** that rabbit is flagged on the dashboard

#### Scenario: A Severe health entry older than 7 days does not flag the rabbit

- **WHEN** a rabbit's most recent Severe health entry is older than 7 days
- **THEN** that entry does not flag the rabbit on the dashboard

#### Scenario: A Mild or Moderate entry does not flag the rabbit

- **WHEN** a rabbit has health entries within the last 7 days but none of them is Severe
- **THEN** those entries do not flag the rabbit on the dashboard

#### Scenario: The flag clears when no recent Severe entry remains

- **WHEN** a Severe health entry is changed to a lower severity, or is deleted, or is no longer within the last 7 days
- **THEN** that entry no longer flags the rabbit
- **AND** the rabbit is no longer flagged unless it has another Severe entry within the last 7 days

### Requirement: Health-only and full combined history views

**DASH-7 (health clause), HL-3.** The rabbit dashboard's History section SHALL provide a health-only
history view and a full combined history view; the weight-only history view is provided by the
weight-tracking capability. The full combined history SHALL present the rabbit's weight entries and
health entries interleaved in one timeline ordered by timestamp. The health-only history SHALL present
only health entries and SHALL let them be restricted by category tag, severity and date range. Both
views SHALL remain available for an archived rabbit, presenting no add, edit or delete action for one.

#### Scenario: The full history interleaves weight and health entries by timestamp

- **WHEN** a rabbit has weight entries and health entries and a person opens its full combined history
- **THEN** both kinds appear in one timeline
- **AND** they are ordered by their timestamps, interleaved with one another

#### Scenario: A rabbit with only health entries shows no empty-state prompt

- **WHEN** a rabbit has health entries but no weight entries and a person opens its history
- **THEN** its health entries are listed
- **AND** no prompt to add a first entry is shown in place of them

#### Scenario: The health-only history shows only health entries

- **WHEN** a person opens the health-only history of a rabbit that has weight entries and health entries
- **THEN** only its health entries are listed

#### Scenario: The health-only history by tag, severity and date range

- **WHEN** a person restricts the health-only history by a category tag, a severity, or a date range
- **THEN** only the health entries matching the restriction are listed

#### Scenario: An archived rabbit's history views are available without add, edit or delete actions

- **WHEN** a person opens the health-only or the full combined history of an archived rabbit
- **THEN** the history is available and viewable
- **AND** no add, edit or delete action is available
