//! The app-wide state the command adapters share, installed once in [`crate::run`]'s `setup`.
//!
//! The state is deliberately one value rather than a collection of managed singletons: the
//! household commands of this section and the read/status commands of the next all need the same
//! few things, and a single installation is what makes "the running app has exactly one transport,
//! one local copy set and one status" a property of construction.
//!
//! # What it carries, and why each part is here
//!
//! * **The shared service**, behind [`Arc<dyn SharedService>`]. Task 2.4's seam exists so the
//!   transport can be substituted; the app holds the real HTTPS client
//!   ([`HttpService`]), and a test holds a [`ScriptedService`](crate::sync::ScriptedService) in the
//!   same slot. The command path never names the concrete client, so adding the double to a test
//!   needs no test-only branch in the command.
//! * **The device's [`GuardedStore`]**, held privately behind a [`Mutex`]. Task 2.3 owns the layout
//!   and task 2.7 the guards; the read path (task 2.5) reaches it only through
//!   [`AppState::load_collection`] (a load that records `loaded_at`), [`AppState::merge_collection`]
//!   (a merge that leaves the marker alone) and [`AppState::read_collection`], each of which locks
//!   internally. No caller receives the store itself, so no command can replace it and discard the
//!   ownership claims and immutable markers; nor can it reach a mutable
//!   [`LocalCopies`](crate::storage::LocalCopies) and write around household isolation.
//! * **The [`StatusTracker`]**, behind a [`Mutex`]. Task 2.13's status commands will record fresh
//!   probe outcomes here; task 2.1 neither reads nor writes it, which is deliberate — the write
//!   path takes its own fresh probe and never consults a remembered status (D0.2, R3).
//! * **The [`HouseholdBook`]**, behind a [`Mutex`]. It holds the process's view of the person's
//!   households, including the operation ledger that makes a replayed create return the existing
//!   household (task 2.1, D0.14). The list and active-household reads and the local switch are
//!   served from it (task 2.2); none of them asks the shared service.
//! * **The current account**, when one exists. P0 has no sign-in or account-acquisition flow (Q25),
//!   so the running app is constructed with `None`; [`AppState::create_household`] then reports
//!   `no_account` before anything is sent, rather than pretending a write could succeed. The slot is
//!   an `Option` rather than a fabricated identity because "no account" is the honest state, and
//!   because no dependency is added for identity: the live-deployment task that provisions the first
//!   account can pass it here.
//!
//! # What it is not
//!
//! It is not a second write path, a queue or a cache of saved data. A create goes through
//! [`crate::domain::create_household`] and therefore through the gate; the book is updated only
//! after the gate returned a receipt, so nothing here can turn a refusal into a local success
//! (SYNC-2, D0.1). Switching the active household is the one book mutation that does **not** go
//! through the gate, and deliberately so: the selection is local state over the held set, not a
//! shared-collection write, and [`crate::domain::household`] records the full reasoning. That
//! command therefore has no probe, no account check and no transport failure, and it adds no second
//! path to the service.
//!
//! The mutexes are `std::sync::Mutex` and every guard is dropped before an `.await`: the command
//! locks the book only after the network call returns, and the status and copy guards are not held
//! across one at all.
//!
//! # The copies and the book outlive the process, on a best-effort basis
//!
//! [`AppState::production`] restores the device's loaded copies and household book from
//! [`CollectionStore`] under the app's own data directory, and a successful load or book change
//! schedules a rewrite of that file on the blocking pool. [`AppState::new`] stays in-memory only —
//! it is the test constructor, and a test states its own persistence explicitly through
//! [`AppState::with_collection_store`]. A persistence **write** failure does not make a read fail:
//! the in-memory copy and book remain the source for the current process, and the next launch falls
//! back to never-loaded only when nothing readable exists. A missing, unreadable or malformed file
//! is handled the same way at construction: the state starts with no copy and an empty book rather
//! than presenting invented data.
//!
//! **The operation ledger is not persisted.** It mirrors what this process already applied, while
//! D0.14's durable deduplication is the shared service's ledger; a create retried after a relaunch is
//! a fresh operation on the service's ledger. The restored book therefore has no local tombstone,
//! which is the documented domain behaviour of [`HouseholdBook::restore`].
//!
//! [`Arc<dyn SharedService>`]: std::sync::Arc
//! [`HttpService`]: crate::sync::HttpService
//! [`Mutex`]: std::sync::Mutex

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use tauri::Manager;

use crate::domain::{
    self, HouseholdBook, HouseholdRecord, RabbitRecord, local_member_id, validate_name,
};
use crate::ipc::types::{
    AccountUserId, CollectionSource, CommandError, Household, HouseholdCollection, HouseholdId,
    HouseholdSummary, Instant, ItemPayload, MemberId, Rabbit, RabbitId, RabbitProfile, RequestId,
    WeightEntry, WeightEntryDraft, WeightEntryId, WeightRange, WeightTrend,
};
use crate::storage::{
    AbsoluteInstant, BoundaryRefusal, CollectionStore, CollectionStoreError, DeviceSnapshot,
    GuardedStore, PersistedHousehold, StoredItem,
};
use crate::sync::http::AccessTokenSource;
use crate::sync::{HttpService, SharedService, StatusTracker};

/// The service origin the app is built with in P0.
///
/// There is **no live deployment** in this change (D0.7: the deployment is outside this repository
/// and outside its gate), so there is no origin to name. It is empty rather than invented, and it is
/// unobservable in P0 for the reason above the struct: no account exists, so
/// [`AppState::create_household`] refuses with `no_account` before the transport is asked anything.
/// The deployment task that first needs a real write supplies the real origin here.
const SERVICE_ORIGIN: &str = "";

/// The values one operation mints for itself and re-sends on every retry (D0.14).
///
/// They are one record rather than two maps because they are keyed by the same operation and live
/// exactly as long: a retry re-issues the same [`RequestId`], and the service refuses the same key
/// with a different payload, so each has to be the first attempt's value on every later attempt.
#[derive(Debug, Clone)]
struct OperationMemo {
    /// The version marker last-write-wins compares, written as the item's `updated_at`.
    marker: Instant,
    /// The weight entry's creation instant, once an entry create needed one from the clock. It is
    /// `None` until then, so an operation that is not an entry create never reads the clock for one
    /// (WT-1). An entry create always needs it — it is the entry's `created_at`, recorded even when
    /// the person supplied a timestamp (WT-7, D1.12).
    entry_created_at: Option<Instant>,
}

/// The one managed state the command adapters borrow.
pub struct AppState {
    /// The shared service, or the scripted double in a test (task 2.4's seam).
    service: Arc<dyn SharedService>,
    /// The current account, or `None` while P0 has no sign-in flow (Q25).
    account: Option<AccountUserId>,
    /// The process's household set and operation ledger (tasks 2.1, 2.2).
    households: Mutex<HouseholdBook>,
    /// The values each operation this process minted re-sends on a retry (D0.14).
    ///
    /// A retry re-issues the same [`RequestId`], and the service refuses the same key with a
    /// *different* payload, so the item write a retry submits has to be byte-identical to the first
    /// attempt's — including the `updated_at` marker last-write-wins compares (D0.7 criteria 3 and 6)
    /// and the creation instant a weight entry recorded when its draft omitted one (WT-1). Both are
    /// therefore minted once here and handed back on every later attempt rather than re-read from the
    /// clock, and they share one entry because they are minted for the same operation. The map is
    /// process-lifetime state like the household book's operation ledger (also unbounded and also not
    /// persisted); an entry is one operation this process attempted, and the service's own ledger is
    /// what bounds the window that matters.
    operations: Mutex<HashMap<RequestId, OperationMemo>>,
    /// The device's guarded copies of the collections it has loaded (tasks 2.3, 2.5 and 2.7).
    copies: Mutex<GuardedStore>,
    /// The last fresh service/account status (task 2.13).
    status: Mutex<StatusTracker>,
    /// The file the device's loaded copies and book are persisted in, for the synchronous test
    /// helper only; production writes through [`AppState::writer`].
    #[cfg(test)]
    persistence: Option<CollectionStore>,
    /// The coalescing writer that serialises persistence writes, when persistence is available.
    ///
    /// It holds the latest pending snapshot and one in-flight write, so an older snapshot can never
    /// overwrite a newer one even when its write was slower. Tests install their own sink through
    /// `with_writer`; the ordinary in-memory test constructor has none.
    writer: Option<Arc<PersistenceWriter>>,
}

impl AppState {
    /// State over one transport and one account, in memory only.
    ///
    /// This is the test constructor: it substitutes the scripted double for the transport and starts
    /// with no copies, so a test states its own persistence explicitly through
    /// [`AppState::with_collection_store`]. The running app uses [`AppState::production`], which adds
    /// the device's collection file to the same shape. There is no test-only state type and no command
    /// branch that knows which transport it received.
    pub fn new(service: Arc<dyn SharedService>, account: Option<AccountUserId>) -> Self {
        Self {
            service,
            account,
            households: Mutex::new(HouseholdBook::new()),
            operations: Mutex::new(HashMap::new()),
            copies: Mutex::new(GuardedStore::new()),
            status: Mutex::new(StatusTracker::new()),
            #[cfg(test)]
            persistence: None,
            writer: None,
        }
    }

    /// State over one transport, one account and one device store, restoring the loaded copies and
    /// the household book the store holds (SYNC-3, HS-2, HS-3; task 2.5).
    ///
    /// This is the seam a unit test uses for a relaunch: two states built over the same directory
    /// are two launches of the same device. A store whose file is absent, unreadable or malformed
    /// restores no copy and an empty book — the explicit never-loaded state — rather than failing
    /// construction or presenting invented data. [`AppState::new`] remains the strictly in-memory
    /// constructor.
    pub(crate) fn with_collection_store(
        service: Arc<dyn SharedService>,
        account: Option<AccountUserId>,
        persistence: CollectionStore,
    ) -> Self {
        let snapshot = persistence.load().unwrap_or_default();
        let copies = restored_copies(&snapshot.collections);
        let households = restored_book(&snapshot);

        #[cfg(not(test))]
        let writer = Some(Arc::new(PersistenceWriter::new(Arc::new(
            persistence.clone(),
        ))));
        #[cfg(test)]
        let writer = None;

        Self {
            service,
            account,
            households: Mutex::new(households),
            operations: Mutex::new(HashMap::new()),
            copies: Mutex::new(copies),
            status: Mutex::new(StatusTracker::new()),
            #[cfg(test)]
            persistence: Some(persistence),
            writer,
        }
    }

    /// A state with a supplied persistence sink, for a test that controls write ordering.
    ///
    /// The ordinary `with_collection_store` has no writer under `cfg(test)`, so the relaunch tests
    /// drive `persist_now` deterministically; this constructor is what the coalescing test uses to
    /// observe the real writer.
    #[cfg(test)]
    fn with_writer(
        service: Arc<dyn SharedService>,
        account: Option<AccountUserId>,
        persistence: CollectionStore,
        sink: Arc<dyn SnapshotSink>,
    ) -> Self {
        let mut state = Self::with_collection_store(service, account, persistence);
        state.writer = Some(Arc::new(PersistenceWriter::new(sink)));

        state
    }

    /// The state the running P0 app installs: the real HTTPS client, no account, and the device's
    /// persisted collection copies and household book.
    ///
    /// The token source answers `None` because the platform Keychain belongs to a sign-in flow this
    /// change does not ship (Q25). That is the honest half of the construction: the transport is
    /// real, but it has no identity to send, and [`AppState::create_household`] reports `no_account`
    /// rather than attempting an unauthenticated write or creating local-only data.
    ///
    /// The device state is restored from [`CollectionStore`] under the app's own data directory. A
    /// directory that cannot be resolved leaves the state in-memory only; a missing, unreadable or
    /// malformed file restores no copy and an empty book (the never-loaded state). Neither is a
    /// startup failure: the transport is usable, and the read path has an explicit state for both.
    ///
    /// # Errors
    ///
    /// The pinned `reqwest` client cannot be built (a TLS backend that fails to initialise). There
    /// is no later call that could recover from it, and P0 has nothing to fall back to, so the app's
    /// `setup` reports it instead of hiding it behind a state that could never work.
    pub fn production(app: &tauri::AppHandle) -> Result<Self, reqwest::Error> {
        let service = HttpService::new(SERVICE_ORIGIN, no_account_token())?;
        let service: Arc<dyn SharedService> = Arc::new(service);

        let Ok(data_dir) = app.path().app_data_dir() else {
            return Ok(Self::new(service, None));
        };

        Ok(Self::with_collection_store(
            service,
            None,
            CollectionStore::in_dir(data_dir),
        ))
    }

    /// The current account, when one is available.
    pub fn account(&self) -> Option<&AccountUserId> {
        self.account.as_ref()
    }

    /// The shared service this app was built with.
    ///
    /// It is the seam task 2.13's probes and task 2.5's refresh will use, and the reason the state
    /// is constructed from `Arc<dyn SharedService>` rather than the concrete client: a test can hand
    /// it the scripted double through [`AppState::new`] without a different code path.
    pub fn service(&self) -> &dyn SharedService {
        self.service.as_ref()
    }

