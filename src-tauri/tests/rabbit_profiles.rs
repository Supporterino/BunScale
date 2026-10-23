//! Integration tests for rabbit profile creation (task 5.1): RAB-1's five creating scenarios,
//! RAB-2's two blank-name scenarios and RAB-3's "Additional rabbits are created without
//! restriction".
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-rabbit-weight-core-loop/specs/rabbit-profiles/spec.md` states, and each test
//! is named after the scenario it covers.
//!
//! # The boundary these tests drive
//!
//! The `ipc::commands::rabbit` adapters are private bodies over [`AppState`]: the `#[tauri::command]`
//! wrapper takes `tauri::State<'_, AppState>`, which has no public constructor outside a running app,
//! so these tests drive the same [`AppState`] methods the command forwards to — the whole behaviour
//! of the command apart from the wrapper — with the published [`ScriptedService`] double as the
//! transport, exactly as `household_creation.rs` does. `app_lib::run()` is never called (it starts the
//! event loop) and `ipc::export_bindings()` is never called (D0.9), as `library_surface.rs` records.
//!
//! Two observables are asserted together, because a profile is only "recorded" if both hold:
//!
//! * **What reaches the service.** The create is one `save_item` into the household's shared
//!   collection, so the body's payload is decoded back through [`ItemPayload`] — the contract's own
//!   shape — rather than read out of a locally composed value. That is the version every member
//!   reads.
//! * **What the person sees afterwards.** The decoded payload is loaded as this device's copy the way
//!   the read path delivers it (SYNC-3), and the profile is read back through
//!   [`AppState::rabbit`], the method `get_rabbit` forwards to.
//!
//! Both platforms (TOOL-1): nothing here is platform-specific.
//!
//! # What this file does not add
//!
//! The state-level tests that already cover RAB-2 and RAB-3 are not duplicated here: RAB-2's create
//! and edit refusals are `state.rs`'s `a_blank_rabbit_name_is_refused_before_the_account_is_checked`,
//! `a_blank_rabbit_name_is_refused_before_the_write_is_sent` and
//! `a_blank_edit_name_is_refused_before_the_account_is_checked`, and RAB-3's unlimited creates are
//! `additional_rabbits_are_created_without_restriction`. This file fills RAB-1's gaps, which existed
//! only as domain-unit coverage.
//!
//! # What is deliberately not asserted here
//!
//! "The person is prompted to supply a name" and "the photo is presented as the rabbit's thumbnail"
//! are rendering decisions: their Rust-observable halves — the `validation` refusal and the photo
//! reference being carried in the profile — are asserted, while the copy and the placeholder are the
//! component tasks' (5.11). Both platforms (TOOL-1).

use std::sync::Arc;

use serde_json::{Value, json};

use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, CommandError, HouseholdCollection,
    HouseholdId, Instant, ItemId, ItemPayload, NeuterStatus, Rabbit, RabbitProfile, RequestId, Sex,
};
use app_lib::state::AppState;
use app_lib::sync::double::ScriptedService;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::{Answer, ServiceReply};

/// The instant the scripted service dates a created household with.
const CREATED_AT: &str = "2026-10-19T10:00:00Z";

/// When the read path loaded this device's copy.
const LOADED_AT: &str = "2026-10-22T09:31:00Z";

/// The version marker the copy records for the one rabbit row.
const VERSION: &str = "2026-10-22T09:30:00Z";

fn account() -> AccountUserId {
    AccountUserId("account-1".to_owned())
}

fn request_id(value: &str) -> RequestId {
    RequestId(value.to_owned())
}

fn household(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

fn at(value: &str) -> Instant {
    Instant(value.to_owned())
}

/// One accepted household create answer, so the book holds a household a rabbit can be created in.
fn accepted_household() -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(
        200,
        json!({
            "outcome": "applied",
            "household": {
                "id": "household-1",
                "owner_member_id": "member-1",
                "created_at": CREATED_AT,
            },
        }),
    ))
}

