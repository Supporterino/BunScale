//! The device's regional settings, for the one question the display unit's default asks (SET-4).
//!
//! SET-4 says the display unit "SHALL default from the device's regional settings", so this module
//! answers exactly that and nothing else: which measurement system the device's regional settings
//! state. [`crate::settings::unit`] composes the answer into a unit and owns the rule, so the two
//! halves stay separable and each is testable on its own.
//!
//! # Where the answer comes from, and why there
//!
//! The pinned `tauri` 2.12.4 exposes **no** locale or region API, and this task may add no
//! dependency for one, so the answer comes from what the platform itself makes readable to a Rust
//! process, in this order:
//!
//! 1. **The POSIX locale environment**: `LC_ALL`, then `LC_MEASUREMENT`, then `LANG`, in that order
//!    (POSIX gives `LC_ALL` precedence over every category, then the category, then `LANG`).
//!    `LC_MEASUREMENT` is the category POSIX defines for measurement units, so this is the setting
//!    the question is actually about rather than a proxy for it.
//! 2. **macOS's own `AppleLocale`**, read through `/usr/bin/defaults` ([`default_system_locale`]),
//!    because a *bundled* macOS app launched from Finder inherits an essentially empty environment
//!    and so usually carries none of the variables above even though the device has a region.
//! 3. **Nothing** — [`MeasurementSystem::Unknown`] — which is what iOS answers: an app there has no
//!    shell environment at all, and the global preference domain that holds `AppleLocale` lies
//!    outside its sandbox.
//!
//! The environment wins when it *names a territory*, which `LC_ALL=C` and `LANG=en` do not: those
//! state a language or an encoding and no region, so they leave the question open for the platform's
//! answer rather than shadowing it — see [`measurement_system_from_sources`], which is the
//! composition and the place that rule is written down.
//!
//! # The iOS gap, and who owns it (Q26)
//!
//! **On iOS the region is not readable from the process at all.** The default there is the canonical
//! unit — the third case of [`crate::settings::unit`]'s rule, *not* a reading of the device's
//! region — and this module cannot do better without a native source. Reading iOS's region needs
//! `NSLocale`, which belongs to this product's Apple-native plugin surface; **Q26 in `questions.md`
//! records that gap and owns the fix**, and a `design.md` line records the same limitation. Until it
//! lands, nothing here adds a dependency or a plugin for it.
//!
//! On macOS the module *expects* the default to follow the device's region, because `AppleLocale` is
//! readable there. **That expectation is unverified.** No test in this crate spawns
//! `/usr/bin/defaults`, the gate's iOS check compiles the read out, and a unit test cannot make a
//! claim about the machine it runs on, so what would confirm it is a recorded observation instead:
//! with no stored choice, a `bun run tauri dev` on a Mac whose `AppleLocale` is `de_DE` must show
//! kilograms in Settings, and one whose `AppleLocale` is `en_US` must show grams.
//!
//! # What a locale says about a measurement system
//!
//! A locale name is `language[_territory][.codeset][@modifier]` (`en_US.UTF-8`, `de_DE@euro`,
//! macOS's `en_US`), and only the **territory** takes part in the answer: the measurement system is
//! chosen per territory, while a language on its own names none — English is the everyday language
//! of the United States *and* of the United Kingdom. A locale with no territory (`en`, `C`,
//! `POSIX`) is therefore [`MeasurementSystem::Unknown`], never a guess from the language.
//!
//! The territories whose everyday system is not metric are the three countries listed in
//! [`NON_METRIC_TERRITORIES`]; every other territory is answered as metric. **That default is
//! adopted, not decided here**: "Metric units SHALL be the default" is **NFR-5 of
//! `add-vet-export-and-reminders`**, which owns the product-wide rule (its scenario is a device whose
//! regional settings indicate no unit system), and this change takes the same direction so a P0
//! default cannot contradict the later requirement that will restate it. The list is the product's
//! whole model of the non-metric world, so a territory it omits — Puerto Rico, say, which uses United
//! States customary units — is answered as metric too: a default, never a claim about a system the
//! device did not state.

#[cfg(target_os = "macos")]
use std::process::Command;

/// The measurement system a device's regional settings state (SET-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementSystem {
    /// The regional settings state the metric system.
    Metric,
    /// The regional settings state another system (see [`NON_METRIC_TERRITORIES`]).
    NonMetric,
    /// The regional settings state no system at all, or no locale could be read. The module
    /// documentation records which platform answers this and why.
    Unknown,
}

/// The territories whose everyday measurement system is not metric.
///
/// The United States, Liberia and Myanmar are the three countries that have not adopted the metric
/// system as their everyday system. The module documentation records what an unlisted territory
/// answers, and why that direction is the honest one.
pub const NON_METRIC_TERRITORIES: [&str; 3] = ["US", "LR", "MM"];

/// The measurement system this device's regional settings state (SET-4).
///
/// This is the device's own answer, composed of the two locale sources the module documentation
/// records: the process environment's, and — only when that one names no territory — the platform's
/// own.
pub fn measurement_system() -> MeasurementSystem {
    let environment = environment_locale();
    let platform = platform_locale(environment.as_deref());

    measurement_system_from_sources(environment.as_deref(), platform.as_deref())
}

