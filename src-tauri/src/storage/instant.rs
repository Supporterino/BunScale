//! Absolute-instant timestamps at the storage boundary (D0.5, DM-6).
//!
//! A timestamp is an **absolute instant**, never a local-time string and never a bare offset. The
//! wire value is the frozen [`Instant`](crate::ipc::types::Instant) — RFC 3339 text carrying an
//! explicit offset — and this module is the one place that interprets its content. The four rules
//! DM-6 and NFR-10 depend on live here rather than at each call site:
//!
//! 1. **Parsing refuses anything but an absolute instant.** [`AbsoluteInstant::parse`] accepts RFC
//!    3339 text with an offset and answers a typed [`InstantError`] otherwise. Text without an
//!    offset (`2026-10-19T10:00:00`) names no moment, so it is refused rather than guessed at, and
//!    the failure never falls back to the Unix epoch or to the device's timezone: either would turn
//!    a corrupt value into a plausible one.
//! 2. **Storage is canonical UTC.** Construction normalises the parsed value to UTC, so
//!    `2026-10-19T12:00:00+02:00`, `2026-10-19T10:00:00Z` and `2026-10-19T05:00:00-05:00` are one
//!    stored value. The writer's offset is presentation, not part of the moment, and it is not kept
//!    as data (D0.4's single canonical representation).
//! 3. **Formatting does not re-interpret.** [`AbsoluteInstant::to_wire`] formats the stored UTC value
//!    and never consults the device's timezone or current offset, so a stored instant means the same
//!    thing after the device's region or clock settings change (DM-6's device-offset scenario).
//! 4. **"Now" is a creation default, never a rewrite.** [`AbsoluteInstant::at_creation`] is the
//!    boundary's entry point: a supplied wire value is parsed and stored unchanged — including an
//!    instant in the past — and only an absent one reads the clock through
//!    [`AbsoluteInstant::now`]. A supplied value is never clamped, rounded or replaced.
//!
//! **Why `time`, and why `local-offset` is absent.** This module uses the `time` crate's RFC 3339
//! parser and formatter; the manifest enables only `parsing` and `formatting`. The
//! `local-offset` feature is deliberately **not** enabled (Q14): it pulls `num_threads`, which fails
//! to build for `aarch64-apple-ios`. Nothing here needs it — [`OffsetDateTime::now_utc`] belongs to
//! `time`'s default `std` feature, and this module never asks for the device's offset.

use std::fmt;

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use crate::ipc::types::Instant;

/// An absolute instant held in canonical UTC (D0.5, DM-6).
///
/// The inner value is always in UTC: [`AbsoluteInstant::parse`] normalises the writer's offset away
/// on construction, and [`AbsoluteInstant::now`] starts in UTC. Equality and ordering therefore
/// follow the instant, not the text that produced it — two values written with different offsets
/// but naming the same moment compare equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbsoluteInstant(OffsetDateTime);

/// Why text supplied as an instant is not one (DM-6).
///
/// A refusal is a typed outcome rather than a panic or a default, so a caller must decide what to
/// report and can never present an unparseable value as a stored timestamp. The offending text is
/// deliberately not carried in the error: the storage boundary handles user data, and NFR-3 keeps
/// content out of diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstantError {
    /// The text is not an RFC 3339 timestamp with an explicit offset, or its UTC value falls outside
    /// RFC 3339's four-digit year range, so it names no storable absolute instant. A local-time
    /// string and a bare offset are both this case.
    NotAnAbsoluteInstant,
}

impl fmt::Display for InstantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAnAbsoluteInstant => {
                formatter.write_str("the text is not an RFC 3339 absolute instant")
            }
        }
    }
}

impl std::error::Error for InstantError {}