/// A rabbit profile supplying every RAB-1 field.
fn full_profile() -> RabbitProfile {
    RabbitProfile {
        name: "Pebble".to_owned(),
        photo_ref: Some("photo-1".to_owned()),
        breed: Some("Mini Lop".to_owned()),
        dob: Some(at("2026-03-01T00:00:00Z")),
        dob_unknown: false,
        sex: Sex::Female,
        neuter_status: NeuterStatus::Spayed,
        adoption_date: Some(at("2026-05-01T00:00:00Z")),
        weight_drop_threshold_percent: 5.0,
    }
}

/// A profile supplying only the fields the contract always carries: a name, a sex and a neuter
/// status. Every optional field is absent.
fn minimal_profile() -> RabbitProfile {
    RabbitProfile {
        name: "Clover".to_owned(),
        photo_ref: None,
        breed: None,
        dob: None,
        dob_unknown: false,
        sex: Sex::Male,
        neuter_status: NeuterStatus::Intact,
        adoption_date: None,
        weight_drop_threshold_percent: 5.0,
    }
}

/// A state whose book holds `household-1` with this account as its member, so a rabbit create can
/// name its writer. The double's last write answer is repeated, so the rabbit write is accepted by
/// the same script.
fn state_with_a_held_household() -> (Arc<ScriptedService>, AppState) {
    let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
    let state = AppState::new(service.clone(), Some(account()));

    block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
        .expect("the household is created and held");

    (service, state)
}

/// One rabbit create through the state method the `create_rabbit` command forwards to.
fn create(
    state: &AppState,
    household_id: &HouseholdId,
    profile: RabbitProfile,
    operation: &str,
) -> Result<Rabbit, CommandError> {
    block_on_ready(state.create_rabbit(household_id.clone(), profile, request_id(operation)))
}

/// Every write the transport was asked to submit, in call order.
fn write_bodies(service: &ScriptedService) -> Vec<Value> {
    service
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            app_lib::sync::double::RecordedCall::Save { request, .. } => {
                Some(request.body().clone())
            }
            _ => None,
        })
        .collect()
}

/// The item content of the `index`-th write, decoded through the contract's own payload shape.
///
/// The shared collection stores items, not profiles, so this is what every member resolves — not the
/// value `create_rabbit` returned locally.
fn stored_payload(service: &ScriptedService, index: usize) -> ItemPayload {
    let bodies = write_bodies(service);
    let body = bodies.get(index).expect("the write reached the service");

    serde_json::from_value(body["payload"].clone())
        .expect("the write's payload is an item payload the contract models")
}

/// The rabbit content of one stored payload.
fn rabbit_of(payload: ItemPayload) -> Rabbit {
    match payload {
        ItemPayload::Rabbit { rabbit } => rabbit,
        other => panic!("the write carries a rabbit, not {other:?}"),
    }
}

/// One row of the household's copy carrying the rabbit's stored content (D1.16).
fn row(rabbit: &Rabbit, payload: ItemPayload) -> CollectionItem {
    CollectionItem {
        item_id: ItemId(rabbit.id.0.clone()),
        updated_at: at(VERSION),
        deleted: false,
        payload: Some(payload),
    }
}

/// Load `items` as the household's copy, exactly as the read path delivers a service read (SYNC-3).
fn load(state: &AppState, household_id: &HouseholdId, items: Vec<CollectionItem>) {
    state
        .load_collection(
            household_id,
            HouseholdCollection {
                household_id: household_id.clone(),
                items,
                source: CollectionSource::Service,
                loaded_at: None,
            },
            at(LOADED_AT),
        )
        .expect("the collection is the household's own");
}

