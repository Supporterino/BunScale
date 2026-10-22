//! Integration tests for identity and the display unit (task 5.11): SET-2's two identity scenarios
//! and SET-4's three display-unit scenarios.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-shared-household-foundation/specs/settings-profile/spec.md` states, and
//! each test is named after the scenario it covers.
//!
//! # The boundary these tests drive
//!
//! SET-2's chain is [`resolve_identity`], a total function over the account id, the account's
//! display name and a locally known member name; the command's body is `Ok` around it, so an
//! unresolved identity is a value rather than a failure. SET-4's preference is
//! [`DisplayUnitStore`], one file in a directory the test owns, and its rule is composed in
//! `settings::unit` from the region's answer ([`MeasurementSystem`]): a stored choice wins;
//! otherwise the region's unit when it can be read; otherwise the canonical gram.
//!
//! # Q26 is binding here: the default-unit test pins the rule, not a device read
//!
//! P0 has no native region source. The region is readable on macOS (`AppleLocale`) and **not** on
//! iOS, where the derivation therefore answers [`MeasurementSystem::Unknown`] and the canonical
//! unit (Q26 owns the native fix). So the "default unit" test asserts the three branches of the
//! rule with the region answer injected — including the `Unknown` branch, which is the iOS-shaped
//! case — and never calls `measurement_system()`, whose answer would depend on the machine running
//! the gate. No locale variable is read and no environment is mutated by these tests.
//!
//! # What is deliberately not asserted here
//!
//! The "You"-style placeholder and "the rest of Settings & Profile stays usable" are rendering
//! decisions (tasks 3.7 and 4.x); the Rust half is the unresolved [`Identity`] value those screens
//! read. P0 stores no weight content yet, so "stored values stay canonical" is represented by the
//! product's own conversion pair around a gram amount plus a sibling household file a unit change
//! must not touch. Both platforms (TOOL-1): nothing here is platform-specific.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

use app_lib::domain::resolve_identity;
use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, HouseholdCollection, HouseholdId, Instant,
    ItemId, WeightUnit,
};
use app_lib::settings::region::{MeasurementSystem, measurement_system_from_locale};
use app_lib::settings::store::{DisplayUnitStore, PREFERENCES_FILE_NAME};
use app_lib::settings::unit::{effective_unit, to_canonical, to_display};
use app_lib::storage::{CollectionStore, DeviceSnapshot};

fn account(value: &str) -> AccountUserId {
    AccountUserId(value.to_owned())
}

fn at(value: &str) -> Instant {
    Instant(value.to_owned())
}

fn household_id(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

/// The commands the shipped bindings declare, sorted: the frontend's whole command surface.
fn shipped_commands() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/bindings.ts");
    let bindings = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let marker = "__TAURI_INVOKE(\"";
    let mut names = Vec::new();
    let mut rest = bindings.as_str();

    while let Some(start) = rest.find(marker) {
        let after = &rest[start + marker.len()..];
        let end = after.find('"').expect("an invoke name is quoted");
        names.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }

    names.sort();
    names
}

/// A fresh directory under the process's temporary area, unique per call.
///
/// A recycled process id and counter cannot collide with a crashed run's directory because any
/// leftover is removed first.
fn temp_data_dir(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);

    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let process = std::process::id();
    let name = format!("bunscale-identity-unit-{process}-{sequence}-{label}");
    let directory = std::env::temp_dir().join(name);

    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a test directory");

    directory
}

/// The names in `directory`, sorted, so a comparison does not depend on directory order.
fn file_names(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .expect("the test directory")
        .map(|entry| {
            let entry = entry.expect("a directory entry");

            entry.file_name().to_string_lossy().into_owned()
        })
        .collect();
    names.sort();

    names
}

fn remove_temp_data_dir(directory: &Path) {
    let _cleanup = fs::remove_dir_all(directory);
}

/// SET-2, "Resolved account name is shown read-only".
///
/// **WHEN** the person's account identity resolves **THEN** Settings & Profile shows that display
/// name, and the name is not editable.
#[test]
fn resolved_account_name_is_shown_read_only() {
    let person = account("account-1");

    // The account tier wins: the name the account service reports is the one shown, and a locally
    // known member name beside it is not borrowed.
    let identity = resolve_identity(Some(&person), Some("Ada Lovelace"), Some("Local Ada"));

    assert_eq!(identity.account_user_id, Some(person.clone()));
    assert_eq!(identity.display_name.as_deref(), Some("Ada Lovelace"));
    assert!(identity.resolved);

    // A blank account name is not a name, so the account tier does not resolve — the same rule the
    // screen's fallback uses.
    let blank = resolve_identity(Some(&person), Some("   "), None);
    assert_eq!(blank.display_name, None);
    assert!(!blank.resolved);

    // Read-only by construction: the chain is a pure function of values the caller supplies and has
    // no state to write to. The shipped command set is pinned to the eleven frozen commands in
    // `failure_surface.rs`, so a setter would have to be added there deliberately; among them,
    // `get_identity` is the only identity command.
    let commands = shipped_commands();
    let identity_commands: Vec<&str> = commands
        .iter()
        .map(String::as_str)
        .filter(|name| name.contains("identity"))
        .collect();
    assert_eq!(identity_commands, ["get_identity"]);
}