    /// The process's household set and operation ledger.
    pub fn households(&self) -> MutexGuard<'_, HouseholdBook> {
        lock(&self.households)
    }

    /// Load one household's collection through the guarded store, recording the load instant
    /// (SYNC-3, NFR-10).
    ///
    /// This is **in-memory only**: it locks the collection store, applies the guarded load and
    /// returns. The caller at the async edge ([`crate::sync::ReadPath::refresh`]) schedules the
    /// best-effort persistence write on the blocking pool, so no file I/O runs on the async worker
    /// (`rust-conventions`). Task 2.5's refresh uses this for a load;
    /// [`AppState::merge_collection`] is the merge that leaves the marker alone.
    ///
    /// # Errors
    ///
    /// [`BoundaryRefusal`] as [`GuardedStore::load_wire_collection`]: a collection describing another
    /// household, a row claimed by another household, or a changed/omitted marked entity.
    pub fn load_collection(
        &self,
        household_id: &HouseholdId,
        collection: HouseholdCollection,
        loaded_at: Instant,
    ) -> Result<(), BoundaryRefusal> {
        lock(&self.copies).load_wire_collection(household_id, collection, loaded_at)
    }

    /// Snapshot the device state this process holds, for persistence.
    ///
    /// The snapshot is taken while each mutex is held only for the copy, so the blocking write that
    /// receives it never touches the locks. Only loaded copies are included; the book is exported in
    /// acquisition order with its active id.
    ///
    /// The clone happens on the calling thread rather than inside the blocking task because the
    /// snapshot must be read under the state's mutexes, and a guard cannot be held across an await or
    /// moved into a `'static` task. P0's snapshot is small; if P6 grows it, the writer can be handed
    /// the state behind an `Arc` instead.
    pub(crate) fn snapshot(&self) -> DeviceSnapshot {
        let collections: Vec<HouseholdCollection> = {
            let copies = lock(&self.copies);

            copies
                .local_copies()
                .iter()
                .filter(|copy| copy.loaded_at().is_some())
                .map(|copy| copy.to_wire(CollectionSource::Local))
                .collect()
        };

        let (households, active_household) = {
            let book = lock(&self.households);

            (
                book.held().map(persisted_household).collect(),
                book.active_id().cloned(),
            )
        };

        DeviceSnapshot {
            collections,
            households,
            active_household,
        }
    }

    /// Schedule a best-effort persistence write through the coalescing writer.
    ///
    /// The snapshot is taken on the calling thread (see [`AppState::snapshot`]) and handed to the
    /// writer, which keeps only the latest pending snapshot and one in-flight write. A later
    /// snapshot taken while a write runs therefore wins; an older snapshot cannot finish last and
    /// overwrite it. The write is best effort: a failure never fails a read and is never reported as
    /// persisted.
    ///
    /// Tests without a writer (`AppState::new`, and `with_collection_store` under `cfg(test)`) are a
    /// no-op here; a relaunch test calls [`AppState::persist_now`] instead, and the coalescing test
    /// installs its own sink through `with_writer`.
    pub(crate) fn schedule_persist(&self) {
        let Some(writer) = &self.writer else {
            return;
        };

        writer.schedule(self.snapshot());
    }

    /// Write the current snapshot synchronously, for a test that needs the relaunch file to exist
    /// before building the next state.
    ///
    /// The same best-effort rule applies: the caller asserts the store/file outcome separately. It
    /// is compiled only for tests, so product code cannot accidentally block an async worker.
    #[cfg(test)]
    pub(crate) fn persist_now(&self) {
        let Some(store) = &self.persistence else {
            return;
        };

        let _persisted = store.save(&self.snapshot());
    }

    /// Merge one household's rows through the guarded store, leaving `loaded_at` untouched
    /// (NFR-10).
    ///
    /// The collection mutex is locked and released inside this method; see
    /// [`AppState::load_collection`] for the load that records the marker.
    ///
    /// # Errors
    ///
    /// [`BoundaryRefusal`] as [`GuardedStore::upsert_wire_collection`].
    pub fn merge_collection(
        &self,
        household_id: &HouseholdId,
        collection: HouseholdCollection,
    ) -> Result<(), BoundaryRefusal> {
        let outcome = lock(&self.copies).upsert_wire_collection(household_id, collection);

        if outcome.is_ok() {
            // A merge changes the held copy even though it leaves the marker alone, so it is part of
            // what a relaunch must see.
            self.schedule_persist();
        }

        outcome
    }

    /// The collection this device holds for `household_id`, labelled with the source of the read
    /// (SYNC-3).
    ///
    /// `None` means this device holds no copy for the household — a different fact from a held copy
    /// whose `loaded_at` is `None`. The collection mutex is locked and released inside this method,
    /// so the read returns the wire value rather than a guard or a store reference.
    pub fn read_collection(
        &self,
        household_id: &HouseholdId,
        source: CollectionSource,
    ) -> Option<HouseholdCollection> {
        lock(&self.copies)
            .collection(household_id)
            .map(|copy| copy.to_wire(source))
    }

    /// Mark an item immutable in this process's copy, for a read-path test that needs the guarded
    /// load to refuse a changed row.
    ///
    /// The product marker is the domain's decision (NFR-10); P0 has no domain entity to mark yet, so
    /// the accessor exists only under `cfg(test)` and adds no command surface.
    #[cfg(test)]
    pub(crate) fn mark_immutable(&self, item_id: &crate::ipc::types::ItemId) {
        lock(&self.copies).mark_immutable(item_id);
    }

    /// The household that claims `item_id` in this process's guarded store, for a restore test that
    /// asserts the ownership claims were rebuilt (NFR-10).
    #[cfg(test)]
    pub(crate) fn item_owner(&self, item_id: &crate::ipc::types::ItemId) -> Option<HouseholdId> {
        lock(&self.copies).household_of(item_id).cloned()
    }

    /// The row this device holds for one item, reached through its household (NFR-10).
    ///
    /// The single-item counterpart of [`AppState::read_collection`]. It is `cfg(test)` for the same
    /// reason [`AppState::item_owner`] is: P1's read commands are later tasks, and a create must not
    /// read its own write back. It exists so a test can prove two rabbits of one household are two
    /// items, each resolvable on its own, rather than reading the list task 2.14 owns.
    #[cfg(test)]
    pub(crate) fn item(
        &self,
        household_id: &HouseholdId,
        item_id: &crate::ipc::types::ItemId,
    ) -> Result<Option<crate::storage::StoredItem>, BoundaryRefusal> {
        lock(&self.copies)
            .item(household_id, item_id)
            .map(|row| row.cloned())
    }

    /// The last fresh service/account status.
    pub fn status(&self) -> MutexGuard<'_, StatusTracker> {
        lock(&self.status)
    }

    /// Create a household with a non-blank name (HS-1, SYNC-2; D0.14).
    ///
    /// The steps are ordered for the person, not for the transport:
    ///
    /// 1. A blank name is `validation` even when no account exists, because the form asked for a
    ///    name and that is what has to change (HS-1).
    /// 2. No account is `no_account` before any probe. P0 always takes this path: it has no
    ///    sign-in flow (Q25), and pretending otherwise would report a write the service never saw.
    /// 3. With both, the domain operation goes through [`crate::sync::SaveGate`] — a fresh probe,
    ///    then the write — and its receipt is the only path that reaches the book.
    ///
    /// The book is recorded after the await, so no mutex guard is held across it. A replay returns
    /// the household the operation already produced and does not change which household is active
    /// (D0.14).
    ///
    /// # Errors
    ///
    /// `validation` for a blank name, `no_account` when no account is available, and otherwise the
    /// gate's mapped causes. A receipt whose household body cannot be read is `unknown`; nothing is
    /// recorded in that case, so a failed read can never become a local household.
    pub async fn create_household(
        &self,
        name: String,
        request_id: RequestId,
    ) -> Result<Household, CommandError> {
        let name = validate_name(name, &request_id)?;
        let Some(account) = self.account.as_ref() else {
            return Err(CommandError::no_account(Some(request_id)));
        };

        let record =
            domain::create_household(self.service.as_ref(), account, name, request_id.clone())
                .await?;

        let (stored, is_active) = {
            let mut households = lock(&self.households);
            let stored = households.record_created(request_id, record);
            let is_active = households.is_active(stored.id());

            (stored, is_active)
        };

        // The book changed: schedule the best-effort rewrite off the async worker. No lock is held
        // here, so the snapshot cannot deadlock on the same book.
        self.schedule_persist();

        Ok(stored.to_wire(account, is_active))
    }

    /// Create a rabbit profile in a household (RAB-1, RAB-2, RAB-3; D1.1, I8).
    ///
    /// The steps are ordered for the person, as [`AppState::create_household`]'s are:
    ///
    /// 1. The profile is recorded first, so the refusals the input itself can produce — a blank name,
    ///    which RAB-2 treats as empty and whitespace-only alike, a date of birth marked unknown
    ///    **and** supplied (I8), and a blank operation identity — are reported as `validation` even
    ///    when no account exists, because that is what the form has to change.
    /// 2. No account is `no_account` before any probe. P0 always takes this path: it has no sign-in
    ///    flow (Q25), and pretending otherwise would report a write the service never saw.
    /// 3. The **writing member** is resolved from the person's own household book: a shared write
    ///    carries `writer_id`, which the service's last-write-wins tuple compares (ADR 0002 criterion
    ///    3), and only the book knows which member this account is in that household. A household
    ///    this device holds no member for is refused rather than attributed to somebody else.
    /// 4. The version marker is minted once per operation (see [`AppState::operation_marker`]) and
    ///    the write then goes through [`crate::sync::SaveGate`] — a fresh probe, then the item write —
    ///    so a rabbit exists once the service has accepted it and never as a local-only value (D1.1,
    ///    SYNC-2).
    ///
    /// The returned rabbit is the recorded one. **This device's copy of the household's collection is
    /// deliberately not written here**: the copy is the read path's (SYNC-3), so a create cannot
    /// present a rabbit the service has not accepted, and no second write path is added.
    ///
    /// # Errors
    ///
    /// `validation` for a blank name, for the contradictory date-of-birth pair, for a blank operation
    /// identity, and for a household with no member for this account; `no_account` when no account is
    /// available; and otherwise the gate's mapped causes: `connectivity`, `no_account`,
    /// `allowance_exceeded`, `rate_limited`, `timeout` and the honest `unknown`. A refusal never
    /// creates a rabbit, and one that is a property of the input never reaches the shared service at
    /// all.
    pub async fn create_rabbit(
        &self,
        household_id: HouseholdId,
        profile: RabbitProfile,
        request_id: RequestId,
    ) -> Result<Rabbit, CommandError> {
        let record = RabbitRecord::created(household_id.clone(), profile, &request_id)?;

        if self.account.is_none() {
            return Err(CommandError::no_account(Some(request_id)));
        }

        let Some(writer) = self.writing_member(&household_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };
        let marker = self.operation_marker(&request_id);

        domain::save_rabbit(self.service.as_ref(), &record, &writer, &marker, request_id).await?;

        Ok(record.to_wire())
    }

    /// Replace an active rabbit's editable field set (RAB-5, HS-8).
    ///
    /// The steps are the create path's, plus the one lookup the frozen signature forces:
    ///
    /// 1. The write target is resolved from this device's guarded copy by the shared archived-target
    ///    check, `AppState::writable_rabbit` — an edit is addressed by the rabbit alone, and the
    ///    collection it writes is the one that **claimed the item** when the rabbit was loaded. The
    ///    item must still be live: a tombstone keeps its claim, so a permanently deleted rabbit is
    ///    refused here rather than resurrected by a full-payload write (RAB-10). A rabbit this device
    ///    holds no live row for is refused as `validation`, and nothing is sent. An **archived** rabbit
    ///    is refused as `validation` too (RAB-8, DM-4; D1.5), and a live row whose content is unknown
    ///    is refused with the unknown-content key because it cannot establish that the rabbit is not
    ///    archived (D1.16). None of these is a retryable backend failure: the state is read from the
    ///    same copy (SYNC-3), so a rabbit the device has not loaded cannot be presented for an edit.
    ///    The one reachable window is a just-created rabbit rendered from
    ///    [`AppState::create_rabbit`]'s return before the read path has loaded the collection, which
    ///    is why the frontend gates the edit control on the detail query instead.
    /// 2. The profile is recorded next, so the refusals the input itself produces — a blank name,
    ///    which RAB-2 treats as empty and whitespace-only alike, the contradictory date-of-birth pair
    ///    (I8), and a blank operation identity — are reported as `validation` even when no account
    ///    exists, because that is what the form has to change.
    /// 3. No account is `no_account` before any probe. P0 always takes this path: it has no sign-in
    ///    flow (Q25), and pretending otherwise would report a write the service never saw.
    /// 4. The **writing member** is resolved from the person's own household book exactly as
    ///    [`AppState::create_rabbit`] does, so the member who performs the edit is named and no
    ///    per-member or device-local copy is involved: every member writes into the one household
    ///    collection (HS-8).
    /// 5. The version marker is the one minted for the operation, so a retry re-sends a byte-identical
    ///    body, and the write then goes through [`crate::sync::SaveGate`] as one `save_item` call into
    ///    the household's collection. Because there is exactly one item, not one per member, every
    ///    member resolves the edited profile — the sharing is by construction, not by a fan-out
    ///    (RAB-5, HS-8).
    ///
    /// The returned rabbit is the recorded one. **This device's copy of the household's collection is
    /// deliberately not written here**, exactly as a create does not write it: the copy is the read
    /// path's (SYNC-3), so the edit reaches this device the same way it reaches every other member —
    /// by the read that follows `rabbit:changed`.
    ///
    /// # Errors
    ///
    /// `validation` for a blank name (RAB-2), for the contradictory date-of-birth pair (I8), for a
    /// blank operation identity, for a blank rabbit id, for a rabbit with no live row (absent or
    /// tombstoned), for an item that is not a rabbit, for an **archived** rabbit (RAB-8, DM-4; D1.5),
    /// and for a household with no member for this account; `unknown` with the unknown-content key when
    /// the rabbit's row has no content this device can describe (D1.16); `no_account` when no account
    /// is available; and otherwise the gate's mapped causes (`connectivity`, `no_account`,
    /// `allowance_exceeded`, `rate_limited`, `timeout`, `unknown`). A refusal never edits a rabbit, and
    /// one that is a property of the local copy never reaches the shared service at all.
    pub async fn update_rabbit(
        &self,
        rabbit_id: RabbitId,
        profile: RabbitProfile,
        request_id: RequestId,
    ) -> Result<Rabbit, CommandError> {
        let (household_id, _target) = self.writable_rabbit(&rabbit_id, &request_id)?;

        let record = RabbitRecord::updated(household_id.clone(), rabbit_id, profile, &request_id)?;

        if self.account.is_none() {
            return Err(CommandError::no_account(Some(request_id)));
        }

        let Some(writer) = self.writing_member(&household_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };
        let marker = self.operation_marker(&request_id);

        domain::save_rabbit(self.service.as_ref(), &record, &writer, &marker, request_id).await?;

        Ok(record.to_wire())
    }

    /// Record a weight entry for one rabbit (WT-1, WT-3, DM-1; D1.1, D1.2, D1.16).
    ///
    /// The steps are the edit path's, with the entry's own rules added:
    ///
    /// 1. The write target is resolved through [`AppState::writable_rabbit`] — the one archived-target
    ///    check every weight-entry write calls. An archived rabbit's create is refused as `validation`
    ///    (DM-4, RAB-8; D1.5), an unknown-content row is refused with the unknown-content key because it
    ///    cannot establish that the rabbit is not archived (D1.16), and an id that names an item which
    ///    is not a rabbit is refused as `validation` too. The household that resolved the rabbit is the
    ///    collection the entry is written into, never the person's active household.
    /// 2. The entry is recorded next, so the refusals the input itself produces — a value of zero or
    ///    less, an unconfirmed value above the plausibility bound (WT-4), a blank operation identity
    ///    and a timestamp that is not an absolute instant — are reported as `validation` even when no
    ///    account exists, because that is what the form has to change. The value is stored in
    ///    canonical grams exactly as supplied and carries no unit (DM-1); a confirmed outlier is
    ///    recorded at its true value, never clamped (D1.9). The supplied timestamp is kept as that
    ///    moment, including in the past, and an omitted one becomes the operation's creation instant
    ///    (WT-1) — minted once per operation by [`AppState::entry_created_at`], so a retry sends the
    ///    same one (D0.14). The note is optional and kept whole.
    /// 3. No account is `no_account` before any probe, and the **writing member** is resolved from the
    ///    person's household book exactly as a rabbit write resolves it (ADR 0002 criterion 3).
    /// 4. The version marker is the one minted for the operation and the write goes through
    ///    [`crate::sync::SaveGate`] — a fresh probe, then the item write — so an entry exists once the
    ///    service has accepted it and never as a local-only value (D1.1, SYNC-2).
    ///
    /// The returned household is the one whose collection holds the entry. The command's answer is the
    /// entry, but the adapter needs the household to scope `weight:changed` — and the entry itself
    /// carries only its rabbit, so it cannot name the household on its own.
    ///
    /// The recorded entry's `has_weight_drop_alert` is always `false`: the flag is derived on read and
    /// never content (D1.3, D1.13, D1.16). **This device's copy is deliberately not written here**,
    /// exactly as a rabbit create does not write it: the copy is the read path's (SYNC-3), so the entry
    /// reaches this device the same way it reaches every other member — by the read that follows
    /// `weight:changed`.
    ///
    /// # Errors
    ///
    /// `validation` for a blank rabbit id, a rabbit this device holds no live row for, one whose row is
    /// not a rabbit, an **archived** rabbit (DM-4, RAB-8; D1.5), a value of zero or less, an
    /// unconfirmed value above the plausibility bound (WT-4), a blank operation identity, a
    /// timestamp that is not an absolute instant, and a household with no member for this account;
    /// `unknown` with the unknown-content key when the rabbit's row has no content this device can
    /// describe (D1.16); `no_account` when no account is available; and otherwise the gate's mapped
    /// causes (`connectivity`, `no_account`, `allowance_exceeded`, `rate_limited`, `timeout`,
    /// `unknown`). A refusal never records an entry, and one that is a property of the local copy never
    /// reaches the shared service at all.
    pub async fn create_weight_entry(
        &self,
        rabbit_id: RabbitId,
        entry: WeightEntryDraft,
        request_id: RequestId,
    ) -> Result<(HouseholdId, WeightEntry), CommandError> {
        let (household_id, _target) = self.writable_rabbit(&rabbit_id, &request_id)?;

        let recorded = domain::weight::record(
            rabbit_id,
            entry,
            || self.entry_created_at(&request_id),
            &request_id,
        )?;

        if self.account.is_none() {
            return Err(CommandError::no_account(Some(request_id)));
        }

        let Some(writer) = self.writing_member(&household_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };
        let marker = self.operation_marker(&request_id);

        domain::weight::save_weight_entry(
            self.service.as_ref(),
            &household_id,
            &recorded,
            &writer,
            &marker,
            request_id,
        )
        .await?;

        Ok((household_id, recorded))
    }

    /// Correct a past weight entry, sharing the change with every member (WT-5, WT-7; D1.13).
    ///
    /// The steps are the create path's, with the entry's own resolution added:
    ///
    /// 1. The target is this device's live row for `entry_id` ([`AppState::live_entry`]), resolved
    ///    with **the household that owns the entry row**. A blank id, an entry this device holds no
    ///    row for, a **tombstoned** one (permanently deleted; D1.17), an item that is not an entry and
    ///    a row whose payload names a different entry id (an item is addressed by the id the
    ///    collection holds, never by the id re-derived from its payload; D1.16, SYNC-1) are all
    ///    refused as `validation`, and nothing is sent. A live row whose content is not on this
    ///    device refuses with the unknown-content key, because a whole-item write cannot be composed
    ///    without it (D1.16).
    /// 2. The entry's own `rabbit_id` — never a rabbit the command has to guess — is passed to
    ///    [`AppState::writable_rabbit`], so an **archived** rabbit refuses the edit as `validation`
    ///    before anything is composed (DM-4, RAB-8; D1.5), an unknown-content rabbit row refuses with
    ///    its own key, and a non-rabbit row is refused too. The rabbit's household must be the
    ///    **entry's**: a copy whose two resolutions disagree is refused as `validation` rather than
    ///    writing the entry into a collection that does not hold it (NFR-10).
    /// 3. The revision is recorded next, so the refusals the input itself produces — a value of zero
    ///    or less, an unconfirmed value above the plausibility bound (WT-4), a blank operation identity
    ///    and an unparseable timestamp — are reported as `validation` even when no account exists.
    ///    The whole editable set is replaced: value, note and timestamp, with `timestamp: None`
    ///    keeping the stored instant so an edit never restamps a back-dated entry. The identity, the
    ///    rabbit and the photo references come from the target, and `has_weight_drop_alert` is written
    ///    `false` because it is derived on read (WT-5, D1.13, D1.16).
    /// 4. No account is `no_account` before any probe, and the **writing member** is resolved from the
    ///    person's household book exactly as a rabbit write resolves it (ADR 0002 criterion 3).
    /// 5. The version marker is the one minted for the operation and the write is one `save_item` of
    ///    the entry's existing item — the id its row is keyed by — through [`crate::sync::SaveGate`],
    ///    so a retry re-sends a byte-identical body and the edit is shared with every member because
    ///    there is one item, not one per member (D0.14, HS-8).
    ///
    /// **The alerts around the entry are not rewritten here.** The edit changes the entry set, and the
    /// drop alert is a derivation over that set on every read (D1.13): the following entry's flag is
    /// re-derived by the trend/detail read (tasks 2.13–2.15), never repaired by a write. This method
    /// touches exactly one row, so no neighbouring stored entry is changed.
    ///
    /// The returned household is the one whose collection holds the entry. The command's answer is the
    /// entry, but the adapter needs the household to scope `weight:changed` — and the entry itself
    /// carries only its rabbit, so it cannot name the household on its own.
    ///
    /// # Errors
    ///
    /// `validation` for a blank entry id, an entry this device holds no live row for (absent or
    /// tombstoned), an item that is not an entry, a row whose payload names another entry, an
    /// **archived** rabbit (DM-4, RAB-8; D1.5), a rabbit claimed by another household, a value of
    /// zero or less, an unconfirmed value above the plausibility bound (WT-4), a blank operation
    /// identity, an unparseable timestamp, and a household with no member for this account; `unknown`
    /// with the unknown-content key when the entry's row or its rabbit's row has no content this
    /// device can describe (D1.16); `no_account` when no account is available; and otherwise the
    /// gate's mapped causes (`connectivity`, `no_account`, `allowance_exceeded`, `rate_limited`,
    /// `timeout`, `unknown`). A refusal never edits an entry, and one that is a property of the local
    /// copy never reaches the shared service at all.
    pub async fn update_weight_entry(
        &self,
        entry_id: WeightEntryId,
        entry: WeightEntryDraft,
        request_id: RequestId,
    ) -> Result<(HouseholdId, WeightEntry), CommandError> {
        if request_id.0.trim().is_empty() {
            return Err(CommandError::validation(Some(request_id)));
        }

        let (household_id, stored) = self.live_entry(&entry_id, Some(&request_id))?;
        let (rabbit_household_id, _rabbit) =
            self.writable_rabbit(&stored.rabbit_id, &request_id)?;

        if rabbit_household_id != household_id {
            // The entry and its rabbit are claimed by different collections, so neither household is
            // the one this write can address (NFR-10).
            return Err(CommandError::validation(Some(request_id)));
        }

        let revised = domain::weight::revise(&stored, entry, &request_id)?;

        if self.account.is_none() {
            return Err(CommandError::no_account(Some(request_id)));
        }

        let Some(writer) = self.writing_member(&household_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };
        let marker = self.operation_marker(&request_id);

        domain::weight::save_weight_entry(
            self.service.as_ref(),
            &household_id,
            &revised,
            &writer,
            &marker,
            request_id,
        )
        .await?;

        Ok((household_id, revised))
    }

    /// Delete a past weight entry, sharing the removal with every member (WT-5, WT-8; D1.13, D1.17).
    ///
    /// The steps are the edit's, with the removal written instead of a revised version:
    ///
    /// 1. The target is resolved exactly as an edit resolves it ([`AppState::live_entry`]): a blank
    ///    id, an absent or tombstoned entry, a non-entry item and a row whose payload names a different
    ///    entry are `validation`, unknown content is the unknown-content refusal, and none of them
    ///    sends anything (D1.16, D1.17). The resolution also carries the household that owns the
    ///    **entry row**, which is the collection the tombstone is written into.
    /// 2. The entry's own `rabbit_id` is passed to [`AppState::writable_rabbit`], so an **archived**
    ///    rabbit refuses the deletion as `validation` before anything is composed (DM-4, RAB-8;
    ///    D1.5). A rabbit claimed by another household is refused too: the entry and its rabbit must
    ///    belong to the same collection (NFR-10).
    /// 3. No account is `no_account` before any probe, and the writing member is resolved from the
    ///    person's household book (ADR 0002 criterion 3).
    /// 4. The operation's fresh version marker is written with the **tombstone**: identity and version
    ///    are kept, the entry's content is retained on the version that removes it, and `deleted` is
    ///    the write's own top-level argument (D1.16, D1.17). The write addresses the item id the row
    ///    is keyed by and goes into the household that holds it, so every member resolves the entry as
    ///    removed (HS-8). A removal needs no derived ledger key: one command writes one item, so the
    ///    operation's `request_id` is the only key (D1.17).
    ///
    /// **The following entry's alert is not rewritten here.** Deleting the entry changes the entry set
    /// the derivation reads, so the entry that now follows is compared against the entry that now
    /// precedes it by the read (tasks 2.13–2.15) — a recomputation is a re-read, never a repair pass
    /// over stored rows (D1.13). This method touches exactly one row.
    ///
    /// The returned household is the one whose collection lost the entry, with the rabbit the entry
    /// belonged to. The command's answer is `()` as the contract fixes, but the adapter needs both to
    /// scope `weight:changed`.
    ///
    /// # Errors
    ///
    /// `validation` for a blank entry id, an entry this device holds no live row for (absent or
    /// tombstoned), an item that is not an entry, a row whose payload names another entry, an
    /// **archived** rabbit (DM-4, RAB-8; D1.5), a rabbit claimed by another household, a blank
    /// operation identity, and a household with no member for this account; `unknown` with the
    /// unknown-content key when the entry's row or its rabbit's row has no content this device can
    /// describe, and `unknown` when the service accepted the call but the removal did not apply
    /// (D1.16, D1.17); `no_account` when no account is available; and otherwise the gate's mapped
    /// causes (`connectivity`, `no_account`, `allowance_exceeded`, `rate_limited`, `timeout`,
    /// `unknown`). A refusal never removes an entry, and one that is a property of the local copy
    /// never reaches the shared service at all.
    pub async fn delete_weight_entry(
        &self,
        entry_id: WeightEntryId,
        request_id: RequestId,
    ) -> Result<(HouseholdId, RabbitId), CommandError> {
        if request_id.0.trim().is_empty() {
            return Err(CommandError::validation(Some(request_id)));
        }

        let (household_id, stored) = self.live_entry(&entry_id, Some(&request_id))?;
        let rabbit_id = stored.rabbit_id.clone();
        let (rabbit_household_id, _rabbit) = self.writable_rabbit(&rabbit_id, &request_id)?;

        if rabbit_household_id != household_id {
            // The entry and its rabbit are claimed by different collections, so neither household is
            // the one this removal can address (NFR-10).
            return Err(CommandError::validation(Some(request_id)));
        }

        if self.account.is_none() {
            return Err(CommandError::no_account(Some(request_id)));
        }

        let Some(writer) = self.writing_member(&household_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };
        let marker = self.operation_marker(&request_id);
        let payload = ItemPayload::WeightEntry {
            weight_entry: WeightEntry {
                // Derived on read, never content: the removing version writes it `false` (D1.16).
                has_weight_drop_alert: false,
                ..stored
            },
        };

        domain::rabbit::save_tombstone(
            self.service.as_ref(),
            &household_id,
            &domain::weight::item_id(&entry_id),
            &payload,
            &writer,
            &marker,
            request_id,
        )
        .await?;

        Ok((household_id, rabbit_id))
    }

    /// One rabbit's profile and archived state, resolved from **this device's copy** (DASH-6, RAB-1's
    /// read side, RAB-5).
    ///
    /// The read is a local one, like [`list_rabbits`](super::ipc::commands::rabbit::list_rabbits): the
    /// copy is what the profile surface renders from, while the collection read owns the refresh
    /// (SYNC-3). It never probes the service, so unreachability is a status fact rather than a read
    /// failure.
    ///
    /// The resolution is [`AppState::live_rabbit`]'s — the same rule an edit, a lifecycle write and
    /// the weight reads use — so the classification cannot differ between reading and writing: a blank
    /// or unknown id, a tombstone and an id that names a non-rabbit item are `validation`; a live row
    /// whose content this device cannot describe is `unknown` with the unknown-content key (D1.16,
    /// D1.17). An **archived** rabbit is readable, as it is for [`AppState::weight_entry`]: RAB-8
    /// presents its timeline read-only and the frontend withholds the write controls from what this
    /// read returns (D1.5).
    ///
    /// # Errors
    ///
    /// `validation` for a blank or unknown rabbit id, a tombstoned rabbit, and an id that names a
    /// non-rabbit item; `unknown` with the unknown-content key when the row's content is not on this
    /// device (D1.16). A read carries no operation identity, so every failure answers
    /// `request_id: None`.
    pub fn rabbit(&self, rabbit_id: &RabbitId) -> Result<Rabbit, CommandError> {
        let (_household_id, rabbit) = self.live_rabbit(rabbit_id, None)?;

        Ok(rabbit)
    }

    /// One entry's detail: the stored entry with its alert re-derived from the rabbit's full entry
    /// set (WT-8; D1.13, D1.16, D1.17).
    ///
    /// The entry is resolved from **this device's copy** through [`AppState::live_entry`], so a blank
    /// or unknown id, a tombstoned row and a non-entry item are refused exactly as the edit path
    /// refuses them. The derived flag is [`crate::domain::weight::detail`]'s:
    /// [`crate::domain::weight::alerts`] over the rabbit's **full** entry set, never the stored
    /// payload's value and never a range window (I2, I3). Every other field — the grams, the
    /// timestamp, the note whole and the photo references — is the stored entry unchanged, and no
    /// display unit is applied here (I1).
    ///
    /// An archived rabbit is readable: RAB-8 presents its timeline read-only, so the read does not
    /// refuse an archived target and the frontend withholds Edit and Delete from what it returns.
    ///
    /// # Errors
    ///
    /// `validation` for a blank or unknown entry id, a tombstoned entry, an id that names a non-entry
    /// item and an entry whose rabbit row the collection does not hold; `unknown` with the
    /// unknown-content key when the entry's row, its rabbit's row or a live row of the collection has
    /// no content this device can describe (D1.16); and `unknown` when a stored timestamp or creation
    /// instant is not an absolute instant. A read carries no operation identity, so every failure
    /// answers `request_id: None`.
    pub fn weight_entry(&self, entry_id: &WeightEntryId) -> Result<WeightEntry, CommandError> {
        let (household_id, entry) = self.live_entry(entry_id, None)?;
        let collection = crate::sync::ReadPath::shared().serve_local(self, &household_id);

        domain::weight::detail(&collection, &entry)
    }

    /// One rabbit's weight entries inside a range window, with its threshold alongside (WT-6, WT-7;
    /// D1.4, D1.16, D1.17).
    ///
    /// The read is resolved against **this device's copy** and never refreshes: like
    /// [`list_rabbits`](super::ipc::commands::rabbit::list_rabbits), the copy is what the frontend
    /// renders from, while the collection read owns the refresh (SYNC-3). An archived rabbit is
    /// readable — RAB-8/DASH-7 present its timeline read-only — so archived is not a refusal.
    ///
    /// The window comes from `now`, which the caller reads at the edge. It is a pure read-side filter
    /// over the collection: nothing is stored per range and no state is shared between calls, so the
    /// trend and the weight-only history can each ask for their own range over the same rabbit (WT-6,
    /// D1.4).
    ///
    /// **A tombstone never makes the read refuse.** A permanently deleted rabbit is `validation`
    /// (unknown rabbit), and a tombstoned entry is excluded before its content is considered
    /// (D1.17). A **live** row whose content is not on this device refuses with the unknown-content
    /// key, because the rabbit's threshold or the entry set could be incomplete (D1.16).
    ///
    /// # Errors
    ///
    /// `validation` for a blank or unknown rabbit id and for an id that names a non-rabbit item;
    /// `unknown` with the unknown-content key when the rabbit's row or a live row of its collection has
    /// no content this device can describe (D1.16); and `unknown` when a stored timestamp or creation
    /// instant is not an absolute instant. A read carries no operation identity, so every failure
    /// answers `request_id: None`.
    pub fn weight_trend(
        &self,
        rabbit_id: &RabbitId,
        range: WeightRange,
        now: &Instant,
    ) -> Result<WeightTrend, CommandError> {
        // Archived is readable: RAB-8 presents an archived rabbit's timeline read-only (DASH-7).
        let (household_id, rabbit) = self.live_rabbit(rabbit_id, None)?;
        let collection = crate::sync::ReadPath::shared().serve_local(self, &household_id);

        domain::weight::trend(&collection, &rabbit, range, now)
    }

    /// Archive a rabbit: a reversible soft delete that discards nothing (RAB-6, RAB-9; D1.16).
    ///
    /// The write is the **whole item**. This device's copy of the household collection supplies the
    /// rabbit's content, which is written back unchanged except for `is_archived`; a payload carrying
    /// only the changed field would replace the stored version and discard the profile and history
    /// (RAB-9). The entries are separate items and are never touched, so archiving removes nothing
    /// from the rabbit's history.
    ///
    /// The steps are the edit path's with the content supplied by the copy:
    ///
    /// 1. The row is resolved through the household that claimed it, and must be **live**: a
    ///    tombstoned rabbit is permanently deleted and is never resurrected by a lifecycle write
    ///    (RAB-10, NFR-10).
    /// 2. A live row whose content is **unknown** refuses with `unknown` and sends nothing. Writing
    ///    now would submit a partial payload; the next successful refresh fills the content and the
    ///    operation can then be retried (D1.16).
    /// 3. An already-archived rabbit is `validation`: archiving twice is refused rather than absorbed.
    /// 4. No account is `no_account` before any probe, and the writing member is resolved from the
    ///    person's household book exactly as a create or an edit resolves it (ADR 0002 criterion 3).
    /// 5. The version marker is the one minted for the operation and the write goes through
    ///    [`crate::sync::SaveGate`], so the archived state reaches every member as one item write and
    ///    a refusal is never reported as archived (D1.1, SYNC-2, HS-8).
    ///
    /// The returned rabbit is the archived version. **This device's copy is deliberately not written
    /// here**: it belongs to the read path (SYNC-3), so the archive reaches this device the same way
    /// it reaches every other member — by the read that follows `rabbit:changed`.
    ///
    /// Because the content comes from this device's copy, the write is item-level last-write-wins
    /// (D0.7 criterion 3): a profile edit with an older marker, or a field written after this copy was
    /// last refreshed, is replaced by the archived whole item — the model D1.16 mandates for every
    /// whole-item write, so it is accepted here rather than repaired with a pre-write refresh, which
    /// is the read path's and task 2.8's decision.
    ///
    /// # Errors
    ///
    /// `validation` for a blank operation identity, a blank rabbit id, a rabbit this device holds no
    /// live row for, one whose row is not a rabbit, and one that is already archived; `unknown` when
    /// the row's content is not on this device; `no_account` when no account is available; and
    /// otherwise the gate's mapped causes (`connectivity`, `no_account`, `allowance_exceeded`,
    /// `rate_limited`, `timeout`, `unknown`). A refusal never archives anything, and one that is a
    /// property of the local copy never reaches the shared service at all.
    pub async fn archive_rabbit(
        &self,
        rabbit_id: RabbitId,
        request_id: RequestId,
    ) -> Result<Rabbit, CommandError> {
        if request_id.0.trim().is_empty() {
            return Err(CommandError::validation(Some(request_id)));
        }

        let Some((household_id, stored)) = self.stored_rabbit(&rabbit_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };

        let Some(payload) = stored.payload else {
            // The row exists but its content is not on this device: a whole-item write cannot be
            // composed, and a partial one would replace the stored version (D1.16, RAB-9).
            return Err(CommandError::household_content_unloaded(Some(request_id)));
        };
        let ItemPayload::Rabbit { rabbit } = payload else {
            // The identity names an item that is not a rabbit, so there is nothing to archive.
            return Err(CommandError::validation(Some(request_id)));
        };

        if rabbit.is_archived {
            return Err(CommandError::validation(Some(request_id)));
        }

        if self.account.is_none() {
            return Err(CommandError::no_account(Some(request_id)));
        }

        let Some(writer) = self.writing_member(&household_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };
        let marker = self.operation_marker(&request_id);
        let archived = domain::rabbit::mark_archived(rabbit);

        domain::rabbit::save_rabbit_version(
            self.service.as_ref(),
            &archived,
            &writer,
            &marker,
            request_id,
        )
        .await?;

        Ok(archived)
    }

    /// Return an archived rabbit to the active list, leaving its history untouched (RAB-7).
    ///
    /// The mirror of [`AppState::archive_rabbit`], and the write is the **whole item** for the same
    /// reason: this device's copy supplies the rabbit's content, which is written back unchanged
    /// except for `is_archived`, because a payload carrying only the changed field would replace the
    /// stored version and discard the profile (RAB-9, D1.16). The weight entries are separate items
    /// this write never names, so a restored rabbit returns with its history unchanged and complete
    /// (RAB-7).
    ///
    /// The steps are the archive's, with the flag reversed:
    ///
    /// 1. The row is resolved through the household that claimed it, and must be **live**: a
    ///    tombstoned rabbit is permanently deleted and is never resurrected by a lifecycle write
    ///    (RAB-10, NFR-10).
    /// 2. A live row whose content is **unknown** refuses with `unknown` and sends nothing. Writing
    ///    now would submit a partial payload; the next successful refresh fills the content and the
    ///    operation can then be retried (D1.16).
    /// 3. An already-active rabbit is `validation`: restoring what was never archived is refused
    ///    rather than absorbed.
    /// 4. No account is `no_account` before any probe, and the writing member is resolved from the
    ///    person's household book exactly as an archive resolves it (ADR 0002 criterion 3).
    /// 5. The version marker is the one minted for the operation and the write goes through
    ///    [`crate::sync::SaveGate`], so the active state reaches every member as one item write and a
    ///    refusal is never reported as restored (D1.1, SYNC-2, HS-8).
    ///
    /// The returned rabbit is the restored version. **This device's copy is deliberately not written
    /// here**: it belongs to the read path (SYNC-3), so the restore reaches this device the same way
    /// it reaches every other member — by the read that follows `rabbit:changed`.
    ///
    /// Because the content comes from this device's copy, the write is item-level last-write-wins
    /// (D0.7 criterion 3), exactly as [`crate::domain::rabbit::save_rabbit_version`] records for
    /// every whole-item write: a field changed after this copy was last refreshed is replaced by the
    /// restored whole item, accepted at this boundary rather than repaired with a pre-write refresh.
    ///
    /// # Errors
    ///
    /// `validation` for a blank operation identity, a blank rabbit id, a rabbit this device holds no
    /// live row for, one whose row is not a rabbit, and one that is not archived; `unknown` when the
    /// row's content is not on this device; `no_account` when no account is available; and otherwise
    /// the gate's mapped causes (`connectivity`, `no_account`, `allowance_exceeded`, `rate_limited`,
    /// `timeout`, `unknown`). A refusal never restores anything, and one that is a property of the
    /// local copy never reaches the shared service at all.
    pub async fn restore_rabbit(
        &self,
        rabbit_id: RabbitId,
        request_id: RequestId,
    ) -> Result<Rabbit, CommandError> {
        if request_id.0.trim().is_empty() {
            return Err(CommandError::validation(Some(request_id)));
        }

        let Some((household_id, stored)) = self.stored_rabbit(&rabbit_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };

        let Some(payload) = stored.payload else {
            // The row exists but its content is not on this device: a whole-item write cannot be
            // composed, and a partial one would replace the stored version (D1.16, RAB-9).
            return Err(CommandError::household_content_unloaded(Some(request_id)));
        };
        let ItemPayload::Rabbit { rabbit } = payload else {
            // The identity names an item that is not a rabbit, so there is nothing to restore.
            return Err(CommandError::validation(Some(request_id)));
        };

        if !rabbit.is_archived {
            return Err(CommandError::validation(Some(request_id)));
        }

        if self.account.is_none() {
            return Err(CommandError::no_account(Some(request_id)));
        }

        let Some(writer) = self.writing_member(&household_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };
        let marker = self.operation_marker(&request_id);
        let restored = domain::rabbit::mark_restored(rabbit);

        domain::rabbit::save_rabbit_version(
            self.service.as_ref(),
            &restored,
            &writer,
            &marker,
            request_id,
        )
        .await?;

        Ok(restored)
    }

    /// Permanently delete an **archived** rabbit and every weight entry it owns (RAB-10).
    ///
    /// The deletion is the whole item written as a tombstone, for the rabbit and for each entry the
    /// domain's plan names, through the same reachability gate as every other rabbit write (D1.1,
    /// D1.16). The steps are the archive's:
    ///
    /// 1. The household is resolved from the row that claimed the item, and the item must still be
    ///    **live**: a tombstone names a rabbit that is already permanently deleted, so it is refused
    ///    rather than deleted twice (NFR-10).
    /// 2. This device's copy supplies the plan. [`crate::domain::rabbit::deletion_plan`] is the one
    ///    rule: the rabbit must be archived (RAB-10's narrowing, D1.8), every live row must be
    ///    describable (D1.16), and the cascade is the weight-entry rows that name it. All of that is
    ///    decided **before** the account check, exactly as the archive's refusals are, so an input the
    ///    copy makes impossible is reported as itself even when no account exists, and nothing is
    ///    probed or sent.
    /// 3. No account is `no_account` before any probe, and the writing member is resolved from the
    ///    person's household book exactly as an archive resolves it (ADR 0002 criterion 3).
    /// 4. The version marker is the one minted for the operation, so a retry re-sends byte-identical
    ///    tombstones, and each item write goes through [`crate::sync::SaveGate`].
    ///
    /// The returned household is the one whose collection lost the rows. The command's answer is `()`
    /// as the contract fixes, but the adapter needs the household to scope `rabbit:changed`, and
    /// returning the deleted rabbit itself would invite reading a value that no longer exists.
    ///
    /// **This device's copy is deliberately not written here**: it belongs to the read path (SYNC-3),
    /// so the removal reaches this device the same way it reaches every other member — by the read that
    /// follows `rabbit:changed`. One collection is what makes the deletion shared by construction: no
    /// per-member or device-local row is involved, so no member resolves the rabbit or its entries once
    /// the tombstones are in the household's collection (HS-8).
    ///
    /// # Errors
    ///
    /// `validation` for a blank operation identity, a blank rabbit id, a rabbit this device holds no
    /// live row for, an **active** rabbit (the deliberate narrowing of RAB-10, D1.8), and an id the
    /// derived ledger keys cannot keep distinct (D1.17); `unknown` with the unknown-content key when the
    /// rabbit's row or another live row of the collection has no content this device can describe;
    /// `no_account` when no account is available; and otherwise the gate's mapped causes
    /// (`connectivity`, `no_account`, `allowance_exceeded`, `rate_limited`, `timeout`, `unknown`),
    /// reported against the operation's identity. A refusal never removes anything, and a cascade that
    /// stops part-way leaves the rabbit archived for the retry to finish (D0.14).
    ///
    /// **A tombstone that did not apply is not retryable.** The service reports a losing
    /// last-write-wins write as a completed call with `applied: false` (D0.7 criterion 3); the domain
    /// stops the cascade before the rabbit on it and answers `unknown`, because the version marker is
    /// memoized for the operation and a retry would lose again (D1.17). A fresh read brings the newer
    /// version into this device's copy, and a new deletion operation then removes it.
    pub async fn delete_rabbit(
        &self,
        rabbit_id: RabbitId,
        request_id: RequestId,
    ) -> Result<HouseholdId, CommandError> {
        if request_id.0.trim().is_empty() {
            return Err(CommandError::validation(Some(request_id)));
        }

        let Some((household_id, _row)) = self.stored_rabbit(&rabbit_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };

        let Some(collection) = self.read_collection(&household_id, CollectionSource::Local) else {
            // The row was resolved through this same copy, so this is unreachable; refusing is still
            // the honest answer rather than inventing a collection to delete from.
            return Err(CommandError::validation(Some(request_id)));
        };

        let plan = domain::rabbit::deletion_plan(&collection, &rabbit_id, &request_id)?;

        if self.account.is_none() {
            return Err(CommandError::no_account(Some(request_id)));
        }

        let Some(writer) = self.writing_member(&household_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };
        let marker = self.operation_marker(&request_id);

        domain::rabbit::delete_rabbit(
            self.service.as_ref(),
            &household_id,
            &plan,
            &writer,
            &marker,
            request_id,
        )
        .await?;

        Ok(household_id)
    }

    /// The member id this person's account has in `household_id`, when this device holds the
    /// household and records them as one of its members.
    ///
    /// It is the writer a shared write names (ADR 0002 criterion 3). `None` means the book cannot
    /// attribute the write: the household is not held, or it holds no member for this account — both
    /// of which mean the person is not writing into their own household's data, so the caller refuses
    /// instead of naming somebody else's member id.
    fn writing_member(&self, household_id: &HouseholdId) -> Option<MemberId> {
        let account = self.account.as_ref()?;
        let households = lock(&self.households);

        households
            .get(household_id)
            .and_then(|record| local_member_id(record.members(), account))
            .cloned()
    }

    /// The **writable** rabbit `rabbit_id` names, with the household whose copy holds it (DM-4,
    /// RAB-8; D1.5).
    ///
    /// This is the one archived-target check at the write boundary, and it is the check every content
    /// write calls before composing anything. `update_rabbit` and `create_weight_entry` call it today.
    /// **The weight-entry edit and delete of task 2.11 must call it too**, with these targets:
    ///
    /// * `create_weight_entry` passes the `rabbit_id` the command names.
    /// * `update_weight_entry` and `delete_weight_entry` first resolve their entry from this device's
    ///   copy and pass the `rabbit_id` that entry's payload carries; an entry is never written before
    ///   its rabbit has passed this check.
    ///
    /// A lifecycle write does **not** call it: restore (2.6) and permanent deletion (2.7) target an
    /// archived rabbit by definition (RAB-7, RAB-10), and archive checks its own transition.
    ///
    /// The resolve-and-decode block is [`AppState::live_rabbit`]'s, so a read and a write classify the
    /// same row identically; this method adds only the archived-target rule a content write owes. The
    /// rules are all read from this device's copy, so a refusal sends nothing:
    ///
    /// * The claim alone is not enough. It survives a tombstone, so a permanently deleted rabbit still
    ///   names its household; the row must exist and be **live**, or a full-payload write would
    ///   resurrect what RAB-10 removed (D1.5, NFR-10).
    /// * The row's content must be a rabbit, so the identity names a rabbit a write may touch.
    /// * The rabbit must not be **archived**: a content write to it is refused (DM-4, RAB-8; D1.5).
    /// * A row whose content is **unknown** cannot establish that its rabbit is not archived, so it is
    ///   refused with the unknown-content key rather than written blind (D1.16). The next successful
    ///   refresh fills the content and the operation can be retried.
    ///
    /// # Errors
    ///
    /// `validation` for a blank rabbit id, an absent or tombstoned rabbit, an item that is not a
    /// rabbit, and an archived rabbit; `unknown` with the unknown-content key when the row's content is
    /// not on this device. All carry the operation's identity and none has sent anything.
    pub(crate) fn writable_rabbit(
        &self,
        rabbit_id: &RabbitId,
        request_id: &RequestId,
    ) -> Result<(HouseholdId, Rabbit), CommandError> {
        let (household_id, rabbit) = self.live_rabbit(rabbit_id, Some(request_id))?;

        // The resolve-and-decode block is [`AppState::live_rabbit`]'s; only the archived-target rule
        // belongs to a content write (DM-4, RAB-8; D1.5).
        domain::rabbit::ensure_writable(&rabbit, request_id)?;

        Ok((household_id, rabbit))
    }

    /// The **live** weight entry `entry_id` names, with the household that owns its row (WT-5, D1.16,
    /// D1.17; SYNC-1).
    ///
    /// The entry counterpart of [`AppState::stored_rabbit`], and the one resolution the edit, the
    /// deletion and the detail read use so the rules cannot differ between them. **The household
    /// returned is the one that claimed the entry's item id**, and it is the only collection the
    /// caller may write the entry into: the rabbit's household is checked against it by the caller,
    /// so an entry can never be edited or removed inside a collection that does not hold it (NFR-10).
    ///
    /// `request_id` is `Some` for a write and `None` for a read: a write carries the operation's
    /// identity on every refusal, while a read is idempotent and names none (D0.14). The resolution
    /// itself is identical, so the detail read (`get_weight_entry`) accepts and refuses exactly the
    /// rows an edit would.
    ///
    /// A tombstone is checked **before** the content, because a tombstone whose content was never read
    /// is stored as unknown content; it names an entry that is already removed, so it is refused as a
    /// **blank/unknown entry** (`validation`) rather than re-submitted (D1.17). A row whose content is
    /// not on this device refuses with the unknown-content key, because a whole-item write cannot be
    /// composed from it (D1.16), and a row that carries a rabbit is not an entry at all.
    ///
    /// **The payload's entry id must be the item id the row is keyed by.** A weight entry's item is
    /// the row the collection holds, never an id re-derived from its payload (the same rule
    /// [`crate::domain::rabbit::deletion_plan`] states for a removal); a row whose `WeightEntry.id`
    /// disagrees with the id it was looked up by would make the write address a different item from
    /// the one resolved. That is refused as `validation` here, so the resolved row's item id is the
    /// single write target for both the edit and the removal.
    ///
    /// # Errors
    ///
    /// `validation` for a blank id, an entry this device holds no live row for, a tombstoned entry, a
    /// non-entry item, and a payload naming another entry id; `unknown` with the unknown-content key
    /// when the live row's content is not on this device. All carry `request_id` and none sends
    /// anything.
    fn live_entry(
        &self,
        entry_id: &WeightEntryId,
        request_id: Option<&RequestId>,
    ) -> Result<(HouseholdId, WeightEntry), CommandError> {
        let refused = || CommandError::validation(request_id.cloned());

        if entry_id.0.trim().is_empty() {
            return Err(refused());
        }

        let item_id = domain::weight::item_id(entry_id);
        let copies = lock(&self.copies);
        let Some(household_id) = copies.household_of(&item_id).cloned() else {
            return Err(refused());
        };
        let Ok(Some(stored)) = copies.item(&household_id, &item_id) else {
            return Err(refused());
        };

        if stored.deleted {
            return Err(refused());
        }

        match &stored.payload {
            Some(ItemPayload::WeightEntry { weight_entry }) if weight_entry.id == *entry_id => {
                Ok((household_id, weight_entry.clone()))
            }
            // The payload names another item than the row it is stored under, so it cannot be the
            // content of the row `entry_id` resolved (D1.16, SYNC-1).
            Some(ItemPayload::WeightEntry { .. }) => Err(refused()),
            // The row exists but its content is not on this device: a whole-item write cannot be
            // composed, and a partial one would replace the stored version (D1.16).
            None => Err(CommandError::household_content_unloaded(
                request_id.cloned(),
            )),
            // The identity names an item that is not an entry, so there is nothing to edit or delete.
            Some(ItemPayload::Rabbit { .. }) => Err(refused()),
        }
    }

    /// The **live** rabbit `rabbit_id` names, with the household whose copy holds it (D1.16, D1.17).
    ///
    /// The one resolve-and-decode rule the profile read ([`AppState::rabbit`]), the weight
    /// trend/history read ([`AppState::weight_trend`]) and the content-write boundary
    /// ([`AppState::writable_rabbit`]) share, so a blank id, an absent or tombstoned rabbit, a
    /// non-rabbit item and a row whose content is not on this device cannot be classified one way by
    /// a read and another by a write.
    ///
    /// `request_id` is `Some` for a write and `None` for a read: a write carries the operation's
    /// identity on every refusal, while a read is idempotent and names none (D0.14).
    ///
    /// # Errors
    ///
    /// `validation` for a blank or unknown id, a tombstoned rabbit and an id that names a non-rabbit
    /// item; `unknown` with the unknown-content key when the live row's content is not on this device
    /// (D1.16). A write applies [`crate::domain::rabbit::ensure_writable`] to the returned rabbit
    /// itself, because an archived rabbit is readable but not writable (D1.5).
    fn live_rabbit(
        &self,
        rabbit_id: &RabbitId,
        request_id: Option<&RequestId>,
    ) -> Result<(HouseholdId, Rabbit), CommandError> {
        if rabbit_id.0.trim().is_empty() {
            return Err(CommandError::validation(request_id.cloned()));
        }

        let Some((household_id, stored)) = self.stored_rabbit(rabbit_id) else {
            return Err(CommandError::validation(request_id.cloned()));
        };

        match stored.payload {
            Some(ItemPayload::Rabbit { rabbit }) => Ok((household_id, rabbit)),
            // The row exists but its content is not on this device: a whole-item write cannot be
            // composed from it and a read cannot answer it (D1.16).
            None => Err(CommandError::household_content_unloaded(
                request_id.cloned(),
            )),
            // The identity names an item that is not a rabbit, so it names no profile and no series.
            Some(ItemPayload::WeightEntry { .. }) => {
                Err(CommandError::validation(request_id.cloned()))
            }
        }
    }

    /// The **live** row this device holds for `rabbit_id`, with the household that claimed it.
    ///
    /// The row lookup behind [`AppState::live_rabbit`], which is the one resolution an edit, a
    /// lifecycle write and a read use. A tombstone keeps its identity claim, but it names a permanently
    /// deleted rabbit, so `None` is answered for it as well as for a blank id and a rabbit this device
    /// has never loaded — none of which names a rabbit that may be read or written (D1.5, NFR-10).
    fn stored_rabbit(&self, rabbit_id: &RabbitId) -> Option<(HouseholdId, StoredItem)> {
        let item_id = domain::rabbit::item_id(rabbit_id);
        let copies = lock(&self.copies);
        let household_id = copies.household_of(&item_id)?.clone();
        let item = copies.item(&household_id, &item_id).ok()??;

        if item.deleted {
            None
        } else {
            Some((household_id, item.clone()))
        }
    }

    /// The version marker for `request_id`: the one already minted for that operation, or a fresh one
    /// recorded now.
    ///
    /// The marker is the client-supplied `updated_at` last-write-wins compares (D0.7 criterion 3), and
    /// it is **minted once per operation** so a retry re-sends a byte-identical body: the service
    /// deduplicates on `request_id` and refuses the same key with a different payload (D0.7 criterion
    /// 6, D0.14). Re-reading the clock on every attempt would do exactly that, which is why the value
    /// is remembered here rather than taken at each call from the storage boundary's clock —
    /// [`AbsoluteInstant::now`] in `storage::instant`, the version-marker side of P0's task 2.8 (not
    /// this change's 2.8, which is archived-rabbit immutability).
    fn operation_marker(&self, request_id: &RequestId) -> Instant {
        self.operation_memo(request_id).marker
    }

    /// The operation's memo, minting a fresh version marker when this is its first write.
    ///
    /// Both values a weight-entry create mints are recorded here, because they are the operation's
    /// own and a retry must re-send them (D0.14): the marker below, and the entry's creation instant
    /// in [`AppState::entry_created_at`].
    fn operation_memo(&self, request_id: &RequestId) -> OperationMemo {
        let mut operations = lock(&self.operations);

        operations
            .entry(request_id.clone())
            .or_insert_with(|| OperationMemo {
                marker: AbsoluteInstant::now().to_wire(),
                entry_created_at: None,
            })
            .clone()
    }

    /// The creation instant for the weight entry `request_id` creates: the one already minted for the
    /// operation, or a fresh one recorded now (WT-1, WT-7, D1.12; D0.14).
    ///
    /// The entry's `created_at` is always this value, so every create calls this method — including
    /// one that supplied its own timestamp, where `created_at` and `timestamp` deliberately differ.
    /// When the draft left the timestamp alone the two coincide, because the defaulted timestamp is
    /// this same instant. It is minted once per operation for the same reason
    /// [`AppState::operation_marker`] is: a retry re-issues the same [`RequestId`], and the service
    /// refuses the same key with a different payload, so both recorded instants have to be the first
    /// attempt's on every later one.
    ///
    /// It shares the operation's memo with the version marker rather than a parallel map: same key,
    /// same mint, same process lifetime.
    fn entry_created_at(&self, request_id: &RequestId) -> Instant {
        let mut operations = lock(&self.operations);
        let memo = operations
            .entry(request_id.clone())
            .or_insert_with(|| OperationMemo {
                marker: AbsoluteInstant::now().to_wire(),
                entry_created_at: None,
            });

        memo.entry_created_at
            .get_or_insert_with(|| AbsoluteInstant::now().to_wire())
            .clone()
    }

    /// Every household the person holds, with the active one marked (HS-3, HS-4, DM-3).
    ///
    /// The read is served from the process's household book and makes no service request, so it has
    /// no failure of its own and an empty list is `Ok(vec![])`. It is still **not** proof that the
    /// person holds no household: only a fresh reachable status may turn it into that conclusion
    /// (SYNC-3, D0.2), which is the rule the command's doc comment carries to the frontend.
    pub fn list_households(&self) -> Vec<HouseholdSummary> {
        lock(&self.households).summaries(self.account.as_ref())
    }

    /// The process's active household, or `None` when none is held (HS-2, HS-3, DM-3).
    ///
    /// Served locally, like [`AppState::list_households`]; `None` is `Ok(None)` and is a conclusion
    /// only beside a fresh reachable status (SYNC-3). At most one household is active because the
    /// book holds one active id (DM-3).
    pub fn active_household(&self) -> Option<Household> {
        let households = lock(&self.households);

        households
            .active()
            .map(|record| record.to_wire_for(self.account.as_ref(), true))
    }

    /// Make a held household the active one and return the post-state (HS-3, HS-4, DM-3).
    ///
    /// This is a **local** state change: it assigns the book's one active id and never reaches the
    /// shared service, because the active household is a selection over the held set rather than an
    /// item of a household collection (the reasoning is in [`crate::domain::household`]). It
    /// therefore takes no probe and needs no account, and it is not a second write path.
    ///
    /// The returned household is read back from the book after the switch, so `is_active` is the
    /// post-state rather than an echo of the argument. A refused switch leaves the previous active
    /// household in place.
    ///
    /// # Errors
    ///
    /// `validation` when the person does not hold the household. That is the only condition this
    /// operation can observe: because nothing leaves the device, the service's causes
    /// (`connectivity`, `no_account`, `allowance_exceeded`, `rate_limited`, `timeout`) are
    /// unreachable here.
    pub fn set_active_household(
        &self,
        household_id: HouseholdId,
        request_id: RequestId,
    ) -> Result<Household, CommandError> {
        let record = {
            let mut households = lock(&self.households);

            households
                .set_active(&household_id)
                .map_err(|_| CommandError::validation(Some(request_id)))?
                .clone()
        };

        // The active id changed: schedule the best-effort rewrite after the guard is dropped.
        self.schedule_persist();

        Ok(record.to_wire_for(self.account.as_ref(), true))
    }
}

