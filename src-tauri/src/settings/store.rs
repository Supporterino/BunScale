//! The per-device preference store: where the chosen unit lives, and what a read or write failure
//! means (SET-4, D0.2 "Rules held here").
//!
//! # Where it lives
//!
//! One file, `device-preferences.json`, directly in the app's own data directory — the one
//! `AppHandle::path().app_data_dir()` resolves, which is per device and inside the app's own area on
//! both iOS and macOS. [`DisplayUnitStore::in_dir`] takes that directory as an argument, so this
//! module knows no path of its own and hardcodes none. The file is **not** a household record and
//! never reaches the shared service: the display unit is one device's presentation choice, and the
//! design's rules put per-device preferences beside the household collection rather than in it
//! (D0.2).
//!
//! # The file's content
//!
//! ```json
//! { "format": 1, "display_weight_unit": "kilograms" }
//! ```
//!
//! The unit is written with exactly the token the contract uses (`grams` / `kilograms`), because the
//! stored preference and the wire value are one representation and a second encoding would be a
//! second thing to keep in step. `format` is the file's own version: a file this version did not
//! write is a failure rather than a guess about which fields it means, while unknown *fields* are
//! ignored, so a later phase can add a preference (P4's language) to the same file without breaking
//! this reader. The file exists exactly when a unit has been chosen, and it carries nothing else — no
//! weight value may appear in it, since a value in the display unit is the one thing D0.4 forbids
//! (the tests below assert the file's keys).
//!
//! # Writes, and why the choice survives a relaunch
//!
//! A write encodes the whole file, writes it to a temporary sibling and renames that into place:
//! `rename` within one directory is atomic, so a reader — this process, the next launch, or a process
//! racing the write — sees either the previous choice or the new one and never a half-written file.
//! The temporary file is written beside its destination rather than in the system's temporary area,
//! because `rename` is atomic only within one filesystem and the app's data area is not guaranteed to
//! share one with it. Nothing is cached in the process, so the next launch reads the same file —
//! which is what SET-4's "it is still in effect the next time the product starts on that device"
//! means here.
//!
//! **A write that never finished leaves its temporary behind, so every write sweeps those.** A crash
//! or a suspension that ends the process between the `fs::write` and the rename cannot damage the
//! store's own file — the rename is what replaces it — but it does leave the temporary sibling in the
//! app's data area, and nothing else would ever remove it. Each write therefore removes siblings of
//! its own file that match the pattern and are older than [`STALE_TEMPORARY_AGE`]. A *younger* sibling
//! may belong to a write another process is performing at that instant and is deliberately left
//! alone: removing it would fail that write for a fault of ours.
//!
//! **What the read-back proves, and what it does not.** The write is reported as done once the rename
//! has returned and the caller has read the unit back, so what is proven is a *completed* write that
//! the very next read observes — the read the next launch performs — and never a half-applied one. No
//! `sync_all` is issued, so this is **not** a power-loss durability guarantee: an unclean shutdown in
//! the seconds after a change may leave the device reading the previous choice. That trade-off is
//! taken deliberately rather than overlooked. SET-4 asks that the choice be in effect the next time
//! the product starts on the device, which the rename and the read-back establish, while fsyncing a
//! presentation unit on every tap would buy durability the requirement does not ask for. Household
//! data is not this store's business, and its durability is argued where that data is written.
//!
//! # What a failure means
//!
//! A **missing** file is not a failure: it is a device on which nobody has chosen a unit yet, which
//! is exactly the case SET-4's regional default exists for. Every other outcome that does not answer
//! with a unit — unreadable, unwritable, or content this version cannot interpret — is a
//! [`PreferenceError`], reported rather than papered over with a default: a read that cannot answer
//! must not be presented as a stored preference, and a write that did not land must not be reported
//! as stored.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use super::region;
use super::unit;
use crate::ipc::types::WeightUnit;

/// The store's file name inside the app's own data directory.
pub const PREFERENCES_FILE_NAME: &str = "device-preferences.json";

/// The format this module writes and the only format it reads.
const FORMAT_VERSION: u32 = 1;