impl AbsoluteInstant {
    /// Parse the frozen wire value as an absolute instant, normalised to UTC.
    ///
    /// RFC 3339 text is accepted only when it carries an explicit offset — `Z` or `±hh:mm` — because
    /// text without one names no moment. Any offset is accepted and then discarded: the moment is
    /// what is stored, so `+02:00`, `Z` and `-05:00` values naming the same instant produce equal
    /// values. Parsing never reads a clock and never consults the device's timezone.
    ///
    /// The normalised year is checked against RFC 3339's four-digit range: an offset can move a
    /// syntactically valid `9999-12-31T23:59:59-05:00` to year 10000 or a valid
    /// `0000-01-01T00:00:00+05:00` to year -1, and neither can be written back as an RFC 3339
    /// instant. Those are refused rather than allowed to reach a formatter that cannot represent
    /// them.
    pub fn parse(text: &Instant) -> Result<Self, InstantError> {
        let parsed = OffsetDateTime::parse(&text.0, &Rfc3339)
            .map_err(|_| InstantError::NotAnAbsoluteInstant)?;

        // `to_offset` panics when the UTC instant falls outside `time`'s own range — the upper-edge
        // input `9999-12-31T23:59:59-05:00` normalises to year 10000 — so the wire value is
        // untrusted input and must go through the checked form. That range is wider than RFC
        // 3339's, though: `0000-01-01T00:00:00+05:00` normalises to year -1, which `time`
        // accepts but the RFC 3339 formatter cannot write. Checking the four-digit year here
        // closes both edges, so every stored value is one `to_wire` can format.
        let utc = parsed
            .checked_to_offset(UtcOffset::UTC)
            .ok_or(InstantError::NotAnAbsoluteInstant)?;
        if !(0..=9999).contains(&utc.year()) {
            return Err(InstantError::NotAnAbsoluteInstant);
        }

        Ok(Self(utc))
    }

    /// The creation moment, in UTC.
    ///
    /// This is the module's only clock read. It exists for the "the person supplied no time" case
    /// and is reached through [`AbsoluteInstant::at_creation`], so a defaulted timestamp is always
    /// recorded at creation and never invented later.
    ///
    /// Unlike [`AbsoluteInstant::parse`], this path cannot refuse its input: the system clock is
    /// trusted to lie inside RFC 3339's four-digit year range (`0000`–`9999`). A device clock set
    /// outside that range is a system fault, not a product input; [`AbsoluteInstant::to_wire`]
    /// would panic rather than silently clamp the value or substitute a different one.
    pub fn now() -> Self {
        Self(OffsetDateTime::now_utc())
    }

    /// The instant to record at creation: the supplied value unchanged, or `now` when none was
    /// supplied.
    ///
    /// The `Some` branch only parses: it never clamps to the present, rounds to seconds or rewrites
    /// the offset into local time. An explicit instant in the past is a valid record of that moment
    /// and is stored exactly as given. An unparseable supplied value is refused through
    /// [`InstantError`] rather than replaced with `now`, so a corrupt input cannot masquerade as a
    /// creation timestamp.
    pub fn at_creation(supplied: Option<&Instant>) -> Result<Self, InstantError> {
        match supplied {
            Some(text) => Self::parse(text),
            None => Ok(Self::now()),
        }
    }