/// The token source P0 can offer: none.
///
/// The refresh token's storage is the platform Keychain, owned by a sign-in flow this change does
/// not implement (D0.7 criterion 4, Q25). Returning `None` is what makes the real client report the
/// absence of an account as itself instead of sending an unauthenticated request.
fn no_account_token() -> AccessTokenSource {
    Arc::new(|| None)
}

/// The write side of persistence, abstracted so a test can hold one write while a newer snapshot
/// arrives (the coalescing test).
trait SnapshotSink: Send + Sync {
    /// Persist one snapshot. A failure is the caller's to ignore; the writer never reports it.
    fn save(&self, snapshot: &DeviceSnapshot) -> Result<(), CollectionStoreError>;
}

impl SnapshotSink for CollectionStore {
    fn save(&self, snapshot: &DeviceSnapshot) -> Result<(), CollectionStoreError> {
        CollectionStore::save(self, snapshot)
    }
}

/// The latest snapshot waiting to be written, and whether one write is already running.
#[derive(Debug, Default)]
struct WriterState {
    /// The newest snapshot not yet picked up. A later schedule replaces it, so only the latest is
    /// ever written after the current in-flight snapshot.
    pending: Option<DeviceSnapshot>,
    /// Whether the writer's one draining task is running.
    in_flight: bool,
}

