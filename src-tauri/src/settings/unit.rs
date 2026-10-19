//! The display unit, and the conversion boundary that is the only place it is interpreted (D0.4,
//! SET-4).
//!
//! # One canonical value, one presentation
//!
//! A weight is stored in grams and in nothing else (D0.4). The display unit is therefore not a
//! second representation of anything stored: it is the only thing that changes when a value is
//! shown or entered, and [`to_display`] / [`to_canonical`] are that change in the two directions a
//! boundary has. Both are total — every `f64`, either unit, no panic — and both are defined by the
//! single constant [`GRAMS_PER_KILOGRAM`]: presenting a gram amount in kilograms divides by it,
//! reading a kilogram amount back multiplies by it, and the gram unit is the identity in both
//! directions, which is what makes grams the canonical presentation as well as the canonical
//! storage.
//!
//! These two functions are the *whole* boundary. Whoever enters or displays a weight converts here
//! and stores grams, and no value is ever stored in the display unit — so the conversion is a pair
//! that preserves the value it carries, and the preference file the unit lives in holds no weight at
//! all (the tests below pin both halves).
//!
//! # The effective unit
//!
//! [`effective_unit`] is SET-4's rule, in three cases: **a stored choice wins; failing that, the
//! region's unit when the device states the metric system; failing that, the canonical gram.** The
//! middle case is reachable only where the device's region is readable — macOS, not iOS, which is the
//! gap `questions.md` **Q26** owns (see [`crate::settings::region`]) — so on a device whose region
//! cannot be read the default *is* the canonical unit, deliberately and visibly, rather than a
//! reading of a region nobody could read.
//!
//! The rule lives here rather than in [`store`](super::store) because choosing between
//! [`WeightUnit`]'s variants is interpretation, and this module is the crate's only place for it.
//!
//! **The two non-metric answers reach the canonical unit for different reasons**, and the difference
//! is what a later change has to respect:
//!
//! - [`MeasurementSystem::NonMetric`] names a system the frozen [`WeightUnit`] cannot express: the
//!   contract carries two *metric* units and no non-metric one, so there is nothing here to present
//!   that region's own unit with. Answering the canonical gram keeps the presentation in the unit the
//!   value is stored in, and serving such a region its everyday unit is a later change's job — a
//!   contract change (a `WeightUnit` variant), not a decision this module can take on its own.
//! - [`MeasurementSystem::Unknown`] means the device stated no system at all, so nothing was asked
//!   for either. Answering the canonical gram presents the stored value unchanged; answering
//!   kilograms would attribute a measurement-system choice to a device that made none.
//!
//! [`MeasurementSystem::Metric`] is the one case where a display unit is applied without the
//! person's own choice, and it selects [`METRIC_EVERYDAY_UNIT`] — the unit a whole-number rabbit
//! weight is read in.

use crate::ipc::types::WeightUnit;

use super::region::MeasurementSystem;

/// How many grams a kilogram is: the one factor the boundary converts by.
pub const GRAMS_PER_KILOGRAM: f64 = 1_000.0;

/// The unit a stored gram amount is presented in when nothing asks for another one: the canonical
/// unit itself (D0.4).
pub const CANONICAL_UNIT: WeightUnit = WeightUnit::Grams;

/// The metric system's everyday unit for a body weight in this product's range.
///
/// The product weighs rabbits, whose weights are read in whole kilograms — which is why this, and
/// not the finer gram, is the unit a region that states the metric system selects.
pub const METRIC_EVERYDAY_UNIT: WeightUnit = WeightUnit::Kilograms;

/// A stored gram amount as the number `unit` presents: the display half of the boundary.
pub fn to_display(grams: f64, unit: WeightUnit) -> f64 {
    match unit {
        WeightUnit::Grams => grams,
        WeightUnit::Kilograms => grams / GRAMS_PER_KILOGRAM,
    }
}

/// A number entered in `unit` as the gram amount to store: the entry half of the boundary.
pub fn to_canonical(entered: f64, unit: WeightUnit) -> f64 {
    match unit {
        WeightUnit::Grams => entered,
        WeightUnit::Kilograms => entered * GRAMS_PER_KILOGRAM,
    }
}

