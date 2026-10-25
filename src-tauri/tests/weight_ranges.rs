//! Integration tests for the weight trend and history ranges (task 5.7): WT-6's three scenarios —
//! a selected range limits the trend to its window, the full-history range presents every entry, and
//! the two viewed ranges are independent.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-rabbit-weight-core-loop/specs/weight-tracking/spec.md` state, and each test
//! is named after the scenario it covers.
//!
//! # The boundary these tests drive
//!
//! The `ipc::commands::weight` adapters are private bodies over [`AppState`]: the `#[tauri::command]`
//! wrapper takes `tauri::State<'_, AppState>`, which has no public constructor outside a running app,
//! so these tests drive the same [`AppState`] method the commands forward to — the whole behaviour of
//! the command apart from the wrapper — with the published [`ScriptedService`] double as the
//! transport, exactly as `weight_entries.rs` (task 5.4) and `drop_alert.rs` (task 5.5) do.
//! `app_lib::run()` is never called (it starts the event loop) and `ipc::export_bindings()` is never
//! called (D0.9), as `library_surface.rs` records.
//!
//! Entries reach both this device and every other member by one route: an entry write is one
//! `save_item` into the household's shared collection, and this device's copy is loaded from those
//! accepted writes ([`refresh`]). Every range below is then read from that copy through
//! [`AppState::weight_trend`], the method `get_weight_entries` forwards to.
//!
//! # Fixed instants, and the one clock that cannot be pinned
//!
//! Every reading timestamp is a fixed instant around a fixed `NOW`; no test sleeps and no assertion
//! depends on elapsed time. The window is measured from `now`, and the `get_weight_entries` wrapper
//! reads that clock at its edge (`AbsoluteInstant::now()`, `ipc/commands/weight.rs`), so the clock
//! cannot be pinned **through the command**. These tests pin it at the method the wrapper forwards
//! to, [`AppState::weight_trend`], and pass the same fixed [`NOW`] to every read; the wrapper's only
//! contribution to a window is that one clock read. Nothing else about the range is left unpinned.
//!
//! The two ranged views (the trend chart and the weight-only history) are **one** command,
//! `get_weight_entries`, called with each view's own [`WeightRange`]; the backend stores no selected
//! range (design D1.4). Independence is therefore a property of the read: two calls with different
//! ranges over the same rabbit each answer their own window and neither changes the other, which is
//! what the third test asserts.
//!
//! # What is deliberately not asserted here
//!
//! Rendering is not involved: "presented in the trend" and "lists only the entries" are asserted as
//! the points the read answers, never as chart or list markup. The window arithmetic is
//! unit-tested beside `domain::weight::trend`/`window`; these tests prove the selected range reaches
//! the read and limits it, with an entry just inside and just outside each edge.
//!
//! Both platforms (TOOL-1): nothing here is platform-specific.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use app_lib::domain::SAVE_ITEM_RPC;
use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, Grams, HouseholdCollection, HouseholdId,
    Instant, ItemId, ItemPayload, NeuterStatus, Rabbit, RabbitProfile, RequestId, Sex, WeightEntry,
    WeightEntryDraft, WeightEntryId, WeightRange, WeightTrend,
};
use app_lib::state::AppState;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::double::{RecordedCall, ScriptedService};
use app_lib::sync::{Answer, ServiceReply};

/// The one household these scenarios live in.
const HOUSEHOLD: &str = "household-1";

/// The instant the scripted service dates a created household with.
const CREATED_AT: &str = "2026-10-19T10:00:00Z";

/// When the read path loaded this device's copy.
const LOADED_AT: &str = "2026-10-25T09:31:00Z";

/// The operation that creates the rabbit every test starts from.
const CREATE_RABBIT: &str = "operation-2";

/// The fixed read clock every window is measured from. It is passed to each read, so the edges are
/// exact: 7 days before is [`SEVEN_DAYS_EDGE`], 30 days before is [`THIRTY_DAYS_EDGE`] and 90 days
/// before is [`NINETY_DAYS_EDGE`].
const NOW: &str = "2026-10-25T12:00:00Z";

/// Six months of history: the oldest entry, outside every bounded window.
const SIX_MONTHS: &str = "2026-04-25T12:00:00Z";

/// One second before the 90-day edge: outside `days_90`, inside `all`.
const NINETY_DAYS_BEFORE: &str = "2026-07-27T11:59:59Z";

/// Exactly 90 days before [`NOW`]: the inclusive lower edge of `days_90`.
const NINETY_DAYS_EDGE: &str = "2026-07-27T12:00:00Z";

/// One second before the 30-day edge: outside `days_30`, inside `days_90`.
const THIRTY_DAYS_BEFORE: &str = "2026-09-25T11:59:59Z";

