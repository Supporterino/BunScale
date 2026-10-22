//! Integration tests for holding more than one household (task 5.2): HS-3's two
//! multiple-household scenarios and DM-3's two set-level scenarios.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-shared-household-foundation/specs/household-sharing/spec.md` states for
//! HS-3 and DM-3, and each test is named after the scenario it covers.
//!
//! # The boundary these tests drive
//!
//! The same shape `household_creation.rs` records: the command adapters in `ipc::commands::household`
//! are thin forwarders over [`AppState`], and a `tauri::State` cannot be constructed outside a
//! running app, so these tests call the `AppState` methods the commands forward to and install the
//! published [`ScriptedService`] as the transport. `app_lib::run()` is never called (it starts the
//! event loop) and `ipc::export_bindings()` is never called (D0.9: `cargo test` precedes the
//! bindings drift check), exactly as `library_surface.rs` records. Both platforms (TOOL-1): nothing
//! here is platform-specific.
//!
//! # What is deliberately not asserted here
//!
//! * "**and the creation onboarding is presented**" (DM-3's last clause) is a rendering decision.
//!   Its Rust-observable input is asserted instead — no household held and none active — which is
//!   the fact HS-2's guard reads; the component and E2E tasks own the presentation, as task 5.1
//!   records for HS-2.
//! * The per-item typed read refusal `GuardedStore::item` answers is task 5.7's subject. This file
//!   asserts the same isolation through the surface the commands use: each household's copy holds
//!   only its own rows, and a row claimed by one household is refused when it is offered to the
//!   other.
//!
//! # Which operation makes a household "no longer held"
//!
//! P0 ships no delete command (deletion is P3's), so DM-3's transition cannot be reached through a
//! command yet. `HouseholdBook::release` is the public set-level operation DM-3's re-election is
//! defined on, and [`AppState::households`] hands the book out, so the test drives the transition a
//! later refresh will drive rather than inventing a command that does not exist.

use std::sync::Arc;

use serde_json::json;

use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, Household, HouseholdCollection, HouseholdId,
    HouseholdSummary, Instant, ItemId, MemberId, RequestId,
};
use app_lib::state::AppState;
use app_lib::storage::BoundaryRefusal;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::{Answer, ScriptedService, ServiceReply};

/// The instant the scripted service dates the households with, in the schema the create path reads.
const CREATED_AT: &str = "2026-10-19T10:00:00Z";

/// The instant a household's collection is loaded at.
const LOADED_AT: &str = "2026-10-19T12:00:00Z";

/// The version marker every loaded row carries. Its value is irrelevant here; only which household
/// the row belongs to is under test.
const VERSION: &str = "2026-10-19T11:00:00Z";

/// The account the person is identified by in these tests: one person, several households.
fn account() -> AccountUserId {
    AccountUserId("account-1".to_owned())
}

fn request_id(value: &str) -> RequestId {
    RequestId(value.to_owned())
}

fn household_id(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

fn member_id(value: &str) -> MemberId {
    MemberId(value.to_owned())
}

fn item_id(value: &str) -> ItemId {
    ItemId(value.to_owned())
}

fn at(value: &str) -> Instant {
    Instant(value.to_owned())
}

/// One create answer in the schema the create path reads: the outcome the gate classifies, and the
/// household the domain takes its identity, owner and instant from.
fn accepted_answer(household: &str, owner: &str) -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(
        200,
        json!({
            "outcome": "applied",
            "household": {
                "id": household,
                "owner_member_id": owner,
                "created_at": CREATED_AT,
            },
        }),
    ))
}

/// Run one create through the state method the `create_household` command forwards to.
fn create(state: &AppState, name: &str, operation: &str) -> Household {
    block_on_ready(state.create_household(name.to_owned(), request_id(operation)))
        .expect("the create completed")
}

