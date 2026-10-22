//! Persistence for the device's loaded collection copies and household book (SYNC-3, D0.2).
//!
//! A copy is a cache of the shared collection, but SYNC-3's "a start with the service unreachable
//! shows what was loaded before" is about a **relaunch**, so the copies have to outlive the process.
//! The household book has the same requirement: after a restart the frontend must still receive the
//! households (and the active id) it needs to ask for a collection at all (HS-2, HS-3). This module
//! is the one place both are written and read: one JSON file in the app's own data directory,
//! addressed by a caller that supplies the directory (exactly as `settings::store` does), so no path
//! is hardcoded here.
//!
//! # The file's content
//!
//! ```json
//! { "format": 1,
//!   "collections": [ { "household_id": "…", "items": [ … ], "loaded_at": "…" } ],
//!   "households": [ { "id": "…", "name": "…", "created_at": "…",
//!                     "owner_member_id": "…", "members": [ … ] } ],
//!   "active_household": "…" }
//! ```
//!
//! Only **loaded** copies are stored: `loaded_at` is required, so a file cannot represent the
//! "rows but never loaded" state the storage boundary deliberately does not construct. The household
//! rows are exactly the fields [`crate::domain::HouseholdRecord::restore`] re-validates: a persisted
//! household is not trusted just because it was written by this app. `format` is the file's own
//! version: a file this version did not write is refused rather than guessed at. Unknown *fields*
//! are ignored so a later phase can extend the file, and the two newer collections are defaulted
//! when an earlier version's file is read.
//!
//! **Instants are validated at decode.** A syntactically valid JSON value whose `loaded_at`, an
//! item's `updated_at` or a household's `created_at` is not an absolute instant is
//! [`CollectionStoreError::Malformed`], not a bogus marker restored into the product: D0.5's rule
//! is enforced at the boundary that reads the file, not only where a value was written.
//!
//! # A failed write is best effort, and the read path says so
//!
//! A load that reached the device is a **successful load even when this file could not be
//! rewritten**: the in-memory copy is the source of truth for the current process, so persistence
//! must not turn a completed load into a read failure, and nothing here reports the copy as
//! persisted. The next launch reads whatever file is actually there; an absent or unreadable file
//! is the explicit never-loaded state (`source: local`, `loaded_at: None`), never a fabricated
//! empty collection. A write is atomic within the destination directory (a temporary sibling is
//! renamed into place), so a launch never sees a half-written file; a write that never finished
//! leaves a `.writing` sibling, and every write sweeps those after [`STALE_TEMPORARY_AGE`].
//!
//! **The operation ledger is not persisted.** It is a process-local mirror of what this process
//! already applied, while D0.14's durable deduplication is the shared service's `request_log`. A
//! create retried after a relaunch is a fresh operation on the service's ledger, and the restored
//! book starts with no local tombstone.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::ipc::types::{
    CollectionItem, CollectionSource, HouseholdCollection, HouseholdId, Instant, Member, MemberId,
};
use crate::storage::AbsoluteInstant;

/// The file name inside the app's own data directory.
pub const COLLECTIONS_FILE_NAME: &str = "loaded-collections.json";

/// The format this module writes and the only one it reads.
const FORMAT_VERSION: u32 = 1;

/// How old a temporary sibling must be before a write treats it as a process that never finished.
///
/// A temporary file lives for as long as one `fs::write` and one `fs::rename` take — microseconds. A
/// minute is far beyond that even on a loaded device, so a sibling at least that old is treated as
/// abandoned, while a younger one may belong to a write another process is performing at this
/// instant and has to be left alone.
const STALE_TEMPORARY_AGE: Duration = Duration::from_secs(60);

/// Everything one device keeps across a relaunch (SYNC-3, HS-3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeviceSnapshot {
    /// The loaded collection copies, each with its load instant.
    pub collections: Vec<HouseholdCollection>,
    /// The held households in acquisition order.
    pub households: Vec<PersistedHousehold>,
    /// The active household's id, when one is active.
    pub active_household: Option<HouseholdId>,
}