/// RAB-1, "A profile is created with the supplied identifying details".
///
/// **WHEN** a person creates a rabbit profile supplying a name, a breed, a date of birth, a sex, a
/// neuter status and an adoption date **THEN** the profile records each of those values **AND** the
/// rabbit is afterwards presented with that name.
#[test]
fn a_profile_is_created_with_the_supplied_identifying_details() {
    let (service, state) = state_with_a_held_household();
    let one = household("household-1");

    let created = create(&state, &one, full_profile(), "operation-2")
        .expect("the double accepted the create");

    // THEN the profile records each value: the version the shared collection received carries every
    // supplied field, unchanged.
    let payload = stored_payload(&service, 1);
    let stored = rabbit_of(payload.clone());
    assert_eq!(stored, created, "the recorded profile is the one returned");
    assert_eq!(stored.name, "Pebble");
    assert_eq!(stored.photo_ref, Some("photo-1".to_owned()));
    assert_eq!(stored.breed, Some("Mini Lop".to_owned()));
    assert_eq!(stored.dob, Some(at("2026-03-01T00:00:00Z")));
    assert!(!stored.dob_unknown);
    assert_eq!(stored.sex, Sex::Female);
    assert_eq!(stored.neuter_status, NeuterStatus::Spayed);
    assert_eq!(stored.adoption_date, Some(at("2026-05-01T00:00:00Z")));
    assert_eq!(stored.household_id, one);
    assert!(!stored.is_archived);
    assert_eq!(stored.weight_drop_threshold_percent, 5.0);

    // AND the rabbit is afterwards presented with that name: the copy the read path delivers reads
    // back field for field through the profile read the detail screen uses.
    load(&state, &one, vec![row(&created, payload)]);
    let read = state
        .rabbit(&created.id)
        .expect("the rabbit is in this device's copy");
    assert_eq!(read, stored, "every RAB-1 field is carried unchanged");
    assert_eq!(read.name, "Pebble");
    assert_eq!(read.id, created.id);

    assert_eq!(
        service.write_count(),
        2,
        "the household create and the rabbit create"
    );
}

/// RAB-1, "Optional fields may be left unset".
///
/// **WHEN** a person creates a rabbit profile supplying only a name and a sex **THEN** the profile is
/// saved **AND** the fields that were not supplied are recorded as absent.
#[test]
fn optional_fields_may_be_left_unset() {
    let (service, state) = state_with_a_held_household();
    let one = household("household-1");

    let created = create(&state, &one, minimal_profile(), "operation-2")
        .expect("a profile with only a name and a sex is savable");

    // Absent, not empty and not defaulted — in the version the service stores.
    let payload = stored_payload(&service, 1);
    let stored = rabbit_of(payload.clone());
    assert_eq!(stored.name, "Clover");
    assert_eq!(stored.sex, Sex::Male);
    assert_eq!(stored.photo_ref, None);
    assert_eq!(stored.breed, None);
    assert_eq!(stored.dob, None);
    assert_eq!(stored.adoption_date, None);
    assert!(
        !stored.dob_unknown,
        "not supplied is not the same as unknown"
    );

    // The absence reaches the shared collection as `null`, never `""`.
    let bodies = write_bodies(&service);
    let body = &bodies[1]["payload"]["rabbit"];
    for field in ["photo_ref", "breed", "dob", "adoption_date"] {
        assert_eq!(body[field], Value::Null, "{field}");
    }
    assert_eq!(body["dob_unknown"], json!(false));

    // And the person reads the same profile back: the omitted fields present nothing to show.
    load(&state, &one, vec![row(&created, payload)]);
    let read = state.rabbit(&created.id).expect("the profile is readable");
    assert_eq!(read, stored);
    assert_eq!(read.breed, None);
    assert_eq!(read.adoption_date, None);
}

/// RAB-1, "Date of birth may be marked unknown".
///
/// **WHEN** a person marks a rabbit's date of birth as unknown **THEN** the profile is saved without
/// a birth date **AND** the profile presents the date of birth as unknown rather than as absent or as
/// a placeholder date.
#[test]
fn date_of_birth_may_be_marked_unknown() {
    let (service, state) = state_with_a_held_household();
    let one = household("household-1");
    let profile = RabbitProfile {
        dob: None,
        dob_unknown: true,
        ..minimal_profile()
    };

    let created =
        create(&state, &one, profile, "operation-2").expect("an unknown date of birth is savable");

    // Saved without a date, with the "unknown" fact recorded as a flag rather than inferred from the
    // missing value.
    let payload = stored_payload(&service, 1);
    let stored = rabbit_of(payload.clone());
    assert_eq!(stored.dob, None);
    assert!(
        stored.dob_unknown,
        "unknown is a stated fact, distinct from absent"
    );

    let bodies = write_bodies(&service);
    assert_eq!(bodies[1]["payload"]["rabbit"]["dob"], Value::Null);
    assert_eq!(bodies[1]["payload"]["rabbit"]["dob_unknown"], json!(true));

    // Presented as unknown, not as absent: the flag survives the read.
    load(&state, &one, vec![row(&created, payload)]);
    let read = state.rabbit(&created.id).expect("the profile is readable");
    assert_eq!(read.dob, None);
    assert!(read.dob_unknown);
}

