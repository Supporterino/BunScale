//! Integration tests for the storage boundary (task 5.7): NFR-10's three scenarios and the claim a
//! removal keeps.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-shared-household-foundation/specs/data-integrity/spec.md` states for
//! NFR-10, and each test is named after the scenario it covers; the removal case has its own test
//! beside them.
//!
//! # The boundary these tests drive
//!
//! The subject is [`GuardedStore`]: the one type that holds this device's copies and the claims the
//! boundary makes about them. Every checked read and write in the product goes through it —
//! `AppState` keeps it behind its mutex and exposes only guarded operations — so the tests drive
//! the guard directly rather than a state (the household-level isolation the commands see is
//! 5.2's). [`BoundaryRefusal`] is the outcome these tests read: an invariant violation is a typed
//! refusal, never an absence and never a message.
//!
//! # What P0 can and cannot carry
//!
//! A collection holds identity, a version marker and a tombstone flag — no content (P1's). The
//! canonical-representation scenario therefore exercises [`Canonical`]/[`CanonicalForm`], the
//! helper the first canonical domain type uses, with a test-only amount; the presentation setting
//! is the product's real [`WeightUnit`] boundary (`settings::unit::to_display` and
//! `to_canonical`), so the stored form is checked against the two units the app can present.
//! `CollectionStore` is used where it is observable: the ownership claim of a removed item
//! survives the persisted copy and is rebuilt when the file is loaded back through the guard.
//!
//! Rendering is not involved, and nothing here is platform-specific (TOOL-1: iOS and macOS).
//!
//! # Why each assertion bites
//!
//! Every test asserts the *outcome* of a guard: the typed refusal, the absent answer, the untouched
//! row. Removing the ownership claim makes a cross-household read resolve instead of refusing, and
//! removing the marker or the canonical check lets the refused write or value through — so none of
//! these tests can pass with the guard bypassed.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use app_lib::ipc::types::{
    CollectionItem, CollectionSource, HouseholdCollection, HouseholdId, Instant, ItemId, WeightUnit,
};
use app_lib::settings::unit::{CANONICAL_UNIT, to_canonical, to_display};
use app_lib::storage::{
    BoundaryRefusal, Canonical, CanonicalForm, CollectionStore, DeviceSnapshot, GuardedStore,
    StoredItem,
};

/// The version both households' rows start at.
const VERSION: &str = "2026-10-19T10:00:00Z";

/// A later version, for the writes that must be refused or must replace a row.
const LATER: &str = "2026-10-19T11:00:00Z";

/// The instant a collection load records.
const LOADED_AT: &str = "2026-10-19T12:00:00Z";

fn household_id(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

fn item_id(value: &str) -> ItemId {
    ItemId(value.to_owned())
}

fn at(value: &str) -> Instant {
    Instant(value.to_owned())
}

/// One live wire row, as a load or merge takes it.
fn wire(id: &str, updated_at: &str) -> CollectionItem {
    CollectionItem {
        item_id: item_id(id),
        updated_at: at(updated_at),
        deleted: false,
    }
}

/// One removed wire row: the removal keeps the identity and the version.
fn wire_tombstone(id: &str, updated_at: &str) -> CollectionItem {
    CollectionItem {
        item_id: item_id(id),
        updated_at: at(updated_at),
        deleted: true,
    }
}

/// One household's wire collection, as a guarded load or merge takes it.
fn collection(household: &HouseholdId, items: Vec<CollectionItem>) -> HouseholdCollection {
    HouseholdCollection {
        household_id: household.clone(),
        items,
        source: CollectionSource::Service,
        loaded_at: None,
    }
}

/// One household's collection as the persisted file holds it: a loaded copy, marker included.
///
/// [`CollectionStore::save`] skips a copy whose `loaded_at` is `None`, because a never-loaded copy
/// is not something the file's shape represents.
fn loaded_collection(household: &HouseholdId, items: Vec<CollectionItem>) -> HouseholdCollection {
    HouseholdCollection {
        loaded_at: Some(at(LOADED_AT)),
        ..collection(household, items)
    }
}

/// A test-only amount, standing in for P1's weight.
///
/// The product's canonical stored form is grams (D0.4), so an amount that carries an explicit
/// presentation unit is admissible only when that unit is the canonical one. P0 ships no weight
/// domain, so this is the same kind of test-only stand-in the guards' unit tests use (their
/// `Lowercase`): it exists to exercise [`CanonicalForm`]'s contract, and it is not a model of
/// anything the product stores yet.
#[derive(Debug, Clone, PartialEq)]
struct Amount {
    grams: u64,
    unit: WeightUnit,
}

impl CanonicalForm for Amount {
    const FORM: &'static str = "grams";

    fn is_canonical(&self) -> bool {
        self.unit == CANONICAL_UNIT
    }
}

/// A fresh directory under the process's temporary area, unique per call.
fn temp_data_dir(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);

    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let process = std::process::id();
    let name = format!("bunscale-storage-boundary-{process}-{sequence}-{label}");
    let directory = std::env::temp_dir().join(name);

    fs::create_dir_all(&directory).expect("a test directory");

    directory
}

