//! The rabbit domain: one rabbit's recorded field set, and the one write path a rabbit is created
//! and edited through (RAB-1, RAB-2, RAB-3, RAB-5; D1.1, D1.14, I8).
//!
//! The contract in [`crate::ipc::types`] fixes the *shape* that crosses the boundary — [`Rabbit`],
//! [`RabbitProfile`], [`Sex`], [`NeuterStatus`]. This module fixes what that shape may not violate:
//!
//! * **A rabbit's name is non-blank.** [`validate_name`] refuses `""` and a name that is entirely
//!   whitespace as `validation` before anything is sent, and it is the one rule both this module's
//!   create path and the update path that follows it call. A name with any non-whitespace character
//!   is kept exactly as supplied — the rule decides whether a name exists, it does not edit one
//!   (RAB-2).
//! * **An edit replaces the whole editable set of an existing rabbit.** [`RabbitRecord::updated`]
//!   takes the target's identity and household from the caller and every other field from the
//!   submitted [`RabbitProfile`], so the profile cannot move a rabbit or rename its item — and the
//!   weight-drop threshold is edited like any other profile field (RAB-5, WT-7).
//! * **A profile is recorded field for field, and a field the person did not supply stays absent.**
//!   [`RabbitRecord::created`] copies the submitted profile into the record as it stands: an
//!   unsupplied photo reference, breed, date of birth or adoption date is `None`, never `Some("")`
//!   and never a value the form did not show (RAB-1).
//! * **`dob_unknown: true` requires `dob: None`.** The pair `true` with a date is refused as
//!   `validation` rather than resolved by precedence, because "unknown" and a date cannot both be
//!   true (I8, RAB-1's unknown-date-of-birth scenario).
//! * **A date of birth and an adoption date are calendar dates, carried unchanged.** They are the
//!   frozen [`Instant`] because that is P0's one absolute-time type, but a date is not a moment: the
//!   day a person entered is the day every member and every device shows, so this module never puts
//!   one through [`AbsoluteInstant`](crate::storage::AbsoluteInstant) — which normalises to UTC and
//!   would move `2026-03-01T00:00:00+02:00` back to 28 February — and never rewrites the offset it
//!   was given (I8; DM-6's instant rule governs timestamps, where the moment is the point).
//! * **A rabbit belongs to exactly one household and carries a stable, opaque id.** The id is the
//!   operation's identity (D0.14): the frontend mints one opaque value per state-changing call and
//!   re-uses that same value on a retry, so a retried create re-saves the same item instead of
//!   creating a second rabbit (RAB-3, ADR 0002 criterion 3).
//! * **A household holds any number of rabbits.** Nothing here counts them, caps them or treats one
//!   as *the* rabbit: each create is an independent item write into the household's collection, so a
//!   household that already holds rabbits accepts another one (RAB-3). Two creates are two rabbits
//!   even when their profiles are byte-identical, because their operation identities differ; and a
//!   re-issued create is the same rabbit for the same reason.
//! * **The write names its writer and its version marker.** `save_item` takes both, because the
//!   service's last-write-wins tuple is `(updated_at, writer_id, request_id)` (ADR 0002 criteria 3
//!   and 6). The writer is the member the person is in that household, resolved by the caller from
//!   its book; the marker is minted **once per operation** and re-sent unchanged by a retry, so a
//!   re-issued write is byte-identical and the service's request ledger recognises it as the same
//!   operation rather than refusing it as a changed payload.
//! * **A lifecycle write submits the whole item.** Archiving and restoring read the rabbit's content
//!   from this device's copy, set or clear `is_archived` and change nothing else, and write that
//!   version — a partial payload would replace the stored item and discard the profile (RAB-6,
//!   RAB-7, RAB-9; D1.16).
//! * **A rabbit list is one selection, never a union, and never partial.** [`summaries`] answers the
//!   active list or the archived list from the rows this device holds, and refuses as `unknown`
//!   rather than omitting a row whose content it cannot describe (D1.16, RAB-6).
//! * **Permanent deletion is archived-only and takes the rabbit's weight entries with it.**
//!   [`deletion_plan`] is the one rule: the target is a live **archived** rabbit, every live row must
//!   be describable, and the plan is that rabbit plus every live weight-entry row whose `rabbit_id`
//!   names it. [`delete_rabbit`] then writes a tombstone per item into the one household collection, so
//!   no member resolves the rabbit or its entries afterwards — the removal is shared by construction,
//!   not fanned out. A tombstone keeps the row's identity, the operation's version marker, the writing
//!   member and its payload, and sets the removal flag as `save_item`'s own top-level `deleted`
//!   argument; every read excludes a tombstone before the unknown-content check, so a removal never
//!   becomes an unreadable row (RAB-10; D1.8, D1.16, D1.17). An active rabbit is refused, which is what
//!   keeps archiving the recoverable step before the irreversible one.
//!
//! **Nothing here touches Tauri, and nothing here writes storage.** One item write serves every
//! mutation — [`save_rabbit_version`] for content and lifecycle writes, [`delete_rabbit`]'s
//! [`save_tombstone`] for removals — and both go through P0's reachability-gated [`SaveGate`]: a
//! rabbit exists, changes or is removed once the shared service has accepted the write, so no
//! local-only rabbit and no second write path can be produced here (D1.1, D0.1, RAB-5). What this
//! device holds afterwards is its **copy of the shared collection**, filled by the read path (SYNC-3)
//! — never by this module writing it.

use serde_json::{Value, json};

use crate::ipc::types::{
    CommandError, HouseholdCollection, HouseholdId, Instant, ItemId, ItemPayload, MemberId,
    NeuterStatus, Rabbit, RabbitId, RabbitProfile, RabbitSummary, RequestId, Sex, WeightEntry,
};
use crate::sync::{SaveGate, SaveRequest, SharedService};

/// The shared-service function one rabbit item write calls (ADR 0002 criterion 3).
///
/// It is the operation's schema, owned here rather than by the gate: the transport uses it as the
/// RPC path (`/rest/v1/rpc/<rpc>`) and the gate keeps it opaque (D0.7). One function serves every
/// item write of the household's collection — a rabbit now, a weight entry in the task that owns
/// WT-1 — because the collection stores items, not per-kind endpoints.
pub const SAVE_ITEM_RPC: &str = "save_item";

/// A rabbit's name, once it is known not to be blank (RAB-2).
///
/// The stored value is the input as supplied: leading and trailing whitespace are preserved, just as
/// a person typed them. What the constructor refuses is a name that is **entirely** whitespace,
/// because that is the state RAB-2 calls blank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RabbitName(String);

impl RabbitName {
    /// Parse a name, or answer `None` when it is blank (RAB-2).
    ///
    /// "Blank" means empty after trimming: `""`, `"   "` and `"\t\n"` are all refused, while a name
    /// with any non-whitespace character is accepted.
    pub fn parse(input: impl Into<String>) -> Option<Self> {
        let input = input.into();

        if input.trim().is_empty() {
            None
        } else {
            Some(Self(input))
        }
    }

    /// The name as it will be stored and shown.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validate a rabbit name and report a blank one through the contract's failure surface (RAB-2).
///
/// This is the **one** name rule: [`RabbitRecord::created`] calls it before anything is sent, and the
/// update path reuses it rather than repeating the check. It runs before the account check, so a
/// person with no account who submits a blank name is prompted for a name rather than told that their
/// account is missing — the same order P0's household name takes. A refused name is not retryable,
/// which is the policy [`crate::ipc::failure`] owns rather than a flag chosen here.
pub fn validate_name(
    input: impl Into<String>,
    request_id: &RequestId,
) -> Result<RabbitName, CommandError> {
    RabbitName::parse(input).ok_or_else(|| CommandError::validation(Some(request_id.clone())))
}

/// One rabbit profile as the domain holds it: the RAB-1 field set, once it does not contradict
/// itself.
///
/// The fields are private and [`RabbitRecord::created`] is the only constructor, so a record whose
/// `dob_unknown` contradicts its `dob` is not constructible at all: the rule is a property of the
/// type rather than a check each caller is trusted to have made (the same shape
/// [`crate::domain::household::HouseholdRecord`] uses for its owner).
///
/// It is deliberately not the frozen [`Rabbit`] itself: the wire type is buildable field by field
/// from anywhere, and a value that may carry the contradiction is exactly what I8 forbids. The
/// record is the domain's answer, and [`RabbitRecord::to_wire`] is the one place it becomes the
/// contract's shape.
#[derive(Debug, Clone, PartialEq)]
pub struct RabbitRecord {
    /// The item's opaque, stable identity within its household's collection.
    id: RabbitId,
    /// The household whose collection holds this rabbit (RAB-3).
    household_id: HouseholdId,
    /// The name as supplied, non-blank by construction (RAB-2).
    name: RabbitName,
    /// The photo reference, or `None` for none. Choosing a photo is PHOTO-2 in P2 (D1.11).
    photo_ref: Option<String>,
    /// The breed, or `None`.
    breed: Option<String>,
    /// The calendar date of birth, or `None` when it was not supplied or is marked unknown.
    dob: Option<Instant>,
    /// Whether the date of birth is explicitly unknown (RAB-1).
    dob_unknown: bool,
    /// The recorded sex (D1.14).
    sex: Sex,
    /// The recorded neuter/spay status (D1.14).
    neuter_status: NeuterStatus,
    /// The calendar adoption date, or `None`.
    adoption_date: Option<Instant>,
    /// Whether the rabbit is archived. A created rabbit is never archived: RAB-6's archiving is an
    /// operation of its own.
    is_archived: bool,
    /// The weight-drop threshold percentage (WT-7), as supplied.
    weight_drop_threshold_percent: f64,
}

impl RabbitRecord {
    /// Record one rabbit profile in `household_id` (RAB-1).
    ///
    /// Every supplied value is recorded as it stands, and every unsupplied one is `None` — never an
    /// empty string and never a default the form did not show. The two facts creation fixes are the
    /// item's identity ([`rabbit_id`], the operation's identity) and `is_archived: false`.
    ///
    /// # Errors
    ///
    /// `validation` when the name is blank — empty or entirely whitespace, which RAB-2 treats as the
    /// same refusal — and when the profile marks the date of birth unknown **and** supplies one: the
    /// two cannot both be true, and refusing the pair is the only answer that cannot disagree with
    /// what the form showed (I8). Also `validation` for a blank operation identity, because the item's
    /// id is minted from it and a blank one would name no item at all. Every refusal happens here —
    /// before the caller checks the account — so a request the *input* makes impossible is reported
    /// as itself rather than as a missing account. Each carries the operation's identity, like every
    /// failure of a state-changing command (D0.14), and none sends anything to the shared service.
    pub fn created(
        household_id: HouseholdId,
        profile: RabbitProfile,
        request_id: &RequestId,
    ) -> Result<Self, CommandError> {
        if request_id.0.trim().is_empty() {
            return Err(CommandError::validation(Some(request_id.clone())));
        }

        let name = validate_name(profile.name, request_id)?;

        if profile.dob_unknown && profile.dob.is_some() {
            return Err(CommandError::validation(Some(request_id.clone())));
        }

        Ok(Self {
            id: rabbit_id(request_id),
            household_id,
            name,
            photo_ref: profile.photo_ref,
            breed: profile.breed,
            dob: profile.dob,
            dob_unknown: profile.dob_unknown,
            sex: profile.sex,
            neuter_status: profile.neuter_status,
            adoption_date: profile.adoption_date,
            is_archived: false,
            weight_drop_threshold_percent: profile.weight_drop_threshold_percent,
        })
    }