/// Exactly 30 days before [`NOW`]: the inclusive lower edge of `days_30`.
const THIRTY_DAYS_EDGE: &str = "2026-09-25T12:00:00Z";

/// One second before the 7-day edge: outside `days_7`, inside `days_30`.
const SEVEN_DAYS_BEFORE: &str = "2026-10-18T11:59:59Z";

/// Exactly 7 days before [`NOW`]: the inclusive lower edge of `days_7`.
const SEVEN_DAYS_EDGE: &str = "2026-10-18T12:00:00Z";

/// The newest entry, a day before [`NOW`].
const RECENT: &str = "2026-10-24T12:00:00Z";

/// After [`NOW`]: outside every bounded window, inside the unbounded full history.
const FUTURE: &str = "2026-11-05T12:00:00Z";

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
                "id": HOUSEHOLD,
                "owner_member_id": "member-1",
                "created_at": CREATED_AT,
            },
        }),
    ))
}

/// A rabbit profile supplying the RAB-1 field set, with the default 5% drop threshold.
fn profile() -> RabbitProfile {
    RabbitProfile {
        name: "Pebble".to_owned(),
        photo_ref: None,
        breed: Some("Mini Lop".to_owned()),
        dob: Some(at("2026-03-01T00:00:00Z")),
        dob_unknown: false,
        sex: Sex::Female,
        neuter_status: NeuterStatus::Spayed,
        adoption_date: None,
        weight_drop_threshold_percent: 5.0,
    }
}

/// A draft supplying only the value: no timestamp, no note, nothing confirmed.
fn draft(value_grams: i32) -> WeightEntryDraft {
    WeightEntryDraft {
        value_grams: Grams(value_grams),
        timestamp: None,
        note: None,
        outlier_confirmed: false,
    }
}

/// The same draft with the person's own reading instant, which may be in the past or the future.
fn draft_at(value_grams: i32, timestamp: &str) -> WeightEntryDraft {
    WeightEntryDraft {
        timestamp: Some(at(timestamp)),
        ..draft(value_grams)
    }
}

/// A state whose book holds the household with this account as its member, so a write can name its
/// writer. The double repeats its last write answer, so every later item write is accepted.
fn held_household() -> (Arc<ScriptedService>, AppState) {
    let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
    let state = AppState::new(service.clone(), Some(account()));

    block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
        .expect("the household is created and held");

    (service, state)
}

/// Every write the transport was asked to submit that addresses one item (`save_item`), in call
/// order. The household create is a different service function and is not an item write.
fn item_bodies(service: &ScriptedService) -> Vec<Value> {
    service
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            RecordedCall::Save { request, .. } if request.rpc() == SAVE_ITEM_RPC => {
                Some(request.body().clone())
            }
            _ => None,
        })
        .collect()
}

