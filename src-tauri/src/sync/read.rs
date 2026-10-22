//! The read path: the locally kept copy first, a refresh from the shared service when the device
//! can reach it, and the three SYNC-3 states told apart exactly (SYNC-1, SYNC-3, NFR-1; D0.2).
//!
//! Reads are resolved in one order, and the order is the point: this device's copy is read *first*,
//! and only then is a refresh attempted. A refresh that fails — no answer, a timeout, a non-success
//! status, a body this boundary cannot read, or a guarded load the storage boundary refuses — leaves
//! the copy exactly as it was and the read still resolves. Reachability decides whether a refresh is
//! *attempted*; it never decides whether a read *succeeds* (NFR-1, D0.2). The copy itself is
//! persisted by the state layer, so "previously loaded" survives a relaunch (see
//! [`crate::storage::CollectionStore`]).
//!
//! # The three states, as the frozen contract expresses them
//!
//! [`HouseholdCollection`] carries `source` and `loaded_at`, so the states SYNC-3 distinguishes are
//! values rather than a convention:
//!
//! | State | `source` | `loaded_at` | Produced by |
//! |---|---|---|---|
//! | refreshed from the service | [`CollectionSource::Service`] | `Some(..)` | [`ReadPath::refresh`], after a guarded load |
//! | previously loaded, served while unreachable | [`CollectionSource::Local`] | `Some(..)` | [`ReadPath::serve_local`] reading the copy |
//! | nothing ever loaded, service unreachable | [`CollectionSource::Local`] | `None` | [`ReadPath::serve_local`] when no copy is held |
//!
//! The last is its own state and is **not** an empty collection presented as current data: an empty
//! item list with `loaded_at: None` says "this device has never loaded this household", while an
//! empty list with `loaded_at: Some(..)` is a loaded household that happens to have no rows. The
//! marker, not the row count, is the distinction — and only a load, never a merge or a failed
//! refresh, moves it.
//!
//! # A refresh replaces the copy; it never duplicates it (SYNC-1)
//!
//! The refresh goes through [`AppState::load_collection`], the guarded **load** path, which replaces
//! the household's copy wholesale and records the load instant beside it. Two consequences follow:
//! an item the previous copy held and the answer repeats appears once, and the copy this device
//! holds for the household stays exactly one. Nothing here merges rows itself, so no second copy of
//! an item can be produced by the read path.
//!
//! **The load instant is this device's, never the answer's.** The recorded instant comes from
//! [`AbsoluteInstant::now`] (task 2.8's clock) at the moment of the successful load. A feed that
//! carried a `loaded_at` of its own would describe the service's read, not this device's, and the
//! guarded load ignores it; the private answer shape below does not even represent it.
//!
//! # P0 asks for the whole collection; P6 owns the cursor and the merge
//!
//! `household_changes` is resumable from a server-assigned `since_change_seq` (ADR 0002), but the
//! frozen [`HouseholdCollection`] carries no cursor field. **P0 sends `0` on every refresh and
//! treats the answer as the household's whole collection**, because a non-zero marker asks for
//! changes *after* it and replacing a copy with a delta would delete every item the delta did not
//! mention. The incremental feed belongs to `add-read-path-performance` (P6): when it lands, the
//! cursor is kept in this module (a device-local position, not part of the collection's data, and
//! not on `storage/`'s fixed load-instant marker), and a delta answer must **merge** the named rows
//! through the guarded merge path and advance the cursor after the merge succeeded — never replace
//! the copy. P0 keeps no cursor at all, so there is nothing that could skip an unapplied change.
//!
//! # A row of a kind this version does not model is dropped, not stored
//!
//! The shared collection is expected to hold items of later phases (P2's health items), and the
//! frozen [`CollectionItem`] has no way to say "foreign": a stored row with no payload would read as
//! an item this version models whose content was not loaded, and every P1 read would have to
//! describe or refuse over it. The answer is therefore classified per row **before** the guarded
//! load (see [`FeedRow`]): only a payload whose string `kind` this version does not model drops its
//! row, while a known kind that fails to decode — like a missing payload or a body with no string
//! `kind` — is kept as **unknown content**. The copy holds the items this version models; the shared
//! collection remains the record, and the phase that models a kind starts storing it when its own
//! type lands (D1.16). One later phase's item can therefore never take a collection read down.
//!
//! # When a refresh is attempted
//!
//! A start or foreground return calls [`ReadPath::load`]; that is the command path. The refresh is
//! attempted only when a **fresh** probe (task 2.4's [`probe`]) says a call is possible
//! ([`ProbeOutcome::permits_call`](crate::sync::ProbeOutcome::permits_call)): the service answered
//! and the account is identified. A remembered status is never consulted, so a stale "reachable"
//! cannot authorize a refresh and a stale "unreachable" cannot suppress one. If the probe refuses,
//! the refresh is skipped and the local copy is returned; the read is not an error. The same rule
//! applies to the answer: only a `2xx` whose body this module can read is a refresh, and every
//! other outcome is a failed refresh.
//!
//! # The call travels on its own seam method
//!
//! [`SharedService::read`] is the read path's call: it carries the household id and the feed
//! position, sends no operation identity (D0.14 is about writes), and is bounded by
//! [`READ_REFRESH_TIMEOUT`] rather than borrowing a save's bound. The body shape is private to this
//! module; the transport only reports status and body.
//!
//! # What this module is not
//!
//! It holds no clock of its own beyond the load instant, no last-write-wins comparator (task 2.6),
//! no status observation (task 2.13) and no persistence: the copy belongs to `storage/` and is
//! written to disk by the state layer. The frozen wire types are not extended.

use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::Value;

use super::READ_REFRESH_TIMEOUT;
use super::probe::probe;
use super::service::SharedService;
use crate::ipc::types::{
    CollectionItem, CollectionSource, CommandError, HouseholdCollection, HouseholdId, Instant,
    ItemId, ItemPayload,
};
use crate::state::AppState;
use crate::storage::{AbsoluteInstant, LocalCopy};

/// The read path over one device: a stateless orchestrator of the local copy and one refresh.
///
/// It holds no collection data and no transport; the copy belongs to the app state, and the
/// transport is passed to each call. That is also why there is no cursor here yet: P0 requests the
/// whole collection every time, and P6 adds the incremental position when it adds the merge.
#[derive(Debug, Default)]
pub struct ReadPath;

