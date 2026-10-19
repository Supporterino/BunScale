//! The device's own preferences (D0.2 "Rules held here", SET-4).
//!
//! `design.md` gives this directory one rule — per-device preferences live on the device and are
//! never part of a household collection — and P0 ships exactly one of them: the display weight unit
//! (SET-4). That rule is what makes this directory a peer of [`crate::storage`] rather than a part of
//! it: `storage/` holds the device's *copy of a household's collection*, which is shared data cached
//! locally, while a unit chosen here is nobody else's business — it neither reaches the shared
//! service nor becomes a second record of anything a household already owns.
//!
//! **Where the unit is applied, and where it is not.** A stored weight has one canonical
//! representation — grams (D0.4) — and nothing here reads, writes or rewrites one. What this module
//! provides is the conversion *boundary*: [`unit::to_display`] turns a stored gram amount into the
//! number to present, [`unit::to_canonical`] turns an entered number back into grams, and both are
//! called only where a value is entered or displayed. Changing the preference is therefore the write
//! of one small local file: there is no code path from the preference to stored data, which is
//! SET-4's "Changing the unit leaves stored values unchanged" as a property of the layout rather
//! than of care.
//!
//! Three pieces, one job each:
//!
//! - [`unit`] — the only place in the crate that interprets a [`WeightUnit`], including the rule
//!   that turns a person's choice and the device's region into one effective unit (SET-4);
//! - [`region`] — the device's regional settings, read for the one question the default asks, with
//!   the source, the adoption of "metric is the default" and the iOS gap (**Q26**, which owns a
//!   native region source) documented there;
//! - [`store`] — the person's choice, persisted in one file in the app's own data area, with a
//!   failure reported rather than a default invented.
//!
//! **SET-4's default is three cases, and the last one is what a device that cannot be read answers:**
//! a stored choice wins; failing that, the region's unit when the device states the metric system;
//! failing that, the canonical gram. The rule is [`unit`]'s and [`region`] owns what "the device
//! states" can reach — iOS is not readable from the process, which is the gap **Q26** in
//! `questions.md` owns.
//!
//! [`WeightUnit`]: crate::ipc::types::WeightUnit

pub mod region;
pub mod store;
pub mod unit;

pub use region::MeasurementSystem;
pub use store::{DisplayUnitStore, PreferenceError};
pub use unit::{effective_unit, to_canonical, to_display};