    /// Replace the editable field set of an already-recorded rabbit (RAB-5).
    ///
    /// An edit **replaces the whole profile** rather than patching single fields: [`RabbitProfile`]
    /// is exactly the editable set — no id, no household, no archived state — so the pre-filled form
    /// submits the full set and no "unchanged" versus "cleared" distinction is needed.
    ///
    /// The two facts an edit may not change come from the target, not from the submission: the item
    /// identity is the [`RabbitId`] the command addressed, and the household is the one whose
    /// collection already holds it. A profile therefore cannot move a rabbit between households or
    /// rename its item.
    ///
    /// **The weight-drop threshold is an ordinary profile field.** Editing it is saved like any other
    /// field, and because the alert is derived from the stored threshold on read (D1.3), the new value
    /// re-derives every alert without rewriting an entry (RAB-5, WT-7).
    ///
    /// # Errors
    ///
    /// `validation` for a blank name — the same [`validate_name`] rule creation uses, whitespace-only
    /// included (RAB-2) — for the contradictory date-of-birth pair (I8), and for a blank operation
    /// identity or a blank rabbit id, because neither names the item being edited. Every refusal
    /// happens here, before the caller checks the account, and none sends anything to the shared
    /// service. Each carries the operation's identity like every failure of a state-changing command
    /// (D0.14).
    pub fn updated(
        household_id: HouseholdId,
        id: RabbitId,
        profile: RabbitProfile,
        request_id: &RequestId,
    ) -> Result<Self, CommandError> {
        if request_id.0.trim().is_empty() || id.0.trim().is_empty() {
            return Err(CommandError::validation(Some(request_id.clone())));
        }

        let name = validate_name(profile.name, request_id)?;

        if profile.dob_unknown && profile.dob.is_some() {
            return Err(CommandError::validation(Some(request_id.clone())));
        }

        Ok(Self {
            id,
            household_id,
            name,
            photo_ref: profile.photo_ref,
            breed: profile.breed,
            dob: profile.dob,
            dob_unknown: profile.dob_unknown,
            sex: profile.sex,
            neuter_status: profile.neuter_status,
            adoption_date: profile.adoption_date,
            // An edit targets an active rabbit (RAB-5). Archiving is task 2.5's operation and the
            // refusal of a write to an archived rabbit is task 2.8's, so no record carries `true`
            // yet, and the archived state is not an input to this constructor — it cannot check it.
            // The refusal belongs in `AppState::update_rabbit`, beside the household resolution;
            // task 2.8 must cover that command as well as the weight-entry writes.
            is_archived: false,
            weight_drop_threshold_percent: profile.weight_drop_threshold_percent,
        })
    }

