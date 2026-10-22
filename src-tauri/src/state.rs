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
    HouseholdSummary, Instant, MemberId, Rabbit, RabbitProfile, RequestId,
};
use crate::storage::{
    AbsoluteInstant, BoundaryRefusal, CollectionStore, CollectionStoreError, DeviceSnapshot,
    GuardedStore, PersistedHousehold,
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

/// The one managed state the command adapters borrow.
pub struct AppState {
    /// The shared service, or the scripted double in a test (task 2.4's seam).
    service: Arc<dyn SharedService>,
    /// The current account, or `None` while P0 has no sign-in flow (Q25).
    account: Option<AccountUserId>,
    /// The process's household set and operation ledger (tasks 2.1, 2.2).
    households: Mutex<HouseholdBook>,
    /// The version markers this process minted, keyed by the operation that minted them (D0.14).
    ///
    /// A retry re-issues the same [`RequestId`], and the service refuses the same key with a
    /// *different* payload, so the item write a retry submits has to be byte-identical to the first
    /// attempt's — including the `updated_at` marker last-write-wins compares (D0.7 criteria 3 and
    /// 6). The marker is therefore minted once here and handed back on every later attempt rather
    /// than re-read from the clock. The map is process-lifetime state like the household book's
    /// operation ledger (also unbounded and also not persisted); an entry is one create this process
    /// attempted, and the service's own ledger is what bounds the window that matters.
    markers: Mutex<HashMap<RequestId, Instant>>,
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
            markers: Mutex::new(HashMap::new()),
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
            markers: Mutex::new(HashMap::new()),
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
        let mut markers = lock(&self.markers);

        markers
            .entry(request_id.clone())
            .or_insert_with(|| AbsoluteInstant::now().to_wire())
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
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    use serde_json::json;

    use super::*;
    use crate::ipc::types::{CollectionItem, ItemId, NeuterStatus, RabbitProfile, SaveCause, Sex};
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