fn remove_temp_data_dir(directory: &Path) {
    let _cleanup = fs::remove_dir_all(directory);
}

/// NFR-10, "No item is readable from another household".
///
/// **WHEN** two households each hold data **THEN** no item of one household is readable from the
/// other, and no household-scoped identifier of one household is referenced by the other.
#[test]
fn no_item_is_readable_from_another_household() {
    let home = household_id("household-1");
    let other = household_id("household-2");
    let held = item_id("item-1");
    let unheld = item_id("item-2");
    let mut boundary = GuardedStore::new();
    let row = StoredItem::live(held.clone(), at(VERSION));
    boundary
        .upsert_item(&home, row.clone())
        .expect("the owning household writes its item");

    // The owner reads its own row.
    assert_eq!(boundary.item(&home, &held), Ok(Some(&row)));

    // A read through a household that does not own the item is a typed refusal, never an absence:
    // "not yours" and "absent" are different facts, and the caller can act on them differently.
    let foreign = boundary.item(&other, &held);
    assert_eq!(
        foreign,
        Err(BoundaryRefusal::ForeignHousehold {
            requested: other.clone(),
            owner: home.clone(),
        })
    );
    assert_ne!(
        foreign,
        Ok(None),
        "a cross-household read is not an empty answer"
    );

    // An item no household holds reads as absent — the other fact, which must stay distinguishable.
    assert_eq!(boundary.item(&other, &unheld), Ok(None));
    assert_eq!(boundary.household_of(&unheld), None);

    // A batch that mixes one foreign item is refused WHOLE: the row the other household could have
    // written does not land either, and no copy is created for it.
    let refused = boundary.upsert_all(
        &other,
        [
            StoredItem::live(unheld.clone(), at(VERSION)),
            StoredItem::live(held.clone(), at(VERSION)),
        ],
    );
    assert_eq!(
        refused,
        Err(BoundaryRefusal::ForeignHousehold {
            requested: other.clone(),
            owner: home.clone(),
        })
    );
    assert_eq!(
        boundary.item(&other, &unheld),
        Ok(None),
        "the batch was refused whole"
    );
    assert_eq!(boundary.household_of(&unheld), None);
    assert!(boundary.collection(&other).is_none(), "no copy was created");

    // A collection that *describes* another household is refused rather than filed under the one
    // it was addressed through, and a collection addressed correctly but carrying the other
    // household's item is refused whole. Both paths are checked: the load that replaces a copy
    // and the merge.
    let foreign_collection = collection(&home, vec![wire("item-1", VERSION)]);
    let foreign_batch = collection(
        &other,
        vec![wire("item-2", VERSION), wire("item-1", VERSION)],
    );
    let foreign = BoundaryRefusal::ForeignHousehold {
        requested: other.clone(),
        owner: home.clone(),
    };
    let refusals = [
        boundary.load_wire_collection(&other, foreign_collection.clone(), at(LOADED_AT)),
        boundary.upsert_wire_collection(&other, foreign_collection),
        boundary.load_wire_collection(&other, foreign_batch.clone(), at(LOADED_AT)),
        boundary.upsert_wire_collection(&other, foreign_batch),
    ];
    for refused in refusals {
        assert_eq!(refused, Err(foreign.clone()));
    }

    // Neither refusal created a copy for the wrong household, and the owning household's row is
    // untouched.
    assert!(
        boundary.collection(&other).is_none(),
        "a refused collection created no copy"
    );
    assert_eq!(boundary.item(&home, &held), Ok(Some(&row)));
}

