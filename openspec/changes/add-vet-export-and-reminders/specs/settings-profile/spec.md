# settings-profile — delta for add-vet-export-and-reminders

Traces requirement **SET-3** — **ADDED only**.
Stories: US-SET-04, US-SET-05.

**Capability ownership.** `settings-profile` is opened by P0 (`add-shared-household-foundation`),
which owns `Settings & Profile entry point on the dashboard` (SET-1), `Current person's name is
shown read-only with fallbacks` (SET-2) and `Display weight unit is per-device and applied at the
boundary` (SET-4). None of those is restated, modified or removed here; no
`## MODIFIED Requirements` section appears in this file.

Stories US-SET-04 and US-SET-05 trace to SET-3 **and** to NFR-5. The split kept clean across the
two capabilities in this change is: **SET-3 (here)** owns the selectable override, its per-device
storage and its next-launch effect; **NFR-5 (`product-quality`, this same change)** owns the
completeness of German and English coverage and the metric default. US-SET-04's acceptance
criterion that all text is German after relaunch is NFR-5's coverage requirement, reached through
the override specified here.

## ADDED Requirements

### Requirement: Product language override is per device and applies on next launch

**SET-3.** The product language SHALL be selectable as **System**, **Deutsch** or **English**. The
choice SHALL be stored **per device**. A change SHALL take effect on the **next launch**: the
running session SHALL keep the language it started with, and the person SHALL be informed that the
change applies on the next launch. When the choice is System, the language SHALL follow the
device's language. The choice SHALL NOT affect another device, and SHALL NOT be shared with other
members.

#### Scenario: The product language is selectable as System, Deutsch or English

- **WHEN** the language setting is presented on a device
- **THEN** System, Deutsch and English are all selectable
- **AND** exactly one of them is in effect for that device

#### Scenario: The chosen language is stored on the device and used on the next launch

- **WHEN** a person selects Deutsch on a device and launches the product again on that device
- **THEN** the product is presented in German without the person choosing again

#### Scenario: A language change applies on the next launch, not in the running session

- **WHEN** a person selects a different language while the product is running
- **THEN** the running session keeps the language it started with
- **AND** the newly selected language is in effect on the next launch

#### Scenario: The person is told the change applies on the next launch

- **WHEN** a person selects a different language while the product is running
- **THEN** the product states that the change takes effect on the next launch
- **AND** the current session's language is not presented as having changed

#### Scenario: System follows the device's language

- **WHEN** the language setting is System
- **THEN** the language presented on the next launch is the device's language

#### Scenario: The choice on one device does not affect another device

- **WHEN** a person changes the language on one of their devices
- **THEN** the language presented on their other device is unchanged
- **AND** no other member's presentation is affected