/// The measurement system a locale names: the whole rule, as a total function of the string.
///
/// Total in the sense that matters here: any string is accepted — a proper locale, `C`, `POSIX`, a
/// garbled value, an empty string — and a value that names no territory is
/// [`MeasurementSystem::Unknown`] rather than a guess.
pub fn measurement_system_from_locale(locale: &str) -> MeasurementSystem {
    let Some(territory) = territory(locale) else {
        return MeasurementSystem::Unknown;
    };

    if NON_METRIC_TERRITORIES.contains(&territory.as_str()) {
        MeasurementSystem::NonMetric
    } else {
        MeasurementSystem::Metric
    }
}

/// The measurement system two locale sources state together: the environment's answer when it names
/// a territory, otherwise the platform's, otherwise none.
///
/// Both sources are arguments here so the composition is testable without a device. **The
/// environment comes first because it is the setting the person's own session made**, and it wins
/// even over a platform answer that differs — a shell that exports `LC_ALL=de_DE` on a device whose
/// `AppleLocale` is `en_US` has stated the region it wants. It only wins, though, when it *names a
/// territory*: `LANG=en` and `LC_ALL=C` state no region at all, so they leave the question open and
/// the platform's answer is the one that answers it. A device whose region is readable must not be
/// told it stated nothing merely because a shell was set to `C`.
fn measurement_system_from_sources(
    environment: Option<&str>,
    platform: Option<&str>,
) -> MeasurementSystem {
    let stated = match environment {
        Some(locale) if names_a_territory(locale) => Some(locale),
        _ => platform,
    };

    match stated {
        Some(locale) => measurement_system_from_locale(locale),
        None => MeasurementSystem::Unknown,
    }
}

/// The locale the process environment states, by POSIX's own precedence.
fn environment_locale() -> Option<String> {
    let lc_all = read_variable("LC_ALL");
    let lc_measurement = read_variable("LC_MEASUREMENT");
    let lang = read_variable("LANG");

    let selected = selected_locale(
        lc_all.as_deref(),
        lc_measurement.as_deref(),
        lang.as_deref(),
    );

    selected.map(str::to_owned)
}

/// The platform's own locale, read only when the environment states no region.
///
/// The read is skipped, not merely outranked, when the environment already names a territory: on
/// macOS it is a process call (see [`default_system_locale`]) that a session naming its region has
/// made unnecessary.
fn platform_locale(environment: Option<&str>) -> Option<String> {
    if environment.is_some_and(names_a_territory) {
        return None;
    }

    default_system_locale()
}

/// The locale a POSIX environment states, by POSIX's own precedence.
///
/// Split out as a pure function because the precedence is the part worth pinning in a test: a test
/// that set these variables would have to mutate the whole process's environment, which edition
/// 2024 makes `unsafe` for exactly the reason it would be a hazard here — the suite runs in one
/// process, in parallel.
fn selected_locale<'a>(
    lc_all: Option<&'a str>,
    lc_measurement: Option<&'a str>,
    lang: Option<&'a str>,
) -> Option<&'a str> {
    lc_all.or(lc_measurement).or(lang)
}

/// The value of one locale variable, with POSIX's "an empty value means unset" rule applied.
fn read_variable(name: &str) -> Option<String> {
    let value = std::env::var(name).ok();

    value.filter(|value| !value.is_empty())
}

/// Whether a locale names a territory, and so states a region.
fn names_a_territory(locale: &str) -> bool {
    territory(locale).is_some()
}

/// The territory a locale names, upper-cased, or `None` when it names none.
///
/// The name is split the way POSIX and macOS write it — `language[_territory][.codeset][@modifier]`
/// — with `-` accepted as the separator as well, because a locale reaches a process in both
/// spellings (`en_US` from the system, `en-US` from a shell). Everything after the territory is
/// dropped, and a locale missing either its language or its territory names none.
fn territory(locale: &str) -> Option<String> {
    let name = locale.split('@').next().unwrap_or(locale);
    let name = name.split('.').next().unwrap_or(name);
    let mut parts = name.split(['_', '-']);

    let language = parts.next().unwrap_or_default();
    let territory = parts.next().unwrap_or_default();

    if language.is_empty() || territory.is_empty() {
        return None;
    }

    Some(territory.to_ascii_uppercase())
}