/// How old a temporary sibling must be before a write treats it as a process that never finished.
///
/// A temporary file lives for as long as one `fs::write` and one `fs::rename` take — microseconds. A
/// minute is far beyond that even on a loaded device, so a sibling at least that old is treated as
/// abandoned, while a younger one may belong to a write another process is performing at this instant
/// and has to be left alone (see [`DisplayUnitStore::sweep_stale_temporaries`]). A *suspended* writer
/// is the one exception: it can resume after this long, find its temporary gone, and fail its rename
/// — which is reported, and is the honest outcome for a write that could not place its file.
const STALE_TEMPORARY_AGE: Duration = Duration::from_secs(60);

/// The per-device preference store: one file in a directory the caller owns.
#[derive(Debug, Clone)]
pub struct DisplayUnitStore {
    path: PathBuf,
}

impl DisplayUnitStore {
    /// The store in `data_dir`: the app's own data directory, as
    /// `AppHandle::path().app_data_dir()` resolves it.
    pub fn in_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(PREFERENCES_FILE_NAME),
        }
    }

    /// The file this store reads and writes: `data_dir` plus [`PREFERENCES_FILE_NAME`].
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The unit chosen on this device, or `None` when nobody has chosen one (SET-4).
    ///
    /// # Errors
    ///
    /// [`PreferenceError::Io`] when the file exists but cannot be read, and
    /// [`PreferenceError::Malformed`] when it exists but is not a preference this version wrote. A
    /// missing file is `Ok(None)` and not a failure — see the module documentation for why.
    pub fn choice(&self) -> Result<Option<WeightUnit>, PreferenceError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        Ok(Some(decode(&bytes)?))
    }

    /// Record `unit` as this device's choice, replacing any previous one (SET-4).
    ///
    /// # Errors
    ///
    /// [`PreferenceError::Io`] when the file cannot be written, and [`PreferenceError::Malformed`]
    /// when the choice cannot be encoded. Either way nothing is reported as stored until the rename
    /// into place has happened.
    pub fn store_choice(&self, unit: WeightUnit) -> Result<(), PreferenceError> {
        self.write(encode(unit)?)
    }

    /// The effective unit for a device whose regional settings state `system`: the choice when there
    /// is one, otherwise the region's unit (SET-4).
    ///
    /// The system is an argument here rather than read inside, so the derivation is testable without
    /// a device; [`DisplayUnitStore::effective_for_this_device`] is the same rule with the device's
    /// own answer behind it.
    ///
    /// # Errors
    ///
    /// As [`DisplayUnitStore::choice`].
    pub fn effective(
        &self,
        system: region::MeasurementSystem,
    ) -> Result<WeightUnit, PreferenceError> {
        Ok(unit::effective_unit(self.choice()?, system))
    }

    /// The effective unit on this device: the choice when there is one, otherwise what this device's
    /// regional settings select (SET-4).
    ///
    /// **This may spawn a process on macOS** — the regional read is `/usr/bin/defaults` (see
    /// [`region`]) — so it must be called from a blocking pool, never directly in an async context.
    /// The read happens only when no choice is stored, so the common path touches no process.
    ///
    /// # Errors
    ///
    /// As [`DisplayUnitStore::choice`].
    pub fn effective_for_this_device(&self) -> Result<WeightUnit, PreferenceError> {
        self.effective_with(region::measurement_system)
    }

    /// The effective unit from a stored choice and a region read performed only when there is none.
    ///
    /// The region is a closure rather than a value so that "a stored choice is the answer, and the
    /// device is not asked" is a property the tests can pin without a device: one of them passes a
    /// closure that fails if it is called at all.
    fn effective_with(
        &self,
        stated_region: impl FnOnce() -> region::MeasurementSystem,
    ) -> Result<WeightUnit, PreferenceError> {
        let choice = self.choice()?;

        match choice {
            Some(chosen) => Ok(chosen),
            None => Ok(unit::regional_unit(stated_region())),
        }
    }

    /// Write `encoded` as the whole file, creating the data directory when it does not exist yet.
    fn write(&self, encoded: Vec<u8>) -> Result<(), PreferenceError> {
        // The directory is there on a device that has run before; creating it keeps the very first
        // write on a fresh install honest instead of failing on a missing parent.
        let parent = self.path.parent();
        let directory = parent.filter(|dir| !dir.as_os_str().is_empty());

        if let Some(directory) = directory {
            fs::create_dir_all(directory)?;
        }

        // Temporaries a write that never finished left behind go first, so this write's own temporary
        // does not exist yet and cannot be swept (see the module documentation).
        self.sweep_stale_temporaries();

        // The whole file goes to a temporary sibling and is renamed into place, so the store's own
        // path only ever holds a complete file (see the module documentation). A temporary file a
        // failing step abandons is removed rather than left for nothing to read.
        let temporary = self.temporary_path();

        if let Err(error) = fs::write(&temporary, encoded) {
            Self::discard(&temporary);

            return Err(error.into());
        }

        if let Err(error) = fs::rename(&temporary, &self.path) {
            Self::discard(&temporary);

            return Err(error.into());
        }

        Ok(())
    }

    /// Remove a temporary file a failed write left behind.
    ///
    /// Without this a half-written or unplaced file would stay in the app's data area under a name
    /// nothing reads. A second failure while removing it has nothing to add to the first, which is
    /// the one the caller acts on.
    fn discard(temporary: &Path) {
        let _cleanup = fs::remove_file(temporary);
    }

    /// Remove temporary siblings a write that never finished left behind.
    ///
    /// The pattern is this store's own file name, a dot, and the write's process id and sequence (see
    /// [`DisplayUnitStore::temporary_path`]), so a sibling of any other file is never touched and the
    /// store's own file — which carries no `.writing` — cannot match. A sibling younger than
    /// [`STALE_TEMPORARY_AGE`] is left alone: it may be another process's write in flight.
    ///
    /// A sweep that cannot list the directory is not reported: the write that follows reads and writes
    /// that very directory, so it fails on the same condition with a cause the caller acts on, and a
    /// second failure here would have nothing to add to it.
    fn sweep_stale_temporaries(&self) {
        let Some(directory) = self.path.parent() else {
            return;
        };
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        let prefix = format!("{PREFERENCES_FILE_NAME}.");

        for entry in entries.flatten() {
            if !names_a_temporary(&entry, &prefix) {
                continue;
            }

            if is_older_than(&entry, STALE_TEMPORARY_AGE) {
                let _cleanup = fs::remove_file(entry.path());
            }
        }
    }

    /// A temporary path beside the destination, unique per process and per write.
    ///
    /// `rename` is atomic only within one filesystem, so the temporary file belongs in the same
    /// directory as its destination rather than in the system's temporary area. Two of these writes
    /// can be in flight at once — Tauri does not serialise commands — so the name carries the
    /// process id and a per-process sequence, and neither write can clobber the other's file.
    fn temporary_path(&self) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut path = self.path.clone().into_os_string();
        path.push(format!(".{}.{sequence}.writing", std::process::id()));

        PathBuf::from(path)
    }
}