/// The unit a device's regional settings select (SET-4).
///
/// See the module documentation for why a system other than the metric one selects the canonical
/// unit rather than a second guess.
pub fn regional_unit(system: MeasurementSystem) -> WeightUnit {
    match system {
        MeasurementSystem::Metric => METRIC_EVERYDAY_UNIT,
        MeasurementSystem::NonMetric | MeasurementSystem::Unknown => CANONICAL_UNIT,
    }
}

/// The effective display unit: the person's choice when there is one, otherwise the unit the
/// device's regional settings select (SET-4).
pub fn effective_unit(choice: Option<WeightUnit>, system: MeasurementSystem) -> WeightUnit {
    choice.unwrap_or_else(|| regional_unit(system))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether two gram amounts are the same value to the precision the conversion carries.
    ///
    /// The boundary multiplies and divides by one `f64`, so a round trip is the same gram count to
    /// within a rounding step of that arithmetic and not necessarily bit-for-bit. The tolerance is
    /// what the arithmetic promises, and it is orders of magnitude finer than the gram a weight is
    /// stored with.
    fn within_one_part_in_a_trillion(left: f64, right: f64) -> bool {
        (left - right).abs() <= right.abs() * 1e-12
    }

    #[test]
    fn the_gram_unit_presents_the_stored_value_unchanged() {
        for grams in [0.0, 1.0, 2_100.0, 2_500.0, 999_999.0] {
            assert_eq!(to_display(grams, WeightUnit::Grams), grams, "{grams}");
            assert_eq!(to_canonical(grams, WeightUnit::Grams), grams, "{grams}");
        }
    }

    #[test]
    fn the_kilogram_unit_converts_by_the_one_factor() {
        // The values whose conversion is exact as an `f64`, so this pins the factor itself rather
        // than the rounding of it.
        assert_eq!(to_display(2_500.0, WeightUnit::Kilograms), 2.5);
        assert_eq!(to_canonical(2.5, WeightUnit::Kilograms), 2_500.0);
        assert_eq!(to_display(1_000.0, WeightUnit::Kilograms), 1.0);
        assert_eq!(to_canonical(0.0, WeightUnit::Kilograms), 0.0);
    }

    #[test]
    fn entry_and_display_are_one_value_preserving_pair() {
        // The boundary's whole contract (D0.4): what a person enters is what is stored and what is
        // stored is what is presented, whichever unit is in effect — so a change of unit moves the
        // presentation and never the value.
        for unit in [WeightUnit::Grams, WeightUnit::Kilograms] {
            for grams in [1.0, 2_100.0, 2_500.0, 12_345.0, 999_999.0] {
                let shown = to_display(grams, unit);
                let round_trip = to_canonical(shown, unit);

                assert!(
                    within_one_part_in_a_trillion(round_trip, grams),
                    "{grams} g round-tripped as {round_trip} g",
                );
            }
        }
    }

    #[test]
    fn the_effective_unit_is_the_persons_choice_whenever_there_is_one() {
        let systems = [
            MeasurementSystem::Metric,
            MeasurementSystem::NonMetric,
            MeasurementSystem::Unknown,
        ];

        for system in systems {
            for chosen in [WeightUnit::Grams, WeightUnit::Kilograms] {
                assert_eq!(effective_unit(Some(chosen), system), chosen, "{system:?}");
            }
        }
    }

    #[test]
    fn the_region_selects_the_metric_everyday_unit_only_when_it_states_metric() {
        let metric = MeasurementSystem::Metric;
        let non_metric = MeasurementSystem::NonMetric;
        let unknown = MeasurementSystem::Unknown;

        assert_eq!(regional_unit(metric), WeightUnit::Kilograms);
        assert_eq!(regional_unit(non_metric), WeightUnit::Grams);
        assert_eq!(regional_unit(unknown), WeightUnit::Grams);

        // The same rule as the one the contract reports, with nothing chosen yet.
        assert_eq!(effective_unit(None, metric), WeightUnit::Kilograms);
        assert_eq!(effective_unit(None, non_metric), WeightUnit::Grams);
        assert_eq!(effective_unit(None, unknown), WeightUnit::Grams);
    }
}