    /// This record as the frozen wire value (RAB-1).
    ///
    /// Nothing is derived here: the record holds the RAB-1 field set and the wire shape presents the
    /// same one, so a value the person supplied reaches the frontend unchanged and an unsupplied one
    /// stays `None`.
    pub fn to_wire(&self) -> Rabbit {
        Rabbit {
            id: self.id.clone(),
            household_id: self.household_id.clone(),
            name: self.name.as_str().to_owned(),
            photo_ref: self.photo_ref.clone(),
            breed: self.breed.clone(),
            dob: self.dob.clone(),
            dob_unknown: self.dob_unknown,
            sex: self.sex,
            neuter_status: self.neuter_status,
            adoption_date: self.adoption_date.clone(),
            is_archived: self.is_archived,
            weight_drop_threshold_percent: self.weight_drop_threshold_percent,
        }
    }
}

/// The item identity one create operation mints (D0.14, ADR 0002 criterion 3).
///
/// The shared collection keys an item on `(household_id, item_id)` and `save_item` takes the item id
/// from the client, so a rabbit's identity is the client's to mint. The operation's own identity is
/// the value the frontend already mints once per create and re-uses on a retry (D0.14), so using it
/// here is what makes a replayed create the *same* rabbit rather than a second one — and it needs no
/// identifier generator on this side, which is what keeps the dependency list at P0's (project
/// rule 6). Nothing may be parsed out of the value: it is opaque, and its only property is that it
/// is unique per operation.
///
/// That property is also why two rabbits cannot share an id: a second create is a second operation
/// with its own id, **even when the profile is identical piece for piece** — content never names an
/// item — while only a genuine retry repeats an id, and that repeat is the item whose write the
/// service's ledger already holds. A collision would therefore need two distinct operations to mint
/// the same value, which is a violation of the caller's uniqueness rule rather than something this
/// mint can produce; there is no counter, clock or device-local state for it to collide on.
fn rabbit_id(request_id: &RequestId) -> RabbitId {
    RabbitId(request_id.0.clone())
}

/// The collection item id a rabbit is stored under (ADR 0002 criterion 3).
///
/// The identity is the [`RabbitId`]'s opaque string: creation mints the rabbit id from its operation
/// and writes it as `item_id`, so every later write — an edit now, an archive or a deletion in later
/// tasks — addresses that same item through that same value. Nothing else may derive an item id,
/// which is what keeps one rabbit from becoming two rows.
pub fn item_id(rabbit_id: &RabbitId) -> ItemId {
    ItemId(rabbit_id.0.clone())
}

/// Write one recorded rabbit — created or edited — as an item of its household's shared collection
/// (RAB-1, RAB-5, D1.1).
///
/// A create and an edit are the same item write. Creation mints the item's identity from its
/// operation; an edit keeps the rabbit's existing [`RabbitId`], so only the `request_id` is the
/// operation's identity there and the item id is never taken from it (D0.14).
///
/// The write goes through P0's reachability gate: the attempt is refused before anything leaves the
/// device when the service cannot be reached or the account is not usable, and a failure is never
/// presented as a saved rabbit (SYNC-2). A receipt is proof the service accepted the write and
/// nothing more — the operation's own answer schema carries no field this path needs, because the
/// rabbit is fully determined by the request (every field is the one the person supplied, and an
/// edit's identity is the target's).
///
/// The rabbit is written to the **household the caller named**, not to the person's active
/// household: the active household is a per-device selection rather than a scope the backend may
/// infer, which is why the command carries the id explicitly.
///
/// `writer` and `marker` are the two facts `save_item` takes beside the item itself, and both come
/// from the caller because both are per-operation: the writer is the member the person is in that
/// household (resolved from the caller's book), and the marker is minted once per operation so a
/// retry re-sends a byte-identical body ([`crate::state::AppState`] mints it and holds that memo, so
/// this module reads no clock at all).
///
/// # Errors
///
/// The gate's own causes for the attempt — `connectivity`, `no_account`, `allowance_exceeded`,
/// `rate_limited`, `timeout`, `validation` and `unknown` — and `validation` for a blank operation
/// identity, which [`SaveRequest::new`] refuses because a blank key would break the service's
/// deduplication (D0.14). No failure is ever reported as a saved rabbit.
pub async fn save_rabbit(
    service: &dyn SharedService,
    record: &RabbitRecord,
    writer: &MemberId,
    marker: &Instant,
    request_id: RequestId,
) -> Result<(), CommandError> {
    save_rabbit_version(service, &record.to_wire(), writer, marker, request_id).await
}

/// The content one lifecycle write submits: `rabbit` with only its archived flag set (RAB-6).
///
/// Everything else is carried exactly as this device's copy held it — the name, the optional
/// profile fields, the dates and the threshold — because a lifecycle command submits a **whole**
/// item and a partial payload would replace the stored version and discard the rest (RAB-9; D1.16).
pub fn mark_archived(rabbit: Rabbit) -> Rabbit {
    Rabbit {
        is_archived: true,
        ..rabbit
    }
}

/// The content one restore submits: `rabbit` with only its archived flag cleared (RAB-7).
///
/// The mirror of [`mark_archived`], and whole for the same reason: every other field is carried as
/// this device's copy held it, and the weight entries are separate items this write never touches, so
/// a restored rabbit returns with its history unchanged (RAB-7; D1.16).
pub fn mark_restored(rabbit: Rabbit) -> Rabbit {
    Rabbit {
        is_archived: false,
        ..rabbit
    }
}

/// Write one rabbit's whole item as it stands (RAB-6, RAB-7, RAB-9; D1.16).
///
/// A create and an edit build the content from a submitted profile through [`save_rabbit`]; a
/// lifecycle command ([`mark_archived`] and [`mark_restored`] now, and [`delete_rabbit`]'s tombstones)
/// already has the content read from this device's copy and submits it unchanged except for the field
/// it changes. Both are the same one item write through P0's reachability gate, so no second write
/// path is added and a refusal is never reported as a saved rabbit (D1.1, SYNC-2).
///
/// **The write is item-level last-write-wins and that is accepted here** (D0.7 criterion 3): the
/// version marker is the operation's, so a change made to another field after this device's copy was
/// last refreshed — or one carrying an older marker — is replaced by this whole item. D1.16 mandates
/// that model for every whole-item write, so it is not repaired with a pre-write refresh, which is the
/// read path's and task 2.8's decision.
///
/// `writer` and `marker` are the two per-operation facts the caller resolves, exactly as
/// [`save_rabbit`] takes them. The body is a function of its inputs alone, so a retried operation
/// submits byte-identical bytes (D0.7 criterion 6, D0.14).
///
/// # Errors
///
/// The gate's own causes for the attempt, and `validation` for a blank operation identity, which
/// [`SaveRequest::new`] refuses because a blank key would break the service's deduplication (D0.14).
pub async fn save_rabbit_version(
    service: &dyn SharedService,
    rabbit: &Rabbit,
    writer: &MemberId,
    marker: &Instant,
    request_id: RequestId,
) -> Result<(), CommandError> {
    let body = save_body(rabbit, writer, marker, &request_id);
    let request = SaveRequest::new(request_id, SAVE_ITEM_RPC, body)?;

    let _accepted = SaveGate::new(service).save(request).await?;

    Ok(())
}

/// The items one permanent deletion removes: the archived rabbit and every weight entry it owns
/// (RAB-10).
///
/// It is a plan rather than a write so the "what does this delete touch" rule is one value a test can
/// assert without a transport: the rabbit, and one `(item id, content)` per entry that names it. A
/// tombstone keeps identity and version, so the item id is the row's — a weight entry's item is the
/// row the collection holds, never an id re-derived from its payload (D1.16, SYNC-1).
#[derive(Debug, Clone, PartialEq)]
pub struct DeletionPlan {
    /// The archived rabbit, as this device's copy holds it. Its tombstone retains that content, which
    /// is the version that removes it (D1.16).
    pub rabbit: Rabbit,
    /// Every live weight-entry row whose `rabbit_id` is this rabbit's, with the row's item id.
    pub entries: Vec<(ItemId, WeightEntry)>,
}

/// What one permanent deletion would remove, or why it is refused (RAB-10; D1.8, D1.16).
///
/// This is the **one** rule a delete is accepted under, and it is decided entirely from this device's
/// copy so a refusal sends nothing:
///
/// 1. The target must be a live row carrying a rabbit — a tombstoned or absent item is `validation`,
///    because there is no rabbit left to delete.
/// 2. The rabbit must be **archived**: permanent deletion of an active rabbit is refused as
///    `validation`, the deliberate narrowing of RAB-10 that keeps archiving the recoverable step
///    (D1.8).
/// 3. Every live row of the household must be **describable**. A row whose content this device does
///    not hold could be one of this rabbit's entries, so the delete refuses with the unknown-content
///    key rather than removing the rabbit and leaving — or blindly overwriting — a row it might own
///    (D1.16). A tombstone is skipped **before** that check: it is already removed, and a tombstone
///    whose content was never read is stored as unknown content, so checking content first would
///    refuse over a row the collection already reports as gone (D1.17).
///
/// The cascade is the entries whose `rabbit_id` names the target: the link the entry payload already
/// carries, so nothing has to be added to the collection for a delete to work. Its ledger keys are
/// derived by [`cascaded_request_id`] and the derivation is checked there before anything is written
/// (D1.17).
///
/// # Errors
///
/// `validation` for a blank or absent rabbit, a tombstoned one, an item that is not a rabbit, and the
/// active-rabbit refusal; `unknown` with [`HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY`] when the rabbit's
/// row or another live row has no content this device can describe. Both carry the operation's
/// identity, and neither sends anything.
///
/// [`HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY`]: crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
pub fn deletion_plan(
    collection: &HouseholdCollection,
    rabbit_id: &RabbitId,
    request_id: &RequestId,
) -> Result<DeletionPlan, CommandError> {
    let target = item_id(rabbit_id);
    let mut rabbit = None;
    let mut entries = Vec::new();

    for item in &collection.items {
        // A tombstone names an item already removed, so it is neither the target nor a cascade row.
        if item.deleted {
            continue;
        }

        let Some(payload) = &item.payload else {
            // The row exists but its content is not on this device. It could be one of this rabbit's
            // entries, and a whole-item write cannot be composed for it (D1.16).
            return Err(CommandError::household_content_unloaded(Some(
                request_id.clone(),
            )));
        };

        match payload {
            ItemPayload::Rabbit { rabbit: row } if item.item_id == target => {
                rabbit = Some(row.clone());
            }
            ItemPayload::WeightEntry { weight_entry } if weight_entry.rabbit_id == *rabbit_id => {
                entries.push((item.item_id.clone(), weight_entry.clone()));
            }
            _ => {}
        }
    }

    let Some(rabbit) = rabbit else {
        // No live item under that identity is a rabbit: there is nothing to delete.
        return Err(CommandError::validation(Some(request_id.clone())));
    };

    if !rabbit.is_archived {
        // RAB-10's narrowing: only an archived rabbit can be permanently deleted (D1.8).
        return Err(CommandError::validation(Some(request_id.clone())));
    }

    Ok(DeletionPlan { rabbit, entries })
}

/// Permanently remove one archived rabbit and every weight entry [`deletion_plan`] named (RAB-10).
///
/// Each item is written as a **tombstone** through the same one item write and the same reachability
/// gate as every other rabbit write (D1.1): identity and version are kept, the content is the version
/// that removes it, and `deleted` is set — so the removal appears in the household's change feed
/// instead of a row silently vanishing, and every member resolves the household without it (D0.7
/// criterion 2, SYNC-1, HS-8).
///
/// **The entries are written before the rabbit.** If the cascade stops part-way, the person is left
/// with the archived rabbit they can retry from, never a removed rabbit whose entries are still
/// readable — RAB-10's "its weight entries are no longer retrievable" is only ever true because all of
/// them were removed first. A retry re-sends identical bodies and the service's ledger completes the
/// remainder (D0.14).
///
/// **Each item write has its own request identity.** The service deduplicates on the operation's
/// `request_id` and refuses the same key with a different payload, so a fan-out cannot re-use it: the
/// rabbit's tombstone carries the operation's id unchanged, and each entry's is derived from the
/// operation id and that entry's own item id by [`cascaded_request_id`]. The derivation is
/// deterministic, so a retry submits the same ids, and no value is parsed out of either id.
///
/// **The version marker and the writer are the operation's.** Every tombstone carries the marker the
/// caller minted once for the operation, so a retry is byte-identical, and the writer is the member
/// the person is in that household ([`crate::state::AppState`] resolves both). The entry content is
/// normalised on the way out: `has_weight_drop_alert` is written `false`, because it is derived on read
/// and never content (D1.16).
///
/// **One item's tombstone that did not apply stops the cascade, and the rabbit is never tombstoned
/// behind it.** The service reports a losing last-write-wins write as a completed call carrying
/// `applied: false` rather than as an error (D0.7 criterion 3), so the receipt is read on every step:
/// a stale copy or a skewed clock can make an entry's removal lose to a newer version, and tombstoning
/// the rabbit afterwards would leave a readable entry with no rabbit — the state the entries-first
/// order exists to prevent. The cascade stops at that write and reports `unknown` against the
/// operation, never as removed (RAB-10).
///
/// **The ledger keys are derived, and the derivation is checked before the first write** (D1.17): the
/// rabbit's tombstone carries the operation's `request_id` unchanged, and each entry's is
/// `<operation>/<entry item id>`. That is injective only while no id contains `/`, so a breaking id
/// refuses the whole operation as `validation` before anything is sent: the operation id once, whatever
/// the cascade size, and every entry item id before the first write — rather than let one ledger key
/// claim two payloads and stop the cascade for good.
///
/// # What a retry does (D0.14)
///
/// A **refused** step (`connectivity`, `no_account`, `allowance_exceeded`, `rate_limited`, `timeout`)
/// leaves the archived rabbit in place: re-issuing the same `request_id` re-sends byte-identical
/// tombstones, the entries already removed deduplicate per item, and the remainder completes. A step
/// that **did not apply** is different: the version marker is memoized for the operation, so the same
/// `request_id` re-sends the same losing marker and loses again. The failure is therefore `unknown`
/// and not retryable; a fresh read brings the winning version into this device's copy, and a new
/// deletion operation then removes it.
///
/// # Errors
///
/// The gate's own causes for any of the writes (`connectivity`, `no_account`, `allowance_exceeded`,
/// `rate_limited`, `timeout`, `validation`, `unknown`), and `validation` for an id the ledger-key
/// derivation cannot keep injective (D1.17), reported against the **operation's** identity so the
/// surface's retry re-issues the command the person ran rather than one entry's derived key (D0.14).
/// A refusal leaves the rabbit archived and is never reported as removed; the entries already
/// tombstoned stay removed.
pub async fn delete_rabbit(
    service: &dyn SharedService,
    household_id: &HouseholdId,
    plan: &DeletionPlan,
    writer: &MemberId,
    marker: &Instant,
    request_id: RequestId,
) -> Result<(), CommandError> {
    // D1.17: the operation id is a component of every derived key, and it is also the rabbit's own raw
    // ledger key when there is nothing to cascade — so a separator in it refuses the operation whatever
    // the cascade size, before anything is removed.
    if request_id.0.contains('/') {
        return Err(CommandError::validation(Some(request_id)));
    }

    // Every entry key is derived before the first write: a malformed item id must refuse the operation
    // while nothing has been removed, not fail half-way through the cascade (D1.17).
    let mut entry_request_ids = Vec::with_capacity(plan.entries.len());
    for (entry_item_id, _entry) in &plan.entries {
        let Some(entry_request_id) = cascaded_request_id(&request_id, entry_item_id) else {
            return Err(CommandError::validation(Some(request_id)));
        };
        entry_request_ids.push(entry_request_id);
    }

    for ((entry_item_id, entry), entry_request_id) in plan.entries.iter().zip(entry_request_ids) {
        let payload = ItemPayload::WeightEntry {
            weight_entry: WeightEntry {
                // Derived on read, never content: the boundary writes it `false` (D1.16).
                has_weight_drop_alert: false,
                ..entry.clone()
            },
        };

        if let Err(error) = save_tombstone(
            service,
            household_id,
            entry_item_id,
            &payload,
            writer,
            marker,
            entry_request_id,
        )
        .await
        {
            // A cascade step's failure is the operation's, not the entry's: the retry the surface
            // offers re-issues the operation's `request_id` (D0.14). Every cause this can be is the
            // gate's, so `for_cause` restores that cause's own key without inventing one.
            return Err(CommandError::for_cause(error.cause, Some(request_id)));
        }
    }

    let payload = ItemPayload::Rabbit {
        rabbit: plan.rabbit.clone(),
    };

    save_tombstone(
        service,
        household_id,
        &item_id(&plan.rabbit.id),
        &payload,
        writer,
        marker,
        request_id,
    )
    .await
}

/// The request identity one cascaded item tombstone carries (D0.14, D1.17).
///
/// The service's ledger is keyed on the operation's `request_id` and refuses the same key with a
/// different payload, so the writes a cascade submits cannot share it. D1.17 records the rule:
/// `<operation>/<entry item id>`, deterministic so a retry resubmits the same keys.
///
/// **The rule is injective only because no id contains `/`** (D1.17). With either component free of
/// the separator, `(operation, item) -> operation + "/" + item` is injective and a derived key can
/// never equal a raw operation id; a component that contains it can make two distinct pairs produce
/// one key, which the ledger would refuse as the same key with two payloads. The format is therefore
/// checked here, where the derivation is built: `None` means the pair cannot be given a distinct key,
/// and the caller refuses the operation before writing anything rather than trusting the minter.
fn cascaded_request_id(operation: &RequestId, item_id: &ItemId) -> Option<RequestId> {
    if operation.0.contains('/') || item_id.0.contains('/') {
        return None;
    }

    Some(RequestId(format!("{}/{}", operation.0, item_id.0)))
}

/// Write one item's tombstone through P0's reachability gate (RAB-10, D1.1).
///
/// It is the whole-item write of [`save_rabbit_version`] with two differences: the removal flag is set,
/// and the payload is any [`ItemPayload`], because a deletion removes entries as well as rabbits. The
/// content is carried so the removing version retains it (D1.16, D1.17).
///
/// A completed call is not automatically a removal: a losing last-write-wins write answers with
/// `applied: false` and is not an error (D0.7 criterion 3), so [`applied`] reads the receipt and a
/// tombstone that did not take effect is reported as `unknown` under the key it was written with.
async fn save_tombstone(
    service: &dyn SharedService,
    household_id: &HouseholdId,
    item_id: &ItemId,
    payload: &ItemPayload,
    writer: &MemberId,
    marker: &Instant,
    request_id: RequestId,
) -> Result<(), CommandError> {
    let body = tombstone_body(household_id, item_id, payload, writer, marker, &request_id);
    let request = SaveRequest::new(request_id, SAVE_ITEM_RPC, body)?;

    let receipt = SaveGate::new(service).save(request).await?;

    if !applied(receipt.body()) {
        // The version marker is the operation's and is memoized, so re-issuing it loses again: this is
        // not a retryable condition (D0.14). A fresh read brings the winning version in; a new
        // operation removes it.
        return Err(CommandError::unknown(Some(receipt.request_id().clone())));
    }

    Ok(())
}

/// Whether the service's write answer states that the version applied.
///
/// A losing last-write-wins write is a completed call: the service returns the winning item with
/// `applied: false` and treats it as no error (D0.7 criterion 3). The flag is therefore read only when
/// it is stated: an answer whose envelope reports the write completed (`applied` or `deduplicated`)
/// without an `applied` field is accepted as a completed write, because P0's recorded loss shape is the
/// explicit `false`. Only that explicit `false` is a write that did not apply.
fn applied(body: &Value) -> bool {
    body.get("applied").and_then(Value::as_bool).unwrap_or(true)
}

/// The body the shared service's `save_item` receives for one removed item (D1.16, D1.17).
///
/// The RPC body is the function's named-argument list, so the removal flag is its own top-level
/// argument: `deleted` is set here and defaults to `false` on the service for every non-removal write
/// ([`save_body`] omits it), which is what keeps the six-argument body valid (D1.17). The rest is
/// [`save_body`]'s argument list: the collection and item it removes, the version marker
/// last-write-wins compares, the writing member, the operation's identity, and the content the
/// removing version retains. Every field is a function of the inputs, so a retry submits the same
/// bytes (D0.7 criterion 6, D0.14).
fn tombstone_body(
    household_id: &HouseholdId,
    item_id: &ItemId,
    payload: &ItemPayload,
    writer: &MemberId,
    marker: &Instant,
    request_id: &RequestId,
) -> Value {
    json!({
        "request_id": request_id.0,
        "household_id": household_id.0,
        "item_id": item_id.0,
        "updated_at": marker.0,
        "deleted": true,
        "writer_id": writer.0,
        "payload": payload,
    })
}

/// The rabbit summaries of one list: the active rabbits, or the archived ones (RAB-6, D1.16).
///
/// The two values select **one** list each, never a union: `false` is the active list and `true` is
/// the archived list, exactly as `list_rabbits`'s contract states. A row is described only when its
/// content is a rabbit, so weight-entry rows of the same household collection are not rows of either
/// list, and a tombstone is in neither — a permanently deleted rabbit is absent from every rabbit
/// list (RAB-10).
///
/// # Errors
///
/// `unknown` (with no operation identity, because this is a read) when a live row carries no content
/// this version can describe. The row could be a rabbit this device cannot read, and omitting it
/// would present a partial list as the household's own — SYNC-1's "every item exactly once" rules
/// that out, so the read refuses rather than answering incompletely (D1.16).
pub fn summaries(
    collection: &HouseholdCollection,
    include_archived: bool,
) -> Result<Vec<RabbitSummary>, CommandError> {
    let mut summaries = Vec::new();

    for item in &collection.items {
        // A tombstone names a removed item, so no rabbit list describes it. Its retained content is
        // never read as a rabbit (D1.16, RAB-10).
        if item.deleted {
            continue;
        }

        match &item.payload {
            Some(ItemPayload::Rabbit { rabbit }) => {
                if rabbit.is_archived == include_archived {
                    summaries.push(RabbitSummary {
                        id: rabbit.id.clone(),
                        name: rabbit.name.clone(),
                        thumbnail_ref: rabbit.photo_ref.clone(),
                        is_archived: rabbit.is_archived,
                    });
                }
            }
            // A weight entry is a row this version models, but it is not a rabbit.
            Some(ItemPayload::WeightEntry { .. }) => {}
            None => return Err(CommandError::household_content_unloaded(None)),
        }
    }

    Ok(summaries)
}

/// The body the shared service's `save_item` receives for one rabbit item (ADR 0002 criterion 3).
///
/// It is the function's own argument list: the collection and item it writes, the version marker
/// last-write-wins compares, the writing member, the operation's identity, and the payload. Every one
/// of them is repeated here because the service's ledger and its LWW tuple read them from the call
/// (D0.7 criteria 3 and 6, D0.14). `payload` is the rabbit itself, so every recorded field reaches
/// the shared collection unchanged — and every field the person did not supply reaches it as `null`,
/// which is the contract's own representation of an absent optional field rather than an empty
/// string or a default.
///
/// **The body is a function of its inputs alone.** It reads no clock and derives nothing from the
/// environment, so a retry that supplies the same rabbit, writer, marker and identity submits the
/// same bytes — which is what the service's request ledger requires of a re-issued operation (D0.7
/// criterion 6, D0.14).
///
/// The removal flag is **omitted**, not sent `false`: `save_item`'s `deleted` defaults to `false`
/// (D1.17), so a non-removal write keeps the six-argument body and the service's default is the only
/// place that state is stated. [`tombstone_body`] is the body that sets it.
fn save_body(
    rabbit: &Rabbit,
    writer: &MemberId,
    marker: &Instant,
    request_id: &RequestId,
) -> Value {
    json!({
        "request_id": request_id.0,
        "household_id": rabbit.household_id.0,
        "item_id": item_id(&rabbit.id).0,
        "updated_at": marker.0,
        "writer_id": writer.0,
        "payload": {
            "kind": "rabbit",
            "rabbit": rabbit,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::types::{
        CollectionItem, CollectionSource, Grams, SaveCause, WeightEntry, WeightEntryId,
    };
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::double::{RecordedCall, ScriptedService};
    use crate::sync::{Answer, ServiceReply};

    fn request_id(value: &str) -> RequestId {
        RequestId(value.to_owned())
    }

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn member(value: &str) -> MemberId {
        MemberId(value.to_owned())
    }

    /// The member the test's household records for the account that writes.
    fn writer() -> MemberId {
        member("member-1")
    }

    /// The version marker one operation minted, as the caller hands it in.
    fn marker() -> Instant {
        at("2026-10-22T09:30:00Z")
    }

    /// A profile with every RAB-1 field supplied.
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

    /// A profile supplying only the two fields that are always present: a name and a sex.
    fn minimal_profile() -> RabbitProfile {
        RabbitProfile {
            name: "Pebble".to_owned(),
            photo_ref: None,
            breed: None,
            dob: None,
            dob_unknown: false,
            sex: Sex::Male,
            neuter_status: NeuterStatus::Unknown,
            adoption_date: None,
            weight_drop_threshold_percent: 5.0,
        }
    }

    /// Create one rabbit exactly as the command path does: record it, then write it through the gate
    /// with the operation's writer and version marker.
    fn create_with(
        service: &ScriptedService,
        household_id: &HouseholdId,
        profile: RabbitProfile,
        request_id: RequestId,
    ) -> Result<Rabbit, CommandError> {
        create_with_marker(service, household_id, profile, marker(), request_id)
    }

    /// As [`create_with`], with the version marker the operation minted supplied explicitly.
    fn create_with_marker(
        service: &ScriptedService,
        household_id: &HouseholdId,
        profile: RabbitProfile,
        marker: Instant,
        request_id: RequestId,
    ) -> Result<Rabbit, CommandError> {
        let record = RabbitRecord::created(household_id.clone(), profile, &request_id)?;

        block_on_ready(save_rabbit(
            service,
            &record,
            &writer(),
            &marker,
            request_id,
        ))?;

        Ok(record.to_wire())
    }

    /// The one write the transport was asked to submit.
    fn written_body(service: &ScriptedService) -> Value {
        let writes: Vec<SaveRequest> = service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RecordedCall::Save { request, .. } => Some(request),
                _ => None,
            })
            .collect();

        assert_eq!(writes.len(), 1, "one create submits exactly one write");
        assert_eq!(writes[0].rpc(), SAVE_ITEM_RPC);

        writes[0].body().clone()
    }

    /// One value out of the written payload's rabbit.
    fn written_field(service: &ScriptedService, field: &str) -> Value {
        written_body(service)["payload"]["rabbit"][field].clone()
    }

    /// Every write the transport was asked to submit, in call order.
    fn written_bodies(service: &ScriptedService) -> Vec<Value> {
        service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RecordedCall::Save { request, .. } => Some(request.body().clone()),
                _ => None,
            })
            .collect()
    }

