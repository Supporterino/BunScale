# product-quality — delta for add-vet-export-and-reminders

Traces requirements **NFR-4, NFR-5** — **ADDED only**.
Stories: US-QUAL-01, US-QUAL-02, US-QUAL-03, and the German-coverage clause of US-SET-04.

**Capability ownership.** `product-quality` is opened by P0 and extended by later phases. The
following requirements already exist in, or are added by, other changes and are **not** restated,
modified or removed here — no `## MODIFIED Requirements` section appears in this file:
`Connectivity model: reachability-gated writes, loaded-data reads` (NFR-1, P0),
`Conflict resolution never fails and never duplicates` (NFR-2, P0),
`Health and weight data stay private; diagnostics exclude content` (NFR-3, P0),
`Every failure path states its cause` (NFR-8, P0, which names export among its failure paths),
`Primary and destructive actions are visible and confirmed` (NFR-9, P1),
`Honest service status` (NFR-6, P3) and
`Bounded cold load and load-once reuse` (NFR-7, P6).

NFR-5 is the requirement that lands the full German/English coverage P0's i18n scaffold (task 3.7,
English keys) deliberately left to this phase; the scaffold and every later phase's string keys are
inputs to it, not competitors of it. US-SET-04's *override* is SET-3
(`settings-profile`, this same change); its *coverage* clause is NFR-5 here.

## ADDED Requirements

### Requirement: Text size, screen-reader labels and an accessible chart summary

**NFR-4.** The product SHALL respect the person's preferred text size: text SHALL scale with that
preference and the screens SHALL remain usable at the supported baselines. Interactive controls
SHALL carry labels that identify them to assistive technology. The weight chart SHALL expose an
**accessible summary** — for example the latest value and the trend direction — that assistive
technology can read, derived from the same values the chart presents and not from a second source
of chart data. The trend direction SHALL compare the **latest** entry with the **previous** entry and
SHALL state the **direction only**, claiming no magnitude. When the chart holds exactly **one** entry,
the summary SHALL report that value and SHALL state that there is no trend yet, inventing no
direction it cannot support. Where there is **no** value to summarise, the summary SHALL say so,
using the product's never-weighed wording, instead of reporting a value or implying a trend.

#### Scenario: Text scales with the preferred text size and the layout stays usable [mobile]

- **WHEN** the preferred text size is increased and the product is used at 390×844
- **THEN** text is presented at the increased size
- **AND** the screens remain usable, with no content or control unreachable

#### Scenario: Text scales with the preferred text size and the layout stays usable [desktop]

- **WHEN** the preferred text size is increased and the product is used at 1280×800
- **THEN** text is presented at the increased size
- **AND** the screens remain usable, with no content or control unreachable

#### Scenario: Interactive controls expose labels to assistive technology

- **WHEN** an interactive control of the product is reached with assistive technology
- **THEN** it is announced with a label that identifies what it does
- **AND** no interactive control is announced without a label

#### Scenario: The weight chart exposes a summary of its latest value and trend direction

- **WHEN** the weight chart presents a series of weight entries
- **THEN** an accessible summary of that chart states the latest value and the trend direction
- **AND** the trend direction is that of the latest entry compared with the previous one, stating the
  direction only and claiming no magnitude
- **AND** the summary is available to assistive technology, not only visible on screen

#### Scenario: The chart summary is read when the chart receives focus

- **WHEN** a screen reader focuses the weight chart
- **THEN** the accessible summary is read

#### Scenario: The chart summary follows the chart's values without a second data source

- **WHEN** the series the chart presents changes
- **THEN** the accessible summary changes with it
- **AND** no additional chart data is requested to produce the summary

#### Scenario: A chart with a single entry reports no trend yet

- **WHEN** the weight chart presents exactly one weight entry
- **THEN** the accessible summary reports that value
- **AND** it states that there is no trend yet and claims no direction

#### Scenario: A chart with no values summarises that instead of a value

- **WHEN** the weight chart has no values to present
- **THEN** the accessible summary states that there is nothing to summarise, using the product's
  never-weighed wording
- **AND** it presents no value and claims no trend direction

### Requirement: German and English coverage with metric defaults

**NFR-5.** German and English SHALL both be supported. **All** user-facing text SHALL be available
in both languages, including labels the product supplies for system-generated states such as a
localized "You" placeholder and a "never weighed" indication; no screen SHALL present an
untranslated string or a missing-text placeholder. Metric units SHALL be the default, and another
unit system SHALL be presented only when the person has chosen it or the device's regional settings
indicate it.

#### Scenario: The whole product is presented in German when German is the product language

- **WHEN** the product language in effect is German
- **THEN** every screen presents its text in German
- **AND** the product is usable end to end without switching to English

#### Scenario: No user-facing string falls back to another language or to a missing-text placeholder

- **WHEN** any screen of the product is presented in German or in English
- **THEN** every user-facing string is present in that language
- **AND** no string is shown in the other language and no string is replaced by a key, an identifier or a placeholder

#### Scenario: System-supplied labels are localized

- **WHEN** the product presents a label it supplies for a system-generated state, such as the current person in a member list or a rabbit that has never been weighed
- **THEN** that label is presented in the language in effect
- **AND** it is not left in a single fixed language

#### Scenario: Metric units are the default

- **WHEN** the person has chosen no display unit and the device's regional settings indicate none
- **THEN** weights are presented in metric units