/// A state whose account holds the given households, created in the order supplied.
///
/// Each entry is `(name, owner member id)`; the created ids are `household-1`, `household-2`, … and
/// the operations `operation-1`, `operation-2`, …, so a test can name a household by its position.
/// The service answers with one distinct write per create — a repeated answer would name one
/// household twice and the second create would be a replay of the first — and every household is
/// created by the same person, which is what HS-3 requires of a person holding several.
fn state_holding(households: &[(&str, &str)]) -> AppState {
    let answers: Vec<ServiceReply> = households
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let number = index + 1;
            let owner = entry.1;

            accepted_answer(&format!("household-{number}"), owner)
        })
        .collect();
    let service = Arc::new(ScriptedService::new().on_write(answers));
    let state = AppState::new(service, Some(account()));

    for (index, entry) in households.iter().enumerate() {
        let number = index + 1;
        let (name, owner) = *entry;
        let created = create(&state, name, &format!("operation-{number}"));

        assert_eq!(created.id, household_id(&format!("household-{number}")));
        assert_eq!(created.owner_member_id, member_id(owner));
    }

    state
}

/// One household's wire collection holding `items`, as a read would hand it over.
fn collection(household: &HouseholdId, items: &[&str]) -> HouseholdCollection {
    HouseholdCollection {
        household_id: household.clone(),
        items: items
            .iter()
            .copied()
            .map(|item| CollectionItem {
                item_id: item_id(item),
                updated_at: at(VERSION),
                deleted: false,
                payload: None,
            })
            .collect(),
        source: CollectionSource::Service,
        loaded_at: None,
    }
}

/// Load `items` into `household`'s copy through the state's guarded load.
fn load(state: &AppState, household: &HouseholdId, items: &[&str]) {
    state
        .load_collection(household, collection(household, items), at(LOADED_AT))
        .expect("the collection is addressed through its own household");
}

/// The ids of one presented collection's rows, in the order they were stored.
fn item_ids(presented: &HouseholdCollection) -> Vec<&str> {
    presented
        .items
        .iter()
        .map(|item| item.item_id.0.as_str())
        .collect()
}

/// The list row for `household`, which the list must carry while the person holds it.
///
/// Looking the row up by id rather than by position keeps the assertions about *which* households
/// are held and marked active; the list's acquisition order is the book's own concern, not this
/// task's.
fn row<'a>(rows: &'a [HouseholdSummary], household: &HouseholdId) -> &'a HouseholdSummary {
    rows.iter()
        .find(|row| &row.id == household)
        .expect("the list carries every held household")
}

/// The ids the list marks active: DM-3 allows at most one, and the selection scenarios require
/// exactly one.
fn active_rows(rows: &[HouseholdSummary]) -> Vec<&HouseholdId> {
    rows.iter()
        .filter(|row| row.is_active)
        .map(|row| &row.id)
        .collect()
}