    /// Format the stored instant as the frozen wire value: RFC 3339 in UTC.
    ///
    /// The value is already normalised to UTC, so the formatter emits `Z` and never a device offset.
    /// This is what makes a round trip stable: parsing the result yields the same moment, whatever
    /// offset the original text used.
    ///
    /// # Panics
    ///
    /// Only for a value produced by [`AbsoluteInstant::now`] from a device clock outside RFC
    /// 3339's four-digit year range (`0000`–`9999`). Every parsed value is year-checked at
    /// construction (see [`AbsoluteInstant::parse`]); the system clock is the one path this type
    /// cannot refuse, and a clock outside that range is a system fault rather than a product input.
    pub fn to_wire(&self) -> Instant {
        // The invariant is checked for every parsed value — `parse` refuses a UTC year outside RFC
        // 3339's four-digit range — and assumed only for `now`, whose value comes from the system
        // clock. A clock outside that range is a system fault, not a product input: the `expect`
        // records the precondition rather than silently clamping or substituting a timestamp.
        let text = self
            .0
            .format(&Rfc3339)
            .expect("the stored year must lie inside RFC 3339's four-digit range");

        Instant(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    #[test]
    fn the_same_moment_written_with_different_offsets_is_one_stored_value() {
        let plus_two = AbsoluteInstant::parse(&instant("2026-10-19T12:00:00+02:00"))
            .expect("a positive offset is an absolute instant");
        let utc = AbsoluteInstant::parse(&instant("2026-10-19T10:00:00Z"))
            .expect("Z is an absolute instant");
        let minus_five = AbsoluteInstant::parse(&instant("2026-10-19T05:00:00-05:00"))
            .expect("a negative offset is an absolute instant");

        // The moment, not the textual offset, is the stored value.
        assert_eq!(plus_two, utc);
        assert_eq!(minus_five, utc);

        let stored = instant("2026-10-19T10:00:00Z");
        assert_eq!(plus_two.to_wire(), stored);
        assert_eq!(utc.to_wire(), stored);
        assert_eq!(minus_five.to_wire(), stored);
    }

    #[test]
    fn parsing_then_formatting_then_parsing_is_stable() {
        let supplied = instant("2026-10-19T12:34:56.123456789+05:30");
        let first = AbsoluteInstant::parse(&supplied).expect("a fractional instant is accepted");
        let formatted = first.to_wire();
        let second = AbsoluteInstant::parse(&formatted).expect("the formatted value parses");

        assert_eq!(second, first);
        assert_eq!(second.to_wire(), formatted);
        assert!(
            formatted.0.ends_with('Z'),
            "a stored instant is formatted in UTC, never with the writer's offset"
        );
    }

    #[test]
    fn a_supplied_past_instant_survives_creation_unchanged() {
        let supplied = instant("2001-02-03T04:05:06Z");

        let created =
            AbsoluteInstant::at_creation(Some(&supplied)).expect("the supplied instant is valid");

        assert_eq!(created.to_wire(), supplied);
        assert_eq!(
            created,
            AbsoluteInstant::parse(&supplied).expect("the same text names the same instant")
        );
    }

    #[test]
    fn a_supplied_instant_is_never_rounded_or_clamped() {
        // A future value with nanoseconds: a clamp to `now` or a rounding to whole seconds would
        // change at least one of them.
        let supplied = instant("2099-12-31T23:59:59.999999999-08:00");

        let created =
            AbsoluteInstant::at_creation(Some(&supplied)).expect("the supplied instant is valid");

        assert_eq!(created.to_wire(), instant("2100-01-01T07:59:59.999999999Z"));
    }

    #[test]
    fn creation_without_a_supplied_instant_defaults_to_the_creation_moment() {
        let before = AbsoluteInstant::now();
        let created = AbsoluteInstant::at_creation(None).expect("no supplied instant is no error");
        let after = AbsoluteInstant::now();

        assert!(
            created >= before,
            "the default is not earlier than the moment before creation"
        );
        assert!(
            created <= after,
            "the default is not later than the moment after creation"
        );
    }

    #[test]
    fn an_invalid_supplied_instant_is_refused_rather_than_replaced_with_now() {
        let result = AbsoluteInstant::at_creation(Some(&instant("2026-10-19T10:00:00")));

        assert_eq!(result, Err(InstantError::NotAnAbsoluteInstant));
    }

    #[test]
    fn text_that_is_not_an_absolute_instant_is_refused() {
        // A missing offset, a date alone, an out-of-range component and prose are all refused; none
        // is silently treated as the epoch or as a local time.
        for text in [
            "2026-10-19T10:00:00",
            "2026-10-19",
            "2026-13-19T10:00:00Z",
            "2026-10-19T25:00:00Z",
            "not a timestamp",
            "",
        ] {
            let wire = instant(text);

            assert_eq!(
                AbsoluteInstant::parse(&wire),
                Err(InstantError::NotAnAbsoluteInstant),
                "{text:?} must be refused rather than defaulted"
            );
        }
    }

    #[test]
    fn an_instant_whose_utc_year_exceeds_the_rfc_3339_range_is_refused() {
        // Valid RFC 3339 at the upper edge, but normalising -05:00 to UTC moves it to year 10000,
        // where `to_offset` panics and the RFC 3339 formatter cannot write it.
        let result = AbsoluteInstant::parse(&instant("9999-12-31T23:59:59-05:00"));

        assert_eq!(result, Err(InstantError::NotAnAbsoluteInstant));
    }

    #[test]
    fn an_instant_whose_utc_year_precedes_the_rfc_3339_range_is_refused() {
        // Valid RFC 3339 at the lower edge, but +05:00 moves it to year -1: `time` accepts that,
        // while the RFC 3339 formatter does not, so construction must refuse it too.
        let result = AbsoluteInstant::parse(&instant("0000-01-01T00:00:00+05:00"));

        assert_eq!(result, Err(InstantError::NotAnAbsoluteInstant));
    }

    #[test]
    fn instants_order_by_their_moment_not_their_text() {
        let earlier = AbsoluteInstant::parse(&instant("2026-10-19T12:00:00+02:00"))
            .expect("the value is valid");
        let later =
            AbsoluteInstant::parse(&instant("2026-10-19T10:00:01Z")).expect("the value is valid");

        // One second after the first moment, although its text sorts earlier.
        assert!(earlier < later);
    }
}