impl ReadPath {
    /// A read path.
    pub fn new() -> Self {
        Self
    }

    /// The process-wide read path the command path uses.
    ///
    /// The read path is stateless: the copies it serves belong to the app state and outlive this
    /// process through the persistence store, so this value is only a process-wide entry point, not
    /// a cache lifetime. Tests construct their own [`ReadPath`]; it observes nothing from another
    /// case.
    pub fn shared() -> &'static Self {
        static SHARED: OnceLock<ReadPath> = OnceLock::new();

        SHARED.get_or_init(Self::new)
    }

    /// Resolve one household's collection: the locally kept copy first, then a refresh when the
    /// device can reach the service (SYNC-3, D0.2).
    ///
    /// The local copy is captured before the refresh is attempted, so a refresh that fails — for
    /// any reason, including a guarded load the storage boundary refuses — returns exactly what this
    /// device held, never an empty or partial value. On a successful refresh the returned value is
    /// the loaded copy with `source: service` and the instant this device recorded.
    ///
    /// The frozen command shape is one request and one result, so it cannot hand the caller the
    /// copy *while* the refresh is outstanding; [`ReadPath::serve_local`] is that immediate read for
    /// a caller that can render before the refresh (P6's concern, D0.2). What this method guarantees
    /// is that the refresh is never a precondition for a read: it either improves the answer or is
    /// ignored.
    ///
    /// # Errors
    ///
    /// `validation` for a blank household id. Unreachability and a failed refresh are **not** errors:
    /// they are reported by the returned value's `source`/`loaded_at` (SYNC-3).
    pub async fn load(
        &self,
        state: &AppState,
        household_id: &HouseholdId,
    ) -> Result<HouseholdCollection, CommandError> {
        if household_id.0.trim().is_empty() {
            return Err(CommandError::validation(None));
        }

        let local = self.serve_local(state, household_id);

        Ok(self.refresh(state, household_id).await.unwrap_or(local))
    }

    /// The copy this device holds for `household_id`, labelled `source: local` (SYNC-3).
    ///
    /// An absent copy is reported as the explicit never-loaded state — `source: local` with
    /// `loaded_at: None` — rather than as an empty collection that could be mistaken for current
    /// data. No copy is created by this read: the value is produced by the storage type that owns
    /// the never-loaded representation.
    ///
    /// The method is `pub(crate)` for the one local-set caller, `list_rabbits`, which answers from the
    /// copy without attempting a refresh; it performs its own blank-id validation, exactly as
    /// [`ReadPath::load`] does, so reaching the copy directly does not bypass that refusal. P6's
    /// progressive render will be the next such caller.
    pub(crate) fn serve_local(
        &self,
        state: &AppState,
        household_id: &HouseholdId,
    ) -> HouseholdCollection {
        state
            .read_collection(household_id, CollectionSource::Local)
            .unwrap_or_else(|| {
                LocalCopy::never_loaded(household_id.clone()).to_wire(CollectionSource::Local)
            })
    }

    /// Refresh one household's copy from the shared service, replacing it wholesale (SYNC-1,
    /// SYNC-3).
    ///
    /// Returns the refreshed value (`source: service`, with this device's load instant), or `None`
    /// when no refresh happened: the fresh probe refused the attempt, the feed answered nothing this
    /// boundary could read, or the guarded load refused the answer. `None` never clears the copy.
    ///
    /// It is `pub(crate)` for the same reason as [`ReadPath::serve_local`]: the public entry point is
    /// [`ReadPath::load`], which validates the id and falls back to the local copy.
    pub(crate) async fn refresh(
        &self,
        state: &AppState,
        household_id: &HouseholdId,
    ) -> Option<HouseholdCollection> {
        // A refresh is a network call that needs an identified account, so it uses the same fresh
        // admission check the write path does, through the neutral predicate. A refusal here only
        // skips the refresh: the read itself is resolved by the caller from the local copy, never
        // refused (NFR-1).
        if !probe(state.service()).await.permits_call() {
            return None;
        }

        let feed = fetch_change_feed(state.service(), household_id).await?;
        let collection = feed.into_collection();

        // The instant is read here, by the device that performed the load, and is never taken from
        // the answer (D0.5, SYNC-3). The guarded load replaces the copy, records it, and persists
        // the loaded copies best-effort through the state layer.
        let loaded_at = AbsoluteInstant::now().to_wire();
        state
            .load_collection(household_id, collection, loaded_at)
            .ok()?;

        // The load is complete in memory; the file write is scheduled off the async worker and is
        // best effort, so a read never waits for disk or fails because of it.
        state.schedule_persist();

        state.read_collection(household_id, CollectionSource::Service)
    }
}

/// The private shape of one `household_changes` answer.
///
/// * `household_id` is **required**. The read path addresses a household and the guarded load refuses
///   an answer describing another one; an answer that omits the id is unreadable rather than being
///   assumed to belong to the requested household.
/// * `items` is required. A body missing it is unreadable, so a malformed answer cannot wipe a
///   loaded copy with an invented empty collection.
/// * `change_seq` is not read in P0: the whole collection was requested, and the cursor is P6's.
///
/// The shape is private on purpose: it is deployment configuration between this device and the
/// service, not a frozen wire contract, and the frontend never sees it. Adding a field to the feed
/// is additive here.
#[derive(Debug, Deserialize)]
struct ChangeFeed {
    /// The household the answer describes.
    household_id: HouseholdId,
    /// The collection's rows, still carrying their raw payload so its provenance can be decided
    /// before anything reaches the store (see [`FeedRow`]).
    items: Vec<FeedRow>,
}

impl ChangeFeed {
    /// This answer as the collection the guarded load takes, without the rows whose payload belongs
    /// to a kind this version does not model.
    ///
    /// `source` and `loaded_at` on the stored value are placeholders: the guarded load keeps the
    /// rows and records the caller's instant, and a subsequent read labels the copy's own source.
    fn into_collection(self) -> HouseholdCollection {
        HouseholdCollection {
            household_id: self.household_id,
            items: self.items.into_iter().filter_map(FeedRow::decode).collect(),
            source: CollectionSource::Service,
            loaded_at: None,
        }
    }
}