/// NFR-10, "Canonical representation survives presentation changes".
///
/// This test drives the [`Canonical`] helper and the product's real display-unit pair
/// (`settings::unit::to_display`/`to_canonical`), not product storage: P0's collection carries no
/// content, so there is no stored value to move, and the name says what is actually asserted.
///
/// **WHEN** a stored value is later displayed under a different presentation setting **THEN** the
/// stored value is unchanged, and no stored value is rewritten into a presentation unit.
#[test]
fn canonical_form_and_presentation_pair_are_value_preserving() {
    let amount = Amount {
        grams: 2_500,
        unit: CANONICAL_UNIT,
    };

    // A value already in the canonical form is admitted, and comes back exactly as it went in.
    let stored = Canonical::admit(amount.clone()).expect("grams are the canonical form");
    assert_eq!(stored.get(), &amount);
    assert_eq!(stored.clone().into_inner(), amount);

    // A value carrying a presentation unit is refused, and the refusal names the form it would have
    // to be in. The literal is asserted rather than the type's own constant, so a changed form is
    // caught here rather than kept in step on both sides.
    let refused = Canonical::admit(Amount {
        grams: 2,
        unit: WeightUnit::Kilograms,
    });
    assert_eq!(
        refused,
        Err(BoundaryRefusal::NonCanonical {
            canonical_form: "grams",
        })
    );

    // Presenting the stored value in either unit moves only the presentation: the stored value is
    // still the same number of grams, in the canonical unit, afterwards. The two directions are one
    // value-preserving pair, so a presentation is never what gets stored.
    for unit in [WeightUnit::Grams, WeightUnit::Kilograms] {
        let presented = to_display(stored.get().grams as f64, unit);

        assert_eq!(
            to_canonical(presented, unit),
            stored.get().grams as f64,
            "{unit:?}: the presentation toggled the value"
        );
        assert_eq!(
            stored.get().unit,
            CANONICAL_UNIT,
            "{unit:?}: the stored form moved to the presentation unit"
        );
        assert_eq!(
            stored.get().grams,
            2_500,
            "{unit:?}: the stored value changed"
        );
    }

    // The kilogram presentation of 2 500 g is 2.5 — a number the product displays and never stores
    // as it stands; storing it goes through `to_canonical` first, which the loop above exercised.
    assert_eq!(to_display(2_500.0, WeightUnit::Kilograms), 2.5);
}

/// NFR-10, "Writes to immutable entities are refused".
///
/// **WHEN** a write targets an entity the storage boundary marks as immutable **THEN** the write is
/// refused, and no stored data changes.
#[test]
fn writes_to_immutable_entities_are_refused() {
    let home = household_id("household-1");
    let marked = item_id("item-1");
    let mutable = item_id("item-2");
    let extra = item_id("item-3");
    let mut boundary = GuardedStore::new();
    let original = StoredItem::live(marked.clone(), at(VERSION));
    let other_row = StoredItem::live(mutable.clone(), at(VERSION));
    boundary
        .upsert_all(&home, [original.clone(), other_row.clone()])
        .expect("the first write");
    boundary.mark_immutable(&marked);
    assert!(boundary.is_immutable(&marked));
    assert!(!boundary.is_immutable(&mutable));

    // A single-item write to the marked entity is refused and leaves the row unchanged.
    let changed = StoredItem::live(marked.clone(), at(LATER));
    assert_eq!(
        boundary.upsert_item(&home, changed.clone()),
        Err(BoundaryRefusal::Immutable {
            item_id: marked.clone(),
        })
    );
    assert_eq!(boundary.item(&home, &marked), Ok(Some(&original)));

    // The batch path consults the marker before writing anything, so the unmarked row that comes
    // first in the batch does not land either.
    assert_eq!(
        boundary.upsert_all(
            &home,
            [
                StoredItem::live(extra.clone(), at(VERSION)),
                changed.clone()
            ],
        ),
        Err(BoundaryRefusal::Immutable {
            item_id: marked.clone(),
        })
    );
    assert_eq!(
        boundary.item(&home, &extra),
        Ok(None),
        "the refused batch wrote nothing"
    );

    // The merge path is the same guard, so it does not bypass the marker either — and its whole
    // batch is refused, leaving the unmarked row as it was.
    assert_eq!(
        boundary.upsert_wire_collection(
            &home,
            collection(&home, vec![wire("item-2", LATER), wire("item-1", LATER)]),
        ),
        Err(BoundaryRefusal::Immutable {
            item_id: marked.clone(),
        })
    );
    assert_eq!(
        boundary.item(&home, &mutable),
        Ok(Some(&other_row)),
        "the refused merge wrote nothing"
    );

    // The load path is a write too when it would change the marked row, and when it would omit it:
    // a replacement that drops a marked entity changes stored data just as a rewrite does.
    for batch in [
        vec![wire("item-1", LATER), wire("item-2", VERSION)],
        vec![wire("item-2", VERSION)],
    ] {
        assert_eq!(
            boundary.load_wire_collection(&home, collection(&home, batch), at(LOADED_AT)),
            Err(BoundaryRefusal::Immutable {
                item_id: marked.clone(),
            })
        );
    }
    assert_eq!(boundary.item(&home, &marked), Ok(Some(&original)));
    assert_eq!(boundary.item(&home, &mutable), Ok(Some(&other_row)));

    // Re-delivering the marked row unchanged is a refresh, not a write, so it is allowed: the
    // marker refuses writes, not reads.
    boundary
        .load_wire_collection(
            &home,
            collection(
                &home,
                vec![wire("item-1", VERSION), wire("item-2", VERSION)],
            ),
            at(LOADED_AT),
        )
        .expect("an unchanged marked row does not make the load a write");

    // Marking one entity leaves the household's other entities writable.
    let replaced = boundary
        .upsert_item(&home, StoredItem::live(mutable.clone(), at(LATER)))
        .expect("item-2 is not marked");
    assert_eq!(replaced, Some(other_row));
    assert_eq!(
        boundary.item(&home, &mutable),
        Ok(Some(&StoredItem::live(mutable.clone(), at(LATER))))
    );
    assert_eq!(
        boundary.item(&home, &marked),
        Ok(Some(&original)),
        "the marker and its row are still the recorded ones"
    );
}