/// macOS's own locale, read from the OS.
///
/// `/usr/bin/defaults read -g AppleLocale` prints the user's locale (`en_US`, `de_DE`) out of the
/// global preference domain — the setting a bundled app launched from Finder has but whose POSIX
/// environment is empty, the case the module documentation explains. The binary is part of macOS at
/// a fixed path and is invoked without a shell, so no `PATH` lookup takes place and nothing is
/// quoted.
///
/// Every way of not answering — no such binary, a non-zero exit (the key is unset), output that is
/// not UTF-8, output that is blank — answers `None`, which the caller reads as "no locale stated".
/// A read that cannot answer must not become a guess about the region, and nothing in this crate
/// verifies that a given Mac does answer: the module documentation records what would.
#[cfg(target_os = "macos")]
fn default_system_locale() -> Option<String> {
    let output = Command::new("/usr/bin/defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let locale = String::from_utf8_lossy(&output.stdout);
    let locale = locale.trim();

    if locale.is_empty() {
        None
    } else {
        Some(locale.to_owned())
    }
}

/// The locale the platform states outside the process environment: none, on every platform whose
/// regional settings are not readable this way (the module documentation names iOS as the one that
/// matters here, and Q26 owns reading it).
#[cfg(not(target_os = "macos"))]
fn default_system_locale() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::types::WeightUnit;
    use crate::settings::unit;

    #[test]
    fn a_metric_territory_states_the_metric_system() {
        for locale in [
            "de_DE",
            "de_DE.UTF-8",
            "de-DE",
            "de_DE@euro",
            "en_GB",
            "ja_JP.UTF-8",
        ] {
            let system = measurement_system_from_locale(locale);

            assert_eq!(system, MeasurementSystem::Metric, "{locale}");
        }
    }

    #[test]
    fn the_territories_that_do_not_use_metric_state_another_system() {
        for locale in ["en_US", "en_US.UTF-8", "en-US", "en_LR", "my_MM"] {
            let system = measurement_system_from_locale(locale);

            assert_eq!(system, MeasurementSystem::NonMetric, "{locale}");
        }
    }

    #[test]
    fn a_locale_without_a_territory_states_no_system() {
        for locale in ["", "C", "POSIX", "en", "de.", "en.UTF-8", "@euro", "_US"] {
            let system = measurement_system_from_locale(locale);

            assert_eq!(system, MeasurementSystem::Unknown, "{locale}");
        }
    }

    #[test]
    fn the_territory_is_read_from_the_name_and_not_from_its_surroundings() {
        assert_eq!(territory("en_us"), Some("US".to_owned()));
        assert_eq!(territory("de_DE@euro"), Some("DE".to_owned()));
        assert_eq!(territory("de_DE.UTF-8"), Some("DE".to_owned()));
        assert_eq!(territory("en-US"), Some("US".to_owned()));
        assert_eq!(territory("en"), None);
        assert_eq!(territory("!"), None);
    }

    #[test]
    fn the_posix_precedence_selects_the_locale() {
        let all = Some("en_US");
        let measurement = Some("de_DE");
        let language = Some("ja_JP");

        assert_eq!(selected_locale(all, measurement, language), all);
        assert_eq!(selected_locale(None, measurement, language), measurement);
        assert_eq!(selected_locale(None, None, language), language);
        assert_eq!(selected_locale(None, None, None), None);
    }

    #[test]
    fn a_territory_less_environment_value_does_not_shadow_a_readable_platform_region() {
        // The composition, which is the whole point of splitting the two sources: `LC_ALL=C` and
        // `LANG=en` name no region, so a readable platform locale is what answers. `en_US` is
        // non-metric, which makes the difference observable downstream as the canonical unit — the
        // unit a device with a readable non-metric region gets, and the one a `C` locale alone would
        // wrongly answer from an `en_US` Mac.
        let from_encoding_only = measurement_system_from_sources(Some("C"), Some("en_US"));
        let from_language_only = measurement_system_from_sources(Some("en"), Some("en_US"));

        assert_eq!(from_encoding_only, MeasurementSystem::NonMetric);
        assert_eq!(from_language_only, MeasurementSystem::NonMetric);
        assert_eq!(unit::regional_unit(from_encoding_only), WeightUnit::Grams);
    }

    #[test]
    fn the_environment_wins_when_it_names_a_territory_and_nothing_stated_is_unknown() {
        let environment_wins = measurement_system_from_sources(Some("de_DE.UTF-8"), Some("en_US"));
        let platform_only = measurement_system_from_sources(None, Some("en_US"));
        let language_only = measurement_system_from_sources(Some("en"), None);
        let neither = measurement_system_from_sources(None, None);

        assert_eq!(environment_wins, MeasurementSystem::Metric);
        assert_eq!(platform_only, MeasurementSystem::NonMetric);
        assert_eq!(language_only, MeasurementSystem::Unknown);
        assert_eq!(neither, MeasurementSystem::Unknown);
    }

    #[test]
    fn the_devices_own_answer_follows_the_environment_when_the_environment_states_a_region() {
        // The device-reading half is total and, when this machine's environment names a territory,
        // it is exactly that environment's answer — the platform read is skipped in that case. What a
        // given Mac reports through `/usr/bin/defaults` is *not* asserted here: the module
        // documentation marks that expectation unverified and names the observation that would
        // confirm it.
        let answer = measurement_system();
        let environment = environment_locale();

        if environment.as_deref().is_some_and(names_a_territory) {
            let locale = environment.expect("the environment states a region");

            assert_eq!(answer, measurement_system_from_locale(&locale));
        }
    }
}