/// One persisted household: the fields the domain re-validates on restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedHousehold {
    /// The household's opaque identifier.
    pub id: HouseholdId,
    /// The name as supplied; the domain parses it again on restore.
    pub name: String,
    /// When the household was created.
    pub created_at: Instant,
    /// The member recorded as the owner.
    pub owner_member_id: MemberId,
    /// The member list, creator first.
    pub members: Vec<Member>,
}

/// The device's persisted collections and household book, in one file under a caller-owned
/// directory.
#[derive(Debug, Clone)]
pub struct CollectionStore {
    path: PathBuf,
}

impl CollectionStore {
    /// The store in `data_dir`: the app's own data directory, as
    /// `AppHandle::path().app_data_dir()` resolves it.
    pub fn in_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(COLLECTIONS_FILE_NAME),
        }
    }

    /// The file this store reads and writes.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The device state this file holds, in file order.
    ///
    /// A missing file is the empty snapshot — a device that has never loaded anything and holds no
    /// household — not a failure.
    ///
    /// # Errors
    ///
    /// [`CollectionStoreError::Io`] when the file exists but cannot be read, and
    /// [`CollectionStoreError::Malformed`] when it exists but is not a file this version wrote, or
    /// an instant in it is not an absolute instant.
    pub fn load(&self) -> Result<DeviceSnapshot, CollectionStoreError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(DeviceSnapshot::default());
            }
            Err(error) => return Err(error.into()),
        };

        decode(&bytes)
    }

    /// Write `snapshot` as the whole file, creating the data directory when it does not exist.
    ///
    /// Copies whose `loaded_at` is `None` are skipped: they are not loaded copies, and the file's
    /// shape deliberately cannot represent them.
    ///
    /// # Errors
    ///
    /// [`CollectionStoreError::Io`] when the file cannot be written, and
    /// [`CollectionStoreError::Malformed`] when the snapshot cannot be encoded.
    pub fn save(&self, snapshot: &DeviceSnapshot) -> Result<(), CollectionStoreError> {
        let stored = StoredDevice {
            format: FORMAT_VERSION,
            collections: snapshot
                .collections
                .iter()
                .filter_map(PersistedCollection::from_wire)
                .collect(),
            households: snapshot.households.clone(),
            active_household: snapshot.active_household.clone(),
        };
        let encoded = serde_json::to_vec(&stored).map_err(|_| CollectionStoreError::Malformed)?;

        self.write(encoded)
    }

    /// Write `encoded` as the whole file through an atomic rename.
    fn write(&self, encoded: Vec<u8>) -> Result<(), CollectionStoreError> {
        let parent = self.path.parent();
        let directory = parent.filter(|dir| !dir.as_os_str().is_empty());

        if let Some(directory) = directory {
            fs::create_dir_all(directory)?;
        }

        // Temporaries a write that never finished left behind go first, so this write's own
        // temporary does not exist yet and cannot be swept.
        self.sweep_stale_temporaries();

        let temporary = self.temporary_path();

        if let Err(error) = fs::write(&temporary, encoded) {
            let _cleanup = fs::remove_file(&temporary);

            return Err(error.into());
        }

        if let Err(error) = fs::rename(&temporary, &self.path) {
            let _cleanup = fs::remove_file(&temporary);

            return Err(error.into());
        }

        Ok(())
    }

    /// Remove temporary siblings a write that never finished left behind.
    ///
    /// The pattern is this store's own file name, a dot, and the write's process id and sequence (see
    /// [`CollectionStore::temporary_path`]), so a sibling of any other file is never touched and the
    /// store's own file — which carries no `.writing` — cannot match. A sibling younger than
    /// [`STALE_TEMPORARY_AGE`] is left alone: it may be another process's write in flight.
    fn sweep_stale_temporaries(&self) {
        let Some(directory) = self.path.parent() else {
            return;
        };
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        let prefix = format!("{COLLECTIONS_FILE_NAME}.");

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

/// One persisted copy: the wire rows plus the marker that makes it a *loaded* copy.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedCollection {
    /// The household the copy belongs to.
    household_id: HouseholdId,
    /// The collection's rows, in the frozen wire shape.
    items: Vec<CollectionItem>,
    /// When this device loaded the copy. Required, because only loaded copies are stored.
    loaded_at: Instant,
}