/// RAB-1, "A neutered and a spayed rabbit are recorded distinctly".
///
/// **WHEN** one rabbit profile is created with the neuter status `neutered` and another with the
/// neuter status `spayed` **THEN** each profile records its own value **AND** the two statuses are
/// not recorded as the same value.
#[test]
fn a_neutered_and_a_spayed_rabbit_are_recorded_distinctly() {
    let (service, state) = state_with_a_held_household();
    let one = household("household-1");

    let neutered = create(
        &state,
        &one,
        RabbitProfile {
            name: "Pebble".to_owned(),
            neuter_status: NeuterStatus::Neutered,
            ..minimal_profile()
        },
        "operation-2",
    )
    .expect("the first profile is created");
    let spayed = create(
        &state,
        &one,
        RabbitProfile {
            name: "Clover".to_owned(),
            neuter_status: NeuterStatus::Spayed,
            ..minimal_profile()
        },
        "operation-3",
    )
    .expect("the second profile is created");

    // Each profile records its own value, and the two are not collapsed into one "fixed" value.
    assert_eq!(neutered.neuter_status, NeuterStatus::Neutered);
    assert_eq!(spayed.neuter_status, NeuterStatus::Spayed);
    assert_ne!(neutered.neuter_status, spayed.neuter_status);

    // The distinction survives the trip to the shared collection, one status per stored item.
    let bodies = write_bodies(&service);
    assert_eq!(
        bodies[1]["payload"]["rabbit"]["neuter_status"],
        json!("neutered")
    );
    assert_eq!(
        bodies[2]["payload"]["rabbit"]["neuter_status"],
        json!("spayed")
    );

    // And each rabbit reads back with its own status.
    let neutered_payload = stored_payload(&service, 1);
    let spayed_payload = stored_payload(&service, 2);
    load(
        &state,
        &one,
        vec![
            row(&neutered, neutered_payload),
            row(&spayed, spayed_payload),
        ],
    );

    assert_eq!(
        state
            .rabbit(&neutered.id)
            .expect("the neutered rabbit is in the copy")
            .neuter_status,
        NeuterStatus::Neutered
    );
    assert_eq!(
        state
            .rabbit(&spayed.id)
            .expect("the spayed rabbit is in the copy")
            .neuter_status,
        NeuterStatus::Spayed
    );
}

/// RAB-1, "Sex and neuter status may each be recorded as unknown".
///
/// **WHEN** a person creates a rabbit profile with the sex `unknown` and the neuter status `unknown`
/// **THEN** the profile is saved **AND** it records those values rather than an invented sex or
/// neuter status.
#[test]
fn sex_and_neuter_status_may_each_be_recorded_as_unknown() {
    let (service, state) = state_with_a_held_household();
    let one = household("household-1");
    let profile = RabbitProfile {
        sex: Sex::Unknown,
        neuter_status: NeuterStatus::Unknown,
        ..minimal_profile()
    };

    let created = create(&state, &one, profile, "operation-2")
        .expect("an unstated sex and neuter status are savable");

    // Recorded values — not absences (`null`) and not invented ones.
    let payload = stored_payload(&service, 1);
    let stored = rabbit_of(payload.clone());
    assert_eq!(stored.sex, Sex::Unknown);
    assert_eq!(stored.neuter_status, NeuterStatus::Unknown);

    let bodies = write_bodies(&service);
    assert_eq!(bodies[1]["payload"]["rabbit"]["sex"], json!("unknown"));
    assert_eq!(
        bodies[1]["payload"]["rabbit"]["neuter_status"],
        json!("unknown")
    );

    // And the read presents the same two values rather than guessing.
    load(&state, &one, vec![row(&created, payload)]);
    let read = state.rabbit(&created.id).expect("the profile is readable");
    assert_eq!(read.sex, Sex::Unknown);
    assert_eq!(read.neuter_status, NeuterStatus::Unknown);
}