/// Whether a directory entry is one of this store's temporary files.
fn names_a_temporary(entry: &fs::DirEntry, prefix: &str) -> bool {
    let name = entry.file_name();
    let name = name.to_string_lossy();

    name.starts_with(prefix) && name.ends_with(".writing")
}

/// Whether an entry was last modified longer ago than `age`.
///
/// A modification time that cannot be read, or one that lies in the future (a clock that moved
/// backwards), is not evidence of an abandoned write, so the entry is left alone.
fn is_older_than(entry: &fs::DirEntry, age: Duration) -> bool {
    let Ok(metadata) = entry.metadata() else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
        return false;
    };

    elapsed > age
}

/// The bytes of the file that records `unit` as the chosen one.
fn encode(unit: WeightUnit) -> Result<Vec<u8>, PreferenceError> {
    let preferences = StoredPreferences {
        format: FORMAT_VERSION,
        display_weight_unit: unit,
    };

    let encoded = serde_json::to_vec(&preferences);

    encoded.map_err(|_| PreferenceError::Malformed)
}

/// The unit one file's bytes record, or a failure when they are not a file this version wrote.
fn decode(bytes: &[u8]) -> Result<WeightUnit, PreferenceError> {
    let parsed = serde_json::from_slice::<StoredPreferences>(bytes);
    let stored = parsed.map_err(|_| PreferenceError::Malformed)?;

    if stored.format != FORMAT_VERSION {
        return Err(PreferenceError::Malformed);
    }

    Ok(stored.display_weight_unit)
}