/// One row of an answer, before its payload's provenance is known (D1.16).
///
/// The payload stays raw JSON on purpose. The frozen [`CollectionItem`] cannot say "foreign", and a
/// tagged enum that refused an unrecognised `kind` would make one later phase's item — P2's health
/// items share this household collection — take the whole answer down. [`FeedRow::decode`] therefore
/// classifies it first, and the drop is a decode step rather than a field on any stored or exported
/// type.
#[derive(Debug, Deserialize)]
struct FeedRow {
    /// The item's opaque, stable identifier within its household.
    item_id: ItemId,
    /// The item's version marker, held unchanged.
    updated_at: Instant,
    /// Whether this row is a tombstone, held unchanged.
    deleted: bool,
    /// The payload exactly as the answer carried it; absent and `null` both mean unknown content.
    #[serde(default)]
    payload: Option<Value>,
}

impl FeedRow {
    /// This row as the shape the store takes, or `None` when it is **foreign** and must be dropped.
    ///
    /// A foreign row is dropped rather than stored because the frozen wire type cannot express
    /// "foreign": a stored row with `payload: None` would be indistinguishable from an item whose
    /// content this device simply has not read, and every P1 read would then describe — or refuse
    /// over — an item of a kind it does not model. Dropping it keeps the copy to the items this
    /// version models, so a rabbit list is never withheld because a health item shares the
    /// household, and no P1 write can reach the row. The shared collection stays the record, and the
    /// phase that models the kind starts storing it when its own type lands (D1.16).
    fn decode(self) -> Option<CollectionItem> {
        let payload = match classify(self.payload) {
            Provenance::Known(payload) => Some(payload),
            Provenance::UnknownContent => None,
            Provenance::Foreign => return None,
        };

        Some(CollectionItem {
            item_id: self.item_id,
            updated_at: self.updated_at,
            deleted: self.deleted,
            payload,
        })
    }
}

/// What one answer's payload is, as far as this version can tell (D1.16).
#[derive(Debug)]
enum Provenance {
    /// A kind this version models, decoded whole.
    Known(ItemPayload),
    /// A kind this version models whose content did not decode, or no payload at all. Stored as
    /// unknown content; the item is one this version must be able to describe.
    UnknownContent,
    /// A kind this version does not model. Its row is dropped before the guarded load.
    Foreign,
}

/// Classify one payload by `kind` (D1.16).
///
/// **Foreign is exactly one case: a string `kind` this version does not model.** Everything else —
/// no payload at all, `null`, a body that is not a tagged object, or a known tag whose body does not
/// decode — is unknown content: the row is kept, because a modelled item whose content this version
/// could not read must make the reads that need it refuse rather than silently omit it (SYNC-1).
fn classify(payload: Option<Value>) -> Provenance {
    let Some(value) = payload.filter(|value| !value.is_null()) else {
        return Provenance::UnknownContent;
    };

    let Some(kind) = value.get("kind").and_then(Value::as_str) else {
        // Not a tagged envelope this version can attribute to a kind, so it is content that failed
        // to decode — never a licence to drop a row that might be one this version models.
        return Provenance::UnknownContent;
    };

    if !ItemPayload::KINDS.contains(&kind) {
        return Provenance::Foreign;
    }

    match serde_json::from_value::<ItemPayload>(value) {
        Ok(payload) => Provenance::Known(payload),
        Err(_) => Provenance::UnknownContent,
    }
}