/// A coalescing persistence writer: one in-flight write and the latest pending snapshot.
///
/// The ordering rule is the point: a schedule while a write is in flight replaces the pending
/// snapshot rather than starting a second write. The drain loop takes the newest pending snapshot
/// after each write finishes, so an older (possibly larger, possibly slower) snapshot can never
/// finish last and overwrite a newer one.
struct PersistenceWriter {
    /// The destination the snapshots are written to.
    sink: Arc<dyn SnapshotSink>,
    /// The pending/in-flight state.
    state: Mutex<WriterState>,
}

impl PersistenceWriter {
    /// A writer over one sink.
    fn new(sink: Arc<dyn SnapshotSink>) -> Self {
        Self {
            sink,
            state: Mutex::new(WriterState::default()),
        }
    }

    /// Offer `snapshot` to the writer, starting the one drain task if none is running.
    fn schedule(self: &Arc<Self>, snapshot: DeviceSnapshot) {
        let start = {
            let mut state = lock(&self.state);

            // Always replace: the newest snapshot is the one a relaunch must see.
            state.pending = Some(snapshot);

            if state.in_flight {
                false
            } else {
                state.in_flight = true;
                true
            }
        };

        if start {
            self.spawn();
        }
    }

    /// Start the drain task on the blocking pool (a test thread under `cfg(test)`).
    fn spawn(self: &Arc<Self>) {
        let writer = Arc::clone(self);

        #[cfg(not(test))]
        {
            let _handle = tauri::async_runtime::spawn_blocking(move || writer.drain());
        }

        #[cfg(test)]
        {
            let _handle = std::thread::spawn(move || writer.drain());
        }
    }

    /// Write the newest pending snapshot until none remains, then stop being in flight.
    fn drain(&self) {
        loop {
            let snapshot = {
                let mut state = lock(&self.state);

                match state.pending.take() {
                    Some(snapshot) => snapshot,
                    None => {
                        state.in_flight = false;

                        return;
                    }
                }
            };

            // Best effort: a failed write is not retried forever and never fails the read. A newer
            // snapshot already pending will still be written next.
            let _persisted = self.sink.save(&snapshot);
        }
    }
}

/// The guarded store restored from `collections`, ignoring refused entries.
///
/// Every persisted copy goes back through [`GuardedStore::load_wire_collection`], so the same
/// household-isolation and marker rules apply as to a network load — including the restored
/// ownership claim for each item. A refused copy is skipped rather than allowed to fail the launch;
/// the rest still restore.
fn restored_copies(collections: &[HouseholdCollection]) -> GuardedStore {
    let mut copies = GuardedStore::new();

    for collection in collections {
        let Some(loaded_at) = collection.loaded_at.clone() else {
            continue;
        };
        let household_id = collection.household_id.clone();
        let _loaded = copies.load_wire_collection(&household_id, collection.clone(), loaded_at);
    }

    copies
}

/// The household book restored from the persisted rows through the domain's restore constructors.
///
/// A row that [`HouseholdRecord::restore`] refuses is skipped: the book never presents a household
/// whose owner/member invariants cannot be met. The persisted active id is passed through, and
/// [`HouseholdBook::restore`] re-elects a held household when it names none that is held.
fn restored_book(snapshot: &DeviceSnapshot) -> HouseholdBook {
    let records = snapshot.households.iter().filter_map(|household| {
        HouseholdRecord::restore(
            household.id.clone(),
            household.name.clone(),
            household.created_at.clone(),
            household.owner_member_id.clone(),
            household.members.clone(),
        )
    });

    HouseholdBook::restore(records, snapshot.active_household.clone())
}

/// One domain record as the stored row.
fn persisted_household(record: &HouseholdRecord) -> PersistedHousehold {
    PersistedHousehold {
        id: record.id().clone(),
        name: record.name().as_str().to_owned(),
        created_at: record.created_at().clone(),
        owner_member_id: record.owner_member_id().clone(),
        members: record.members().to_vec(),
    }
}