/// The file's content: the format marker and the one preference this version stores.
///
/// The shape and what may be added to it are the module documentation's subject. The unit is
/// required rather than optional because the file's existence *is* the choice: a file without one is
/// not a file this version wrote, and reading it as "no choice" would be exactly the silent default
/// this store exists not to produce.
#[derive(Debug, Serialize, Deserialize)]
struct StoredPreferences {
    /// The format this file was written in: [`FORMAT_VERSION`], or a value this version cannot
    /// interpret (a failure rather than a guess).
    format: u32,
    /// The unit the person chose.
    display_weight_unit: WeightUnit,
}

/// Why the store could not answer (SET-4).
///
/// This is the store's own classification and not a failure surface: nothing here crosses the IPC
/// boundary, and the command that calls the store maps either variant onto the contract's single
/// [`CommandError`](crate::ipc::types::CommandError), whose honest cause for a local file is
/// `unknown` — a per-device fault is neither connectivity, the account, nor the person's input.
#[derive(Debug)]
pub enum PreferenceError {
    /// The store's file could not be read or written.
    Io(std::io::Error),
    /// The store's content is not a preference this version can interpret — a damaged file, or one a
    /// later format wrote — or a choice could not be encoded into it.
    Malformed,
}

impl std::fmt::Display for PreferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "the device preference store failed: {error}"),
            Self::Malformed => formatter.write_str(
                "the device preference store holds no preference this version can interpret",
            ),
        }
    }
}

impl std::error::Error for PreferenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Malformed => None,
        }
    }
}