    /// Edit one rabbit exactly as the command path does: build the replacement record for the
    /// existing identity, then write it through the same gate a create uses.
    fn update_with(
        service: &ScriptedService,
        household_id: &HouseholdId,
        rabbit_id: RabbitId,
        profile: RabbitProfile,
        request_id: RequestId,
    ) -> Result<Rabbit, CommandError> {
        let record = RabbitRecord::updated(household_id.clone(), rabbit_id, profile, &request_id)?;

        block_on_ready(save_rabbit(
            service,
            &record,
            &writer(),
            &marker(),
            request_id,
        ))?;

        Ok(record.to_wire())
    }

    #[test]
    fn a_profile_is_created_with_the_supplied_identifying_details() {
        let service = ScriptedService::new();
        let one = household("household-1");

        let rabbit = create_with(&service, &one, full_profile(), request_id("operation-1"))
            .expect("the double accepted the write");

        // Every supplied value is recorded, and the two facts creation fixes are set.
        assert_eq!(rabbit.name, "Pebble");
        assert_eq!(rabbit.photo_ref, Some("photo-1".to_owned()));
        assert_eq!(rabbit.breed, Some("Mini Lop".to_owned()));
        assert_eq!(rabbit.dob, Some(at("2026-03-01T00:00:00Z")));
        assert!(!rabbit.dob_unknown);
        assert_eq!(rabbit.sex, Sex::Female);
        assert_eq!(rabbit.neuter_status, NeuterStatus::Spayed);
        assert_eq!(rabbit.adoption_date, Some(at("2026-05-01T00:00:00Z")));
        assert_eq!(rabbit.household_id, one);
        assert!(!rabbit.is_archived);
        assert_eq!(rabbit.weight_drop_threshold_percent, 5.0);

        // The rabbit is the household's item: the write names the collection it belongs to, the item
        // it writes, the member writing it and the operation's version marker, and the payload carries
        // the same field set (ADR 0002 criteria 3 and 6).
        let body = written_body(&service);
        assert_eq!(body["household_id"], json!(one.0));
        assert_eq!(body["item_id"], json!(rabbit.id.0));
        assert_eq!(body["request_id"], json!("operation-1"));
        assert_eq!(body["writer_id"], json!(writer().0));
        assert_eq!(body["updated_at"], json!(marker().0));
        assert_eq!(written_field(&service, "name"), json!("Pebble"));
        assert_eq!(written_field(&service, "breed"), json!("Mini Lop"));
        assert_eq!(
            written_field(&service, "dob"),
            json!("2026-03-01T00:00:00Z")
        );
        assert_eq!(written_field(&service, "sex"), json!("female"));
        assert_eq!(written_field(&service, "neuter_status"), json!("spayed"));
        assert_eq!(
            written_field(&service, "adoption_date"),
            json!("2026-05-01T00:00:00Z")
        );
        assert_eq!(service.write_count(), 1);
    }