/// HS-3, "A second household coexists without mixing data".
///
/// **WHEN** a person who already holds a household creates another one **THEN** both households
/// exist for that person, each with its own members, and no member or item of one appears as a
/// member or item of the other.
#[test]
fn a_second_household_coexists_without_mixing_data() {
    let state = state_holding(&[("Home", "member-1"), ("Allotment", "member-2")]);
    let home = household_id("household-1");
    let allotment = household_id("household-2");

    // THEN both households exist for that person. The second create does not replace the first.
    let rows = state.list_households();
    assert_eq!(rows.len(), 2, "both are held");
    assert_eq!(row(&rows, &home).name, "Home");
    assert_eq!(row(&rows, &allotment).name, "Allotment");

    // ... each with its own members. Every household holds exactly its own creator, and the two
    // member lists share no entry.
    let (home_members, allotment_members) = {
        let book = state.households();
        let home_record = book.get(&home).expect("Home is held");
        let allotment_record = book.get(&allotment).expect("the allotment is held");

        (
            home_record.members().to_vec(),
            allotment_record.members().to_vec(),
        )
    };
    assert_eq!(home_members.len(), 1);
    assert_eq!(allotment_members.len(), 1);
    assert_eq!(home_members[0].id, member_id("member-1"));
    assert_eq!(allotment_members[0].id, member_id("member-2"));
    assert_ne!(
        home_members[0], allotment_members[0],
        "the two households' members are distinct records"
    );
    for member in &home_members {
        assert!(
            !allotment_members.contains(member),
            "no member of one household appears as a member of the other"
        );
    }
    assert_eq!(row(&rows, &home).member_count, 1);
    assert_eq!(row(&rows, &allotment).member_count, 1);

    // AND no item of one household appears as an item of the other: each copy holds only the rows
    // loaded through it.
    load(&state, &home, &["home-item-1"]);
    load(&state, &allotment, &["allotment-item-1"]);

    let home_data = state
        .read_collection(&home, CollectionSource::Local)
        .expect("this device holds Home's copy");
    let allotment_data = state
        .read_collection(&allotment, CollectionSource::Local)
        .expect("this device holds the allotment's copy");

    assert_eq!(home_data.household_id, home);
    assert_eq!(item_ids(&home_data), ["home-item-1"]);
    assert_eq!(allotment_data.household_id, allotment);
    assert_eq!(item_ids(&allotment_data), ["allotment-item-1"]);

    // A row loaded into one is unreadable from the other: Home's row cannot be brought into the
    // allotment's copy, and the refusal names the two households rather than reporting an absence.
    let refused = state.load_collection(
        &allotment,
        collection(&allotment, &["home-item-1"]),
        at(LOADED_AT),
    );
    assert_eq!(
        refused,
        Err(BoundaryRefusal::ForeignHousehold {
            requested: allotment.clone(),
            owner: home.clone(),
        }),
        "Home's row belongs to Home, whatever household it is addressed through"
    );

    // The refused load changed nothing: the allotment still presents only its own rows.
    let allotment_data = state
        .read_collection(&allotment, CollectionSource::Local)
        .expect("this device holds the allotment's copy");
    assert_eq!(item_ids(&allotment_data), ["allotment-item-1"]);

    // AND the active read presents only the active household's data. The active household is the one
    // the last create activated; its data is its own copy and carries none of Home's rows.
    let active = state
        .active_household()
        .expect("the second create is active");
    assert_eq!(active.id, allotment);

    let presented = state
        .read_collection(&active.id, CollectionSource::Local)
        .expect("the active household's copy is held");
    assert_eq!(presented.household_id, allotment);
    assert_eq!(item_ids(&presented), ["allotment-item-1"]);
}

/// HS-3, "Switching the active household changes the presented data".
///
/// **WHEN** a person holding two households selects the one that is not active **THEN** the selected
/// household becomes the active household, and the data presented as active is that household's data
/// and no longer the previously active household's data.
#[test]
fn switching_the_active_household_changes_the_presented_data() {
    let state = state_holding(&[("Home", "member-1"), ("Allotment", "member-2")]);
    let home = household_id("household-1");
    let allotment = household_id("household-2");

    load(&state, &home, &["home-item-1"]);
    load(&state, &allotment, &["allotment-item-1"]);

    // Before the switch the presented household is the one the last create activated, and the data
    // presented as active is that household's.
    let before = state.active_household().expect("one is active");
    assert_eq!(before.id, allotment);
    let presented_before = state
        .read_collection(&before.id, CollectionSource::Local)
        .expect("the active household's copy is held");
    assert_eq!(item_ids(&presented_before), ["allotment-item-1"]);

    let selected = state
        .set_active_household(home.clone(), request_id("operation-3"))
        .expect("the person holds Home");

    // THEN the selected household becomes the active household.
    assert_eq!(selected.id, home);
    assert!(selected.is_active);
    let active = state.active_household().expect("one is active");
    assert_eq!(active.id, home);
    assert!(active.is_active);

    // AND the data presented as active is that household's data ...
    let presented = state
        .read_collection(&active.id, CollectionSource::Local)
        .expect("the active household's copy is held");
    assert_eq!(presented.household_id, home);
    assert_eq!(item_ids(&presented), ["home-item-1"]);

    // ... and no longer the previously active household's data. The allotment's rows are still
    // stored, but nothing presents them as the active household's.
    let previous = state
        .read_collection(&allotment, CollectionSource::Local)
        .expect("the previously active household's copy is still held");
    assert_eq!(item_ids(&previous), ["allotment-item-1"]);
    assert_ne!(presented, previous);

    // Exactly one list row is marked active, and it is the selected one.
    let rows = state.list_households();
    assert_eq!(active_rows(&rows), [&home]);
    assert!(!row(&rows, &allotment).is_active);
}