/// Lock a mutex, recovering a poisoned one rather than cascading the panic.
///
/// The critical sections here only move small values between fields and read them back; none can
/// leave the protected value inconsistent, so a panic elsewhere should not make this state
/// permanently unusable. The alternative — `expect` on every lock — would turn an unrelated panic
/// into a second failure in whichever command happens to run next.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    use serde_json::json;

    use super::*;
    use crate::ipc::types::{
        CollectionItem, Grams, ItemId, NeuterStatus, PLAUSIBILITY_BOUND_GRAMS, RabbitId,
        RabbitProfile, SaveCause, Sex, WeightEntry, WeightEntryId, WeightRange,
    };
    use crate::sync::double::ScriptedService;
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::{Answer, ServiceReply};

    fn account() -> AccountUserId {
        AccountUserId("account-1".to_owned())
    }

    /// A RAB-1 profile supplying every field, so a test states its one difference from a full one.
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

    fn request_id(value: &str) -> RequestId {
        RequestId(value.to_owned())
    }

    /// A weight-entry draft supplying only the value: no timestamp, no note, nothing confirmed.
    fn weight_entry_draft() -> WeightEntryDraft {
        WeightEntryDraft {
            value_grams: Grams(1_800),
            timestamp: None,
            note: None,
            outlier_confirmed: false,
        }
    }

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn wire(item_id: &str, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(item_id.to_owned()),
            updated_at: at(updated_at),
            deleted: false,
            payload: None,
        }
    }

    /// A removal, as a load delivers it: a tombstone row that keeps its identity and version.
    fn tombstone(item_id: &str, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(item_id.to_owned()),
            updated_at: at(updated_at),
            deleted: true,
            payload: None,
        }
    }

    fn wire_collection(
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

    /// Every write the transport was asked to submit, in call order.
    fn write_bodies(service: &ScriptedService) -> Vec<serde_json::Value> {
        service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                crate::sync::double::RecordedCall::Save { request, .. } => {
                    Some(request.body().clone())
                }
                _ => None,
            })
            .collect()
    }

    /// A rabbit content as this device's copy holds it, built through the create constructor so
    /// every RAB-1 field is present.
    fn stored_rabbit(id: &str, name: &str) -> Rabbit {
        RabbitRecord::created(
            household("household-1"),
            RabbitProfile {
                name: name.to_owned(),
                ..profile()
            },
            &request_id(id),
        )
        .expect("the profile is valid")
        .to_wire()
    }

    /// One row carrying the rabbit's content (D1.16).
    fn row_with_rabbit(id: &str, updated_at: &str, rabbit: &Rabbit) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(id.to_owned()),
            updated_at: at(updated_at),
            deleted: false,
            payload: Some(ItemPayload::Rabbit {
                rabbit: rabbit.clone(),
            }),
        }
    }

    /// One weight-entry row beside the rabbit's own row.
    fn row_with_weight_entry(id: &str, rabbit: &Rabbit) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(id.to_owned()),
            updated_at: at("2026-10-20T09:00:00Z"),
            deleted: false,
            payload: Some(ItemPayload::WeightEntry {
                weight_entry: WeightEntry {
                    id: WeightEntryId(id.to_owned()),
                    rabbit_id: rabbit.id.clone(),
                    value_grams: Grams(1_800),
                    timestamp: at("2026-10-20T09:00:00Z"),
                    created_at: at("2026-10-20T09:00:00Z"),
                    note: Some("morning".to_owned()),
                    photo_refs: Vec::new(),
                    has_weight_drop_alert: false,
                },
            }),
        }
    }

    /// One weight-entry row at a stated value and timestamp, so the tests can order several entries.
    fn row_with_entry(id: &str, rabbit: &Rabbit, value: i32, timestamp: &str) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(id.to_owned()),
            updated_at: at(timestamp),
            deleted: false,
            payload: Some(ItemPayload::WeightEntry {
                weight_entry: WeightEntry {
                    id: WeightEntryId(id.to_owned()),
                    rabbit_id: rabbit.id.clone(),
                    value_grams: Grams(value),
                    timestamp: at(timestamp),
                    created_at: at(timestamp),
                    note: Some("morning".to_owned()),
                    photo_refs: Vec::new(),
                    has_weight_drop_alert: false,
                },
            }),
        }
    }

    /// A state that holds `household-1`, one created rabbit, and the copy the read path delivers
    /// after that create — the state an edit starts from, since an edit is addressed by the rabbit
    /// alone and the copy is what names its household.
    fn state_with_one_rabbit() -> (Arc<ScriptedService>, AppState, Rabbit) {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        let rabbit =
            block_on_ready(state.create_rabbit(one.clone(), profile(), request_id("operation-2")))
                .expect("the rabbit is created");
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![row_with_rabbit(
                        &rabbit.id.0,
                        "2026-10-22T09:30:00Z",
                        &rabbit,
                    )],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        (service, state, rabbit)
    }

    /// One accepted create answer the domain can read.
    fn accepted_answer(household_id: &str) -> ServiceReply {
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

    /// 2.8's non-rabbit branch: an identity that names a **weight-entry** row is not a rabbit, so a
    /// weight-entry create that addresses it is refused as `validation` with nothing written — there is
    /// no rabbit whose archived state the write could be accepted under (D1.5, D1.16).
    #[test]
    fn a_create_on_a_row_that_is_a_weight_entry_is_refused_as_validation() {
        let (service, state, rabbit) = state_with_one_rabbit();
        let one = household("household-1");

        // The copy the read path would deliver for a household whose item `rabbit-1` is an entry: the
        // identity is known and live, but its content is not a rabbit.
        state
            .load_collection(
                &one,
                wire_collection(&one, vec![row_with_weight_entry("rabbit-1", &rabbit)]),
                at("2026-10-22T09:32:00Z"),
            )
            .expect("the collection is the household's own");
        let calls_before = service.calls();
        let request_id = request_id("operation-3");

        let error = block_on_ready(state.create_weight_entry(
            RabbitId("rabbit-1".to_owned()),
            weight_entry_draft(),
            request_id.clone(),
        ))
        .expect_err("the row is not a rabbit");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// D0.14, WT-1: a retried create re-sends the creation instant it minted the first time, so the two
    /// attempts submit byte-identical bodies and the service's ledger recognises one operation instead
    /// of refusing a changed payload.
    #[test]
    fn a_retried_create_reuses_the_creation_instant_it_minted() {
        let (service, state, rabbit) = state_with_one_rabbit();
        let operation = request_id("operation-3");

        let (_, first) = block_on_ready(state.create_weight_entry(
            rabbit.id.clone(),
            weight_entry_draft(),
            operation.clone(),
        ))
        .expect("the double accepted the write");
        let (_, retried) = block_on_ready(state.create_weight_entry(
            rabbit.id.clone(),
            weight_entry_draft(),
            operation,
        ))
        .expect("the retry is accepted too");

        assert_eq!(
            retried.timestamp, first.timestamp,
            "the defaulted instant is the operation's, not a fresh clock read"
        );
        assert_eq!(
            retried.created_at, first.created_at,
            "the recorded creation instant is re-sent unchanged by a retry (WT-7, D1.12)"
        );

        let bodies = write_bodies(&service);
        assert_eq!(
            bodies.len(),
            4,
            "the household create, the rabbit create and the two entry attempts"
        );
        assert_eq!(
            bodies[2], bodies[3],
            "a retry submits the bytes the first attempt submitted"
        );
    }

    /// WT-1, WT-7 through the state: the supplied moment is kept as the timestamp and the operation's
    /// own instant is recorded separately as `created_at` — `record` consults the memo unconditionally,
    /// because every create records a creation order even when the person chose the reading moment.
    #[test]
    fn a_supplied_timestamp_is_written_unchanged_and_never_defaulted() {
        let (service, state, rabbit) = state_with_one_rabbit();
        let supplied = at("2026-01-02T03:04:05Z");

        let (_, created) = block_on_ready(state.create_weight_entry(
            rabbit.id.clone(),
            WeightEntryDraft {
                timestamp: Some(supplied.clone()),
                ..weight_entry_draft()
            },
            request_id("operation-3"),
        ))
        .expect("the double accepted the write");

        assert_eq!(created.timestamp, supplied, "the past moment is kept");
        assert_ne!(
            created.created_at, supplied,
            "creation order is the operation's instant, not the supplied reading moment"
        );
        let body = write_bodies(&service)
            .pop()
            .expect("the entry write reached the transport");
        assert_eq!(
            body["payload"]["weight_entry"]["timestamp"],
            json!(supplied.0),
            "the stored payload carries the supplied moment"
        );
        assert_eq!(
            body["payload"]["weight_entry"]["created_at"],
            json!(created.created_at.0),
            "the stored payload carries the recorded creation instant beside it"
        );
    }

    #[test]
    fn without_an_account_a_create_is_refused_with_no_account_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let request_id = request_id("operation-1");

        let create = state.create_household("Home".to_owned(), request_id.clone());
        let error = block_on_ready(create).expect_err("P0 has no account");

        assert_eq!(error.cause, SaveCause::NoAccount);
        assert_eq!(error.message_key, "error.no_account");
        assert!(error.retryable);
        assert_eq!(error.request_id, Some(request_id));

        // The honest outcome is a refusal, not an unauthenticated attempt and not a local-only
        // household (Q25, SYNC-2).
        assert!(service.calls().is_empty());
        assert_eq!(service.write_count(), 0);
        assert!(state.households().is_empty());
    }

    #[test]
    fn a_blank_name_is_refused_before_the_account_is_checked() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let request_id = request_id("operation-1");

        for blank in ["", " \t\n"] {
            let create = state.create_household(blank.to_owned(), request_id.clone());
            let error = block_on_ready(create).expect_err("a blank name is refused");

            assert_eq!(error.cause, SaveCause::Validation);
            assert_eq!(error.request_id, Some(request_id.clone()));
        }

        assert!(service.calls().is_empty());
        assert!(state.households().is_empty());
    }

    #[test]
    fn a_rabbit_create_without_an_account_is_refused_with_no_account_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let request_id = request_id("operation-1");

        let error = block_on_ready(state.create_rabbit(
            household("household-1"),
            profile(),
            request_id.clone(),
        ))
        .expect_err("P0 has no account");

        assert_eq!(error.cause, SaveCause::NoAccount);
        assert_eq!(error.message_key, "error.no_account");
        assert!(error.retryable);
        assert_eq!(error.request_id, Some(request_id));

        // The honest outcome is a refusal, not an unauthenticated attempt (Q25, SYNC-2).
        assert!(service.calls().is_empty());
        assert_eq!(service.write_count(), 0);
    }

    #[test]
    fn a_contradictory_date_of_birth_is_refused_before_the_account_is_checked() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let request_id = request_id("operation-1");
        // The one refusal the RAB-1 field set produces: "unknown" and a date cannot both be true.
        let contradictory = RabbitProfile {
            dob: Some(at("2026-03-01T00:00:00Z")),
            dob_unknown: true,
            ..profile()
        };

        let error = block_on_ready(state.create_rabbit(
            household("household-1"),
            contradictory,
            request_id.clone(),
        ))
        .expect_err("the pair is refused as invalid input");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));

        // A property of the input: it is answered before the missing account is reported, and
        // nothing was probed or sent.
        assert!(service.calls().is_empty());
    }

    #[test]
    fn a_blank_rabbit_name_is_refused_before_the_account_is_checked() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let request_id = request_id("operation-1");

        // RAB-2 treats an empty name and a whitespace-only one as the same blank name.
        for blank in ["", " \t\n"] {
            let error = block_on_ready(state.create_rabbit(
                household("household-1"),
                RabbitProfile {
                    name: blank.to_owned(),
                    ..profile()
                },
                request_id.clone(),
            ))
            .expect_err("a blank name is refused");

            assert_eq!(error.cause, SaveCause::Validation, "{blank:?}");
            assert_eq!(error.message_key, "error.validation", "{blank:?}");
            assert!(!error.retryable, "{blank:?}");
            assert_eq!(error.request_id, Some(request_id.clone()), "{blank:?}");
        }

        // The person is prompted for a name rather than told their account is missing, and nothing
        // was probed or sent.
        assert!(service.calls().is_empty());
    }

    #[test]
    fn a_blank_rabbit_name_is_refused_before_the_write_is_sent() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));

        // The writer can be named, so only the name itself stands between this call and a write.
        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");

        let error = block_on_ready(state.create_rabbit(
            household("household-1"),
            RabbitProfile {
                name: "   ".to_owned(),
                ..profile()
            },
            request_id("operation-2"),
        ))
        .expect_err("whitespace is not a name");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(
            service.write_count(),
            1,
            "only the household create reached the service"
        );
    }

    #[test]
    fn a_substituted_double_drives_the_real_rabbit_create_path() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        // The writer is the member this account is in the household, so the household is held first —
        // exactly the order the dashboard's create flow produces (RAB-3, ADR 0002 criterion 3).
        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");

        let rabbit =
            block_on_ready(state.create_rabbit(one.clone(), profile(), request_id("operation-2")))
                .expect("the double accepted the write");

        // The recorded profile is what the command returns, and the write was one item write into
        // the household the caller named (RAB-1, RAB-3).
        assert_eq!(rabbit.household_id, one);
        assert_eq!(rabbit.name, "Pebble");
        assert_eq!(rabbit.breed, Some("Mini Lop".to_owned()));
        assert_eq!(rabbit.dob, Some(at("2026-03-01T00:00:00Z")));
        assert_eq!(rabbit.sex, Sex::Female);
        assert_eq!(rabbit.neuter_status, NeuterStatus::Spayed);
        assert_eq!(rabbit.adoption_date, None);
        assert!(!rabbit.is_archived);
        assert_eq!(rabbit.weight_drop_threshold_percent, 5.0);
        assert_eq!(
            service.write_count(),
            2,
            "the household create and the rabbit create"
        );
    }

    /// RAB-3's "Additional rabbits are created without restriction": a household that already holds
    /// rabbits accepts another one, and nothing counts or caps them.
    #[test]
    fn additional_rabbits_are_created_without_restriction() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");

        // Every create is its own operation identity and submits the *same* profile: distinct
        // rabbits are separated by their identity, never by their content, and no count stops the
        // eleventh rabbit of a household that already holds ten (RAB-3).
        let rabbits: Vec<_> = (2..=11)
            .map(|sequence| {
                block_on_ready(state.create_rabbit(
                    one.clone(),
                    profile(),
                    request_id(&format!("operation-{sequence}")),
                ))
                .expect("a household accepts a rabbit however many it already holds")
            })
            .collect();

        assert_eq!(rabbits.len(), 10);
        assert_eq!(
            rabbits
                .iter()
                .map(|rabbit| rabbit.id.clone())
                .collect::<HashSet<_>>()
                .len(),
            rabbits.len(),
            "each create is its own rabbit, even with an identical payload"
        );
        assert!(rabbits.iter().all(|rabbit| rabbit.household_id == one));
        assert!(rabbits.iter().all(|rabbit| !rabbit.is_archived));
        assert_eq!(
            service.write_count(),
            11,
            "the household and each of the ten rabbits was written"
        );
    }

    /// RAB-3's household invariant: two rabbits of one household are two items, each resolvable on
    /// its own, and neither is reachable through another household.
    #[test]
    fn two_rabbits_of_one_household_stay_distinct_and_each_reads_as_its_own_item() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");

        let first =
            block_on_ready(state.create_rabbit(one.clone(), profile(), request_id("operation-2")))
                .expect("the first rabbit is created");
        let second = block_on_ready(state.create_rabbit(
            one.clone(),
            RabbitProfile {
                name: "Clover".to_owned(),
                ..profile()
            },
            request_id("operation-3"),
        ))
        .expect("the second rabbit is created");

        assert_ne!(first.id, second.id, "two creates are two rabbits");

        // The service's copy of the household's collection, as a load delivers it (SYNC-3): one row
        // per rabbit, each at the version its own write named. Reading that list is task 2.14's;
        // here each item is resolved by its own identity through P0's guard.
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        wire(&first.id.0, "2026-10-22T09:30:00Z"),
                        wire(&second.id.0, "2026-10-22T09:31:00Z"),
                    ],
                ),
                at("2026-10-22T09:32:00Z"),
            )
            .expect("the collection is the household's own");

        for (rabbit, updated_at) in [
            (&first, "2026-10-22T09:30:00Z"),
            (&second, "2026-10-22T09:31:00Z"),
        ] {
            let item_id = ItemId(rabbit.id.0.clone());
            let row = state
                .item(&one, &item_id)
                .expect("the item is this household's")
                .expect("the row is stored");

            assert_eq!(row.item_id, item_id, "each rabbit is its own item");
            assert_eq!(row.updated_at, at(updated_at), "at its own version");
            assert_eq!(
                state.item_owner(&item_id),
                Some(one.clone()),
                "the item belongs to exactly one household"
            );
        }

        // Reached through another household the same two identities are a foreign attempt, never an
        // absence: one item cannot be presented as a rabbit of the other household (RAB-3).
        let two = household("household-2");

        for rabbit in [&first, &second] {
            assert_eq!(
                state.item(&two, &ItemId(rabbit.id.0.clone())),
                Err(BoundaryRefusal::ForeignHousehold {
                    requested: two.clone(),
                    owner: one.clone(),
                }),
                "the guard refuses a cross-household read"
            );
        }
    }

    #[test]
    fn a_rabbit_cannot_be_written_into_a_household_the_book_holds_no_member_for() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), Some(account()));
        let request_id = request_id("operation-1");

        // Nothing was created, so the book cannot name the writer. The write is refused rather than
        // attributed to some other member: a `writer_id` the person does not own would order the
        // service's last-write-wins tuple against the wrong writer (ADR 0002 criterion 3).
        let error = block_on_ready(state.create_rabbit(
            household("household-1"),
            profile(),
            request_id.clone(),
        ))
        .expect_err("the write cannot name its writer");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert!(service.calls().is_empty(), "nothing was probed or sent");
    }

    #[test]
    fn a_retried_create_resends_the_marker_its_first_attempt_minted() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");
        let operation = request_id("operation-2");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        block_on_ready(state.create_rabbit(one.clone(), profile(), operation.clone()))
            .expect("the first attempt completed");
        // The retry of that same operation, as the failure surface's retry re-issues it (D0.14).
        block_on_ready(state.create_rabbit(one, profile(), operation))
            .expect("the re-issued attempt completed");

        let bodies: Vec<serde_json::Value> = service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                crate::sync::double::RecordedCall::Save { request, .. } => {
                    Some(request.body().clone())
                }
                _ => None,
            })
            .collect();

        assert_eq!(
            bodies.len(),
            3,
            "the household create and two rabbit writes"
        );
        assert_eq!(
            bodies[1], bodies[2],
            "a retry re-sends a byte-identical write, marker included"
        );

        // The marker is the client-supplied `updated_at` last-write-wins compares: an absolute
        // instant, minted at the operation's creation rather than supplied by the person.
        let marker = bodies[1]["updated_at"]
            .as_str()
            .expect("the marker is an instant string")
            .to_owned();
        let recorded = AbsoluteInstant::parse(&Instant(marker.clone()))
            .expect("the minted marker is an absolute instant");
        assert_eq!(recorded.to_wire(), Instant(marker));
        assert!(
            recorded <= AbsoluteInstant::now(),
            "the marker is recorded at creation, never in the future"
        );
        assert_eq!(bodies[1]["writer_id"], json!("member-1"));
    }

    /// The state offers the read path checked operations and never a replaceable store handle.
    ///
    /// There is deliberately no `copies()` accessor any more: a guard over `GuardedStore` could be
    /// overwritten wholesale (`*state.copies() = GuardedStore::new()`), discarding every ownership
    /// claim and immutable marker. These methods lock internally and return values or refusals, so
    /// that operation does not exist in the public API.
    #[test]
    fn the_state_offers_checked_collection_operations_and_no_replaceable_store() {
        let state = AppState::new(Arc::new(ScriptedService::new()), None);
        let one = household("household-1");
        let two = household("household-2");
        let loaded_at = at("2026-10-19T11:00:00Z");

        state
            .load_collection(
                &one,
                wire_collection(&one, vec![wire("item-1", "2026-10-19T10:00:00Z")]),
                loaded_at.clone(),
            )
            .expect("the collection is addressed correctly");

        let read = state
            .read_collection(&one, CollectionSource::Local)
            .expect("this device holds the copy");
        assert_eq!(read.loaded_at, Some(loaded_at));
        assert_eq!(read.items.len(), 1);

        // The state's handle is the guard: a collection addressed through the wrong household is
        // refused rather than written, so the narrow API does not weaken the invariant.
        assert!(matches!(
            state.merge_collection(&two, wire_collection(&one, vec![])),
            Err(BoundaryRefusal::ForeignHousehold { .. })
        ));
    }

    #[test]
    fn a_substituted_double_drives_the_real_create_path() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));

        let create = state.create_household("Home".to_owned(), request_id("operation-1"));
        let household = block_on_ready(create).expect("the double accepted the write");

        assert_eq!(household.name, "Home");
        assert!(household.is_active);
        assert!(household.is_owner);
        assert_eq!(service.write_count(), 1);
        assert_eq!(
            state.households().active().expect("one is active").id(),
            &household.id
        );
    }

    #[test]
    fn the_state_lists_every_held_household_and_switches_the_active_one() {
        let service = Arc::new(ScriptedService::new().on_write([
            accepted_answer("household-1"),
            accepted_answer("household-2"),
        ]));
        let state = AppState::new(service.clone(), Some(account()));

        block_on_ready(state.create_household("First".to_owned(), request_id("operation-1")))
            .expect("the first create completed");
        block_on_ready(state.create_household("Second".to_owned(), request_id("operation-2")))
            .expect("the second create completed");

        // Every call the two creates made: their probes and their writes. The switch must add none
        // of them — no probe, no write and no read of any kind.
        let calls_after_creates = service.calls();

        let rows = state.list_households();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows.iter().filter(|row| row.is_active).count(), 1);
        assert!(!rows[0].is_active);
        assert!(rows[1].is_active);
        assert!(rows.iter().all(|row| row.is_owner));

        let active = state.active_household().expect("one is active");
        assert_eq!(active.id, household("household-2"));
        assert!(active.is_active);

        let switched = state
            .set_active_household(household("household-1"), request_id("operation-3"))
            .expect("the person holds household-1");

        assert_eq!(switched.id, household("household-1"));
        assert!(switched.is_active);
        assert!(switched.is_owner);

        let rows = state.list_households();
        assert_eq!(rows.iter().filter(|row| row.is_active).count(), 1);
        assert!(rows[0].is_active);
        assert!(!rows[1].is_active);

        // The switch is local state: it made no service call at all, so the observed calls are
        // exactly the two creates' — a probe or read added to the switch would fail here.
        assert_eq!(service.calls(), calls_after_creates);
        assert_eq!(service.write_count(), 2);
    }

    #[test]
    fn switching_to_a_household_the_person_does_not_hold_is_refused_and_changes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service, Some(account()));

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the create completed");

        let request_id = request_id("operation-2");
        let error = state
            .set_active_household(household("household-2"), request_id.clone())
            .expect_err("the person does not hold household-2");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));

        let active = state
            .active_household()
            .expect("the previous active remains");
        assert_eq!(active.id, household("household-1"));
    }

    /// A sink whose first write is held until the test releases it, so a second snapshot can be
    /// offered while the first write is genuinely in flight.
    struct GatedSink {
        store: CollectionStore,
        first: AtomicBool,
        started: Mutex<Option<mpsc::Sender<()>>>,
        release: Mutex<mpsc::Receiver<()>>,
        saved: Mutex<mpsc::Sender<()>>,
        writes: Mutex<Vec<DeviceSnapshot>>,
    }

    impl SnapshotSink for GatedSink {
        fn save(&self, snapshot: &DeviceSnapshot) -> Result<(), CollectionStoreError> {
            self.writes
                .lock()
                .expect("the writes are readable")
                .push(snapshot.clone());

            if self.first.swap(false, Ordering::SeqCst) {
                if let Some(started) = self.started.lock().expect("started").as_ref() {
                    let _ = started.send(());
                }

                let release = self.release.lock().expect("release");
                let _ = release.recv();
            }

            let outcome = self.store.save(snapshot);
            let _ = self.saved.lock().expect("saved").send(());

            outcome
        }
    }

    /// A sink that records every write and signals completion, without gating.
    struct RecordingSink {
        store: CollectionStore,
        saved: Mutex<mpsc::Sender<()>>,
    }

    impl SnapshotSink for RecordingSink {
        fn save(&self, snapshot: &DeviceSnapshot) -> Result<(), CollectionStoreError> {
            let outcome = self.store.save(snapshot);
            let _ = self.saved.lock().expect("saved").send(());

            outcome
        }
    }

    #[test]
    fn a_later_snapshot_wins_over_an_older_in_flight_write() {
        let directory = temp_data_dir("coalesce");
        let store = CollectionStore::in_dir(&directory);
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (saved_tx, saved_rx) = mpsc::channel();
        let sink = Arc::new(GatedSink {
            store: store.clone(),
            first: AtomicBool::new(true),
            started: Mutex::new(Some(started_tx)),
            release: Mutex::new(release_rx),
            saved: Mutex::new(saved_tx),
            writes: Mutex::new(Vec::new()),
        });
        let state = AppState::with_writer(
            Arc::new(ScriptedService::new()),
            None,
            CollectionStore::in_dir(&directory),
            sink.clone(),
        );
        let one = household("household-1");

        state
            .load_collection(
                &one,
                wire_collection(&one, vec![wire("item-1", "2026-10-19T10:00:00Z")]),
                at("2026-10-19T11:00:00Z"),
            )
            .expect("the first copy loads");
        state.schedule_persist();
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the first write started");

        // The first write is now held. Change the copy and offer the newer snapshot, then let the
        // first write finish.
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        wire("item-1", "2026-10-19T10:00:00Z"),
                        wire("item-2", "2026-10-19T10:00:00Z"),
                    ],
                ),
                at("2026-10-19T12:00:00Z"),
            )
            .expect("the second copy loads");
        let expected = state.snapshot();
        state.schedule_persist();

        release_tx.send(()).expect("release the first write");
        saved_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the first write finished");
        saved_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the later snapshot finished");

        assert_eq!(
            store.load().expect("the file is readable"),
            expected,
            "the file must hold the later snapshot"
        );

        let writes = sink.writes.lock().expect("the writes are readable");
        assert_eq!(writes.len(), 2, "the two writes were coalesced, not queued");
        assert_eq!(
            writes[0].collections[0].items.len(),
            1,
            "the older snapshot was the first to be written"
        );
        assert_eq!(
            writes[1], expected,
            "the later snapshot is the one written last"
        );

        drop(writes);
        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_merge_schedules_the_merged_copy_for_persistence() {
        let directory = temp_data_dir("merge-persist");
        let store = CollectionStore::in_dir(&directory);
        let (saved_tx, saved_rx) = mpsc::channel();
        let sink = Arc::new(RecordingSink {
            store: store.clone(),
            saved: Mutex::new(saved_tx),
        });
        let state = AppState::with_writer(
            Arc::new(ScriptedService::new()),
            None,
            CollectionStore::in_dir(&directory),
            sink,
        );
        let one = household("household-1");

        state
            .load_collection(
                &one,
                wire_collection(&one, vec![wire("item-1", "2026-10-19T10:00:00Z")]),
                at("2026-10-19T11:00:00Z"),
            )
            .expect("the base copy loads");
        state
            .merge_collection(
                &one,
                wire_collection(&one, vec![wire("item-2", "2026-10-19T10:00:00Z")]),
            )
            .expect("the merge lands");

        saved_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the merge is scheduled for persistence");

        let stored = store.load().expect("the file is readable");
        assert_eq!(stored.collections.len(), 1);
        let ids: Vec<&str> = stored.collections[0]
            .items
            .iter()
            .map(|item| item.item_id.0.as_str())
            .collect();
        assert_eq!(ids, ["item-1", "item-2"]);

        remove_temp_data_dir(&directory);
    }

    /// RAB-5 and HS-8: an edit including the threshold is one write of the household's single rabbit
    /// item, so every member resolves it, and it never becomes a second, device-local copy.
    #[test]
    fn editing_a_rabbit_writes_the_one_shared_item_every_member_resolves() {
        let (service, state, rabbit) = state_with_one_rabbit();
        let one = household("household-1");
        let held_before = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the read path's copy is held");

        let edited = block_on_ready(state.update_rabbit(
            rabbit.id.clone(),
            RabbitProfile {
                name: "Clover".to_owned(),
                breed: Some("Netherland Dwarf".to_owned()),
                weight_drop_threshold_percent: 15.0,
                ..profile()
            },
            request_id("operation-3"),
        ))
        .expect("the double accepted the edit");

        // The edit lands on the same rabbit, in the same household, with every submitted field.
        assert_eq!(edited.id, rabbit.id);
        assert_eq!(edited.household_id, one);
        assert_eq!(edited.name, "Clover");
        assert_eq!(edited.breed, Some("Netherland Dwarf".to_owned()));
        assert_eq!(edited.weight_drop_threshold_percent, 15.0);

        // Exactly one write was added, and it is one `save_item` of the rabbit's existing item into
        // the household's collection: one item every member reads, not one per member (HS-8).
        let bodies = write_bodies(&service);
        assert_eq!(bodies.len(), 3, "the household, the create and the edit");
        let body = &bodies[2];
        assert_eq!(body["household_id"], json!(one.0));
        assert_eq!(body["item_id"], json!(rabbit.id.0));
        assert_eq!(body["request_id"], json!("operation-3"));
        assert_eq!(body["writer_id"], json!("member-1"));
        assert_eq!(
            body["payload"]["rabbit"]["weight_drop_threshold_percent"],
            json!(15.0)
        );
        assert_eq!(body["payload"]["rabbit"]["name"], json!("Clover"));
        assert_eq!(
            body["payload"]["rabbit"]["breed"],
            json!("Netherland Dwarf")
        );

        // The device's own copy belongs to the read path: this write must not update it, or the edit
        // would exist twice and could disagree with what the other members read (SYNC-3).
        let held_after = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is still held");
        assert_eq!(held_after.items, held_before.items);
        assert_eq!(held_after.loaded_at, held_before.loaded_at);
    }

    #[test]
    fn an_edit_of_a_rabbit_this_device_holds_no_copy_of_is_refused_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(state.update_rabbit(
            RabbitId("rabbit-1".to_owned()),
            profile(),
            request_id.clone(),
        ))
        .expect_err("the device holds no rabbit to edit");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// RAB-10 / D1.5: a permanently deleted rabbit keeps its identity claim, and that claim alone must
    /// not authorize a write. A full-payload edit sent to a tombstoned item would resurrect it, so the
    /// boundary refuses it and nothing is sent.
    #[test]
    fn an_edit_of_a_permanently_deleted_rabbit_is_refused_and_never_resurrects_it() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        // The deletion (task 2.7) leaves a tombstone row, and the guard keeps the item's household
        // claim after the row is removed (NFR-10).
        state
            .load_collection(
                &one,
                wire_collection(&one, vec![tombstone("rabbit-1", "2026-10-22T10:00:00Z")]),
                at("2026-10-22T10:01:00Z"),
            )
            .expect("the collection is the household's own");
        assert_eq!(
            state.item_owner(&ItemId("rabbit-1".to_owned())),
            Some(one.clone()),
            "the identity claim deliberately survives the tombstone"
        );

        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(state.update_rabbit(
            RabbitId("rabbit-1".to_owned()),
            profile(),
            request_id.clone(),
        ))
        .expect_err("a deleted rabbit is not editable");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");

        // The row is still the tombstone it was: no write reached the device copy either.
        let held = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is held");
        assert_eq!(
            held.items,
            vec![tombstone("rabbit-1", "2026-10-22T10:00:00Z")]
        );
    }

    #[test]
    fn a_blank_edit_name_is_refused_before_the_account_is_checked() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let one = household("household-1");

        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![row_with_rabbit(
                        "rabbit-1",
                        "2026-10-22T09:30:00Z",
                        &stored_rabbit("rabbit-1", "Pebble"),
                    )],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");
        let request_id = request_id("operation-2");

        // The same validate_name rule creation uses, and it runs before the account check: the person
        // is prompted for a name rather than told their account is missing (RAB-2).
        for blank in ["", " \t\n"] {
            let error = block_on_ready(state.update_rabbit(
                RabbitId("rabbit-1".to_owned()),
                RabbitProfile {
                    name: blank.to_owned(),
                    ..profile()
                },
                request_id.clone(),
            ))
            .expect_err("a blank name is refused");

            assert_eq!(error.cause, SaveCause::Validation, "{blank:?}");
            assert_eq!(error.message_key, "error.validation", "{blank:?}");
            assert!(!error.retryable, "{blank:?}");
            assert_eq!(error.request_id, Some(request_id.clone()), "{blank:?}");
        }

        assert!(service.calls().is_empty(), "nothing was probed or sent");
    }

    #[test]
    fn an_edit_without_an_account_is_refused_with_no_account_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let one = household("household-1");

        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![row_with_rabbit(
                        "rabbit-1",
                        "2026-10-22T09:30:00Z",
                        &stored_rabbit("rabbit-1", "Pebble"),
                    )],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");
        let request_id = request_id("operation-2");

        let error = block_on_ready(state.update_rabbit(
            RabbitId("rabbit-1".to_owned()),
            profile(),
            request_id.clone(),
        ))
        .expect_err("P0 has no account");

        assert_eq!(error.cause, SaveCause::NoAccount);
        assert_eq!(error.message_key, "error.no_account");
        assert!(error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert!(service.calls().is_empty(), "nothing was probed or sent");
    }

    /// RAB-8's first scenario at the boundary the weight writes use: a write that would add an entry
    /// to an archived rabbit is refused as `validation`, carries the operation's identity, and sends
    /// nothing (DM-4; D1.5).
    #[test]
    fn an_archived_rabbit_refuses_the_write_that_would_add_a_weight_entry() {
        let (service, state, rabbit) = state_with_an_archived_rabbit();
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = state
            .writable_rabbit(&rabbit.id, &request_id)
            .expect_err("an archived rabbit's timeline cannot grow");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// RAB-8's second scenario, in the order task 2.11 must follow: an entry is resolved first and its
    /// `rabbit_id` is then checked, so an archived rabbit refuses the edit or deletion of any of its
    /// entries before anything is written (DM-4; D1.5).
    #[test]
    fn an_archived_rabbit_refuses_the_write_that_would_edit_or_delete_an_entry() {
        let (service, state, rabbit) = state_with_an_archived_rabbit_and_entries();
        let one = household("household-1");
        let collection = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the read path's copy is held");
        let entry_rabbit_id = collection
            .items
            .iter()
            .find_map(|item| match &item.payload {
                Some(ItemPayload::WeightEntry { weight_entry }) => {
                    Some(weight_entry.rabbit_id.clone())
                }
                _ => None,
            })
            .expect("the archived rabbit has an entry");
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        assert_eq!(entry_rabbit_id, rabbit.id);
        let error = state
            .writable_rabbit(&entry_rabbit_id, &request_id)
            .expect_err("an archived rabbit's entries are read-only");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// D1.16: a live row whose content is unknown cannot establish that its rabbit is not archived,
    /// so an edit is refused with the unknown-content key instead of being written blind.
    #[test]
    fn an_edit_of_a_rabbit_whose_content_is_unknown_is_refused_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        state
            .load_collection(
                &one,
                wire_collection(&one, vec![wire("rabbit-1", "2026-10-22T09:30:00Z")]),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(state.update_rabbit(
            RabbitId("rabbit-1".to_owned()),
            profile(),
            request_id.clone(),
        ))
        .expect_err("the archived flag cannot be established without the content");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert!(!error.retryable);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_ne!(error.message_key, "error.unknown");
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// A state whose book holds `household-1` and whose copy holds one rabbit with its content —
    /// the state an archive starts from, since the archive is addressed by the rabbit alone.
    fn state_with_a_stored_rabbit() -> (Arc<ScriptedService>, AppState, Rabbit) {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        let rabbit = stored_rabbit("rabbit-1", "Pebble");
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![row_with_rabbit("rabbit-1", "2026-10-22T09:30:00Z", &rabbit)],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        (service, state, rabbit)
    }

    /// RAB-6, RAB-9: the archive is one write of the rabbit's **whole** item, with only the archived
    /// flag changed. A partial payload would replace the stored version and discard the profile.
    #[test]
    fn archiving_writes_the_whole_rabbit_item_with_only_the_flag_changed() {
        let (service, state, rabbit) = state_with_a_stored_rabbit();

        let archived =
            block_on_ready(state.archive_rabbit(rabbit.id.clone(), request_id("operation-2")))
                .expect("the double accepted the archive");

        // The returned content is the copy's content with the flag set: every other field survives.
        assert!(archived.is_archived);
        assert_eq!(
            archived,
            Rabbit {
                is_archived: true,
                ..rabbit.clone()
            },
            "the archive changes nothing but the flag"
        );

        // One added write, into the rabbit's existing item in the household's collection, carrying
        // the whole rabbit as its payload (RAB-9, D1.16).
        let bodies = write_bodies(&service);
        assert_eq!(bodies.len(), 2, "the household create and the archive");
        let body = &bodies[1];
        assert_eq!(body["household_id"], json!(rabbit.household_id.0));
        assert_eq!(body["item_id"], json!(rabbit.id.0));
        assert_eq!(body["request_id"], json!("operation-2"));
        assert_eq!(body["writer_id"], json!("member-1"));

        let mut expected = serde_json::to_value(&rabbit).expect("the rabbit serialises");
        expected["is_archived"] = json!(true);
        assert_eq!(
            body["payload"]["rabbit"], expected,
            "the payload is the whole stored item, not a partial patch"
        );
    }

    /// D1.16: a live row whose content is not on this device cannot be archived. A whole-item write
    /// needs the content; submitting a partial payload would be the data loss RAB-9 forbids.
    #[test]
    fn archiving_a_rabbit_whose_content_is_unknown_is_refused_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        state
            .load_collection(
                &one,
                wire_collection(&one, vec![wire("rabbit-1", "2026-10-22T09:30:00Z")]),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(
            state.archive_rabbit(RabbitId("rabbit-1".to_owned()), request_id.clone()),
        )
        .expect_err("the content is not on this device");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert!(!error.retryable);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_ne!(error.message_key, "error.unknown");
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// RAB-6: archiving twice is refused as `validation` and nothing is sent.
    #[test]
    fn archiving_an_already_archived_rabbit_is_refused_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        let mut rabbit = stored_rabbit("rabbit-1", "Pebble");
        rabbit.is_archived = true;
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![row_with_rabbit("rabbit-1", "2026-10-22T09:30:00Z", &rabbit)],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(state.archive_rabbit(rabbit.id.clone(), request_id.clone()))
            .expect_err("the rabbit is already archived");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// RAB-10, D1.16: a tombstone keeps its identity claim but names a permanently deleted rabbit, so
    /// a lifecycle write must not reach it — and the archive never resurrects one.
    #[test]
    fn archiving_a_tombstoned_rabbit_is_refused_and_never_resurrects_it() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        state
            .load_collection(
                &one,
                wire_collection(&one, vec![tombstone("rabbit-1", "2026-10-22T10:00:00Z")]),
                at("2026-10-22T10:01:00Z"),
            )
            .expect("the collection is the household's own");
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(
            state.archive_rabbit(RabbitId("rabbit-1".to_owned()), request_id.clone()),
        )
        .expect_err("a deleted rabbit is not archivable");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// RAB-9's two scenarios: archiving the rabbit leaves every entry stored, and the archived
    /// history is still present after a relaunch — nothing ages or purges it.
    #[test]
    fn archiving_discards_no_entry_and_the_archived_history_survives_a_relaunch() {
        let directory = temp_data_dir("archive-retention");
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::with_collection_store(
            service,
            Some(account()),
            CollectionStore::in_dir(&directory),
        );
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        let rabbit = stored_rabbit("rabbit-1", "Pebble");
        let first_entry = row_with_weight_entry("entry-1", &rabbit);
        let second_entry = row_with_weight_entry("entry-2", &rabbit);
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        row_with_rabbit("rabbit-1", "2026-10-22T09:30:00Z", &rabbit),
                        first_entry.clone(),
                        second_entry.clone(),
                    ],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");
        state.persist_now();

        let archived =
            block_on_ready(state.archive_rabbit(rabbit.id.clone(), request_id("operation-2")))
                .expect("the double accepted the archive");

        // "Archiving discards nothing": the write is the rabbit item alone, and both entries are
        // still the rows the copy holds.
        let held = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is held");
        assert!(
            held.items.contains(&first_entry) && held.items.contains(&second_entry),
            "both entries recorded before archiving are still stored"
        );

        // The read path's next refresh stores the accepted archived version beside the same entries;
        // the relaunch is the arbitrary interval after archiving.
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        row_with_rabbit("rabbit-1", "2026-10-22T09:32:00Z", &archived),
                        first_entry.clone(),
                        second_entry.clone(),
                    ],
                ),
                at("2026-10-22T09:33:00Z"),
            )
            .expect("the refreshed collection is the household's own");
        state.persist_now();

        let relaunched = AppState::with_collection_store(
            Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse])),
            None,
            CollectionStore::in_dir(&directory),
        );
        let restored = relaunched
            .read_collection(&one, CollectionSource::Local)
            .expect("the archived copy survives the relaunch");

        let stored = restored
            .items
            .iter()
            .find(|item| item.item_id == ItemId(rabbit.id.0.clone()))
            .expect("the archived rabbit is still stored");
        let Some(ItemPayload::Rabbit {
            rabbit: restored_rabbit,
        }) = &stored.payload
        else {
            panic!("the archived rabbit carries its content");
        };
        assert!(restored_rabbit.is_archived);

        for entry in [&first_entry, &second_entry] {
            assert!(
                restored.items.contains(entry),
                "the archived history survives the interval unchanged"
            );
        }

        remove_temp_data_dir(&directory);
    }

    /// A state whose book holds `household-1` and whose copy holds one **archived** rabbit with its
    /// content — the state a restore starts from, since the restore is addressed by the rabbit alone.
    fn state_with_an_archived_rabbit() -> (Arc<ScriptedService>, AppState, Rabbit) {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        let mut rabbit = stored_rabbit("rabbit-1", "Pebble");
        rabbit.is_archived = true;
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![row_with_rabbit("rabbit-1", "2026-10-22T09:30:00Z", &rabbit)],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        (service, state, rabbit)
    }

    /// RAB-7: the restore is one write of the rabbit's **whole** item, with only the archived flag
    /// cleared, so the profile comes back exactly as it was archived.
    #[test]
    fn restoring_writes_the_whole_rabbit_item_with_only_the_flag_cleared() {
        let (service, state, rabbit) = state_with_an_archived_rabbit();

        let restored =
            block_on_ready(state.restore_rabbit(rabbit.id.clone(), request_id("operation-2")))
                .expect("the double accepted the restore");

        // The returned content is the copy's content with the flag cleared: every other field
        // survives.
        assert!(!restored.is_archived);
        assert_eq!(
            restored,
            Rabbit {
                is_archived: false,
                ..rabbit.clone()
            },
            "the restore changes nothing but the flag"
        );

        // One added write, into the rabbit's existing item in the household's collection, carrying
        // the whole rabbit as its payload (RAB-7, D1.16).
        let bodies = write_bodies(&service);
        assert_eq!(bodies.len(), 2, "the household create and the restore");
        let body = &bodies[1];
        assert_eq!(body["household_id"], json!(rabbit.household_id.0));
        assert_eq!(body["item_id"], json!(rabbit.id.0));
        assert_eq!(body["request_id"], json!("operation-2"));
        assert_eq!(body["writer_id"], json!("member-1"));

        assert_eq!(
            body["payload"]["rabbit"],
            serde_json::to_value(&restored).expect("the rabbit serialises"),
            "the payload is the whole stored item, not a partial patch"
        );
        assert_eq!(body["payload"]["rabbit"]["is_archived"], json!(false));
    }

    /// D1.16: a live row whose content is not on this device cannot be restored. A whole-item write
    /// needs the content; submitting a partial payload would discard the profile.
    #[test]
    fn restoring_a_rabbit_whose_content_is_unknown_is_refused_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        state
            .load_collection(
                &one,
                wire_collection(&one, vec![wire("rabbit-1", "2026-10-22T09:30:00Z")]),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(
            state.restore_rabbit(RabbitId("rabbit-1".to_owned()), request_id.clone()),
        )
        .expect_err("the content is not on this device");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert!(!error.retryable);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_ne!(error.message_key, "error.unknown");
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// RAB-7: restoring a rabbit that is not archived is refused as `validation` — restoring what was
    /// never archived is not a no-op request this path absorbs — and nothing is sent.
    #[test]
    fn restoring_an_already_active_rabbit_is_refused_and_nothing_is_sent() {
        let (service, state, rabbit) = state_with_a_stored_rabbit();
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(state.restore_rabbit(rabbit.id.clone(), request_id.clone()))
            .expect_err("the rabbit is already active");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// RAB-10, D1.16: a tombstone keeps its identity claim but names a permanently deleted rabbit, so
    /// a lifecycle write must not reach it — and a restore never resurrects one.
    #[test]
    fn restoring_a_tombstoned_rabbit_is_refused_and_never_resurrects_it() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        state
            .load_collection(
                &one,
                wire_collection(&one, vec![tombstone("rabbit-1", "2026-10-22T10:00:00Z")]),
                at("2026-10-22T10:01:00Z"),
            )
            .expect("the collection is the household's own");
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(
            state.restore_rabbit(RabbitId("rabbit-1".to_owned()), request_id.clone()),
        )
        .expect_err("a deleted rabbit is not restorable");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// RAB-7's second scenario: an archived rabbit that carries weight entries is restored, and every
    /// entry is still present, unchanged and readable afterwards — the restore writes the rabbit's
    /// item alone and never names an entry.
    #[test]
    fn restoring_leaves_every_weight_entry_present_unchanged_and_readable() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        let mut rabbit = stored_rabbit("rabbit-1", "Pebble");
        rabbit.is_archived = true;
        let first_entry = row_with_weight_entry("entry-1", &rabbit);
        let second_entry = row_with_weight_entry("entry-2", &rabbit);
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        row_with_rabbit("rabbit-1", "2026-10-22T09:30:00Z", &rabbit),
                        first_entry.clone(),
                        second_entry.clone(),
                    ],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        let restored =
            block_on_ready(state.restore_rabbit(rabbit.id.clone(), request_id("operation-2")))
                .expect("the double accepted the restore");

        // Nothing the restore wrote names an entry, so both entries are still the rows the copy
        // holds — byte for byte the ones recorded before archiving.
        let held = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is held");
        assert!(
            held.items.contains(&first_entry) && held.items.contains(&second_entry),
            "both entries recorded before archiving are still stored and unchanged"
        );

        // The read path's next refresh stores the restored version beside the same entries; they are
        // still readable from the restored rabbit.
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        row_with_rabbit("rabbit-1", "2026-10-22T09:32:00Z", &restored),
                        first_entry.clone(),
                        second_entry.clone(),
                    ],
                ),
                at("2026-10-22T09:33:00Z"),
            )
            .expect("the refreshed collection is the household's own");

        let refreshed = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is held");
        let stored = refreshed
            .items
            .iter()
            .find(|item| item.item_id == ItemId(rabbit.id.0.clone()))
            .expect("the restored rabbit is stored");
        let Some(ItemPayload::Rabbit {
            rabbit: restored_rabbit,
        }) = &stored.payload
        else {
            panic!("the restored rabbit carries its content");
        };
        assert!(!restored_rabbit.is_archived);

        for entry in [&first_entry, &second_entry] {
            let readable = refreshed
                .items
                .iter()
                .find(|item| item.item_id == entry.item_id)
                .expect("the entry is still stored beside the restored rabbit");
            assert_eq!(
                readable, entry,
                "the entry is unchanged and reads back with its content"
            );
        }
    }

    /// A state whose book holds `household-1` and whose copy holds one archived rabbit with two
    /// entries it owns — the state a permanent deletion starts from (RAB-10).
    fn state_with_an_archived_rabbit_and_entries() -> (Arc<ScriptedService>, AppState, Rabbit) {
        let (service, state, rabbit) = state_with_an_archived_rabbit();
        let one = household("household-1");

        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        row_with_rabbit("rabbit-1", "2026-10-22T09:31:00Z", &rabbit),
                        row_with_weight_entry("entry-1", &rabbit),
                        row_with_weight_entry("entry-2", &rabbit),
                    ],
                ),
                at("2026-10-22T09:32:00Z"),
            )
            .expect("the collection is the household's own");

        (service, state, rabbit)
    }

    /// RAB-10, D1.16: the deletion is one tombstone per item — the entries first, the rabbit last —
    /// into the household's shared collection, naming the writing member and the operation's marker.
    /// The device's own copy is the read path's and is not written.
    #[test]
    fn permanently_deleting_an_archived_rabbit_writes_a_tombstone_for_it_and_every_entry() {
        let (service, state, rabbit) = state_with_an_archived_rabbit_and_entries();
        let one = household("household-1");
        let held_before = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the read path's copy is held");

        let deleted_household =
            block_on_ready(state.delete_rabbit(rabbit.id.clone(), request_id("operation-2")))
                .expect("the double accepted the deletion");

        assert_eq!(
            deleted_household, one,
            "the answer names the collection that lost the rows"
        );

        let bodies = write_bodies(&service);
        assert_eq!(
            bodies.len(),
            4,
            "the household create and one tombstone per removed item"
        );
        // The entries are removed before the rabbit: a stopped cascade never leaves a removed rabbit
        // with readable history (RAB-10).
        assert_eq!(bodies[1]["item_id"], json!("entry-1"));
        assert_eq!(bodies[2]["item_id"], json!("entry-2"));
        assert_eq!(bodies[3]["item_id"], json!(rabbit.id.0));

        for body in &bodies[1..] {
            assert_eq!(body["household_id"], json!(one.0));
            assert_eq!(body["writer_id"], json!("member-1"));
            assert_eq!(body["deleted"], json!(true));
        }
        assert_eq!(bodies[1]["payload"]["kind"], json!("weight_entry"));
        assert_eq!(
            bodies[1]["payload"]["weight_entry"]["note"],
            json!("morning"),
            "the entry's content is retained on the version that removes it"
        );
        assert_eq!(bodies[3]["payload"]["kind"], json!("rabbit"));
        assert_eq!(bodies[3]["payload"]["rabbit"]["name"], json!("Pebble"));

        // The device's own copy belongs to the read path: this write must not update it, or the
        // removal would exist twice and could disagree with what the other members read (SYNC-3).
        let held_after = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is still held");
        assert_eq!(held_after.items, held_before.items);
        assert_eq!(held_after.loaded_at, held_before.loaded_at);
    }

    /// RAB-10's second scenario: an active rabbit is refused as `validation`, and nothing is probed or
    /// sent.
    #[test]
    fn permanently_deleting_an_active_rabbit_is_refused_and_nothing_is_sent() {
        let (service, state, rabbit) = state_with_a_stored_rabbit();
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(state.delete_rabbit(rabbit.id.clone(), request_id.clone()))
            .expect_err("an active rabbit cannot be permanently deleted");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// A blank operation identity names no operation, so the deletion is refused before the local
    /// copy is even consulted (D0.14): the failure carries the blank id and nothing is probed or sent.
    #[test]
    fn permanently_deleting_a_rabbit_with_a_blank_operation_identity_is_refused_and_nothing_is_sent()
     {
        let (service, state, rabbit) = state_with_an_archived_rabbit();
        let calls_before = service.calls();
        let blank = request_id("   ");

        let error = block_on_ready(state.delete_rabbit(rabbit.id.clone(), blank.clone()))
            .expect_err("a blank identity names no operation");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(blank));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// D1.16: a live row whose content is unknown could be one of the rabbit's entries, so the delete
    /// refuses with its own key and sends nothing.
    #[test]
    fn permanently_deleting_a_rabbit_whose_content_is_unknown_is_refused_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        state
            .load_collection(
                &one,
                wire_collection(&one, vec![wire("rabbit-1", "2026-10-22T09:30:00Z")]),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");
        let calls_before = service.calls();
        let request_id = request_id("operation-2");

        let error = block_on_ready(
            state.delete_rabbit(RabbitId("rabbit-1".to_owned()), request_id.clone()),
        )
        .expect_err("the content is not on this device");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert!(!error.retryable);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_ne!(error.message_key, "error.unknown");
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// A rabbit this device holds no live row for cannot be deleted from here: the refusal is
    /// `validation` and nothing is sent (RAB-10, NFR-10).
    #[test]
    fn permanently_deleting_a_rabbit_this_device_holds_no_row_for_is_refused_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let request_id = request_id("operation-2");

        for collection in [
            None,
            Some(vec![tombstone("rabbit-1", "2026-10-22T10:00:00Z")]),
        ] {
            let one = household("household-1");

            if let Some(items) = collection {
                state
                    .load_collection(
                        &one,
                        wire_collection(&one, items),
                        at("2026-10-22T10:01:00Z"),
                    )
                    .expect("the collection is the household's own");
            }

            let error = block_on_ready(
                state.delete_rabbit(RabbitId("rabbit-1".to_owned()), request_id.clone()),
            )
            .expect_err("there is no live rabbit to delete");

            assert_eq!(error.cause, SaveCause::Validation);
            assert_eq!(error.request_id, Some(request_id.clone()));
        }

        assert!(service.calls().is_empty(), "nothing was probed or sent");
    }

    /// A state whose book holds `household-1`, whose copy holds exactly `items`, and whose account is
    /// the household's member — the starting point for the entry-resolution refusals (D1.16, D1.17).
    fn state_with_copy(items: Vec<CollectionItem>) -> (Arc<ScriptedService>, AppState) {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        state
            .load_collection(
                &one,
                wire_collection(&one, items),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        (service, state)
    }

    /// A state that holds one active rabbit and three entries in timestamp order — the state an edit or
    /// a deletion of a past entry starts from (WT-5, WT-7).
    fn state_with_one_rabbit_and_three_entries() -> (Arc<ScriptedService>, AppState, Rabbit) {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");
        let rabbit =
            block_on_ready(state.create_rabbit(one.clone(), profile(), request_id("operation-2")))
                .expect("the rabbit is created");
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        row_with_rabbit(&rabbit.id.0, "2026-10-22T09:30:00Z", &rabbit),
                        row_with_entry("entry-1", &rabbit, 2_000, "2026-10-20T09:00:00Z"),
                        row_with_entry("entry-2", &rabbit, 1_800, "2026-10-21T09:00:00Z"),
                        row_with_entry("entry-3", &rabbit, 1_700, "2026-10-22T09:00:00Z"),
                    ],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        (service, state, rabbit)
    }

    /// Assert that both entry writes refuse `entry_id` with `cause` and `message_key`, carrying the
    /// operation's identity, with nothing probed or sent.
    fn assert_both_entry_writes_refused(
        state: &AppState,
        service: &ScriptedService,
        entry_id: &str,
        cause: SaveCause,
        message_key: &str,
    ) {
        let calls_before = service.calls();
        let operation = request_id("operation-2");

        let errors = [
            block_on_ready(state.update_weight_entry(
                WeightEntryId(entry_id.to_owned()),
                weight_entry_draft(),
                operation.clone(),
            ))
            .expect_err("the edit is refused"),
            block_on_ready(
                state.delete_weight_entry(WeightEntryId(entry_id.to_owned()), operation.clone()),
            )
            .expect_err("the deletion is refused"),
        ];

        for error in errors {
            assert_eq!(error.cause, cause, "{entry_id}");
            assert_eq!(error.message_key, message_key, "{entry_id}");
            assert!(!error.retryable, "{entry_id}");
            assert_eq!(error.request_id, Some(operation.clone()), "{entry_id}");
        }

        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// WT-5, HS-8: correcting a past entry is one write of the entry's **whole** item into the
    /// household's collection, naming the writing member — so every member resolves the correction —
    /// and the stored timestamp is kept when the draft omits it.
    ///
    /// WT-7, D1.13: the edit is alert-affecting, but the re-derivation is a read-side property. This
    /// test proves the write side touches exactly the edited row: no write names a neighbour, and the
    /// rows this device holds (the neighbours included) are unchanged. The following entry's flag is
    /// re-derived by the read of tasks 2.13–2.15.
    #[test]
    fn editing_a_past_entry_writes_the_one_shared_item_and_leaves_the_neighbours_stored() {
        let (service, state, rabbit) = state_with_one_rabbit_and_three_entries();
        let one = household("household-1");
        let held_before = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the read path's copy is held");
        let stored_timestamp = at("2026-10-21T09:00:00Z");
        let operation = request_id("operation-3");

        let (household_id, revised) = block_on_ready(state.update_weight_entry(
            WeightEntryId("entry-2".to_owned()),
            WeightEntryDraft {
                value_grams: Grams(1_500),
                timestamp: None,
                note: Some("corrected".to_owned()),
                outlier_confirmed: false,
            },
            operation.clone(),
        ))
        .expect("the double accepted the edit");

        assert_eq!(household_id, one);
        assert_eq!(revised.id, WeightEntryId("entry-2".to_owned()));
        assert_eq!(revised.rabbit_id, rabbit.id);
        assert_eq!(revised.value_grams, Grams(1_500));
        assert_eq!(revised.note.as_deref(), Some("corrected"));
        assert_eq!(
            revised.timestamp, stored_timestamp,
            "an omitted timestamp keeps the entry's stored instant"
        );
        assert_eq!(
            revised.created_at,
            at("2026-10-21T09:00:00Z"),
            "an edit preserves the entry's recorded creation instant (WT-7, D1.12)"
        );

        // One added write: the entry's existing item, whole, in the household's collection (HS-8).
        let bodies = write_bodies(&service);
        assert_eq!(bodies.len(), 3, "the household, the rabbit and the edit");
        let body = &bodies[2];
        assert_eq!(body["household_id"], json!(one.0));
        assert_eq!(body["item_id"], json!("entry-2"));
        assert_eq!(body["request_id"], json!("operation-3"));
        assert_eq!(body["writer_id"], json!("member-1"));
        assert!(body.get("deleted").is_none(), "an edit is not a removal");
        assert_eq!(body["payload"]["kind"], json!("weight_entry"));
        assert_eq!(body["payload"]["weight_entry"]["value_grams"], json!(1_500));
        assert_eq!(
            body["payload"]["weight_entry"]["timestamp"],
            json!(stored_timestamp.0),
            "the stored payload carries the kept instant, not a restamp"
        );
        assert_eq!(
            body["payload"]["weight_entry"]["created_at"],
            json!("2026-10-21T09:00:00Z"),
            "the stored payload carries the entry's original creation instant (D1.12)"
        );
        assert_eq!(
            body["payload"]["weight_entry"]["has_weight_drop_alert"],
            json!(false),
            "the derived alert is written false and re-derived on read (D1.16, D1.13)"
        );

        // WT-7, D1.13: no neighbour is rewritten. The only entry write addresses the edited row.
        for neighbour in ["entry-1", "entry-3"] {
            assert!(
                bodies
                    .iter()
                    .all(|body| body["item_id"] != json!(neighbour)),
                "the write never names {neighbour}"
            );
        }
        let held_after = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is still held");
        assert_eq!(
            held_after.items, held_before.items,
            "no stored row is rewritten, the neighbours included"
        );
    }

    /// WT-5, HS-8, D1.17: deleting a past entry is one tombstone of its existing item, retaining its
    /// content and naming the writing member.
    ///
    /// WT-7, D1.13: the deletion is alert-affecting, but the following entry's re-derivation is a
    /// read-side property. No stored neighbour is rewritten here; the read of tasks 2.13–2.15 compares
    /// the following entry against the one that now precedes it.
    #[test]
    fn deleting_a_past_entry_writes_the_one_tombstone_and_leaves_the_neighbours_stored() {
        let (service, state, rabbit) = state_with_one_rabbit_and_three_entries();
        let one = household("household-1");
        let held_before = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the read path's copy is held");
        let operation = request_id("operation-3");

        let (household_id, rabbit_id) = block_on_ready(
            state.delete_weight_entry(WeightEntryId("entry-2".to_owned()), operation.clone()),
        )
        .expect("the double accepted the deletion");

        assert_eq!(household_id, one);
        assert_eq!(rabbit_id, rabbit.id);

        let bodies = write_bodies(&service);
        assert_eq!(
            bodies.len(),
            3,
            "the household, the rabbit and the deletion"
        );
        let body = &bodies[2];
        assert_eq!(body["household_id"], json!(one.0));
        assert_eq!(body["item_id"], json!("entry-2"));
        assert_eq!(body["request_id"], json!("operation-3"));
        assert_eq!(body["writer_id"], json!("member-1"));
        assert_eq!(body["deleted"], json!(true));
        assert_eq!(body["payload"]["kind"], json!("weight_entry"));
        assert_eq!(
            body["payload"]["weight_entry"]["value_grams"],
            json!(1_800),
            "the removing version retains the entry's content"
        );
        assert_eq!(body["payload"]["weight_entry"]["note"], json!("morning"));
        assert_eq!(
            body["payload"]["weight_entry"]["has_weight_drop_alert"],
            json!(false)
        );

        for neighbour in ["entry-1", "entry-3"] {
            assert!(
                bodies
                    .iter()
                    .all(|body| body["item_id"] != json!(neighbour)),
                "the removal never names {neighbour}"
            );
        }
        let held_after = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is still held");
        assert_eq!(
            held_after.items, held_before.items,
            "no stored row is rewritten, the neighbours included"
        );
    }

    /// D0.14: a retried edit re-sends the operation's marker and the kept timestamp, so the two bodies
    /// are byte-identical and the service's ledger recognises one operation rather than refusing a
    /// changed payload.
    #[test]
    fn a_retried_edit_submits_a_byte_identical_body() {
        let (service, state, _rabbit) = state_with_one_rabbit_and_three_entries();
        let operation = request_id("operation-3");
        let correction = || WeightEntryDraft {
            value_grams: Grams(1_500),
            timestamp: None,
            note: Some("corrected".to_owned()),
            outlier_confirmed: false,
        };

        for _attempt in 1..=2 {
            block_on_ready(state.update_weight_entry(
                WeightEntryId("entry-2".to_owned()),
                correction(),
                operation.clone(),
            ))
            .expect("the double accepted the edit");
        }

        let bodies = write_bodies(&service);
        assert_eq!(
            bodies.len(),
            4,
            "the household, the rabbit and two attempts"
        );
        assert_eq!(
            bodies[2], bodies[3],
            "a retry submits the bytes the first attempt submitted"
        );
    }

    /// WT-4: an edit applies the create path's value rule — zero is refused, an unconfirmed outlier is
    /// refused — with nothing probed or sent.
    #[test]
    fn an_edit_with_an_invalid_value_is_refused_and_writes_nothing() {
        for refused in [
            WeightEntryDraft {
                value_grams: Grams(0),
                ..weight_entry_draft()
            },
            WeightEntryDraft {
                value_grams: Grams(PLAUSIBILITY_BOUND_GRAMS.0 + 1),
                outlier_confirmed: false,
                ..weight_entry_draft()
            },
        ] {
            let (service, state, _rabbit) = state_with_one_rabbit_and_three_entries();
            let calls_before = service.calls();
            let operation = request_id("operation-3");

            let error = block_on_ready(state.update_weight_entry(
                WeightEntryId("entry-2".to_owned()),
                refused,
                operation.clone(),
            ))
            .expect_err("the value is refused");

            assert_eq!(error.cause, SaveCause::Validation);
            assert_eq!(error.request_id, Some(operation));
            assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
        }
    }

    /// DM-4, WT-5: an archived rabbit's entries cannot be edited or deleted, whichever entry point
    /// reaches them; both refuse as `validation` and neither sends anything (D1.5, RAB-8).
    #[test]
    fn an_archived_rabbits_entry_cannot_be_edited_or_deleted() {
        let (service, state, _rabbit) = state_with_an_archived_rabbit_and_entries();

        assert_both_entry_writes_refused(
            &state,
            &service,
            "entry-1",
            SaveCause::Validation,
            "error.validation",
        );
    }

    /// D1.17: a tombstone names an entry that is already removed, so both writes refuse it as an
    /// unknown entry (`validation`) rather than re-submitting or resurrecting it.
    #[test]
    fn a_tombstoned_entry_cannot_be_edited_or_deleted() {
        let rabbit = stored_rabbit("rabbit-1", "Pebble");
        let (service, state) = state_with_copy(vec![
            row_with_rabbit("rabbit-1", "2026-10-22T09:30:00Z", &rabbit),
            tombstone("entry-1", "2026-10-22T10:00:00Z"),
        ]);

        assert_both_entry_writes_refused(
            &state,
            &service,
            "entry-1",
            SaveCause::Validation,
            "error.validation",
        );
    }

    /// D1.16: a live entry row whose content is not on this device cannot be re-submitted as a whole
    /// item, so both writes refuse with the unknown-content key and send nothing.
    #[test]
    fn an_entry_whose_content_is_unknown_cannot_be_edited_or_deleted() {
        let rabbit = stored_rabbit("rabbit-1", "Pebble");
        let (service, state) = state_with_copy(vec![
            row_with_rabbit("rabbit-1", "2026-10-22T09:30:00Z", &rabbit),
            wire("entry-1", "2026-10-21T09:00:00Z"),
        ]);

        assert_both_entry_writes_refused(
            &state,
            &service,
            "entry-1",
            SaveCause::Unknown,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY,
        );
    }

    /// D1.16: the entry is known but its rabbit's content is not, so the archived check cannot be
    /// established and both writes refuse with the unknown-content key instead of writing blind.
    #[test]
    fn an_entry_whose_rabbits_content_is_unknown_cannot_be_edited_or_deleted() {
        let rabbit = stored_rabbit("rabbit-1", "Pebble");
        let (service, state) = state_with_copy(vec![
            wire("rabbit-1", "2026-10-22T09:30:00Z"),
            row_with_entry("entry-1", &rabbit, 1_800, "2026-10-21T09:00:00Z"),
        ]);

        assert_both_entry_writes_refused(
            &state,
            &service,
            "entry-1",
            SaveCause::Unknown,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY,
        );
    }

    /// NFR-10, D1.16: the entry row and its rabbit's row must belong to the same collection. A copy
    /// whose entry is claimed by one household while its rabbit is claimed by another is refused as
    /// `validation` by both writes — neither may "succeed" by writing into the rabbit's household and
    /// leaving the entry's own row untouched, and nothing is sent.
    ///
    /// Household-2 is put in the **household book** with this account as a member, so the unguarded
    /// path can name a writer and reach the write: the refusal has to come from the household
    /// mismatch, not from a book that cannot attribute the write.
    #[test]
    fn an_entry_whose_rabbit_is_claimed_by_another_household_is_refused_by_both_writes() {
        let service = Arc::new(ScriptedService::new().on_write([
            accepted_answer("household-1"),
            accepted_answer("household-2"),
        ]));
        let state = AppState::new(service.clone(), Some(account()));
        let one = household("household-1");
        let two = household("household-2");

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the first household is created and held");
        block_on_ready(state.create_household("Other".to_owned(), request_id("operation-book-2")))
            .expect("the second household is created and held");
        assert_eq!(
            state
                .list_households()
                .iter()
                .map(|household| household.id.clone())
                .collect::<Vec<_>>(),
            vec![one.clone(), two.clone()],
            "both households are in the book"
        );
        let rabbit = Rabbit {
            household_id: two.clone(),
            ..stored_rabbit("rabbit-1", "Pebble")
        };
        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![row_with_entry(
                        "entry-1",
                        &rabbit,
                        1_800,
                        "2026-10-21T09:00:00Z",
                    )],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is household-1's own");
        state
            .load_collection(
                &two,
                wire_collection(
                    &two,
                    vec![row_with_rabbit("rabbit-1", "2026-10-22T09:30:00Z", &rabbit)],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is household-2's own");

        assert_both_entry_writes_refused(
            &state,
            &service,
            "entry-1",
            SaveCause::Validation,
            "error.validation",
        );
    }

    /// D1.16, SYNC-1: a weight entry's item is the row the collection holds, never an id re-derived
    /// from its payload. A row whose payload names another entry is refused by both writes, so the
    /// resolved row's item id stays the single write target and a different item is never edited or
    /// tombstoned in its place.
    #[test]
    fn an_entry_row_whose_payload_names_another_entry_is_refused_by_both_writes() {
        let rabbit = stored_rabbit("rabbit-1", "Pebble");
        let mut misplaced = row_with_entry("entry-1", &rabbit, 1_800, "2026-10-21T09:00:00Z");
        let Some(ItemPayload::WeightEntry { weight_entry }) = &mut misplaced.payload else {
            panic!("the row is an entry");
        };
        weight_entry.id = WeightEntryId("entry-2".to_owned());

        let (service, state) = state_with_copy(vec![
            row_with_rabbit("rabbit-1", "2026-10-22T09:30:00Z", &rabbit),
            misplaced,
        ]);

        assert_both_entry_writes_refused(
            &state,
            &service,
            "entry-1",
            SaveCause::Validation,
            "error.validation",
        );
    }

    /// D1.16: an id that names a rabbit row is not an entry, so both writes refuse it as `validation`.
    #[test]
    fn an_id_that_names_a_rabbit_row_cannot_be_edited_or_deleted_as_an_entry() {
        let rabbit = stored_rabbit("entry-1", "Pebble");
        let (service, state) = state_with_copy(vec![row_with_rabbit(
            "entry-1",
            "2026-10-22T09:30:00Z",
            &rabbit,
        )]);

        assert_both_entry_writes_refused(
            &state,
            &service,
            "entry-1",
            SaveCause::Validation,
            "error.validation",
        );
    }

    /// An entry this device holds no row for — never loaded, or a blank id — is refused as `validation`
    /// and sends nothing; the write never invents a row it cannot read.
    #[test]
    fn an_entry_this_device_holds_no_row_for_cannot_be_edited_or_deleted() {
        let (service, state) = state_with_copy(Vec::new());

        for entry_id in ["entry-1", "   "] {
            assert_both_entry_writes_refused(
                &state,
                &service,
                entry_id,
                SaveCause::Validation,
                "error.validation",
            );
        }
    }

    /// A blank operation identity names no operation, so both entry writes refuse before the copy is
    /// even consulted (D0.14).
    #[test]
    fn a_blank_operation_identity_is_refused_by_the_edit_and_the_delete() {
        let (service, state, _rabbit) = state_with_one_rabbit_and_three_entries();
        let calls_before = service.calls();
        let blank = request_id("   ");

        let edit = block_on_ready(state.update_weight_entry(
            WeightEntryId("entry-2".to_owned()),
            weight_entry_draft(),
            blank.clone(),
        ))
        .expect_err("a blank identity names no operation");
        let deletion = block_on_ready(
            state.delete_weight_entry(WeightEntryId("entry-2".to_owned()), blank.clone()),
        )
        .expect_err("a blank identity names no operation");

        for error in [edit, deletion] {
            assert_eq!(error.cause, SaveCause::Validation);
            assert_eq!(error.message_key, "error.validation");
            assert!(!error.retryable);
            assert_eq!(error.request_id, Some(blank.clone()));
        }

        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    }

    /// Without an account both writes are refused with `no_account` before any probe; the copy is
    /// still consulted first, so the refusal names the real state.
    #[test]
    fn an_entry_edit_without_an_account_is_refused_with_no_account_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let one = household("household-1");
        let rabbit = stored_rabbit("rabbit-1", "Pebble");

        state
            .load_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        row_with_rabbit("rabbit-1", "2026-10-22T09:30:00Z", &rabbit),
                        row_with_entry("entry-1", &rabbit, 1_800, "2026-10-21T09:00:00Z"),
                    ],
                ),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");
        let operation = request_id("operation-2");

        let edit = block_on_ready(state.update_weight_entry(
            WeightEntryId("entry-1".to_owned()),
            weight_entry_draft(),
            operation.clone(),
        ))
        .expect_err("P0 has no account");
        let deletion = block_on_ready(
            state.delete_weight_entry(WeightEntryId("entry-1".to_owned()), operation.clone()),
        )
        .expect_err("P0 has no account");

        for error in [edit, deletion] {
            assert_eq!(error.cause, SaveCause::NoAccount);
            assert_eq!(error.message_key, "error.no_account");
            assert!(error.retryable);
            assert_eq!(error.request_id, Some(operation.clone()));
        }

        assert!(service.calls().is_empty(), "nothing was probed or sent");
    }

    /// A state whose copy holds one rabbit (optionally archived) and one entry row per
    /// `(id, value, timestamp)` — the state the trend/history read resolves from this device's copy
    /// (WT-6).
    fn state_with_weight_series(
        rabbit_archived: bool,
        entries: &[(&str, i32, &str)],
    ) -> (AppState, Rabbit) {
        let (_service, state, rabbit) = state_with_one_rabbit();
        let one = household("household-1");
        let rabbit = Rabbit {
            is_archived: rabbit_archived,
            ..rabbit
        };
        let mut items = vec![row_with_rabbit(
            &rabbit.id.0,
            "2026-10-22T09:30:00Z",
            &rabbit,
        )];
        items.extend(
            entries
                .iter()
                .map(|(id, value, timestamp)| row_with_entry(id, &rabbit, *value, timestamp)),
        );

        state
            .load_collection(
                &one,
                wire_collection(&one, items),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        (state, rabbit)
    }

    /// The entry ids of a read, in the order it returned them.
    fn point_ids(trend: &WeightTrend) -> Vec<&str> {
        trend
            .points
            .iter()
            .map(|point| point.id.0.as_str())
            .collect()
    }

    /// WT-6, "Selecting a range limits the trend to that window": a six-month series read at 30 days
    /// yields only the last 30 days, oldest first, through the real state read.
    #[test]
    fn a_thirty_day_window_limits_the_trend_to_that_window_through_the_read() {
        let (state, rabbit) = state_with_weight_series(
            false,
            &[
                ("entry-1", 1_500, "2026-04-23T00:00:00Z"),
                ("entry-2", 1_600, "2026-09-23T00:00:00Z"),
                ("entry-3", 1_700, "2026-10-01T00:00:00Z"),
                ("entry-4", 1_800, "2026-10-17T00:00:00Z"),
                ("entry-5", 1_900, "2026-10-22T00:00:00Z"),
            ],
        );

        let trend = state
            .weight_trend(&rabbit.id, WeightRange::Days30, &at("2026-10-23T00:00:00Z"))
            .expect("the series is readable");

        assert_eq!(trend.range, WeightRange::Days30);
        assert_eq!(trend.drop_threshold_percent, 5.0);
        assert_eq!(
            point_ids(&trend),
            vec!["entry-2", "entry-3", "entry-4", "entry-5"],
            "the April entry is outside the window and the rest are oldest first"
        );
    }

    /// WT-6, "The full-history range presents every entry": `All` returns the rabbit's whole series.
    #[test]
    fn the_full_history_range_presents_every_entry_through_the_read() {
        let (state, rabbit) = state_with_weight_series(
            false,
            &[
                ("entry-1", 1_500, "2026-01-01T00:00:00Z"),
                ("entry-2", 1_700, "2026-10-01T00:00:00Z"),
                ("entry-3", 1_900, "2026-10-22T00:00:00Z"),
            ],
        );

        let trend = state
            .weight_trend(&rabbit.id, WeightRange::All, &at("2026-10-23T00:00:00Z"))
            .expect("the series is readable");

        assert_eq!(
            point_ids(&trend),
            vec!["entry-1", "entry-2", "entry-3"],
            "the full history has no lower bound"
        );
    }

    /// WT-6, "The weight history range is independent of the trend range": two ranges over the same
    /// rabbit answer their own windows and neither changes the other.
    #[test]
    fn the_trend_and_history_ranges_are_independent_through_the_read() {
        let (state, rabbit) = state_with_weight_series(
            false,
            &[
                ("entry-1", 1_500, "2026-08-01T00:00:00Z"),
                ("entry-2", 1_600, "2026-10-01T00:00:00Z"),
                ("entry-3", 1_800, "2026-10-21T00:00:00Z"),
            ],
        );
        let now = at("2026-10-23T00:00:00Z");

        let trend = state
            .weight_trend(&rabbit.id, WeightRange::Days30, &now)
            .expect("the series is readable");
        let history = state
            .weight_trend(&rabbit.id, WeightRange::Days7, &now)
            .expect("the series is readable");
        let trend_again = state
            .weight_trend(&rabbit.id, WeightRange::Days30, &now)
            .expect("the series is readable");

        assert_eq!(point_ids(&trend), vec!["entry-2", "entry-3"]);
        assert_eq!(point_ids(&history), vec!["entry-3"]);
        assert_eq!(
            trend_again.points, trend.points,
            "the earlier range is unchanged by the later read"
        );
    }

    /// WT-6's boundary: the cut-off is inclusive. Exactly seven days before `now` is inside; one
    /// second earlier is outside.
    #[test]
    fn an_entry_exactly_at_the_seven_day_cutoff_is_inside() {
        let (state, rabbit) = state_with_weight_series(
            false,
            &[
                ("entry-early", 1_500, "2026-10-15T23:59:59Z"),
                ("entry-edge", 1_600, "2026-10-16T00:00:00Z"),
                ("entry-late", 1_700, "2026-10-22T00:00:00Z"),
            ],
        );

        let trend = state
            .weight_trend(&rabbit.id, WeightRange::Days7, &at("2026-10-23T00:00:00Z"))
            .expect("the series is readable");

        assert_eq!(
            point_ids(&trend),
            vec!["entry-edge", "entry-late"],
            "the entry at the cut-off is inside; the one a second earlier is outside"
        );
    }

    /// DASH-7, RAB-8: an archived rabbit's weight series is still readable, so archived is not a
    /// refusal here.
    #[test]
    fn an_archived_rabbits_weight_series_is_readable() {
        let (state, rabbit) =
            state_with_weight_series(true, &[("entry-1", 1_800, "2026-10-22T00:00:00Z")]);

        let trend = state
            .weight_trend(&rabbit.id, WeightRange::All, &at("2026-10-23T00:00:00Z"))
            .expect("an archived rabbit's timeline is viewable");

        assert_eq!(point_ids(&trend), vec!["entry-1"]);
    }

    /// WT-6: an unknown rabbit has no series; the read refuses as `validation` and carries no
    /// operation identity.
    #[test]
    fn an_unknown_rabbit_is_refused_as_validation() {
        let (_service, state, _rabbit) = state_with_one_rabbit();

        let error = state
            .weight_trend(
                &RabbitId("rabbit-unknown".to_owned()),
                WeightRange::All,
                &at("2026-10-23T00:00:00Z"),
            )
            .expect_err("no rabbit has that identity");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.request_id, None);
    }

    /// D1.16: a live row whose content is not on this device could be one of this rabbit's entries,
    /// so the read refuses rather than answering a partial set.
    #[test]
    fn a_live_row_with_unknown_content_refuses_the_weight_read() {
        let (state, rabbit) =
            state_with_weight_series(false, &[("entry-1", 1_800, "2026-10-22T00:00:00Z")]);
        let one = household("household-1");
        let mut items = vec![row_with_rabbit(
            &rabbit.id.0,
            "2026-10-22T09:30:00Z",
            &rabbit,
        )];
        items.push(row_with_entry(
            "entry-1",
            &rabbit,
            1_800,
            "2026-10-22T00:00:00Z",
        ));
        items.push(wire("entry-unknown", "2026-10-22T00:00:00Z"));
        state
            .load_collection(
                &one,
                wire_collection(&one, items),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        let error = state
            .weight_trend(&rabbit.id, WeightRange::All, &at("2026-10-23T00:00:00Z"))
            .expect_err("an unreadable row leaves the entry set partial");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_eq!(error.request_id, None);
    }

    /// D1.16: the returned threshold and every derived value come from the rabbit row, so a live row
    /// whose content is not on this device refuses the read even when every entry is known.
    #[test]
    fn a_rabbit_row_with_unknown_content_refuses_the_weight_read() {
        let (_service, state, rabbit) = state_with_one_rabbit();
        let one = household("household-1");
        state
            .load_collection(
                &one,
                wire_collection(&one, vec![wire(&rabbit.id.0, "2026-10-22T09:30:00Z")]),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        let error = state
            .weight_trend(&rabbit.id, WeightRange::All, &at("2026-10-23T00:00:00Z"))
            .expect_err("the rabbit's content is not on this device");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_eq!(error.request_id, None);
    }

    /// WT-6: an id that names a live item which is not a rabbit names no weight series, so the read
    /// refuses as `validation` with no operation identity.
    #[test]
    fn an_id_that_names_a_non_rabbit_item_is_refused_as_validation() {
        let (_service, state, rabbit) = state_with_one_rabbit();
        let one = household("household-1");
        state
            .load_collection(
                &one,
                wire_collection(&one, vec![row_with_weight_entry("rabbit-1", &rabbit)]),
                at("2026-10-22T09:32:00Z"),
            )
            .expect("the collection is the household's own");

        let error = state
            .weight_trend(
                &RabbitId("rabbit-1".to_owned()),
                WeightRange::All,
                &at("2026-10-23T00:00:00Z"),
            )
            .expect_err("that item is an entry, not a rabbit");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.request_id, None);
    }

    /// D1.17: a tombstoned entry is excluded before the content check, so it neither appears in the
    /// trend nor refuses the read.
    #[test]
    fn a_tombstoned_entry_is_excluded_from_the_trend() {
        let (state, rabbit) =
            state_with_weight_series(false, &[("entry-1", 1_800, "2026-10-22T00:00:00Z")]);
        let one = household("household-1");
        let items = vec![
            row_with_rabbit(&rabbit.id.0, "2026-10-22T09:30:00Z", &rabbit),
            row_with_entry("entry-1", &rabbit, 1_800, "2026-10-22T00:00:00Z"),
            tombstone("entry-2", "2026-10-22T09:00:00Z"),
        ];
        state
            .load_collection(
                &one,
                wire_collection(&one, items),
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        let trend = state
            .weight_trend(&rabbit.id, WeightRange::All, &at("2026-10-23T00:00:00Z"))
            .expect("a tombstone is not unreadable content");

        assert_eq!(point_ids(&trend), vec!["entry-1"]);
    }

    /// A fresh directory under the process's temporary area, unique per call.
    fn temp_data_dir(label: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        let name = format!("bunscale-state-{process}-{sequence}-{label}");
        let directory = std::env::temp_dir().join(name);

        fs::create_dir_all(&directory).expect("a test directory");

        directory
    }

    fn remove_temp_data_dir(directory: &Path) {
        let _cleanup = fs::remove_dir_all(directory);
    }
}