    #[test]
    fn optional_fields_may_be_left_unset() {
        let service = ScriptedService::new();

        let rabbit = create_with(
            &service,
            &household("household-1"),
            minimal_profile(),
            request_id("operation-1"),
        )
        .expect("a profile with only a name and a sex is savable");

        // Absent, not empty and not defaulted.
        assert_eq!(rabbit.name, "Pebble");
        assert_eq!(rabbit.sex, Sex::Male);
        assert_eq!(rabbit.photo_ref, None);
        assert_eq!(rabbit.breed, None);
        assert_eq!(rabbit.dob, None);
        assert_eq!(rabbit.adoption_date, None);
        assert!(
            !rabbit.dob_unknown,
            "not supplied is not the same as unknown"
        );

        // The same absence reaches the shared collection: `null`, never `""` and never a value the
        // form did not show.
        for field in ["photo_ref", "breed", "dob", "adoption_date"] {
            assert_eq!(written_field(&service, field), Value::Null, "{field}");
        }
        assert_eq!(written_field(&service, "dob_unknown"), json!(false));
    }

    #[test]
    fn date_of_birth_may_be_marked_unknown() {
        let service = ScriptedService::new();
        let profile = RabbitProfile {
            dob: None,
            dob_unknown: true,
            ..minimal_profile()
        };

        let rabbit = create_with(
            &service,
            &household("household-1"),
            profile,
            request_id("operation-1"),
        )
        .expect("an unknown date of birth is a savable profile");

        assert_eq!(rabbit.dob, None);
        assert!(
            rabbit.dob_unknown,
            "the profile records the date as unknown rather than as absent"
        );
        assert_eq!(written_field(&service, "dob"), Value::Null);
        assert_eq!(written_field(&service, "dob_unknown"), json!(true));
    }