/// DM-3, "Selecting a household leaves exactly one active".
///
/// **WHEN** a person holding more than one household selects a different household as active
/// **THEN** exactly one household is the active household afterwards, and the household that was
/// active before is no longer indicated as active.
///
/// The selection is repeated so the invariant is asserted after more than one switch: a caller that
/// only cleared the previous active id, or that accumulated active ids, would satisfy the first
/// switch and fail a later one.
#[test]
fn selecting_a_household_leaves_exactly_one_active() {
    let state = state_holding(&[
        ("Home", "member-1"),
        ("Allotment", "member-2"),
        ("Workshop", "member-3"),
    ]);
    let home = household_id("household-1");
    let allotment = household_id("household-2");
    let workshop = household_id("household-3");

    // The last create activated the workshop, so the first selection is a different household.
    assert_eq!(
        state.active_household().expect("one is active").id,
        workshop
    );

    let selected = state
        .set_active_household(home.clone(), request_id("operation-4"))
        .expect("the person holds Home");
    assert_eq!(selected.id, home);

    // THEN exactly one household is the active household afterwards ...
    let rows = state.list_households();
    assert_eq!(rows.len(), 3, "the switch changes no membership");
    assert_eq!(active_rows(&rows), [&home]);
    // ... and the household that was active before is no longer indicated as active.
    assert!(!row(&rows, &workshop).is_active);
    assert!(!row(&rows, &allotment).is_active);
    assert!(row(&rows, &home).is_active);

    // The book behind the list holds one active id, and it is the selected one: the list cannot
    // disagree with it.
    {
        let book = state.households();
        assert_eq!(book.active_id(), Some(&home));
        assert!(book.is_active(&home));
        assert!(!book.is_active(&workshop));
    }
    assert_eq!(state.active_household().expect("one is active").id, home);

    // A second selection moves the one active id again and leaves the previous selection inactive.
    let selected = state
        .set_active_household(allotment.clone(), request_id("operation-5"))
        .expect("the person holds the allotment");
    assert_eq!(selected.id, allotment);

    let rows = state.list_households();
    assert_eq!(active_rows(&rows), [&allotment]);
    assert!(
        !row(&rows, &home).is_active,
        "the previous selection is no longer active"
    );
    assert!(!row(&rows, &workshop).is_active);
    assert_eq!(
        state.active_household().expect("one is active").id,
        allotment
    );
}

/// DM-3, "Active household is replaced when it is no longer held".
///
/// **WHEN** the household that was active is no longer among the households the person holds
/// **THEN** exactly one of the remaining households becomes active, and when no household remains,
/// no household is active.
///
/// Removing a household from the held set is `HouseholdBook::release` — the operation the module
/// documents DM-3's re-election on. P0 has no delete command, so this is the transition a later
/// refresh drives (see the module documentation).
#[test]
fn active_household_is_replaced_when_it_is_no_longer_held() {
    let state = state_holding(&[("Home", "member-1"), ("Allotment", "member-2")]);
    let home = household_id("household-1");
    let allotment = household_id("household-2");

    // The household about to leave is the active one.
    assert_eq!(
        state.active_household().expect("one is active").id,
        allotment
    );

    let released = {
        let mut book = state.households();
        book.release(&allotment)
    }
    .expect("the person held the allotment");
    assert_eq!(released.id(), &allotment);

    // THEN exactly one of the remaining households becomes active.
    let active = state
        .active_household()
        .expect("a remaining household is elected");
    assert_eq!(active.id, home);
    assert!(active.is_active);

    let rows = state.list_households();
    assert_eq!(rows.len(), 1, "the released household is no longer held");
    assert_eq!(active_rows(&rows), [&home]);
    assert!(state.households().get(&allotment).is_none());

    // AND when no household remains, no household is active.
    let released = {
        let mut book = state.households();
        book.release(&home)
    }
    .expect("the person held Home");
    assert_eq!(released.id(), &home);

    assert!(state.list_households().is_empty());
    assert!(state.active_household().is_none());
    assert!(state.households().active_id().is_none());

    // "and the creation onboarding is presented" is the frontend's decision; the Rust-observable
    // input it reads is asserted above: no household held and none active (HS-2).
}