/// The household's one shared collection, rebuilt from the item writes the service accepted.
///
/// This is what every member resolves: the service stores one row per item in the household's
/// collection, so replaying the writes in call order yields the stored version of each item. The
/// replay fails if an item write addresses another collection — that is the "shared by construction"
/// assertion.
fn shared_rows(service: &ScriptedService, household_id: &HouseholdId) -> Vec<CollectionItem> {
    let mut rows: BTreeMap<String, CollectionItem> = BTreeMap::new();

    for body in item_bodies(service) {
        assert_eq!(
            body["household_id"],
            json!(household_id.0.clone()),
            "every item write addresses the household's one shared collection"
        );

        let item_id: ItemId = serde_json::from_value(body["item_id"].clone())
            .expect("the write names the item it writes");
        let updated_at: Instant = serde_json::from_value(body["updated_at"].clone())
            .expect("the write carries its version marker");
        let deleted = match body.get("deleted") {
            None => false,
            Some(Value::Bool(deleted)) => *deleted,
            Some(other) => panic!("a write's `deleted` is a boolean when present, not {other:?}"),
        };
        let payload: ItemPayload = serde_json::from_value(body["payload"].clone())
            .expect("the write carries the item's content");

        rows.insert(
            item_id.0.clone(),
            CollectionItem {
                item_id,
                updated_at,
                deleted,
                payload: Some(payload),
            },
        );
    }

    rows.into_values().collect()
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

/// This device's copy, refreshed from the writes the service accepted. Every read below runs against
/// this copy, which is the version the household shares.
fn refresh(state: &AppState, service: &ScriptedService, household_id: &HouseholdId) {
    load(state, household_id, shared_rows(service, household_id));
}

/// A state holding the household's one active rabbit, with the copy the read path delivers after its
/// create — the state a weight write starts from, since a write resolves the rabbit from the copy.
fn household_with_a_rabbit() -> (Arc<ScriptedService>, AppState, Rabbit) {
    let (service, state) = held_household();
    let one = household(HOUSEHOLD);

    let rabbit =
        block_on_ready(state.create_rabbit(one.clone(), profile(), request_id(CREATE_RABBIT)))
            .expect("the double accepted the create");
    refresh(&state, &service, &one);

    (service, state, rabbit)
}

/// One entry create through the state method the `create_weight_entry` command forwards to.
fn recorded(
    state: &AppState,
    rabbit: &Rabbit,
    entry: WeightEntryDraft,
    operation: &str,
) -> WeightEntry {
    block_on_ready(state.create_weight_entry(rabbit.id.clone(), entry, request_id(operation)))
        .map(|(_household, entry)| entry)
        .expect("the double accepted the entry")
}

/// A rabbit's six months of entries, oldest first, recorded through the write path and loaded as
/// the copy the reads resolve. Each entry is just inside or just outside one of the three bounded
/// windows, so a range that mishandles an edge cannot answer the same list as a correct one.
fn six_month_history() -> (
    Arc<ScriptedService>,
    AppState,
    Rabbit,
    Vec<(&'static str, WeightEntry)>,
) {
    let (service, state, rabbit) = household_with_a_rabbit();

    let fixed = [
        ("six_months", 1_900, SIX_MONTHS),
        ("ninety_days_before", 1_950, NINETY_DAYS_BEFORE),
        ("ninety_days_edge", 1_960, NINETY_DAYS_EDGE),
        ("thirty_days_before", 1_970, THIRTY_DAYS_BEFORE),
        ("thirty_days_edge", 1_980, THIRTY_DAYS_EDGE),
        ("seven_days_before", 1_990, SEVEN_DAYS_BEFORE),
        ("seven_days_edge", 2_000, SEVEN_DAYS_EDGE),
        ("recent", 2_010, RECENT),
    ];

    let history: Vec<(&'static str, WeightEntry)> = fixed
        .into_iter()
        .enumerate()
        .map(|(index, (name, grams, timestamp))| {
            let operation = format!("operation-{}", index + 3);
            (
                name,
                recorded(&state, &rabbit, draft_at(grams, timestamp), &operation),
            )
        })
        .collect();

    refresh(&state, &service, &household(HOUSEHOLD));

    (service, state, rabbit, history)
}

/// One rabbit's series for one selected range, through the state method the `get_weight_entries`
/// command forwards to, measured from the fixed [`NOW`].
fn trend_at(state: &AppState, rabbit: &Rabbit, range: WeightRange) -> WeightTrend {
    state
        .weight_trend(&rabbit.id, range, &at(NOW))
        .expect("the rabbit's history is readable")
}

/// The ids of one series' points, oldest first — the order the contract fixes.
fn ids(trend: &WeightTrend) -> Vec<WeightEntryId> {
    trend.points.iter().map(|point| point.id.clone()).collect()
}

/// The expected ids of the named entries, in the given chronological order.
fn ids_of(history: &[(&str, WeightEntry)], names: &[&str]) -> Vec<WeightEntryId> {
    names
        .iter()
        .map(|name| {
            history
                .iter()
                .find(|(entry_name, _)| entry_name == name)
                .unwrap_or_else(|| panic!("{name} is part of the fixture"))
                .1
                .id
                .clone()
        })
        .collect()
}

/// WT-6, "Selecting a range limits the trend to that window".
///
/// **WHEN** a rabbit has weight entries spanning six months and the 30-day range is selected **THEN**
/// only the entries of the last 30 days are presented in the trend.
#[test]
fn selecting_a_range_limits_the_trend_to_that_window() {
    let (_service, state, rabbit, history) = six_month_history();

    let thirty = trend_at(&state, &rabbit, WeightRange::Days30);

    // THEN only the entries of the last 30 days are presented, edges inclusive and ordered oldest
    // first: the entry exactly 30 days old is in, and the one a second older is out.
    assert_eq!(
        thirty.range,
        WeightRange::Days30,
        "the answer states the window it was read for"
    );
    assert_eq!(
        ids(&thirty),
        ids_of(
            &history,
            &[
                "thirty_days_edge",
                "seven_days_before",
                "seven_days_edge",
                "recent",
            ]
        ),
        "only the last 30 days are presented"
    );
    assert!(
        !ids(&thirty).contains(&ids_of(&history, &["six_months"])[0]),
        "an entry six months old is outside the 30-day window"
    );
    assert!(
        !ids(&thirty).contains(&ids_of(&history, &["thirty_days_before"])[0]),
        "an entry a second before the 30-day edge is outside the window"
    );

    // The same boundaries hold for the other selectable windows: 7 days keeps only the last week, and
    // 90 days keeps the 60-day-old entry that 30 days excludes.
    let seven = trend_at(&state, &rabbit, WeightRange::Days7);
    assert_eq!(
        ids(&seven),
        ids_of(&history, &["seven_days_edge", "recent"]),
        "the 7-day range presents the last week, both edges inclusive"
    );

    let ninety = trend_at(&state, &rabbit, WeightRange::Days90);
    assert_eq!(
        ids(&ninety),
        ids_of(
            &history,
            &[
                "ninety_days_edge",
                "thirty_days_before",
                "thirty_days_edge",
                "seven_days_before",
                "seven_days_edge",
                "recent",
            ]
        ),
        "the 90-day range includes the entries 30 days excludes and stops at its own edge"
    );
}

/// WT-6, "The full-history range presents every entry".
///
/// **WHEN** the full-history range is selected **THEN** every weight entry of the rabbit is presented
/// in the trend.
#[test]
fn the_full_history_range_presents_every_entry() {
    let (service, state, rabbit, history) = six_month_history();

    // The full history is unbounded on both sides, so an entry stamped after the read clock is one of
    // the rabbit's entries and is presented too — unlike every bounded window above.
    let future = recorded(&state, &rabbit, draft_at(2_020, FUTURE), "operation-11");
    refresh(&state, &service, &household(HOUSEHOLD));

    let all = trend_at(&state, &rabbit, WeightRange::All);

    assert_eq!(
        all.range,
        WeightRange::All,
        "the answer states the window it was read for"
    );

    let expected: Vec<WeightEntryId> = history
        .iter()
        .map(|(_, entry)| entry.id.clone())
        .chain(std::iter::once(future.id.clone()))
        .collect();

    assert_eq!(
        ids(&all),
        expected,
        "every weight entry of the rabbit is presented, oldest first"
    );
    assert_eq!(
        all.points.len(),
        history.len() + 1,
        "no entry is dropped by the unbounded range"
    );
    assert!(
        all.points
            .iter()
            .zip(
                history
                    .iter()
                    .chain(std::iter::once(&(FUTURE, future.clone())))
            )
            .all(|(point, (_, entry))| {
                point.value_grams == entry.value_grams && point.timestamp == entry.timestamp
            }),
        "each presented point is the stored entry, not a placeholder or a re-dated one"
    );

    // The same entry is outside every bounded window: a 30-day read stops at the read clock.
    let bounded = trend_at(&state, &rabbit, WeightRange::Days30);
    assert!(
        !ids(&bounded).contains(&future.id),
        "an entry stamped after the read clock is outside the 30-day window"
    );
}

/// WT-6, "The weight history range is independent of the trend range".
///
/// **WHEN** the trend is set to 30 days and the weight-only history is set to 7 days **THEN** the
/// weight-only history lists only the entries of the last 7 days **AND** the trend still presents the
/// last 30 days.
#[test]
fn the_weight_history_range_is_independent_of_the_trend_range() {
    let (_service, state, rabbit, history) = six_month_history();

    // The trend's read at 30 days, then the history's read at 7 days over the same rabbit and the
    // same copy — two calls of the one ranged read, each with its own window (D1.4).
    let trend_thirty = trend_at(&state, &rabbit, WeightRange::Days30);
    let history_seven = trend_at(&state, &rabbit, WeightRange::Days7);

    // THEN the weight-only history lists only the entries of the last 7 days.
    assert_eq!(
        history_seven.range,
        WeightRange::Days7,
        "the history answers its own window"
    );
    assert_eq!(
        ids(&history_seven),
        ids_of(&history, &["seven_days_edge", "recent"]),
        "the history's 7-day read lists the last week only"
    );
    assert!(
        !ids(&history_seven).contains(&ids_of(&history, &["seven_days_before"])[0]),
        "the history's 7-day read does not widen to the trend's 30 days"
    );

    // AND the trend still presents the last 30 days: re-reading it after the 7-day read answers the
    // same series, so the narrower read neither narrowed nor changed it.
    let trend_thirty_again = trend_at(&state, &rabbit, WeightRange::Days30);
    assert_eq!(
        trend_thirty_again, trend_thirty,
        "the trend's 30-day read is unchanged by the history's 7-day read"
    );
    assert_eq!(
        ids(&trend_thirty_again),
        ids_of(
            &history,
            &[
                "thirty_days_edge",
                "seven_days_before",
                "seven_days_edge",
                "recent",
            ]
        ),
        "the trend still presents the last 30 days"
    );
}