    #[test]
    fn an_unknown_date_of_birth_with_a_date_is_refused_and_nothing_is_sent() {
        let service = ScriptedService::new();
        let request_id = request_id("operation-1");
        let contradictory = RabbitProfile {
            dob: Some(at("2026-03-01T00:00:00Z")),
            dob_unknown: true,
            ..minimal_profile()
        };

        let error = create_with(
            &service,
            &household("household-1"),
            contradictory,
            request_id.clone(),
        )
        .expect_err("unknown and a date cannot both be true");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));

        // The refusal is a property of the input: nothing was probed and nothing was sent.
        assert!(service.calls().is_empty());
        assert_eq!(service.write_count(), 0);
    }

    #[test]
    fn an_empty_name_is_refused_and_nothing_is_sent() {
        let service = ScriptedService::new();
        let request_id = request_id("operation-1");

        let error = create_with(
            &service,
            &household("household-1"),
            RabbitProfile {
                name: String::new(),
                ..minimal_profile()
            },
            request_id.clone(),
        )
        .expect_err("a rabbit needs a name");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));

        // A property of the input: nothing was probed and nothing was sent (RAB-2).
        assert!(service.calls().is_empty());
        assert_eq!(service.write_count(), 0);
    }

    #[test]
    fn a_whitespace_only_name_is_treated_as_blank_and_refused() {
        let service = ScriptedService::new();
        let request_id = request_id("operation-1");

        // "Blank" is empty after trimming, so every one of these names is refused (RAB-2).
        for blank in [" ", "   ", "\t", "\n", " \t\n "] {
            let error = create_with(
                &service,
                &household("household-1"),
                RabbitProfile {
                    name: blank.to_owned(),
                    ..minimal_profile()
                },
                request_id.clone(),
            )
            .expect_err("whitespace is not a name");

            assert_eq!(error.cause, SaveCause::Validation, "{blank:?}");
            assert_eq!(error.message_key, "error.validation", "{blank:?}");
            assert_eq!(error.request_id, Some(request_id.clone()), "{blank:?}");
        }

        assert!(service.calls().is_empty());
        assert_eq!(service.write_count(), 0);
    }

    #[test]
    fn a_name_with_a_non_whitespace_character_is_recorded_exactly_as_supplied() {
        let service = ScriptedService::new();

        let rabbit = create_with(
            &service,
            &household("household-1"),
            RabbitProfile {
                name: "  Pebble  ".to_owned(),
                ..minimal_profile()
            },
            request_id("operation-1"),
        )
        .expect("one non-whitespace character makes a name");

        // The rule decides whether a name exists; it does not edit what the person typed.
        assert_eq!(rabbit.name, "  Pebble  ");
        assert_eq!(written_field(&service, "name"), json!("  Pebble  "));
    }

    #[test]
    fn a_neutered_and_a_spayed_rabbit_are_recorded_distinctly() {
        let service = ScriptedService::new();
        let one = household("household-1");

        let neutered = create_with(
            &service,
            &one,
            RabbitProfile {
                neuter_status: NeuterStatus::Neutered,
                ..minimal_profile()
            },
            request_id("operation-1"),
        )
        .expect("the first profile is created");
        let spayed = create_with(
            &service,
            &one,
            RabbitProfile {
                neuter_status: NeuterStatus::Spayed,
                ..minimal_profile()
            },
            request_id("operation-2"),
        )
        .expect("the second profile is created");

        assert_eq!(neutered.neuter_status, NeuterStatus::Neutered);
        assert_eq!(spayed.neuter_status, NeuterStatus::Spayed);
        assert_ne!(
            neutered.neuter_status, spayed.neuter_status,
            "the two statuses are not collapsed into one fixed value"
        );

        // Each write carries its own status, so the distinction survives the trip to the collection.
        let statuses: Vec<Value> = service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RecordedCall::Save { request, .. } => {
                    Some(request.body()["payload"]["rabbit"]["neuter_status"].clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(statuses, vec![json!("neutered"), json!("spayed")]);
    }

    #[test]
    fn sex_and_neuter_status_may_each_be_recorded_as_unknown() {
        let service = ScriptedService::new();

        let rabbit = create_with(
            &service,
            &household("household-1"),
            RabbitProfile {
                sex: Sex::Unknown,
                neuter_status: NeuterStatus::Unknown,
                ..minimal_profile()
            },
            request_id("operation-1"),
        )
        .expect("an unstated sex and neuter status are savable");

        // Recorded values, not absences and not invented ones.
        assert_eq!(rabbit.sex, Sex::Unknown);
        assert_eq!(rabbit.neuter_status, NeuterStatus::Unknown);
        assert_eq!(written_field(&service, "sex"), json!("unknown"));
        assert_eq!(written_field(&service, "neuter_status"), json!("unknown"));
    }

    #[test]
    fn a_calendar_date_is_recorded_as_supplied_and_never_timezone_shifted() {
        let service = ScriptedService::new();
        // A date pinned to +02:00: normalising it to UTC would move it to 28 February, which is the
        // day-shift I8 forbids. The day a person entered is the day every member must see.
        let profile = RabbitProfile {
            dob: Some(at("2026-03-01T00:00:00+02:00")),
            adoption_date: Some(at("2026-05-01T00:00:00+02:00")),
            ..minimal_profile()
        };

        let rabbit = create_with(
            &service,
            &household("household-1"),
            profile,
            request_id("operation-1"),
        )
        .expect("the double accepted the write");

        assert_eq!(rabbit.dob, Some(at("2026-03-01T00:00:00+02:00")));
        assert_eq!(rabbit.adoption_date, Some(at("2026-05-01T00:00:00+02:00")));
        assert_eq!(
            written_field(&service, "dob"),
            json!("2026-03-01T00:00:00+02:00"),
            "the date is carried unchanged, never re-derived at another offset"
        );
        assert_eq!(
            written_field(&service, "adoption_date"),
            json!("2026-05-01T00:00:00+02:00")
        );
        assert_ne!(rabbit.dob, Some(at("2026-02-28T22:00:00Z")));
    }

    #[test]
    fn a_created_rabbit_carries_its_operation_as_a_stable_item_identity() {
        let service = ScriptedService::new();
        let operation = request_id("operation-1");

        let first = create_with(
            &service,
            &household("household-1"),
            minimal_profile(),
            operation.clone(),
        )
        .expect("the first create completed");
        // The retry of that same operation: the same identity, so the same item (D0.14).
        let replay = create_with(
            &service,
            &household("household-1"),
            minimal_profile(),
            operation.clone(),
        )
        .expect("the re-issued create completed");

        assert_eq!(first.id, RabbitId(operation.0.clone()));
        assert_eq!(
            first.id, replay.id,
            "a retry is the same rabbit, never a second one"
        );

        // Both writes name that one item, so the service's `(household_id, item_id)` key sees one
        // rabbit however often the create is re-issued.
        let item_ids: Vec<Value> = service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RecordedCall::Save { request, .. } => Some(request.body()["item_id"].clone()),
                _ => None,
            })
            .collect();
        assert_eq!(item_ids, vec![json!(first.id.0), json!(first.id.0)]);
    }

    #[test]
    fn a_retried_create_submits_a_byte_identical_write() {
        let service = ScriptedService::new();
        let operation = request_id("operation-1");
        let marker = marker();
        let household_id = household("household-1");

        // The first attempt and the retry: the same operation, the same item, and — because the
        // caller hands back the marker it minted for the operation — the same body. The service's
        // ledger refuses a re-issued key with a different payload, so identical bytes are what makes
        // the retry safe (D0.7 criterion 6, D0.14).
        create_with_marker(
            &service,
            &household_id,
            minimal_profile(),
            marker.clone(),
            operation.clone(),
        )
        .expect("the first create completed");
        create_with_marker(
            &service,
            &household_id,
            minimal_profile(),
            marker,
            operation,
        )
        .expect("the re-issued create completed");

        let bodies: Vec<Value> = service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RecordedCall::Save { request, .. } => Some(request.body().clone()),
                _ => None,
            })
            .collect();

        assert_eq!(bodies.len(), 2, "one create submits exactly one write");
        assert_eq!(bodies[0], bodies[1], "a retry re-sends the same write");
    }

    #[test]
    fn a_blank_operation_identity_is_refused_before_anything_is_sent() {
        let service = ScriptedService::new();

        let error = RabbitRecord::created(
            household("household-1"),
            minimal_profile(),
            &request_id("   "),
        )
        .expect_err("a blank identity names no item");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id("   ")));
        assert!(service.calls().is_empty());
    }

    #[test]
    fn a_gate_refusal_is_reported_unchanged() {
        let service = ScriptedService::new().on_service_check([ServiceReply::NoResponse]);
        let request_id = request_id("operation-1");

        let error = create_with(
            &service,
            &household("household-1"),
            minimal_profile(),
            request_id.clone(),
        )
        .expect_err("the gate refuses an unreachable service");

        assert_eq!(error.cause, SaveCause::Connectivity);
        assert_eq!(error.message_key, "error.connectivity");
        assert!(error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.write_count(), 0, "nothing was sent");
    }

    /// RAB-5's edit: every field of the profile is replaceable, including the weight-drop threshold,
    /// and the write is one item write of the rabbit's **existing** item in the household's
    /// collection — which is what every member resolves (RAB-5, HS-8).
    #[test]
    fn an_edit_replaces_every_editable_field_including_the_weight_drop_threshold() {
        let service = ScriptedService::new();
        let one = household("household-1");
        let target = RabbitId("rabbit-1".to_owned());
        let edited = RabbitProfile {
            name: "Clover".to_owned(),
            photo_ref: Some("photo-2".to_owned()),
            breed: Some("Netherland Dwarf".to_owned()),
            dob: Some(at("2025-06-01T00:00:00Z")),
            dob_unknown: false,
            sex: Sex::Male,
            neuter_status: NeuterStatus::Neutered,
            adoption_date: Some(at("2025-08-01T00:00:00Z")),
            weight_drop_threshold_percent: 15.0,
        };

        let rabbit = update_with(
            &service,
            &one,
            target.clone(),
            edited,
            request_id("operation-2"),
        )
        .expect("the double accepted the edit");

        // A whole-profile replacement: every submitted value is recorded.
        assert_eq!(rabbit.id, target);
        assert_eq!(rabbit.household_id, one);
        assert_eq!(rabbit.name, "Clover");
        assert_eq!(rabbit.photo_ref, Some("photo-2".to_owned()));
        assert_eq!(rabbit.breed, Some("Netherland Dwarf".to_owned()));
        assert_eq!(rabbit.dob, Some(at("2025-06-01T00:00:00Z")));
        assert!(!rabbit.dob_unknown);
        assert_eq!(rabbit.sex, Sex::Male);
        assert_eq!(rabbit.neuter_status, NeuterStatus::Neutered);
        assert_eq!(rabbit.adoption_date, Some(at("2025-08-01T00:00:00Z")));
        assert_eq!(rabbit.weight_drop_threshold_percent, 15.0);
        assert!(!rabbit.is_archived);

        // One write, to the target's existing item in the household's shared collection, carrying
        // the operation's identity and the writing member like a create does (ADR 0002 criteria 3
        // and 6, D0.14).
        let body = written_body(&service);
        assert_eq!(body["household_id"], json!(one.0));
        assert_eq!(body["item_id"], json!(target.0));
        assert_eq!(body["request_id"], json!("operation-2"));
        assert_eq!(body["writer_id"], json!(writer().0));
        assert_eq!(body["updated_at"], json!(marker().0));
        assert_eq!(
            written_field(&service, "weight_drop_threshold_percent"),
            json!(15.0)
        );
        assert_eq!(written_field(&service, "name"), json!("Clover"));
        assert_eq!(written_field(&service, "id"), json!(target.0));
    }

    /// A retried edit is the same item write again: the item id is the rabbit's, never the
    /// operation's, so a re-issued edit can neither create a second rabbit nor miss the one it edits
    /// (D0.14).
    #[test]
    fn a_retried_edit_resends_the_same_item_write_and_never_mints_a_rabbit() {
        let service = ScriptedService::new();
        let one = household("household-1");
        let target = RabbitId("rabbit-1".to_owned());
        let operation = request_id("operation-2");

        update_with(
            &service,
            &one,
            target.clone(),
            minimal_profile(),
            operation.clone(),
        )
        .expect("the first attempt completed");
        update_with(&service, &one, target.clone(), minimal_profile(), operation)
            .expect("the re-issued attempt completed");

        let bodies = written_bodies(&service);
        assert_eq!(bodies.len(), 2, "one attempt submits exactly one write");
        assert_eq!(bodies[0], bodies[1], "a retry re-sends the same write");

        for body in bodies {
            assert_eq!(
                body["item_id"],
                json!(target.0),
                "the edit never mints an item id from the operation"
            );
        }
    }

    #[test]
    fn an_edit_with_a_blank_name_is_refused_and_nothing_is_sent() {
        let service = ScriptedService::new();
        let request_id = request_id("operation-2");

        // The same validate_name rule creation uses, so an edit cannot slip a blank name through
        // (RAB-2).
        for blank in ["", " ", "   ", "\t\n"] {
            let error = update_with(
                &service,
                &household("household-1"),
                RabbitId("rabbit-1".to_owned()),
                RabbitProfile {
                    name: blank.to_owned(),
                    ..minimal_profile()
                },
                request_id.clone(),
            )
            .expect_err("a rabbit needs a name, on edit as on create");

            assert_eq!(error.cause, SaveCause::Validation, "{blank:?}");
            assert_eq!(error.message_key, "error.validation", "{blank:?}");
            assert!(!error.retryable, "{blank:?}");
            assert_eq!(error.request_id, Some(request_id.clone()), "{blank:?}");
        }

        // A property of the input: nothing was probed and nothing was sent (RAB-2).
        assert!(service.calls().is_empty());
        assert_eq!(service.write_count(), 0);
    }

    #[test]
    fn an_edit_with_a_blank_identity_is_refused_before_anything_is_sent() {
        let service = ScriptedService::new();
        let household_id = household("household-1");

        for blank in ["", "   "] {
            let error = update_with(
                &service,
                &household_id,
                RabbitId(blank.to_owned()),
                minimal_profile(),
                request_id("operation-2"),
            )
            .expect_err("a blank rabbit id names no item");

            assert_eq!(error.cause, SaveCause::Validation, "{blank:?}");
            assert_eq!(
                error.request_id,
                Some(request_id("operation-2")),
                "{blank:?}"
            );
        }

        // A blank operation identity is the same refusal: the write could not be deduplicated.
        let error = RabbitRecord::updated(
            household_id,
            RabbitId("rabbit-1".to_owned()),
            minimal_profile(),
            &request_id("   "),
        )
        .expect_err("a blank identity names no operation");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.request_id, Some(request_id("   ")));
        assert!(service.calls().is_empty());
    }

    /// A rabbit content, as this device's copy holds it: built through the create constructor so
    /// every RAB-1 field is present, then marked archived when the test needs an archived one.
    fn stored_rabbit(id: &str, name: &str, archived: bool) -> Rabbit {
        let mut rabbit = RabbitRecord::created(
            household("household-1"),
            RabbitProfile {
                name: name.to_owned(),
                ..full_profile()
            },
            &request_id(id),
        )
        .expect("the profile is valid")
        .to_wire();
        rabbit.is_archived = archived;

        rabbit
    }

    fn rabbit_row(rabbit: &Rabbit, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: crate::ipc::types::ItemId(rabbit.id.0.clone()),
            updated_at: at(updated_at),
            deleted: false,
            payload: Some(ItemPayload::Rabbit {
                rabbit: rabbit.clone(),
            }),
        }
    }

    fn weight_entry_row(rabbit_id: &RabbitId) -> CollectionItem {
        CollectionItem {
            item_id: crate::ipc::types::ItemId("entry-1".to_owned()),
            updated_at: at("2026-10-20T09:00:00Z"),
            deleted: false,
            payload: Some(ItemPayload::WeightEntry {
                weight_entry: WeightEntry {
                    id: WeightEntryId("entry-1".to_owned()),
                    rabbit_id: rabbit_id.clone(),
                    value_grams: Grams(1_800),
                    timestamp: at("2026-10-20T09:00:00Z"),
                    note: None,
                    photo_refs: Vec::new(),
                    has_weight_drop_alert: false,
                },
            }),
        }
    }

    fn collection(items: Vec<CollectionItem>) -> HouseholdCollection {
        HouseholdCollection {
            household_id: household("household-1"),
            items,
            source: CollectionSource::Local,
            loaded_at: None,
        }
    }

    /// RAB-6, RAB-9: the archive changes the archived flag and nothing else — the rest of the
    /// profile is carried unchanged into the write.
    #[test]
    fn marking_a_rabbit_archived_changes_nothing_but_the_flag() {
        let rabbit = stored_rabbit("rabbit-1", "Pebble", false);

        let archived = mark_archived(rabbit.clone());

        assert!(archived.is_archived);
        assert_eq!(
            archived,
            Rabbit {
                is_archived: true,
                ..rabbit
            },
            "every other field is the content the copy held"
        );
    }

    /// RAB-7: the restore clears the archived flag and nothing else, so the whole item it submits is
    /// the content this device's copy held.
    #[test]
    fn marking_a_rabbit_restored_changes_nothing_but_the_flag() {
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);

        let restored = mark_restored(rabbit.clone());

        assert!(!restored.is_archived);
        assert_eq!(
            restored,
            Rabbit {
                is_archived: false,
                ..rabbit
            },
            "every other field is the content the copy held"
        );
    }

    /// RAB-7's first scenario at the selection level: the restored rabbit is the active list's and no
    /// longer the archived list's, while the entry written beside it is still a row of the collection
    /// (the write names the rabbit's item alone, so an entry is never touched).
    #[test]
    fn a_restored_rabbit_is_in_the_active_selection_and_its_entry_is_untouched() {
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);
        let entry = weight_entry_row(&rabbit.id);
        let restored = mark_restored(rabbit);
        let copy = collection(vec![
            rabbit_row(&restored, "2026-10-22T09:32:00Z"),
            entry.clone(),
        ]);

        let active = summaries(&copy, false).expect("every content is known");
        assert_eq!(
            active,
            vec![RabbitSummary {
                id: restored.id.clone(),
                name: "Pebble".to_owned(),
                thumbnail_ref: Some("photo-1".to_owned()),
                is_archived: false,
            }],
            "the restored rabbit is in the active list"
        );
        assert!(
            summaries(&copy, true)
                .expect("every content is known")
                .is_empty(),
            "and no longer in the archived one"
        );
        assert_eq!(copy.items[1], entry, "the entry is exactly what it was");
    }

    /// RAB-6: the two values select one list each — the active rabbits, or the archived ones — and
    /// never a union. A weight entry of the same collection is not a rabbit and belongs to neither.
    #[test]
    fn the_active_list_and_the_archived_list_are_disjoint_selections() {
        let active = stored_rabbit("rabbit-1", "Pebble", false);
        let archived = stored_rabbit("rabbit-2", "Clover", true);
        let collection = collection(vec![
            rabbit_row(&active, "2026-10-22T09:30:00Z"),
            rabbit_row(&archived, "2026-10-22T09:31:00Z"),
            weight_entry_row(&active.id),
        ]);

        let active_list = summaries(&collection, false).expect("every content is known");
        assert_eq!(
            active_list,
            vec![RabbitSummary {
                id: active.id.clone(),
                name: "Pebble".to_owned(),
                thumbnail_ref: Some("photo-1".to_owned()),
                is_archived: false,
            }]
        );

        let archived_list = summaries(&collection, true).expect("every content is known");
        assert_eq!(
            archived_list,
            vec![RabbitSummary {
                id: archived.id.clone(),
                name: "Clover".to_owned(),
                thumbnail_ref: Some("photo-1".to_owned()),
                is_archived: true,
            }]
        );
    }

    /// D1.16: a live row whose content this version cannot describe could be a rabbit, so a list
    /// refuses rather than omitting it and presenting a partial list as complete.
    #[test]
    fn a_list_refuses_when_a_row_it_cannot_describe_is_present() {
        let known = stored_rabbit("rabbit-1", "Pebble", false);
        let collection = collection(vec![
            rabbit_row(&known, "2026-10-22T09:30:00Z"),
            CollectionItem {
                item_id: crate::ipc::types::ItemId("item-2".to_owned()),
                updated_at: at("2026-10-22T09:31:00Z"),
                deleted: false,
                payload: None,
            },
        ]);

        for include_archived in [false, true] {
            let error = summaries(&collection, include_archived)
                .expect_err("a row whose content is unknown makes the set read refuse");

            assert_eq!(error.cause, SaveCause::Unknown, "{include_archived}");
            assert!(!error.retryable, "{include_archived}");
            assert_eq!(
                error.message_key,
                crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY,
                "{include_archived}"
            );
            assert_ne!(error.message_key, "error.unknown", "{include_archived}");
            assert_eq!(error.request_id, None, "{include_archived}");
        }
    }

    /// RAB-10, D1.16: a tombstone names a permanently removed rabbit, so neither list describes it —
    /// even when its retained content is absent.
    #[test]
    fn a_tombstoned_row_is_in_neither_list() {
        let collection = collection(vec![CollectionItem {
            item_id: crate::ipc::types::ItemId("rabbit-1".to_owned()),
            updated_at: at("2026-10-22T10:00:00Z"),
            deleted: true,
            payload: None,
        }]);

        assert!(
            summaries(&collection, false)
                .expect("a tombstone is skipped")
                .is_empty()
        );
        assert!(
            summaries(&collection, true)
                .expect("a tombstone is skipped")
                .is_empty()
        );
    }

    /// One weight-entry row of `rabbit` with its own item id, so the cascade can be told row from row.
    fn entry_row(item_id: &str, rabbit_id: &RabbitId, alert: bool) -> CollectionItem {
        CollectionItem {
            item_id: crate::ipc::types::ItemId(item_id.to_owned()),
            updated_at: at("2026-10-20T09:00:00Z"),
            deleted: false,
            payload: Some(ItemPayload::WeightEntry {
                weight_entry: WeightEntry {
                    id: WeightEntryId(item_id.to_owned()),
                    rabbit_id: rabbit_id.clone(),
                    value_grams: Grams(1_800),
                    timestamp: at("2026-10-20T09:00:00Z"),
                    note: None,
                    photo_refs: Vec::new(),
                    has_weight_drop_alert: alert,
                },
            }),
        }
    }

    /// RAB-10: the plan is the archived rabbit plus exactly the weight entries that name it — never
    /// another rabbit, and never another rabbit's entry.
    #[test]
    fn a_deletion_plan_names_the_archived_rabbit_and_only_its_own_entries() {
        let target = stored_rabbit("rabbit-1", "Pebble", true);
        let other = stored_rabbit("rabbit-2", "Clover", false);
        let copy = collection(vec![
            rabbit_row(&target, "2026-10-22T09:30:00Z"),
            rabbit_row(&other, "2026-10-22T09:31:00Z"),
            entry_row("entry-1", &target.id, false),
            entry_row("entry-2", &target.id, false),
            entry_row("entry-3", &other.id, false),
        ]);

        let plan = deletion_plan(&copy, &target.id, &request_id("operation-9"))
            .expect("the archived rabbit is deletable");

        assert_eq!(plan.rabbit, target);
        let items: Vec<&str> = plan
            .entries
            .iter()
            .map(|(item_id, _entry)| item_id.0.as_str())
            .collect();
        assert_eq!(items, ["entry-1", "entry-2"]);
        assert!(
            plan.entries
                .iter()
                .all(|(_item_id, entry)| entry.rabbit_id == target.id),
            "the cascade is the entries that name this rabbit, not every entry"
        );
    }

    /// RAB-10's second scenario: permanent deletion is offered for an archived rabbit only.
    #[test]
    fn a_deletion_plan_refuses_an_active_rabbit() {
        let active = stored_rabbit("rabbit-1", "Pebble", false);
        let copy = collection(vec![rabbit_row(&active, "2026-10-22T09:30:00Z")]);
        let operation = request_id("operation-9");

        let error = deletion_plan(&copy, &active.id, &operation)
            .expect_err("an active rabbit cannot be permanently deleted");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(operation));
    }

    /// A tombstoned rabbit is already removed, and an item that is not a rabbit is nothing to delete:
    /// both are refused rather than absorbed (RAB-10, D1.16).
    #[test]
    fn a_deletion_plan_refuses_a_rabbit_that_is_already_removed_or_not_a_rabbit() {
        let operation = request_id("operation-9");
        let gone = collection(vec![CollectionItem {
            item_id: crate::ipc::types::ItemId("rabbit-1".to_owned()),
            updated_at: at("2026-10-22T10:00:00Z"),
            deleted: true,
            payload: None,
        }]);
        let not_a_rabbit = collection(vec![entry_row(
            "rabbit-1",
            &RabbitId("rabbit-1".to_owned()),
            false,
        )]);

        for copy in [gone, not_a_rabbit] {
            let error = deletion_plan(&copy, &RabbitId("rabbit-1".to_owned()), &operation)
                .expect_err("there is no live rabbit to delete");

            assert_eq!(error.cause, SaveCause::Validation);
            assert_eq!(error.request_id, Some(operation.clone()));
        }
    }

    /// D1.16: a live row this device cannot describe could be one of the rabbit's entries, so the
    /// delete refuses with its own key instead of removing a subset — or overwriting a row it cannot
    /// read.
    #[test]
    fn a_deletion_plan_refuses_when_a_live_row_cannot_be_described() {
        let target = stored_rabbit("rabbit-1", "Pebble", true);
        let copy = collection(vec![
            rabbit_row(&target, "2026-10-22T09:30:00Z"),
            CollectionItem {
                item_id: crate::ipc::types::ItemId("entry-1".to_owned()),
                updated_at: at("2026-10-22T09:31:00Z"),
                deleted: false,
                payload: None,
            },
        ]);
        let operation = request_id("operation-9");

        let error = deletion_plan(&copy, &target.id, &operation)
            .expect_err("a row that might be this rabbit's entry cannot be removed blind");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert!(!error.retryable);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_ne!(error.message_key, "error.unknown");
        assert_eq!(error.request_id, Some(operation));
    }

    /// RAB-10, D1.16: the deletion is one tombstone per item into the household's shared collection —
    /// the entries first, the rabbit last — each keeping identity and content, naming the writing
    /// member and the operation's version marker, with the derived alert flag written `false`.
    #[test]
    fn deleting_a_rabbit_writes_a_tombstone_for_it_and_every_entry() {
        let service = ScriptedService::new();
        let one = household("household-1");
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);
        let copy = collection(vec![
            rabbit_row(&rabbit, "2026-10-22T09:30:00Z"),
            entry_row("entry-1", &rabbit.id, true),
            entry_row("entry-2", &rabbit.id, false),
        ]);
        let plan = deletion_plan(&copy, &rabbit.id, &request_id("operation-9"))
            .expect("the archived rabbit is deletable");

        block_on_ready(delete_rabbit(
            &service,
            &one,
            &plan,
            &writer(),
            &marker(),
            request_id("operation-9"),
        ))
        .expect("the double accepted every tombstone");

        let bodies = written_bodies(&service);
        assert_eq!(bodies.len(), 3, "one tombstone per item");

        // The entries are removed before the rabbit, so a stopped cascade never leaves a removed
        // rabbit with readable history (RAB-10).
        assert_eq!(bodies[0]["item_id"], json!("entry-1"));
        assert_eq!(bodies[1]["item_id"], json!("entry-2"));
        assert_eq!(bodies[2]["item_id"], json!("rabbit-1"));

        for body in &bodies {
            assert_eq!(body["deleted"], json!(true), "each write is a tombstone");
            assert_eq!(body["household_id"], json!(one.0));
            assert_eq!(body["writer_id"], json!(writer().0));
            assert_eq!(body["updated_at"], json!(marker().0));
        }

        // The content is retained on the removing version, and the derived flag is never content.
        assert_eq!(bodies[0]["payload"]["kind"], json!("weight_entry"));
        assert_eq!(
            bodies[0]["payload"]["weight_entry"]["has_weight_drop_alert"],
            json!(false)
        );
        assert_eq!(
            bodies[0]["payload"]["weight_entry"]["value_grams"],
            json!(1_800)
        );
        assert_eq!(bodies[2]["payload"]["kind"], json!("rabbit"));
        assert_eq!(bodies[2]["payload"]["rabbit"]["name"], json!("Pebble"));

        // The rabbit carries the operation's identity; each entry carries its own derived key, because
        // the service's ledger refuses one key with several payloads (D0.14).
        assert_eq!(bodies[0]["request_id"], json!("operation-9/entry-1"));
        assert_eq!(bodies[1]["request_id"], json!("operation-9/entry-2"));
        assert_eq!(bodies[2]["request_id"], json!("operation-9"));
    }

    /// D0.14: a retried deletion re-sends byte-identical tombstones, derived keys included.
    #[test]
    fn a_retried_deletion_resends_the_same_tombstones() {
        let service = ScriptedService::new();
        let one = household("household-1");
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);
        let copy = collection(vec![
            rabbit_row(&rabbit, "2026-10-22T09:30:00Z"),
            entry_row("entry-1", &rabbit.id, false),
        ]);
        let operation = request_id("operation-9");
        let plan = deletion_plan(&copy, &rabbit.id, &operation).expect("deletable");

        for _attempt in 0..2 {
            block_on_ready(delete_rabbit(
                &service,
                &one,
                &plan,
                &writer(),
                &marker(),
                operation.clone(),
            ))
            .expect("the double accepted the cascade");
        }

        let bodies = written_bodies(&service);
        assert_eq!(bodies.len(), 4);
        assert_eq!(&bodies[..2], &bodies[2..]);
    }

    /// A cascade step's failure is the operation's failure: the surface's retry re-issues the
    /// command's identity, and the rabbit is never removed behind a stopped cascade (D0.14, RAB-10).
    #[test]
    fn a_refused_cascade_step_is_reported_against_the_operations_identity() {
        let service = ScriptedService::new().on_write([ServiceReply::NoResponse]);
        let one = household("household-1");
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);
        let copy = collection(vec![
            rabbit_row(&rabbit, "2026-10-22T09:30:00Z"),
            entry_row("entry-1", &rabbit.id, false),
        ]);
        let operation = request_id("operation-9");
        let plan = deletion_plan(&copy, &rabbit.id, &operation).expect("deletable");

        let error = block_on_ready(delete_rabbit(
            &service,
            &one,
            &plan,
            &writer(),
            &marker(),
            operation.clone(),
        ))
        .expect_err("the entry's tombstone was not accepted");

        assert_eq!(error.cause, SaveCause::Connectivity);
        assert_eq!(error.message_key, "error.connectivity");
        assert!(error.retryable);
        assert_eq!(error.request_id, Some(operation));
        assert_eq!(
            service.write_count(),
            1,
            "the cascade stopped before the rabbit's tombstone"
        );
    }

    /// One write answer carrying the service's own envelope and `applied` flag (D0.7 criterion 3).
    fn answered(applied: bool) -> ServiceReply {
        ServiceReply::Answered(Answer::with_body(
            200,
            json!({ "outcome": "applied", "applied": applied }),
        ))
    }

    /// D1.17: the derived ledger keys are injective only while no id contains `/`, so an id that
    /// breaks the rule refuses the operation before anything is written rather than let one key claim
    /// two payloads.
    #[test]
    fn a_deletion_refuses_an_id_the_ledger_key_derivation_cannot_keep_injective() {
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);

        // The operation id is a component of every derived key.
        let copy = collection(vec![
            rabbit_row(&rabbit, "2026-10-22T09:30:00Z"),
            entry_row("entry-1", &rabbit.id, false),
        ]);
        let slash_operation = request_id("operation/9");
        let plan = deletion_plan(&copy, &rabbit.id, &slash_operation).expect("deletable");
        let service = ScriptedService::new();

        let error = block_on_ready(delete_rabbit(
            &service,
            &household("household-1"),
            &plan,
            &writer(),
            &marker(),
            slash_operation.clone(),
        ))
        .expect_err("the operation id cannot be a component of a derived key");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(slash_operation));
        assert_eq!(
            service.write_count(),
            0,
            "nothing is written under an ambiguous key"
        );

        // An entry's item id is the other component.
        let slash_entry = collection(vec![
            rabbit_row(&rabbit, "2026-10-22T09:30:00Z"),
            entry_row("entry/1", &rabbit.id, false),
        ]);
        let operation = request_id("operation-9");
        let plan = deletion_plan(&slash_entry, &rabbit.id, &operation).expect("deletable");

        let error = block_on_ready(delete_rabbit(
            &service,
            &household("household-1"),
            &plan,
            &writer(),
            &marker(),
            operation.clone(),
        ))
        .expect_err("the entry's item id cannot be a component of a derived key");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.request_id, Some(operation));
        assert_eq!(service.write_count(), 0, "the refusal precedes every write");
    }

    /// RAB-10, D0.7 criterion 3: a losing last-write-wins write is a completed call, not an error, so
    /// the cascade must read the receipt — a tombstone that did not apply stops it before the rabbit,
    /// never after.
    #[test]
    fn a_cascade_stops_before_the_rabbit_when_a_tombstone_does_not_apply() {
        // The first entry's removal applies; the second's loses to a newer version.
        let service = ScriptedService::new().on_write([answered(true), answered(false)]);
        let one = household("household-1");
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);
        let copy = collection(vec![
            rabbit_row(&rabbit, "2026-10-22T09:30:00Z"),
            entry_row("entry-1", &rabbit.id, false),
            entry_row("entry-2", &rabbit.id, false),
        ]);
        let operation = request_id("operation-9");
        let plan = deletion_plan(&copy, &rabbit.id, &operation).expect("deletable");

        let error = block_on_ready(delete_rabbit(
            &service,
            &one,
            &plan,
            &writer(),
            &marker(),
            operation.clone(),
        ))
        .expect_err("the second entry's tombstone did not apply");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert_eq!(error.message_key, "error.unknown");
        assert!(
            !error.retryable,
            "the memoized marker would lose again on the same operation"
        );
        assert_eq!(error.request_id, Some(operation));

        let bodies = written_bodies(&service);
        assert_eq!(bodies.len(), 2, "the cascade stopped at the losing write");
        assert!(
            bodies
                .iter()
                .all(|body| body["item_id"] != json!(rabbit.id.0)),
            "the rabbit was never tombstoned behind a removal that did not apply"
        );
    }

    /// RAB-10, D0.7 criterion 3: the rabbit's own tombstone is checked too — a completed call whose
    /// answer says the version did not apply is never reported as a removal.
    #[test]
    fn a_deletion_is_never_reported_as_removed_when_its_rabbit_tombstone_does_not_apply() {
        let service = ScriptedService::new().on_write([answered(false)]);
        let one = household("household-1");
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);
        let copy = collection(vec![rabbit_row(&rabbit, "2026-10-22T09:30:00Z")]);
        let operation = request_id("operation-9");
        let plan = deletion_plan(&copy, &rabbit.id, &operation).expect("deletable");

        let error = block_on_ready(delete_rabbit(
            &service,
            &one,
            &plan,
            &writer(),
            &marker(),
            operation.clone(),
        ))
        .expect_err("the rabbit's tombstone did not apply");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert_eq!(error.message_key, "error.unknown");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(operation));
        assert_eq!(service.write_count(), 1);
    }

    /// D0.14, RAB-10: a stopped cascade is completed by the retry — the entry already removed is
    /// re-sent byte-identically and deduplicated, and the **second** attempt ends with the rabbit's
    /// tombstone.
    #[test]
    fn a_stopped_cascade_is_completed_by_the_retry() {
        // Attempt 1: entry-1 applies, entry-2 is refused. Attempt 2 and later: every write applies.
        let service = ScriptedService::new().on_write([
            answered(true),
            ServiceReply::NoResponse,
            answered(true),
        ]);
        let one = household("household-1");
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);
        let copy = collection(vec![
            rabbit_row(&rabbit, "2026-10-22T09:30:00Z"),
            entry_row("entry-1", &rabbit.id, false),
            entry_row("entry-2", &rabbit.id, false),
        ]);
        let operation = request_id("operation-9");
        let plan = deletion_plan(&copy, &rabbit.id, &operation).expect("deletable");

        let error = block_on_ready(delete_rabbit(
            &service,
            &one,
            &plan,
            &writer(),
            &marker(),
            operation.clone(),
        ))
        .expect_err("the first attempt stopped at the refused entry");

        assert_eq!(error.cause, SaveCause::Connectivity);
        let first_attempt = written_bodies(&service);
        assert_eq!(first_attempt.len(), 2);
        assert!(
            first_attempt
                .iter()
                .all(|body| body["item_id"] != json!(rabbit.id.0)),
            "the first attempt stopped before the rabbit"
        );

        block_on_ready(delete_rabbit(
            &service,
            &one,
            &plan,
            &writer(),
            &marker(),
            operation,
        ))
        .expect("the retry completed the cascade");

        let bodies = written_bodies(&service);
        assert_eq!(bodies.len(), 5, "two writes, then the retry's three");
        assert_eq!(
            bodies[0], bodies[2],
            "the removed entry re-sends the same body"
        );
        assert_eq!(
            bodies.last().expect("the rabbit's tombstone")["item_id"],
            json!(rabbit.id.0),
            "the retry ends with the rabbit's tombstone"
        );
        assert_eq!(
            bodies.last().expect("the rabbit's tombstone")["deleted"],
            json!(true)
        );
    }

    /// One write answer whose envelope reports `outcome` and states no `applied` flag — P0's shape for
    /// a completed write, including the replay's `deduplicated` (D0.7 criterion 5, D0.14).
    fn answered_outcome(outcome: &str) -> ServiceReply {
        ServiceReply::Answered(Answer::with_body(200, json!({ "outcome": outcome })))
    }

    /// D1.17: the operation id is the rabbit's own raw ledger key even when there is nothing to
    /// cascade, so it is checked whatever the cascade size — a later operation's `<op>/<item>` must
    /// never be able to claim it.
    #[test]
    fn a_deletion_refuses_a_slash_bearing_operation_id_even_with_no_entries() {
        let service = ScriptedService::new();
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);
        let copy = collection(vec![rabbit_row(&rabbit, "2026-10-22T09:30:00Z")]);
        let operation = request_id("operation/9");
        let plan = deletion_plan(&copy, &rabbit.id, &operation).expect("deletable");

        let error = block_on_ready(delete_rabbit(
            &service,
            &household("household-1"),
            &plan,
            &writer(),
            &marker(),
            operation.clone(),
        ))
        .expect_err("the operation id cannot be a raw ledger key that contains the separator");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(operation));
        assert_eq!(
            service.write_count(),
            0,
            "nothing is written under that key"
        );
    }

    /// D0.7 criterion 5, D0.14: a replay's `deduplicated` answer is a completed write. It carries no
    /// `applied` field, so the stop rule must not read it as a loss: the cascade continues and ends
    /// with the rabbit's tombstone.
    #[test]
    fn a_deduplicated_tombstone_is_a_completed_write_and_the_cascade_continues() {
        let service = ScriptedService::new().on_write([answered_outcome("deduplicated")]);
        let one = household("household-1");
        let rabbit = stored_rabbit("rabbit-1", "Pebble", true);
        let copy = collection(vec![
            rabbit_row(&rabbit, "2026-10-22T09:30:00Z"),
            entry_row("entry-1", &rabbit.id, false),
            entry_row("entry-2", &rabbit.id, false),
        ]);
        let operation = request_id("operation-9");
        let plan = deletion_plan(&copy, &rabbit.id, &operation).expect("deletable");

        block_on_ready(delete_rabbit(
            &service,
            &one,
            &plan,
            &writer(),
            &marker(),
            operation,
        ))
        .expect("a deduplicated write is a completed write");

        let bodies = written_bodies(&service);
        assert_eq!(
            bodies.len(),
            3,
            "every step was accepted, so the cascade reached the rabbit"
        );
        assert_eq!(
            bodies.last().expect("the rabbit's tombstone")["item_id"],
            json!(rabbit.id.0)
        );
    }
}