impl PersistedCollection {
    /// A loaded wire collection as the file's row, or `None` when it was never loaded.
    fn from_wire(collection: &HouseholdCollection) -> Option<Self> {
        Some(Self {
            household_id: collection.household_id.clone(),
            items: collection.items.clone(),
            loaded_at: collection.loaded_at.clone()?,
        })
    }

    /// The file's row as the wire value the guarded load restores.
    fn into_wire(self) -> HouseholdCollection {
        HouseholdCollection {
            household_id: self.household_id,
            items: self.items,
            // A restored value is a local read; the guarded load takes the marker it is given and
            // ignores this field, so it is not state the file has to carry.
            source: CollectionSource::Local,
            loaded_at: Some(self.loaded_at),
        }
    }
}

/// The file's content: the format marker, the loaded copies, the book and the active id.
#[derive(Debug, Serialize, Deserialize)]
struct StoredDevice {
    /// The format this file was written in.
    format: u32,
    /// The device's loaded copies.
    #[serde(default)]
    collections: Vec<PersistedCollection>,
    /// The held households.
    #[serde(default)]
    households: Vec<PersistedHousehold>,
    /// The active household's id.
    #[serde(default)]
    active_household: Option<HouseholdId>,
}

/// Why the collection store could not answer.
///
/// This is the store's own classification and never crosses the IPC boundary: the state layer
/// degrades to the in-memory copy (and an empty book) on a write failure, or to the never-loaded
/// state on a read failure, and documents that degradation rather than inventing a command failure.
#[derive(Debug)]
pub enum CollectionStoreError {
    /// The file could not be read or written.
    Io(std::io::Error),
    /// The file's content is not a device file this version can interpret, an instant is not
    /// absolute, or a value could not be encoded into one.
    Malformed,
}

impl std::fmt::Display for CollectionStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "the collection store failed: {error}"),
            Self::Malformed => {
                formatter.write_str("the collection store holds no file this version can interpret")
            }
        }
    }
}

impl std::error::Error for CollectionStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Malformed => None,
        }
    }
}

impl From<std::io::Error> for CollectionStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// The device state one file's bytes hold, or a failure when they are not a file this version wrote.
fn decode(bytes: &[u8]) -> Result<DeviceSnapshot, CollectionStoreError> {
    let parsed = serde_json::from_slice::<StoredDevice>(bytes)
        .map_err(|_| CollectionStoreError::Malformed)?;

    if parsed.format != FORMAT_VERSION {
        return Err(CollectionStoreError::Malformed);
    }

    validate_identifiers(&parsed)?;
    validate_instants(&parsed)?;

    Ok(DeviceSnapshot {
        collections: parsed
            .collections
            .into_iter()
            .map(PersistedCollection::into_wire)
            .collect(),
        households: parsed.households,
        active_household: parsed.active_household,
    })
}

/// Refuse a file whose opaque identifiers are blank.
///
/// Ids are opaque, but a blank one names nothing: a household, member, account or active id must
/// carry at least one non-whitespace character. The refusal is the file's own classification, so a
/// damaged identifier is `Malformed` and the state starts empty rather than restoring an entity the
/// product cannot address.
fn validate_identifiers(stored: &StoredDevice) -> Result<(), CollectionStoreError> {
    fn non_blank(value: &str) -> Result<(), CollectionStoreError> {
        if value.trim().is_empty() {
            Err(CollectionStoreError::Malformed)
        } else {
            Ok(())
        }
    }

    for copy in &stored.collections {
        non_blank(&copy.household_id.0)?;
    }

    for household in &stored.households {
        non_blank(&household.id.0)?;
        non_blank(&household.owner_member_id.0)?;

        for member in &household.members {
            non_blank(&member.id.0)?;
            non_blank(&member.account_user_id.0)?;
        }
    }

    if let Some(active) = &stored.active_household {
        non_blank(&active.0)?;
    }

    Ok(())
}