impl From<std::io::Error> for PreferenceError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::settings::region::MeasurementSystem;

    #[test]
    fn a_device_with_no_file_has_no_choice_and_that_is_not_a_failure() {
        let directory = temp_data_dir("no-file");
        let store = DisplayUnitStore::in_dir(&directory);

        assert_eq!(choose(&store), None);

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_choice_is_still_in_effect_for_a_store_that_reads_it_again() {
        // A second store over the same directory is the closest a unit test gets to a relaunch:
        // nothing is kept in the process, so the answer can only have come from the file.
        let directory = temp_data_dir("relaunch");
        let before_restart = DisplayUnitStore::in_dir(&directory);
        store_unit(&before_restart, WeightUnit::Kilograms);

        let after_restart = DisplayUnitStore::in_dir(&directory);

        assert_eq!(choose(&after_restart), Some(WeightUnit::Kilograms));

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn storing_a_unit_replaces_the_previous_choice() {
        let directory = temp_data_dir("replaces");
        let store = DisplayUnitStore::in_dir(&directory);

        store_unit(&store, WeightUnit::Grams);
        assert_eq!(choose(&store), Some(WeightUnit::Grams));

        store_unit(&store, WeightUnit::Kilograms);
        assert_eq!(choose(&store), Some(WeightUnit::Kilograms));

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_first_write_creates_the_data_directory() {
        let base = temp_data_dir("first-write");
        let directory = base.join("data");
        let store = DisplayUnitStore::in_dir(&directory);

        assert!(!directory.exists());
        store_unit(&store, WeightUnit::Kilograms);

        assert_eq!(choose(&store), Some(WeightUnit::Kilograms));

        remove_temp_data_dir(&base);
    }

    #[test]
    fn the_preference_file_carries_no_weight_value() {
        // D0.4: no value is stored in the display unit, and this file is where such a value would
        // have to be. Its keys are the whole of what it may carry.
        let directory = temp_data_dir("no-weight-value");
        let store = DisplayUnitStore::in_dir(&directory);
        store_unit(&store, WeightUnit::Kilograms);

        let content = fs::read(store.path()).expect("the preference file");
        let parsed: serde_json::Value = serde_json::from_slice(&content).expect("JSON");
        let object = parsed.as_object().expect("an object");
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();

        assert_eq!(keys, ["display_weight_unit", "format"]);

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_file_this_version_did_not_write_is_a_failure_rather_than_a_default() {
        let directory = temp_data_dir("foreign-file");
        let store = DisplayUnitStore::in_dir(&directory);

        for content in [
            // A later format, whose fields this version cannot be sure of.
            br#"{"format":2,"display_weight_unit":"kilograms"}"#.as_slice(),
            // A file missing what it must carry, and a file that is not JSON at all.
            b"{}".as_slice(),
            b"not json at all".as_slice(),
            // A unit this version does not know.
            br#"{"format":1,"display_weight_unit":"pounds"}"#.as_slice(),
        ] {
            fs::write(store.path(), content).expect("a foreign file");

            let outcome = store.choice();
            let malformed = matches!(outcome, Err(PreferenceError::Malformed));

            assert!(malformed, "{content:?}");
        }

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_file_a_later_version_extended_still_reads_this_preference() {
        // Adding a *field* does not change the format marker (P4's language preference is the
        // expected one), and this reader ignores what it does not know.
        let directory = temp_data_dir("extended-file");
        let store = DisplayUnitStore::in_dir(&directory);
        let content = br#"{"format":1,"display_weight_unit":"grams","language":"de"}"#;
        fs::write(store.path(), content).expect("an extended file");

        assert_eq!(choose(&store), Some(WeightUnit::Grams));

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn without_a_choice_the_effective_unit_is_the_one_the_region_states() {
        let directory = temp_data_dir("regional-default");
        let store = DisplayUnitStore::in_dir(&directory);
        let metric = MeasurementSystem::Metric;
        let non_metric = MeasurementSystem::NonMetric;
        let unknown = MeasurementSystem::Unknown;

        assert_eq!(effective(&store, metric), WeightUnit::Kilograms);
        assert_eq!(effective(&store, non_metric), WeightUnit::Grams);
        assert_eq!(effective(&store, unknown), WeightUnit::Grams);

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_stored_choice_is_the_effective_unit_whatever_the_region_states() {
        let directory = temp_data_dir("stored-wins");
        let store = DisplayUnitStore::in_dir(&directory);
        store_unit(&store, WeightUnit::Grams);

        let systems = [
            MeasurementSystem::Metric,
            MeasurementSystem::NonMetric,
            MeasurementSystem::Unknown,
        ];

        for system in systems {
            assert_eq!(effective(&store, system), WeightUnit::Grams);
        }

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_stored_choice_is_honoured_without_reading_the_region() {
        // The rule's first case, pinned without a device: the region read is a closure here, and this
        // one fails the test if it is called at all — a device that has chosen a unit must not be
        // asked about its region.
        let directory = temp_data_dir("choice-first");
        let store = DisplayUnitStore::in_dir(&directory);
        store_unit(&store, WeightUnit::Grams);

        let by_the_closure = store.effective_with(|| panic!("the region is not read"));
        let on_this_device = store.effective_for_this_device();

        assert_eq!(by_the_closure.expect("readable"), WeightUnit::Grams);
        assert_eq!(on_this_device.expect("readable"), WeightUnit::Grams);

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_malformed_file_does_not_block_storing_a_choice() {
        // A damaged file is a read failure, not a reason to refuse the change: a write replaces the
        // whole file, so choosing a unit is what repairs the device.
        let directory = temp_data_dir("recovery");
        let store = DisplayUnitStore::in_dir(&directory);
        let damaged = fs::write(store.path(), b"not json at all");
        damaged.expect("a damaged file");

        assert!(matches!(store.choice(), Err(PreferenceError::Malformed)));

        store_unit(&store, WeightUnit::Grams);

        assert_eq!(choose(&store), Some(WeightUnit::Grams));

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn an_unreadable_store_fails_rather_than_answering_no_choice() {
        // "Nothing has been chosen" and "the store cannot be read" are different facts: the first
        // answers the regional default, the second must be reported rather than presented as a device
        // nobody has chosen on. A directory where the file belongs makes the read fail without
        // pretending the file is absent.
        let directory = temp_data_dir("unreadable");
        let store = DisplayUnitStore::in_dir(&directory);
        let placed = fs::create_dir(store.path());
        placed.expect("a directory where the file belongs");

        let outcome = store.choice();
        let unreadable = matches!(outcome, Err(PreferenceError::Io(_)));

        assert!(unreadable);

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_write_sweeps_a_temporary_a_write_that_never_finished_left_behind() {
        // The suspension case the module documentation records: a process that ended between the
        // write and the rename leaves a `.writing` sibling, and nothing else would ever remove it.
        let directory = temp_data_dir("sweep");
        let store = DisplayUnitStore::in_dir(&directory);
        let abandoned = directory.join(format!("{PREFERENCES_FILE_NAME}.1.0.writing"));
        write_stale(&abandoned);

        store_unit(&store, WeightUnit::Kilograms);

        assert!(!abandoned.exists());
        assert_eq!(choose(&store), Some(WeightUnit::Kilograms));

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_write_leaves_a_fresh_temporary_alone() {
        // A sibling younger than the sweep's threshold may belong to a write another process is
        // performing at this instant; removing it would fail that write for a fault of ours.
        let directory = temp_data_dir("fresh-temporary");
        let store = DisplayUnitStore::in_dir(&directory);
        let in_flight = directory.join(format!("{PREFERENCES_FILE_NAME}.2.0.writing"));
        let wrote = fs::write(&in_flight, b"{}");
        wrote.expect("an in-flight temporary");

        store_unit(&store, WeightUnit::Kilograms);

        assert!(in_flight.exists());

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn changing_the_unit_leaves_held_data_untouched() {
        // SET-4's "Changing the unit leaves stored values unchanged", at the level this module owns:
        // the store writes exactly one file and reads no other, and the conversion itself is a
        // value-preserving pair (`unit`'s tests pin that). A device's held data is represented here
        // by a sibling file the same data area would carry, so a write that touched anything but the
        // preference would fail this test.
        let directory = temp_data_dir("held-data");
        let store = DisplayUnitStore::in_dir(&directory);
        let held = directory.join("household-items.json");
        let content = br#"{"item_id":"rabbit-1","value_grams":2100}"#;
        fs::write(&held, content).expect("held data");
        let before = fs::read(&held).expect("held data");

        store_unit(&store, WeightUnit::Grams);
        store_unit(&store, WeightUnit::Kilograms);

        assert_eq!(choose(&store), Some(WeightUnit::Kilograms));
        assert_eq!(fs::read(&held).expect("held data"), before);

        let expected = [
            PREFERENCES_FILE_NAME.to_owned(),
            "household-items.json".to_owned(),
        ];

        assert_eq!(files_in(&directory), expected);

        remove_temp_data_dir(&directory);
    }

    /// A fresh directory under the process's temporary area, unique per call.
    ///
    /// The tests run in parallel and this crate has no temporary-directory dependency, so the name
    /// is built from the process id and a per-process sequence: two tests cannot share a directory,
    /// and two runs of the suite cannot either.
    fn temp_data_dir(label: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        let name = format!("bunscale-display-unit-{}-{sequence}-{label}", process);
        let directory = std::env::temp_dir().join(name);

        fs::create_dir_all(&directory).expect("a test directory");

        directory
    }

    /// The names of the files in `directory`, sorted, so the comparison does not depend on order.
    fn files_in(directory: &Path) -> Vec<String> {
        let mut names = Vec::new();

        for entry in fs::read_dir(directory).expect("the test directory") {
            let entry = entry.expect("an entry");
            let name = entry.file_name();

            names.push(name.to_string_lossy().into_owned());
        }

        names.sort();

        names
    }

    /// Best-effort removal of a test's directory.
    ///
    /// The assertions are the test's subject, and a leftover directory under the temporary area is
    /// not one of them, so this does not fail a passing test — and it cannot hide a failing one,
    /// because it runs after the assertions.
    fn remove_temp_data_dir(directory: &Path) {
        let _cleanup = fs::remove_dir_all(directory);
    }

    /// Store `unit` through the store, failing the test when the write does.
    fn store_unit(store: &DisplayUnitStore, unit: WeightUnit) {
        store.store_choice(unit).expect("the unit is stored");
    }

    /// The unit the store reads, failing the test when the file cannot be read.
    fn choose(store: &DisplayUnitStore) -> Option<WeightUnit> {
        store.choice().expect("the choice is readable")
    }

    /// Create a file at `path` whose modification time is well past the sweep's threshold.
    fn write_stale(path: &Path) {
        let file = fs::File::create(path).expect("a stale temporary");
        let modified = SystemTime::now() - Duration::from_secs(3_600);
        let times = fs::FileTimes::new().set_modified(modified);

        file.set_times(times).expect("an old modification time");
    }

    /// The effective unit the store reports for `system`, failing the test on a read failure.
    fn effective(store: &DisplayUnitStore, system: MeasurementSystem) -> WeightUnit {
        store.effective(system).expect("the choice is readable")
    }
}