/// Submit one whole-collection request and read its answer, or `None` when no readable answer
/// arrived.
///
/// `since_change_seq` is always `0` in P0 (see the module documentation): a non-zero position asks
/// for a delta, which must merge rather than replace, and the merge is P6's.
async fn fetch_change_feed(
    service: &dyn SharedService,
    household_id: &HouseholdId,
) -> Option<ChangeFeed> {
    match service.read(household_id, 0, READ_REFRESH_TIMEOUT).await {
        super::service::ServiceReply::Answered(answer) if answer.is_success() => {
            serde_json::from_value(answer.into_body()?).ok()
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::json;

    use super::*;
    use crate::ipc::types::{AccountUserId, Instant, ItemId, ItemPayload, RequestId, SaveCause};
    use crate::storage::{CollectionStore, DeviceSnapshot};
    use crate::sync::MergePath;
    use crate::sync::REACHABILITY_PROBE_TIMEOUT;
    use crate::sync::READ_REFRESH_TIMEOUT;
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::double::{RecordedCall, ScriptedService};
    use crate::sync::{Answer, ServiceReply};

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn item(item_id: &str, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(item_id.to_owned()),
            updated_at: at(updated_at),
            deleted: false,
            payload: None,
        }
    }

    /// One rabbit item's content, as `save_item` submits it (D1.16).
    fn rabbit_payload(is_archived: bool) -> ItemPayload {
        ItemPayload::Rabbit {
            rabbit: crate::ipc::types::Rabbit {
                id: crate::ipc::types::RabbitId("item-1".to_owned()),
                household_id: household("household-1"),
                name: "Pebble".to_owned(),
                photo_ref: None,
                breed: Some("Mini Lop".to_owned()),
                dob: None,
                dob_unknown: true,
                sex: crate::ipc::types::Sex::Female,
                neuter_status: crate::ipc::types::NeuterStatus::Spayed,
                adoption_date: None,
                is_archived,
                weight_drop_threshold_percent: 5.0,
            },
        }
    }

    /// One row carrying `payload` at `updated_at`.
    fn row_with(item_id: &str, updated_at: &str, payload: ItemPayload) -> CollectionItem {
        CollectionItem {
            payload: Some(payload),
            ..item(item_id, updated_at)
        }
    }

    fn stored_collection(
        household_id: &HouseholdId,
        items: Vec<CollectionItem>,
    ) -> HouseholdCollection {
        HouseholdCollection {
            household_id: household_id.clone(),
            items,
            source: CollectionSource::Service,
            loaded_at: None,
        }
    }

    fn persisted_collection(
        household_id: &HouseholdId,
        items: Vec<CollectionItem>,
    ) -> HouseholdCollection {
        HouseholdCollection {
            household_id: household_id.clone(),
            items,
            source: CollectionSource::Local,
            loaded_at: Some(at("2026-10-19T11:00:00Z")),
        }
    }

    fn account() -> AccountUserId {
        AccountUserId("account-1".to_owned())
    }

    fn request_id(value: &str) -> RequestId {
        RequestId(value.to_owned())
    }

    /// One accepted create answer the domain can read.
    fn accepted_create(household_id: &str) -> ServiceReply {
        ServiceReply::Answered(Answer::with_body(
            200,
            json!({
                "outcome": "applied",
                "household": {
                    "id": household_id,
                    "owner_member_id": "member-1",
                    "created_at": "2026-10-19T10:00:00Z",
                },
            }),
        ))
    }

    /// One readable whole-collection answer.
    fn feed(household_id: &HouseholdId, items: Vec<CollectionItem>) -> ServiceReply {
        ServiceReply::Answered(Answer::with_body(
            200,
            json!({
                "household_id": household_id.0.as_str(),
                "items": items,
                // Present in the deployment's answer but not read in P0.
                "change_seq": 7,
            }),
        ))
    }

    /// One readable whole-collection answer whose rows are given as raw JSON, so a test can send a
    /// foreign kind or a known kind with an undecodable body.
    fn feed_rows(household_id: &HouseholdId, items: Vec<serde_json::Value>) -> ServiceReply {
        ServiceReply::Answered(Answer::with_body(
            200,
            json!({
                "household_id": household_id.0.as_str(),
                "items": items,
                "change_seq": 7,
            }),
        ))
    }

    /// Every read the double was asked to perform, in call order.
    fn read_calls(service: &ScriptedService) -> Vec<RecordedCall> {
        service
            .calls()
            .into_iter()
            .filter(|call| matches!(call, RecordedCall::Read { .. }))
            .collect()
    }

    #[test]
    fn previously_loaded_data_is_served_when_the_service_is_unreachable() {
        let one = household("household-1");
        let service = Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
        let state = AppState::new(service.clone(), None);
        let loaded_at = at("2026-10-19T11:00:00Z");
        let held = vec![item("item-1", "2026-10-19T10:00:00Z")];
        state
            .load_collection(
                &one,
                stored_collection(&one, held.clone()),
                loaded_at.clone(),
            )
            .expect("the copy is addressed correctly");

        let path = ReadPath::new();
        let read = block_on_ready(path.load(&state, &one))
            .expect("a read is not refused while unreachable");

        assert_eq!(read.source, CollectionSource::Local);
        assert_eq!(read.loaded_at, Some(loaded_at));
        assert_eq!(read.items, held);

        // The probe was the only call: a refused refresh never reached the transport, and the
        // loaded copy was not touched.
        assert_eq!(
            service.calls(),
            vec![RecordedCall::CheckService {
                bound: REACHABILITY_PROBE_TIMEOUT,
            }]
        );
    }

    #[test]
    fn a_start_with_connectivity_refreshes_and_replaces_the_loaded_copy() {
        let one = household("household-1");
        let old_load = at("2026-10-19T10:00:00Z");
        let service = Arc::new(ScriptedService::new().on_read([feed(
            &one,
            vec![
                item("item-1", "2026-10-19T12:00:00Z"),
                // A repeated row inside one answer collapses too: the load is an upsert by item id.
                item("item-1", "2026-10-19T13:00:00Z"),
                item("item-2", "2026-10-19T12:00:00Z"),
            ],
        )]));
        let state = AppState::new(service.clone(), None);
        state
            .load_collection(
                &one,
                stored_collection(&one, vec![item("item-1", "2026-10-19T09:00:00Z")]),
                old_load.clone(),
            )
            .expect("the previous copy loads");

        let path = ReadPath::new();
        let read = block_on_ready(path.load(&state, &one)).expect("the read resolves");

        assert_eq!(read.source, CollectionSource::Service);
        assert!(
            read.loaded_at.is_some(),
            "a refresh records this device's load instant"
        );
        assert_ne!(read.loaded_at, Some(old_load), "the load moved the marker");
        assert_eq!(
            read.items.len(),
            2,
            "the refresh replaced the copy, never appended"
        );

        let stored = read
            .items
            .iter()
            .find(|row| row.item_id == ItemId("item-1".to_owned()))
            .expect("item-1 is present exactly once");
        assert_eq!(stored.updated_at, at("2026-10-19T13:00:00Z"));

        // The guarded load stored exactly the value the read returned: one row per item.
        let held = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the loaded copy is held");
        assert_eq!(held.items, read.items);
        assert_eq!(held.loaded_at, read.loaded_at);

        // The call is a read, with the read bound, and P0 asks for the whole collection.
        assert_eq!(
            read_calls(&service),
            vec![RecordedCall::Read {
                bound: READ_REFRESH_TIMEOUT,
                household_id: one,
                since_change_seq: 0,
            }]
        );
        assert_eq!(
            service.write_count(),
            0,
            "a read is never recorded as a write"
        );
        assert_eq!(service.read_count(), 1);
    }

    #[test]
    fn nothing_ever_loaded_and_unreachable_is_its_own_state_not_an_empty_collection() {
        let one = household("household-1");
        let service = Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
        let state = AppState::new(service.clone(), None);

        let path = ReadPath::new();
        let read = block_on_ready(path.load(&state, &one)).expect("the read resolves");

        assert_eq!(read.household_id, one);
        assert_eq!(read.source, CollectionSource::Local);
        assert_eq!(read.loaded_at, None);
        assert!(read.items.is_empty());

        // The value is produced from the absence of a copy, not from a fabricated empty one: the
        // device still holds no copy, which is a different fact from a loaded empty collection.
        assert!(
            state
                .read_collection(&one, CollectionSource::Local)
                .is_none()
        );
        assert_eq!(service.calls().len(), 1, "no refresh was attempted");
    }

    #[test]
    fn a_loaded_empty_collection_is_not_the_never_loaded_state() {
        let one = household("household-1");
        let service = Arc::new(
            ScriptedService::new()
                .on_service_check([
                    ServiceReply::Answered(Answer::status_only(200)),
                    ServiceReply::NoResponse,
                ])
                .on_read([feed(&one, Vec::new())]),
        );
        let state = AppState::new(service, None);
        let path = ReadPath::new();

        let refreshed = block_on_ready(path.load(&state, &one)).expect("the first read resolves");
        assert_eq!(refreshed.source, CollectionSource::Service);
        assert!(refreshed.items.is_empty());
        let loaded_at = refreshed.loaded_at.clone();
        assert!(loaded_at.is_some());

        // The service is now unreachable, but the loaded-empty copy is still a loaded copy.
        let served = block_on_ready(path.load(&state, &one)).expect("the second read resolves");
        assert_eq!(served.source, CollectionSource::Local);
        assert_eq!(served.items, Vec::new());
        assert_eq!(served.loaded_at, loaded_at);
        assert_ne!(served.loaded_at, None);
    }

    #[test]
    fn a_failed_refresh_leaves_the_loaded_copy_exactly_as_it_was() {
        let one = household("household-1");
        let loaded_at = at("2026-10-19T11:00:00Z");
        let held = vec![item("item-1", "2026-10-19T10:00:00Z")];
        let service = Arc::new(ScriptedService::new().on_read([ServiceReply::NoResponse]));
        let state = AppState::new(service, None);
        state
            .load_collection(
                &one,
                stored_collection(&one, held.clone()),
                loaded_at.clone(),
            )
            .expect("the copy loads");

        let path = ReadPath::new();
        let read = block_on_ready(path.load(&state, &one)).expect("the read resolves");

        assert_eq!(read.source, CollectionSource::Local);
        assert_eq!(read.loaded_at, Some(loaded_at.clone()));
        assert_eq!(read.items, held);

        let stored = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is still held");
        assert_eq!(stored.loaded_at, Some(loaded_at));
        assert_eq!(stored.items, held);
    }

    #[test]
    fn a_feed_describing_another_household_does_not_replace_the_local_copy() {
        let one = household("household-1");
        let two = household("household-2");
        let loaded_at = at("2026-10-19T11:00:00Z");
        let held = vec![item("item-1", "2026-10-19T10:00:00Z")];
        let service = Arc::new(
            ScriptedService::new()
                .on_read([feed(&two, vec![item("item-9", "2026-10-19T12:00:00Z")])]),
        );
        let state = AppState::new(service, None);
        state
            .load_collection(
                &one,
                stored_collection(&one, held.clone()),
                loaded_at.clone(),
            )
            .expect("the copy loads");

        let path = ReadPath::new();
        let read = block_on_ready(path.load(&state, &one)).expect("the read resolves");

        assert_eq!(read.source, CollectionSource::Local);
        assert_eq!(read.loaded_at, Some(loaded_at.clone()));
        assert_eq!(read.items, held);

        // The guarded load refused the foreign answer whole, so the copy did not change.
        let stored = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is still held");
        assert_eq!(stored.loaded_at, Some(loaded_at));
        assert_eq!(stored.items, held);
    }

    #[test]
    fn an_answer_without_a_household_id_is_not_a_refresh() {
        let one = household("household-1");
        let loaded_at = at("2026-10-19T11:00:00Z");
        let held = vec![item("item-1", "2026-10-19T10:00:00Z")];
        let service = Arc::new(ScriptedService::new().on_read([ServiceReply::Answered(
            Answer::with_body(
                200,
                json!({ "items": [item("item-2", "2026-10-19T12:00:00Z")] }),
            ),
        )]));
        let state = AppState::new(service, None);
        state
            .load_collection(
                &one,
                stored_collection(&one, held.clone()),
                loaded_at.clone(),
            )
            .expect("the copy loads");

        let path = ReadPath::new();
        let read = block_on_ready(path.load(&state, &one)).expect("the read resolves");

        assert_eq!(read.source, CollectionSource::Local);
        assert_eq!(read.loaded_at, Some(loaded_at));
        assert_eq!(read.items, held);
    }

    #[test]
    fn a_full_answer_that_omits_an_item_replaces_the_copy() {
        let one = household("household-1");
        let service = Arc::new(ScriptedService::new().on_read([feed(
            &one,
            // The whole collection as the service holds it now: item-2 is gone.
            vec![item("item-1", "2026-10-19T12:00:00Z")],
        )]));
        let state = AppState::new(service, None);
        state
            .load_collection(
                &one,
                stored_collection(
                    &one,
                    vec![
                        item("item-1", "2026-10-19T10:00:00Z"),
                        item("item-2", "2026-10-19T10:00:00Z"),
                    ],
                ),
                at("2026-10-19T11:00:00Z"),
            )
            .expect("the previous copy loads");

        let read = block_on_ready(ReadPath::new().load(&state, &one)).expect("the read resolves");

        assert_eq!(read.source, CollectionSource::Service);
        assert_eq!(read.items.len(), 1);
        assert_eq!(read.items[0].item_id, ItemId("item-1".to_owned()));
        let stored = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the loaded copy is held");
        assert_eq!(stored.items, read.items);
    }

    #[test]
    fn every_refresh_requests_the_whole_collection() {
        let one = household("household-1");
        let service = Arc::new(
            ScriptedService::new()
                .on_read([feed(&one, vec![item("item-1", "2026-10-19T10:00:00Z")])]),
        );
        let state = AppState::new(service.clone(), None);
        let path = ReadPath::new();

        block_on_ready(path.load(&state, &one)).expect("the first read resolves");
        block_on_ready(path.load(&state, &one)).expect("the second read resolves");
        block_on_ready(path.load(&state, &one)).expect("the third read resolves");

        let calls = read_calls(&service);
        assert_eq!(calls.len(), 3);

        for call in calls {
            assert_eq!(
                call,
                RecordedCall::Read {
                    bound: READ_REFRESH_TIMEOUT,
                    household_id: one.clone(),
                    // A non-zero position would ask for a delta, which replacing would lose.
                    since_change_seq: 0,
                }
            );
        }
    }

    #[test]
    fn a_refused_guarded_load_leaves_the_copy_intact_and_the_next_request_is_still_whole() {
        let one = household("household-1");
        let original = item("item-1", "2026-10-19T10:00:00Z");
        let service = Arc::new(ScriptedService::new().on_read([
            // A changed row for an entity the domain marked immutable: the guarded load refuses it.
            feed(&one, vec![item("item-1", "2026-10-19T12:00:00Z")]),
            // The next answer is acceptable; the failed attempt left nothing behind.
            feed(&one, vec![original.clone()]),
        ]));
        let state = AppState::new(service.clone(), None);
        let loaded_at = at("2026-10-19T11:00:00Z");
        state
            .load_collection(
                &one,
                stored_collection(&one, vec![original.clone()]),
                loaded_at.clone(),
            )
            .expect("the copy loads");
        state.mark_immutable(&original.item_id);

        let path = ReadPath::new();
        let refused = block_on_ready(path.load(&state, &one)).expect("the read resolves");
        assert_eq!(refused.source, CollectionSource::Local);
        assert_eq!(refused.loaded_at, Some(loaded_at.clone()));
        assert_eq!(refused.items, vec![original.clone()]);

        let accepted = block_on_ready(path.load(&state, &one)).expect("the next read resolves");
        assert_eq!(accepted.source, CollectionSource::Service);
        assert_eq!(accepted.items, vec![original]);

        // Neither attempt advanced a cursor: there is none in P0, and both asked for the whole
        // collection again.
        assert_eq!(
            read_calls(&service),
            vec![
                RecordedCall::Read {
                    bound: READ_REFRESH_TIMEOUT,
                    household_id: one.clone(),
                    since_change_seq: 0,
                },
                RecordedCall::Read {
                    bound: READ_REFRESH_TIMEOUT,
                    household_id: one,
                    since_change_seq: 0,
                },
            ]
        );
    }

    #[test]
    fn the_recorded_load_instant_is_the_devices_not_the_feeds() {
        let one = household("household-1");
        let answer = Answer::with_body(
            200,
            json!({
                "household_id": one.0.as_str(),
                "items": [],
                // Not part of the answer shape this module reads; if it were adopted, the
                // assertion below would observe it.
                "loaded_at": "1999-01-01T00:00:00Z",
            }),
        );
        let service = Arc::new(ScriptedService::new().on_read([ServiceReply::Answered(answer)]));
        let state = AppState::new(service, None);

        let path = ReadPath::new();
        let read = block_on_ready(path.load(&state, &one)).expect("the read resolves");

        assert_eq!(read.source, CollectionSource::Service);
        assert_ne!(read.loaded_at, Some(at("1999-01-01T00:00:00Z")));
        let recorded = AbsoluteInstant::parse(read.loaded_at.as_ref().expect("a load instant"))
            .expect("the recorded instant is an absolute instant");
        assert_eq!(recorded.to_wire(), read.loaded_at.expect("a load instant"));
    }

    #[test]
    fn a_restart_serves_the_persisted_copy_while_unreachable() {
        let one = household("household-1");
        let directory = temp_data_dir("restart");
        let held = vec![item("item-1", "2026-10-19T10:00:00Z")];
        let first_service = Arc::new(ScriptedService::new().on_read([feed(&one, held.clone())]));
        let first = AppState::with_collection_store(
            first_service,
            None,
            CollectionStore::in_dir(&directory),
        );

        block_on_ready(ReadPath::new().load(&first, &one)).expect("the first launch loads");
        // Tests do not spawn the production write; force the same snapshot synchronously.
        first.persist_now();
        let persisted = first
            .read_collection(&one, CollectionSource::Local)
            .expect("the first launch holds the copy");
        assert!(persisted.loaded_at.is_some());

        // A second state over the same directory is the next launch. The service is now unreachable.
        let second_service =
            Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
        let second = AppState::with_collection_store(
            second_service,
            None,
            CollectionStore::in_dir(&directory),
        );
        let read = block_on_ready(ReadPath::new().load(&second, &one))
            .expect("the relaunch read resolves");

        assert_eq!(read.source, CollectionSource::Local);
        assert_eq!(read.items, held);
        assert_eq!(read.loaded_at, persisted.loaded_at);
        assert!(read.loaded_at.is_some());

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_payload_survives_storage_a_snapshot_a_feed_and_a_merge_unchanged() {
        let one = household("household-1");
        let directory = temp_data_dir("payload-round-trip");
        let payload = rabbit_payload(false);
        let first = "2026-10-19T10:00:00Z";
        let second = "2026-10-19T11:00:00Z";
        let third = "2026-10-19T12:00:00Z";

        // Storage: a row carrying content enters through the guarded load and is persisted.
        let writer = AppState::with_collection_store(
            Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse])),
            None,
            CollectionStore::in_dir(&directory),
        );
        writer
            .load_collection(
                &one,
                stored_collection(&one, vec![row_with("item-1", first, payload.clone())]),
                at("2026-10-19T10:30:00Z"),
            )
            .expect("the copy is addressed correctly");
        writer.persist_now();

        // Snapshot: the next launch restores the content from the file.
        let restored = AppState::with_collection_store(
            Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse])),
            None,
            CollectionStore::in_dir(&directory),
        );
        let held = restored
            .read_collection(&one, CollectionSource::Local)
            .expect("the restored copy is held");
        assert_eq!(held.items[0].payload, Some(payload.clone()));

        // Feed: a newer version of the same item, same content, arrives through the read path.
        let fed = AppState::with_collection_store(
            Arc::new(ScriptedService::new().on_read([feed(
                &one,
                vec![row_with("item-1", second, payload.clone())],
            )])),
            None,
            CollectionStore::in_dir(&directory),
        );
        let read = block_on_ready(ReadPath::new().load(&fed, &one)).expect("the read resolves");
        assert_eq!(read.source, CollectionSource::Service);
        assert_eq!(read.items[0].payload, Some(payload.clone()));

        // Merge: a newer version that still carries the same content, then a read.
        let outcome = MergePath::new().merge(
            &fed,
            &one,
            stored_collection(&one, vec![row_with("item-1", third, payload.clone())]),
        );
        assert!(outcome.changed());

        let after = fed
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is held");
        assert_eq!(after.items[0].updated_at, at(third));
        assert_eq!(after.items[0].payload, Some(payload));

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_payload_is_classified_by_provenance() {
        // The three cases D1.16 distinguishes, decided before anything reaches the store.
        assert!(matches!(classify(None), Provenance::UnknownContent));
        assert!(matches!(
            classify(Some(json!(null))),
            Provenance::UnknownContent
        ));

        // A kind this version models, decoded whole: a payload.
        assert!(matches!(
            classify(Some(
                serde_json::to_value(rabbit_payload(false)).expect("encodable")
            )),
            Provenance::Known(ItemPayload::Rabbit { .. })
        ));
        assert!(matches!(
            classify(Some(json!({
                "kind": "weight_entry",
                "weight_entry": {
                    "id": "entry-1",
                    "rabbit_id": "item-1",
                    "value_grams": 1250,
                    "timestamp": "2026-10-20T09:00:00Z",
                    "note": null,
                    "photo_refs": [],
                    "has_weight_drop_alert": false,
                },
            }))),
            Provenance::Known(ItemPayload::WeightEntry { .. })
        ));

        // A kind this version models whose content did not decode: unknown content, still stored.
        assert!(matches!(
            classify(Some(
                json!({ "kind": "rabbit", "rabbit": { "name": "Pebble" } })
            )),
            Provenance::UnknownContent
        ));
        assert!(matches!(
            classify(Some(json!({ "kind": "weight_entry" }))),
            Provenance::UnknownContent
        ));

        // A present payload this version cannot attribute to a kind failed to decode: unknown
        // content, never a licence to drop a row that might be one this version models.
        assert!(matches!(
            classify(Some(json!({ "rabbit": { "name": "Pebble" } }))),
            Provenance::UnknownContent
        ));
        assert!(matches!(
            classify(Some(json!("not a tagged payload"))),
            Provenance::UnknownContent
        ));
        assert!(matches!(
            classify(Some(json!(7))),
            Provenance::UnknownContent
        ));

        // Foreign is exactly a string kind outside the modelled set: the row is dropped.
        assert!(matches!(
            classify(Some(
                json!({ "kind": "health", "health": { "weight_kg": 2.1 } })
            )),
            Provenance::Foreign
        ));
        assert!(matches!(
            classify(Some(json!({ "kind": "future_kind" }))),
            Provenance::Foreign
        ));
    }

    #[test]
    fn a_foreign_row_is_dropped_and_does_not_stop_a_known_row_from_being_read() {
        let one = household("household-1");
        let service = Arc::new(ScriptedService::new().on_read([feed_rows(
            &one,
            vec![
                json!({
                    "item_id": "item-1",
                    "updated_at": "2026-10-19T10:00:00Z",
                    "deleted": false,
                    "payload": rabbit_payload(false),
                }),
                // A known kind whose body does not decode: one this version models, content unknown.
                json!({
                    "item_id": "item-2",
                    "updated_at": "2026-10-19T10:00:00Z",
                    "deleted": false,
                    "payload": { "kind": "rabbit", "rabbit": { "name": "Pebble" } },
                }),
                // P2's kind: not modelled here, so its row is dropped, not stored.
                json!({
                    "item_id": "item-3",
                    "updated_at": "2026-10-19T10:00:00Z",
                    "deleted": false,
                    "payload": { "kind": "health", "health": { "weight_kg": 2.1 } },
                }),
            ],
        )]));
        let state = AppState::new(service, None);

        let read = block_on_ready(ReadPath::new().load(&state, &one)).expect("the read resolves");

        assert_eq!(
            read.source,
            CollectionSource::Service,
            "a foreign row does not stop the refresh"
        );
        let ids: Vec<&str> = read
            .items
            .iter()
            .map(|row| row.item_id.0.as_str())
            .collect();
        assert_eq!(ids, ["item-1", "item-2"]);
        assert_eq!(read.items[0].payload, Some(rabbit_payload(false)));
        assert_eq!(
            read.items[1].payload, None,
            "a known kind that did not decode is unknown content"
        );

        let stored = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is held");
        assert_eq!(stored.items, read.items);
        assert_eq!(
            state.item_owner(&ItemId("item-3".to_owned())),
            None,
            "a dropped row never claims its id"
        );
    }

    #[test]
    fn a_foreign_row_is_neither_written_back_nor_a_tombstone() {
        let one = household("household-1");
        let directory = temp_data_dir("foreign-drop");
        let service = Arc::new(ScriptedService::new().on_read([feed_rows(
            &one,
            vec![
                // A known removal stands as delivered; it is not what is under test here.
                json!({
                    "item_id": "item-2",
                    "updated_at": "2026-10-19T11:00:00Z",
                    "deleted": true,
                }),
                // A live foreign row and a foreign row that *claims* to be a tombstone. Neither may
                // become a row: a foreign row is not a removal this version can apply, and storing
                // the deleted one would let it look like a tombstone of an item it does not model.
                json!({
                    "item_id": "item-3",
                    "updated_at": "2026-10-19T11:00:00Z",
                    "deleted": false,
                    "payload": { "kind": "health", "health": { "note": "P2" } },
                }),
                json!({
                    "item_id": "item-4",
                    "updated_at": "2026-10-19T12:00:00Z",
                    "deleted": true,
                    "payload": { "kind": "health", "health": { "note": "P2" } },
                }),
            ],
        )]));
        let state =
            AppState::with_collection_store(service, None, CollectionStore::in_dir(&directory));

        let read = block_on_ready(ReadPath::new().load(&state, &one)).expect("the read resolves");

        let removed = read
            .items
            .iter()
            .find(|row| row.item_id == ItemId("item-2".to_owned()))
            .expect("the known removal is stored");
        assert!(removed.deleted);
        for dropped in ["item-3", "item-4"] {
            assert!(
                !read
                    .items
                    .iter()
                    .any(|row| row.item_id == ItemId(dropped.to_owned())),
                "{dropped} has no row at all — not a tombstone, not unknown content"
            );
            assert_eq!(
                state.item_owner(&ItemId(dropped.to_owned())),
                None,
                "{dropped} never claimed its id"
            );
        }

        // Not written back: the next launch reads a file that held neither dropped row, live or
        // tombstoned.
        state.persist_now();
        let relaunched = AppState::with_collection_store(
            Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse])),
            None,
            CollectionStore::in_dir(&directory),
        );
        let restored = relaunched
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is restored");
        for dropped in ["item-3", "item-4"] {
            assert!(
                restored
                    .items
                    .iter()
                    .all(|row| row.item_id != ItemId(dropped.to_owned())),
                "{dropped} is never persisted, whatever its deleted flag said"
            );
        }
        assert!(
            restored
                .items
                .iter()
                .any(|row| row.item_id == ItemId("item-2".to_owned()) && row.deleted),
            "the known removal survived the relaunch"
        );

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_corrupt_or_absent_store_degrades_to_never_loaded() {
        let one = household("household-1");
        let directory = temp_data_dir("corrupt-store");
        let store = CollectionStore::in_dir(&directory);

        // Absent: no file at all.
        let absent_service =
            Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
        let absent = AppState::with_collection_store(
            absent_service,
            None,
            CollectionStore::in_dir(&directory),
        );
        let absent_read = block_on_ready(ReadPath::new().load(&absent, &one))
            .expect("an absent store does not panic");
        assert_eq!(absent_read.source, CollectionSource::Local);
        assert_eq!(absent_read.loaded_at, None);
        assert!(absent_read.items.is_empty());

        // Corrupt: the file exists but is not a collection file this version wrote.
        fs::write(store.path(), b"not a collection file").expect("a corrupt file");
        let corrupt_service =
            Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
        let corrupt = AppState::with_collection_store(
            corrupt_service,
            None,
            CollectionStore::in_dir(&directory),
        );
        let corrupt_read = block_on_ready(ReadPath::new().load(&corrupt, &one))
            .expect("a corrupt store does not panic");
        assert_eq!(corrupt_read.source, CollectionSource::Local);
        assert_eq!(corrupt_read.loaded_at, None);
        assert!(corrupt_read.items.is_empty());

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_failed_persistence_write_does_not_fail_the_load() {
        let one = household("household-1");
        let directory = temp_data_dir("failed-write");
        let store = CollectionStore::in_dir(&directory);
        // A directory where the file belongs makes the atomic rename fail without pretending the
        // file was written.
        fs::create_dir(store.path()).expect("a directory where the file belongs");

        let state = AppState::with_collection_store(
            Arc::new(ScriptedService::new()),
            None,
            CollectionStore::in_dir(&directory),
        );
        let held = vec![item("item-1", "2026-10-19T10:00:00Z")];

        state
            .load_collection(
                &one,
                stored_collection(&one, held.clone()),
                at("2026-10-19T11:00:00Z"),
            )
            .expect("a persistence failure does not fail the in-memory load");

        // The in-memory copy is the source for this process; the failed write is not presented as a
        // persisted file.
        let read = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the in-memory copy is held");
        assert_eq!(read.items, held);

        // The write itself fails because a directory occupies the file's path; the load above still
        // succeeded and reported the in-memory copy.
        state.persist_now();
        assert!(store.load().is_err(), "nothing was persisted");

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_restart_restores_the_book_and_the_copy() {
        let one = household("household-1");
        let two = household("household-2");
        let directory = temp_data_dir("restart-book");
        let held = vec![item("item-1", "2026-10-19T10:00:00Z")];
        let first_service = Arc::new(
            ScriptedService::new()
                .on_write([
                    accepted_create("household-1"),
                    accepted_create("household-2"),
                ])
                .on_read([feed(&one, held.clone())]),
        );
        let first = AppState::with_collection_store(
            first_service,
            Some(account()),
            CollectionStore::in_dir(&directory),
        );

        block_on_ready(first.create_household("Home".to_owned(), request_id("op-1")))
            .expect("the first household is created");
        block_on_ready(first.create_household("Cabin".to_owned(), request_id("op-2")))
            .expect("the second household is created");
        first
            .set_active_household(one.clone(), request_id("op-3"))
            .expect("the person holds household-1");
        block_on_ready(ReadPath::new().load(&first, &one)).expect("the collection loads");
        first.persist_now();

        // A second state over the same directory is the next launch. The service is unreachable, so
        // the book and the copy can only come from the file.
        let second_service =
            Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
        let second = AppState::with_collection_store(
            second_service,
            Some(account()),
            CollectionStore::in_dir(&directory),
        );

        let rows = second.list_households();
        assert_eq!(rows.len(), 2, "the book survived the relaunch");
        assert_eq!(
            rows.iter().map(|row| &row.id).collect::<Vec<_>>(),
            vec![&one, &two],
            "acquisition order survived"
        );
        assert_eq!(
            rows.iter().filter(|row| row.is_active).count(),
            1,
            "exactly one active household"
        );
        assert!(rows[0].is_active);
        assert!(!rows[1].is_active);
        assert_eq!(
            second.active_household().expect("one is active").id,
            one,
            "the persisted active id survived"
        );

        let read = block_on_ready(ReadPath::new().load(&second, &one))
            .expect("the relaunch read resolves");
        assert_eq!(read.source, CollectionSource::Local);
        assert_eq!(read.items, held);
        assert!(read.loaded_at.is_some());

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn restoring_copies_rebuilds_their_ownership_claims() {
        let one = household("household-1");
        let two = household("household-2");
        let directory = temp_data_dir("claims");
        let store = CollectionStore::in_dir(&directory);
        let item_id = ItemId("item-1".to_owned());

        store
            .save(&DeviceSnapshot {
                collections: vec![
                    persisted_collection(&one, vec![item("item-1", "2026-10-19T10:00:00Z")]),
                    // The same item id under another household: the guard must refuse this whole
                    // collection rather than let the id belong to two households.
                    persisted_collection(&two, vec![item("item-1", "2026-10-19T10:00:00Z")]),
                ],
                ..DeviceSnapshot::default()
            })
            .expect("the state is stored");

        let state = AppState::with_collection_store(
            Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse])),
            None,
            CollectionStore::in_dir(&directory),
        );

        assert_eq!(state.item_owner(&item_id), Some(one));
        assert!(
            state
                .read_collection(&two, CollectionSource::Local)
                .is_none(),
            "the refused second collection did not restore a copy"
        );

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_blank_household_id_is_refused_as_invalid_input() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);

        let path = ReadPath::new();
        let error = block_on_ready(path.load(&state, &household("   ")))
            .expect_err("a blank id names no household");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.request_id, None);
        assert!(service.calls().is_empty());
    }

    #[test]
    fn a_read_never_creates_a_copy_when_no_refresh_happens() {
        let one = household("household-1");
        let service = Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
        let state = AppState::new(service, None);

        let path = ReadPath::new();
        let first = block_on_ready(path.load(&state, &one)).expect("the read resolves");
        let second = block_on_ready(path.load(&state, &one)).expect("the read resolves");

        assert_eq!(first, second);
        assert_eq!(first.loaded_at, None);
        assert!(
            state
                .read_collection(&one, CollectionSource::Local)
                .is_none()
        );
    }

    /// A fresh directory under the process's temporary area, unique per call.
    fn temp_data_dir(label: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        let name = format!("bunscale-read-path-{process}-{sequence}-{label}");
        let directory = std::env::temp_dir().join(name);

        fs::create_dir_all(&directory).expect("a test directory");

        directory
    }

    fn remove_temp_data_dir(directory: &Path) {
        let _cleanup = fs::remove_dir_all(directory);
    }
}