/// The removal case: a tombstone keeps its household's claim, so it is not readable from another
/// household and the id cannot be re-claimed.
///
/// The claim is rebuilt from the persisted copy as well, so a relaunch cannot hand another
/// household an id the removed item used to hold.
#[test]
fn a_removal_keeps_its_households_claim() {
    let home = household_id("household-1");
    let other = household_id("household-2");
    let removed = item_id("item-1");
    let mut boundary = GuardedStore::new();
    let tombstone = StoredItem::tombstone(removed.clone(), at(LATER));
    boundary
        .upsert_item(&home, tombstone.clone())
        .expect("the removal is the owning household's write");

    // The identity stays the household's: the owner still resolves its tombstone.
    assert_eq!(boundary.household_of(&removed), Some(&home));
    assert_eq!(boundary.item(&home, &removed), Ok(Some(&tombstone)));

    // And another household can neither read it nor re-claim the id with a live row.
    let foreign = BoundaryRefusal::ForeignHousehold {
        requested: other.clone(),
        owner: home.clone(),
    };
    assert_eq!(boundary.item(&other, &removed), Err(foreign.clone()));
    assert_eq!(
        boundary.upsert_item(&other, StoredItem::live(removed.clone(), at(VERSION))),
        Err(foreign.clone())
    );
    assert_eq!(boundary.household_of(&removed), Some(&home));

    // The claim survives the persisted copy: loading the file's collections back through the same
    // guard rebuilds it, so a relaunch does not reset an item to unowned.
    let directory = temp_data_dir("tombstone-claim");
    let store = CollectionStore::in_dir(&directory);
    store
        .save(&DeviceSnapshot {
            collections: vec![loaded_collection(
                &home,
                vec![wire_tombstone("item-1", LATER)],
            )],
            ..DeviceSnapshot::default()
        })
        .expect("the snapshot is written");

    let restored = store.load().expect("the snapshot is readable");
    assert_eq!(restored.collections.len(), 1);

    let mut relaunched = GuardedStore::new();
    for collection in restored.collections {
        let household_id = collection.household_id.clone();
        relaunched
            .load_wire_collection(&household_id, collection, at(LOADED_AT))
            .expect("the persisted collection is addressed correctly");
    }

    assert_eq!(relaunched.household_of(&removed), Some(&home));
    assert_eq!(relaunched.item(&other, &removed), Err(foreign));

    remove_temp_data_dir(&directory);
}