/// Refuse a file whose markers are not absolute instants (D0.5).
///
/// A missing offset or an empty string is not defaulted to "now" or to the epoch: it makes the whole
/// file malformed, so a damaged value cannot become a plausible load instant or item version.
fn validate_instants(stored: &StoredDevice) -> Result<(), CollectionStoreError> {
    fn absolute(instant: &Instant) -> Result<(), CollectionStoreError> {
        AbsoluteInstant::parse(instant)
            .map(|_| ())
            .map_err(|_| CollectionStoreError::Malformed)
    }

    for copy in &stored.collections {
        absolute(&copy.loaded_at)?;

        for item in &copy.items {
            absolute(&item.updated_at)?;
        }
    }

    for household in &stored.households {
        absolute(&household.created_at)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::json;

    use super::*;
    use crate::ipc::types::{ItemId, ItemPayload, MemberId};

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
    fn rabbit_payload() -> ItemPayload {
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
                is_archived: false,
                weight_drop_threshold_percent: 5.0,
            },
        }
    }

    fn loaded(household_id: &HouseholdId, items: Vec<CollectionItem>) -> HouseholdCollection {
        HouseholdCollection {
            household_id: household_id.clone(),
            items,
            source: CollectionSource::Local,
            loaded_at: Some(at("2026-10-19T11:00:00Z")),
        }
    }

    fn snapshot(collections: Vec<HouseholdCollection>) -> DeviceSnapshot {
        DeviceSnapshot {
            collections,
            ..DeviceSnapshot::default()
        }
    }

    fn persisted_household(id: &str) -> PersistedHousehold {
        let owner = Member {
            id: MemberId("member-1".to_owned()),
            account_user_id: crate::ipc::types::AccountUserId("account-1".to_owned()),
            display_name: None,
        };

        PersistedHousehold {
            id: household(id),
            name: "Home".to_owned(),
            created_at: at("2026-10-19T10:00:00Z"),
            owner_member_id: owner.id.clone(),
            members: vec![owner],
        }
    }

    #[test]
    fn a_store_with_no_file_holds_no_copy_and_that_is_not_a_failure() {
        let directory = temp_data_dir("no-file");
        let store = CollectionStore::in_dir(&directory);

        assert!(
            store
                .load()
                .expect("a missing file is not a failure")
                .collections
                .is_empty()
        );

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_loaded_copy_round_trips_through_the_file() {
        let directory = temp_data_dir("round-trip");
        let store = CollectionStore::in_dir(&directory);
        let one = household("household-1");
        let copies = vec![loaded(&one, vec![item("item-1", "2026-10-19T10:00:00Z")])];

        store
            .save(&snapshot(copies.clone()))
            .expect("the copies are stored");

        let restored = store.load().expect("the copies are readable");
        assert_eq!(restored.collections, copies);

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_payload_round_trips_through_the_file() {
        let directory = temp_data_dir("payload-round-trip");
        let store = CollectionStore::in_dir(&directory);
        let one = household("household-1");
        let row = CollectionItem {
            payload: Some(rabbit_payload()),
            ..item("item-1", "2026-10-19T10:00:00Z")
        };
        let copies = vec![loaded(&one, vec![row])];

        store
            .save(&snapshot(copies.clone()))
            .expect("the copies are stored");

        let restored = store.load().expect("the copies are readable");
        assert_eq!(restored.collections, copies);
        assert_eq!(
            restored.collections[0].items[0].payload,
            Some(rabbit_payload())
        );

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_file_item_without_a_payload_reads_as_unknown_content() {
        // A file written before D1.16 existed carries no payload field at all. It must read as
        // "content unknown", not make the whole file unreadable (SYNC-3, D1.16).
        let directory = temp_data_dir("payload-absent");
        let store = CollectionStore::in_dir(&directory);
        let content = json!({
            "format": 1,
            "collections": [{
                "household_id": "household-1",
                "items": [{ "item_id": "item-1", "updated_at": "2026-10-19T10:00:00Z",
                            "deleted": false }],
                "loaded_at": "2026-10-19T11:00:00Z"
            }]
        });
        fs::write(store.path(), serde_json::to_vec(&content).expect("JSON"))
            .expect("a file from before the field existed");

        let restored = store.load().expect("an absent payload is not malformed");
        assert_eq!(restored.collections[0].items[0].payload, None);

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn the_book_round_trips_with_its_active_id() {
        let directory = temp_data_dir("book");
        let store = CollectionStore::in_dir(&directory);
        let book = DeviceSnapshot {
            households: vec![
                persisted_household("household-1"),
                persisted_household("household-2"),
            ],
            active_household: Some(household("household-1")),
            ..DeviceSnapshot::default()
        };

        store.save(&book).expect("the book is stored");

        assert_eq!(store.load().expect("the book is readable"), book);

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_never_loaded_collection_is_not_persisted() {
        let directory = temp_data_dir("never-loaded");
        let store = CollectionStore::in_dir(&directory);
        let one = household("household-1");
        let never = HouseholdCollection {
            loaded_at: None,
            ..loaded(&one, vec![item("item-1", "2026-10-19T10:00:00Z")])
        };

        store
            .save(&snapshot(vec![never]))
            .expect("the file is written");

        assert!(
            store
                .load()
                .expect("the file is readable")
                .collections
                .is_empty()
        );

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_corrupt_file_is_a_failure_rather_than_an_empty_device() {
        let directory = temp_data_dir("corrupt");
        let store = CollectionStore::in_dir(&directory);

        for content in [
            b"not json at all".as_slice(),
            b"{}".as_slice(),
            br#"{"format":2,"collections":[]}"#.as_slice(),
        ] {
            fs::write(store.path(), content).expect("a foreign file");

            let outcome = store.load();
            assert!(
                matches!(outcome, Err(CollectionStoreError::Malformed)),
                "{content:?}"
            );
        }

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_file_with_a_blank_identifier_is_malformed() {
        let directory = temp_data_dir("bad-ids");
        let store = CollectionStore::in_dir(&directory);
        let household = json!({
            "id": "household-1",
            "name": "Home",
            "created_at": "2026-10-19T10:00:00Z",
            "owner_member_id": "member-1",
            "members": [{
                "id": "member-1",
                "account_user_id": "account-1"
            }]
        });

        for content in [
            // A collection addressed by a blank household id.
            json!({
                "format": 1,
                "collections": [{
                    "household_id": "  ",
                    "items": [],
                    "loaded_at": "2026-10-19T11:00:00Z"
                }]
            }),
            // A blank household id in the book.
            json!({
                "format": 1,
                "households": [{
                    "id": " ",
                    "name": "Home",
                    "created_at": "2026-10-19T10:00:00Z",
                    "owner_member_id": "member-1",
                    "members": [{ "id": "member-1", "account_user_id": "account-1" }]
                }]
            }),
            // A blank owner id.
            json!({
                "format": 1,
                "households": [{
                    "id": "household-1",
                    "name": "Home",
                    "created_at": "2026-10-19T10:00:00Z",
                    "owner_member_id": " ",
                    "members": [{ "id": " ", "account_user_id": "account-1" }]
                }]
            }),
            // A blank member account id.
            json!({
                "format": 1,
                "households": [{
                    "id": "household-1",
                    "name": "Home",
                    "created_at": "2026-10-19T10:00:00Z",
                    "owner_member_id": "member-1",
                    "members": [{ "id": "member-1", "account_user_id": " " }]
                }]
            }),
            // A blank active id.
            json!({
                "format": 1,
                "households": [household],
                "active_household": "  "
            }),
        ] {
            fs::write(store.path(), serde_json::to_vec(&content).expect("JSON"))
                .expect("a foreign file");

            assert!(
                matches!(store.load(), Err(CollectionStoreError::Malformed)),
                "{content}"
            );
        }

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_file_with_a_non_absolute_instant_is_malformed() {
        let directory = temp_data_dir("bad-instants");
        let store = CollectionStore::in_dir(&directory);

        for content in [
            // An empty load marker.
            json!({
                "format": 1,
                "collections": [{ "household_id": "h", "items": [], "loaded_at": "" }]
            }),
            // A local-time value with no offset.
            json!({
                "format": 1,
                "collections": [{
                    "household_id": "h",
                    "items": [],
                    "loaded_at": "2026-10-19T11:00:00"
                }]
            }),
            // A valid marker but an item version that names no moment.
            json!({
                "format": 1,
                "collections": [{
                    "household_id": "h",
                    "items": [{ "item_id": "i", "updated_at": "", "deleted": false }],
                    "loaded_at": "2026-10-19T11:00:00Z"
                }]
            }),
            // A household's creation instant is validated too.
            json!({
                "format": 1,
                "households": [{
                    "id": "h",
                    "name": "Home",
                    "created_at": "2026-10-19T10:00:00",
                    "owner_member_id": "m",
                    "members": []
                }]
            }),
        ] {
            fs::write(store.path(), serde_json::to_vec(&content).expect("JSON"))
                .expect("a foreign file");

            assert!(
                matches!(store.load(), Err(CollectionStoreError::Malformed)),
                "{content}"
            );
        }

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_second_store_over_the_same_directory_reads_what_the_first_wrote() {
        let directory = temp_data_dir("relaunch");
        let one = household("household-1");
        let written = DeviceSnapshot {
            collections: vec![loaded(&one, vec![item("item-1", "2026-10-19T10:00:00Z")])],
            households: vec![persisted_household("household-1")],
            active_household: Some(one),
        };
        CollectionStore::in_dir(&directory)
            .save(&written)
            .expect("the state is stored");

        // Nothing is kept in a process-wide cache: this second store can only answer from the file.
        let after_restart = CollectionStore::in_dir(&directory);

        assert_eq!(
            after_restart.load().expect("the state is readable"),
            written
        );

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_write_sweeps_a_temporary_a_write_that_never_finished_left_behind() {
        let directory = temp_data_dir("sweep");
        let store = CollectionStore::in_dir(&directory);
        let abandoned = directory.join(format!("{COLLECTIONS_FILE_NAME}.1.0.writing"));
        write_stale(&abandoned);

        store
            .save(&DeviceSnapshot::default())
            .expect("the state is stored");

        assert!(!abandoned.exists());
        assert!(
            store
                .load()
                .expect("the state is readable")
                .households
                .is_empty()
        );

        remove_temp_data_dir(&directory);
    }

    #[test]
    fn a_write_leaves_a_fresh_temporary_alone() {
        let directory = temp_data_dir("fresh-temporary");
        let store = CollectionStore::in_dir(&directory);
        let in_flight = directory.join(format!("{COLLECTIONS_FILE_NAME}.2.0.writing"));
        fs::write(&in_flight, b"{}").expect("an in-flight temporary");

        store
            .save(&DeviceSnapshot::default())
            .expect("the state is stored");

        assert!(in_flight.exists());

        remove_temp_data_dir(&directory);
    }

    /// A fresh directory under the process's temporary area, unique per call.
    fn temp_data_dir(label: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        let name = format!("bunscale-collections-{process}-{sequence}-{label}");
        let directory = std::env::temp_dir().join(name);

        fs::create_dir_all(&directory).expect("a test directory");

        directory
    }

    /// Create a file at `path` whose modification time is well past the sweep's threshold.
    fn write_stale(path: &Path) {
        let file = fs::File::create(path).expect("a stale temporary");
        let modified = SystemTime::now() - Duration::from_secs(3_600);
        let times = fs::FileTimes::new().set_modified(modified);

        file.set_times(times).expect("an old modification time");
    }

    fn remove_temp_data_dir(directory: &Path) {
        let _cleanup = fs::remove_dir_all(directory);
    }
}