/// SET-2, "Unresolved identity falls back without breaking the screen".
///
/// **WHEN** the person's account identity cannot be resolved **THEN** the locally known member
/// name is shown, or the placeholder when no local name is known, and the remaining content of
/// Settings & Profile stays usable.
#[test]
fn unresolved_identity_falls_back_without_breaking_the_screen() {
    let person = account("account-1");

    // No account name, but a locally known member name for the same account: that name is shown,
    // and `resolved` says it is the fallback tier rather than the account tier.
    let fallback = resolve_identity(Some(&person), None, Some("Ada"));
    assert_eq!(fallback.account_user_id, Some(person.clone()));
    assert_eq!(fallback.display_name.as_deref(), Some("Ada"));
    assert!(!fallback.resolved, "a local name is the fallback tier");

    // With no account, a member name cannot be attributed to the person, so it is not borrowed: the
    // unresolved marker is the honest answer.
    let unattributed = resolve_identity(None, None, Some("Ada"));
    assert_eq!(unattributed.account_user_id, None);
    assert_eq!(unattributed.display_name, None);
    assert!(!unattributed.resolved);

    // Neither name is known: the unresolved marker. It is a value, not a failure — the chain is
    // total, so the screen receives an `Identity` and renders its own placeholder.
    let unresolved = resolve_identity(None, None, None);
    assert_eq!(unresolved.account_user_id, None);
    assert_eq!(unresolved.display_name, None);
    assert!(!unresolved.resolved);

    // A whitespace-only local name is not a name: the screen falls through to its placeholder
    // rather than showing an empty label.
    let blank = resolve_identity(Some(&person), None, Some("   "));
    assert_eq!(blank.display_name, None);
    assert!(!blank.resolved);
}

