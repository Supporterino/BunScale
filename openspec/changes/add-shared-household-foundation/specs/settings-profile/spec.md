# settings-profile — delta for add-shared-household-foundation

Traces requirements **SET-1, SET-2, SET-4**.
Stories: US-SET-01, US-SET-02, US-SET-03, US-SET-06.
(SET-3, the product-language override, is not part of this change.)

## ADDED Requirements

### Requirement: Settings & Profile entry point on the dashboard

**SET-1.** Settings & Profile SHALL be reachable from a gear-shaped control on the dashboard and
SHALL open as its own screen.

#### Scenario: Gear control opens Settings as its own screen

- **WHEN** the dashboard is presented
- **THEN** a gear-shaped control is present on it
- **AND** activating that control opens Settings & Profile as its own screen

#### Scenario: Settings is its own screen on mobile [mobile]

- **WHEN** Settings & Profile is opened on a 390×844 viewport
- **THEN** it is presented as its own screen, replacing the dashboard
- **AND** a control is present that returns to the dashboard

#### Scenario: Settings is its own screen on desktop [desktop]

- **WHEN** Settings & Profile is opened on a 1280×800 viewport
- **THEN** it is presented as its own screen
- **AND** the dashboard remains reachable without leaving the product

### Requirement: Current person's name is shown read-only with fallbacks

**SET-2.** Settings & Profile SHALL show the current person's name read-only, resolved from their
account identity. When the identity cannot be resolved it SHALL fall back to the locally known
member name, and then to a localized "You"-style placeholder. An unresolved name SHALL NOT
disrupt the screen or prevent the rest of its content from being used.

#### Scenario: Resolved account name is shown read-only

- **WHEN** the person's account identity resolves
- **THEN** Settings & Profile shows that display name
- **AND** the name is not editable

#### Scenario: Unresolved identity falls back without breaking the screen

- **WHEN** the person's account identity cannot be resolved
- **THEN** the locally known member name is shown, or a localized "You"-style placeholder when no local name is known
- **AND** the remaining content of Settings & Profile stays usable

### Requirement: Display weight unit is per-device and applied at the boundary

**SET-4.** The display weight unit (grams or kilograms) SHALL be selectable in Settings, SHALL be
stored per device, and SHALL be applied only where a weight value is entered or displayed. Changing
it SHALL NOT change any stored value. When the person has not chosen a unit, it SHALL default from
the device's regional settings.

#### Scenario: Unit choice is stored on the device

- **WHEN** a person selects grams or kilograms in Settings
- **THEN** the choice is stored for that device
- **AND** it is still in effect the next time the product starts on that device

#### Scenario: Changing the unit leaves stored values unchanged

- **WHEN** the display weight unit is changed
- **THEN** weight values are converted only at the point where they are entered or displayed
- **AND** every stored value remains in the canonical unit

#### Scenario: Default unit follows the device region

- **WHEN** a person has not chosen a display weight unit
- **THEN** the unit is derived from the device's regional settings
