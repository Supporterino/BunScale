# dashboard — delta for add-rabbit-weight-core-loop

Traces requirements **DASH-1, DASH-2, DASH-3, DASH-6, DASH-8, DASH-9**, plus the weight-scoped parts
of **DASH-4** and **DASH-7**.
Stories: US-DASH-01 … US-DASH-05, US-DASH-07, US-DASH-08, US-DASH-09, US-DASH-10, US-WT-11, US-WT-12.

`dashboard` is opened by three phases. This change adds the nine requirements above and no others.
The remaining parts of DASH-4 and DASH-7 are added by their owning phases under the same names and
are deliberately not specified here:

| Requirement part | Owner |
|---|---|
| Severe health entry within the last 7 days flags a rabbit (DASH-4) | `add-health-and-photo-history` (P2) |
| Health-only and full combined history views (DASH-7) | `add-health-and-photo-history` (P2) |
| Active-medication informational badge (DASH-5) and the medications section (DASH-7) | `add-medication-tracking` (P5) |

## ADDED Requirements

### Requirement: Dashboard lists non-archived rabbits with name, thumbnail and last-weighed date

**DASH-1.** On launch the dashboard SHALL present every non-archived rabbit of the active household,
each with its name, its thumbnail and its last-weighed date. An archived rabbit SHALL NOT appear in
this list.

#### Scenario: Both active rabbits are listed with their details

- **WHEN** the active household holds two non-archived rabbits and the product is opened
- **THEN** both rabbits are listed
- **AND** each row shows the rabbit's name, its thumbnail and its last-weighed date

#### Scenario: An archived rabbit is not listed

- **WHEN** the active household holds one archived rabbit and one non-archived rabbit
- **THEN** the dashboard list contains the non-archived rabbit
- **AND** it does not contain the archived one

### Requirement: A never-weighed rabbit shows a never-weighed indication

**DASH-2.** A rabbit with no weight entries SHALL present a localized "never weighed" indication in
place of a last-weighed date.

#### Scenario: A rabbit with no entries is marked as never weighed

- **WHEN** a rabbit has no weight entries
- **THEN** its dashboard row presents a "never weighed" indication
- **AND** it presents no date

### Requirement: Rabbits are sorted by most recent weigh-in with never-weighed last

**DASH-3.** The dashboard list SHALL be sorted by most recent weigh-in, newest first. Rabbits that
have never been weighed SHALL be sorted last, regardless of the order in which they were created.

#### Scenario: The most recently weighed rabbit sorts first

- **WHEN** one rabbit was weighed today and another a week ago
- **THEN** the rabbit weighed today appears above the other one

#### Scenario: A never-weighed rabbit sorts last

- **WHEN** the household holds rabbits that were weighed and rabbits that never were, and a rabbit that never was is created first
- **THEN** every never-weighed rabbit appears below every weighed rabbit

### Requirement: Rabbits with a weight-drop alert are flagged on the dashboard

**DASH-4.** A rabbit SHALL be visually flagged on the dashboard when one of its entries carries an
active weight-drop alert (WT-7). The flag SHALL be distinct from any informational badge and SHALL
disappear when the alert no longer applies, for example once the rabbit's threshold is raised above
the existing drop. (The severe-health clause of DASH-4 belongs to P2 and is not specified here.)

#### Scenario: A rabbit with a flagged drop is flagged

- **WHEN** a rabbit's latest entry triggers a drop beyond the rabbit's threshold
- **THEN** the rabbit's dashboard row presents a warning indicator

#### Scenario: A rabbit without a flagged drop is not flagged

- **WHEN** none of a rabbit's entries is flagged by a drop beyond its threshold
- **THEN** its dashboard row presents no warning indicator

#### Scenario: The flag clears when the threshold is raised

- **WHEN** a rabbit is flagged by a drop and its threshold is afterwards raised so that the drop no longer exceeds it
- **THEN** the warning indicator is no longer presented for that rabbit

### Requirement: Selecting a rabbit opens that rabbit's dashboard

**DASH-6.** Selecting a rabbit on the dashboard SHALL open that rabbit's dashboard.

#### Scenario: Selecting a rabbit opens its dashboard

- **WHEN** a person selects a rabbit in the dashboard list
- **THEN** that rabbit's dashboard is presented
- **AND** it is the dashboard of the selected rabbit

#### Scenario: The same rabbit dashboard opens on desktop [desktop]

- **WHEN** a person selects a rabbit in the dashboard list on a 1280×800 viewport
- **THEN** that rabbit's dashboard is presented
- **AND** the navigation back to the list remains available

### Requirement: Rabbit dashboard with weight trend and history entry points

**DASH-7.** A rabbit's dashboard SHALL present the weight trend chart with the selectable ranges
7 days, 30 days, 90 days and full history, and a History section that offers an entry point to the
weight-only history whose range is independent of the chart's (WT-6). For an archived rabbit the
dashboard and its sub-views SHALL be presented read-only, without add, edit or delete actions. (This
requirement covers the weight part of DASH-7 only: health-only and full combined history belong to
P2, and the medications section to P5.)

#### Scenario: The rabbit dashboard presents the weight trend and the history entry point

- **WHEN** a rabbit's dashboard is opened
- **THEN** the weight trend chart is presented
- **AND** a History section is presented that includes the weight-only history entry point

#### Scenario: The chart's range selection limits the presented trend

- **WHEN** a range is selected on the weight trend chart of the rabbit dashboard
- **THEN** the trend presents only the entries within that range

#### Scenario: An archived rabbit's dashboard is read-only

- **WHEN** an archived rabbit's dashboard is opened
- **THEN** its weight trend and its history are presented
- **AND** no control that adds, edits or deletes weight data is available

### Requirement: Zero rabbits shows an empty state prompting to add the first rabbit

**DASH-8.** With no rabbits in the active household the dashboard SHALL present an empty state that
prompts the person to add their first rabbit.

#### Scenario: The empty state prompts for the first rabbit

- **WHEN** the active household holds no rabbits
- **THEN** the dashboard presents an empty state prompting the person to add a rabbit
- **AND** that prompt leads to the rabbit-creation form

#### Scenario: The empty state is not presented once a rabbit exists

- **WHEN** the active household holds at least one rabbit
- **THEN** the rabbit list is presented instead of the empty state

### Requirement: Each dashboard row carries a per-rabbit quick-weight control

**DASH-9.** Every dashboard row SHALL carry a quick-weight control for that row's rabbit, which
opens the weight entry form pre-bound to that rabbit.

#### Scenario: The row control opens the form bound to that rabbit

- **WHEN** a person uses the quick-weight control of a dashboard row
- **THEN** the weight entry form opens pre-bound to that row's rabbit
- **AND** saving records the entry for that rabbit

#### Scenario: The row control is directly visible on a phone [mobile]

- **WHEN** the dashboard is presented at a 390×844 viewport
- **THEN** each row carries its quick-weight control as a directly visible, touchable control
- **AND** it is not reached through a gesture such as a long press