/// SET-4, "Unit choice is stored on the device".
///
/// **WHEN** a person selects grams or kilograms in Settings **THEN** the choice is stored for that
/// device, and it is still in effect the next time the product starts on that device.
#[test]
fn unit_choice_is_stored_on_the_device() {
    let directory = temp_data_dir("unit-choice");
    let store = DisplayUnitStore::in_dir(&directory);

    // No choice yet is `Ok(None)` — a device nobody has chosen on, not a failure.
    assert_eq!(store.choice().expect("the store is readable"), None);

    store
        .store_choice(WeightUnit::Kilograms)
        .expect("the choice is stored");

    // A second store over the same directory is the next launch: nothing is cached in the process,
    // so the answer can only come from the file.
    let next_launch = DisplayUnitStore::in_dir(&directory);
    assert_eq!(
        next_launch.choice().expect("the store is readable"),
        Some(WeightUnit::Kilograms)
    );

    // The file carries the one preference and nothing else: no weight value, no household record.
    let bytes = fs::read(store.path()).expect("the preference file");
    let stored: Value = serde_json::from_slice(&bytes).expect("the preference file is JSON");
    let object = stored
        .as_object()
        .expect("the preference file is an object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["display_weight_unit", "format"]);

    // A new choice replaces the previous one, read back through a fresh store over the directory.
    store
        .store_choice(WeightUnit::Grams)
        .expect("the choice is stored");
    let after_change = DisplayUnitStore::in_dir(&directory);
    assert_eq!(
        after_change.choice().expect("the store is readable"),
        Some(WeightUnit::Grams)
    );

    // And the device preference never enters a household collection: the collection store beside it
    // has no file at all, a load answers the default snapshot, and the data directory holds exactly
    // the preference file.
    let collections = CollectionStore::in_dir(&directory);
    assert!(
        !collections.path().exists(),
        "writing the unit created a collection file"
    );
    assert_eq!(
        collections
            .load()
            .expect("the collection file is readable or absent"),
        DeviceSnapshot::default(),
        "a unit preference is not household data"
    );
    assert_eq!(file_names(&directory), [PREFERENCES_FILE_NAME]);

    remove_temp_data_dir(&directory);
}

/// SET-4, "Changing the unit leaves stored values unchanged".
///
/// **WHEN** the display weight unit is changed **THEN** weight values are converted only at the
/// point where they are entered or displayed, and every stored value remains in the canonical
/// unit.
///
/// The held value is the **real** collection store's file beside the preference file
/// (`CollectionStore::in_dir`), not a fixture name, so the guard watches a file production code
/// actually writes.
#[test]
fn changing_the_unit_leaves_stored_values_unchanged() {
    // A stored weight is grams; the unit is a presentation choice only (D0.4).
    let stored_grams = 2_500_u64;
    let directory = temp_data_dir("unit-change");
    let store = DisplayUnitStore::in_dir(&directory);

    // The device's data area holds the household collection in the collection store beside the
    // preference file; nothing in a unit change may touch it. The copy carries a load marker,
    // because only loaded copies are persisted.
    let collections = CollectionStore::in_dir(&directory);
    collections
        .save(&DeviceSnapshot {
            collections: vec![HouseholdCollection {
                household_id: household_id("household-1"),
                items: vec![CollectionItem {
                    item_id: ItemId("item-1".to_owned()),
                    updated_at: at("2026-10-19T10:00:00Z"),
                    deleted: false,
                    payload: None,
                }],
                source: CollectionSource::Service,
                loaded_at: Some(at("2026-10-19T11:00:00Z")),
            }],
            ..DeviceSnapshot::default()
        })
        .expect("the household collection is stored");
    let before = fs::read(collections.path()).expect("the stored collection");

    for unit in [WeightUnit::Grams, WeightUnit::Kilograms, WeightUnit::Grams] {
        store.store_choice(unit).expect("the choice is stored");

        // The presentation changes with the unit ...
        let presented = to_display(stored_grams as f64, unit);
        let expected = if unit == WeightUnit::Kilograms {
            2.5
        } else {
            2_500.0
        };
        assert_eq!(presented, expected, "{unit:?}");

        // ... and the value does not: the round trip is the same gram amount, so nothing stored in
        // the presentation unit was rewritten.
        assert_eq!(
            to_canonical(presented, unit),
            stored_grams as f64,
            "{unit:?}"
        );
        assert_eq!(
            fs::read(collections.path()).expect("the stored collection"),
            before,
            "{unit:?}: a unit change touched held data"
        );
    }

    // The preference file itself stores the unit and no value, so no stored weight was rewritten in
    // the presentation unit.
    let bytes = fs::read(store.path()).expect("the preference file");
    let stored: Value = serde_json::from_slice(&bytes).expect("the preference file is JSON");
    assert_eq!(stored["display_weight_unit"], "grams");
    assert_eq!(
        stored.as_object().expect("an object").len(),
        2,
        "the preference file carries only its format and the unit"
    );

    remove_temp_data_dir(&directory);
}

/// SET-4, "Default unit follows the device region".
///
/// **WHEN** a person has not chosen a display weight unit **THEN** the unit is derived from the
/// device's regional settings.
///
/// This pins the **rule** (Q26): a stored choice wins; otherwise the region's unit when the
/// process can read it; otherwise the canonical unit. It does not read a device: P0 has no native
/// region source, so iOS answers `Unknown` and therefore grams, while macOS reads `AppleLocale`.
/// The `Unknown` branch below is that iOS-shaped case, asserted with the region injected rather
/// than read.
#[test]
fn default_unit_follows_the_device_region() {
    // The region derivation, from locales: a readable region states a system ...
    assert_eq!(
        measurement_system_from_locale("de_DE.UTF-8"),
        MeasurementSystem::Metric
    );
    assert_eq!(
        measurement_system_from_locale("en_US"),
        MeasurementSystem::NonMetric
    );

    // ... while a locale that names no territory states none — the case iOS is always in, and the
    // one where the canonical unit is the honest default rather than a guess.
    assert_eq!(
        measurement_system_from_locale("en"),
        MeasurementSystem::Unknown
    );

    // The rule as the pure composition, all three branches.
    assert_eq!(
        effective_unit(Some(WeightUnit::Grams), MeasurementSystem::Metric),
        WeightUnit::Grams,
        "a stored choice wins"
    );
    assert_eq!(
        effective_unit(None, MeasurementSystem::Metric),
        WeightUnit::Kilograms,
        "a readable metric region selects the everyday unit"
    );
    assert_eq!(
        effective_unit(None, MeasurementSystem::NonMetric),
        WeightUnit::Grams,
        "the frozen unit list has no non-metric unit, so the canonical one answers"
    );
    assert_eq!(
        effective_unit(None, MeasurementSystem::Unknown),
        WeightUnit::Grams,
        "an unreadable region answers the canonical unit (iOS)"
    );

    // The store applies the same rule with its file behind it: no choice plus an injected
    // region ...
    let directory = temp_data_dir("regional-default");
    let store = DisplayUnitStore::in_dir(&directory);
    assert_eq!(
        store
            .effective(MeasurementSystem::Metric)
            .expect("the store is readable"),
        WeightUnit::Kilograms
    );
    assert_eq!(
        store
            .effective(MeasurementSystem::NonMetric)
            .expect("the store is readable"),
        WeightUnit::Grams
    );
    assert_eq!(
        store
            .effective(MeasurementSystem::Unknown)
            .expect("the store is readable"),
        WeightUnit::Grams
    );

    // ... and a stored choice wins over every region, including through the device-reading entry
    // point (which therefore does not consult the machine's region at all).
    store
        .store_choice(WeightUnit::Grams)
        .expect("the choice is stored");
    for system in [
        MeasurementSystem::Metric,
        MeasurementSystem::NonMetric,
        MeasurementSystem::Unknown,
    ] {
        assert_eq!(
            store.effective(system).expect("the store is readable"),
            WeightUnit::Grams,
            "{system:?}"
        );
    }
    assert_eq!(
        store
            .effective_for_this_device()
            .expect("the store is readable"),
        WeightUnit::Grams,
        "a chosen unit is not overridden by the device"
    );

    remove_temp_data_dir(&directory);
}
