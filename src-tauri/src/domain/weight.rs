//! The weight-entry domain: one recorded value, its timestamp and its optional note, and the one
//! write path an entry is created through (WT-1, WT-3; D1.2, D1.16, D1.17).
//!
//! The contract in [`crate::ipc::types`] fixes the *shape* that crosses the boundary — [`WeightEntry`],
//! [`WeightEntryDraft`], [`Grams`](crate::ipc::types::Grams). This module fixes what that shape may not violate:
//!
//! * **Every recorded value is grams, and an entry has no unit of its own.** [`Grams`] is the only
//!   weight representation in the payload; the display unit is applied in
//!   `src/features/weight/api.ts`, before a draft reaches this side and after a stored value leaves it
//!   (D1.2, DM-1, WT-3). A consequence worth keeping: nothing here converts, rounds or re-interprets
//!   the value, so changing the person's display unit cannot rewrite a stored entry.
//! * **A timestamp is the moment, stored as supplied.** [`record`] takes the supplied instant
//!   unchanged — including one in the past — and never clamps it to the present (WT-1). When the
//!   draft omits it, the caller's creation instant is recorded instead; that instant is minted once
//!   per operation by [`crate::state::AppState`], not read here, so a retried create re-sends the same
//!   timestamp and the service's ledger recognises one operation rather than refusing a changed
//!   payload (D0.14). Only the text is normalised to UTC by the storage boundary, exactly as every
//!   absolute instant is (DM-6): two spellings of one moment are one stored timestamp.
//! * **The note is optional and kept whole.** A note the person supplied is recorded as it stands —
//!   never trimmed, truncated or defaulted to an empty string — and an omitted one stays `None`
//!   (WT-1, WT-8).
//! * **An entry's identity is its create operation's identity.** A weight entry has no id before its
//!   create, so [`entry_id`] mints it from the [`RequestId`] the frontend already re-uses on a retry
//!   (D0.14), exactly as a rabbit's identity is minted from its create operation. The collection item
//!   id is that same value, so every later write addresses the entry the create produced and a
//!   replayed create is the *same* entry rather than a second row (D0.14, D1.17). No generator is
//!   added, and nothing may be parsed out of either id.
//! * **The stored `has_weight_drop_alert` is not content: it is written `false`.** [`record`] sets it
//!   `false` and [`save_weight_entry`] normalises it on the way out, because the flag is derived from
//!   the rabbit's threshold and the entry set on every read (D1.3, D1.13, D1.16). Persisting a derived
//!   value would let it disagree with the threshold it came from, which is the snapshot the derivation
//!   forbids.
//! * **Creation order is recorded, not derived.** [`record`] sets `created_at` once from the
//!   operation's creation instant — even when the person supplied the reading timestamp — and
//!   [`revise`] carries the stored value unchanged, because WT-1's editable timestamp and the LWW
//!   `updated_at` cannot stand in for it (WT-7, D1.12). [`ordered`] is the one comparator every reader
//!   uses: `(timestamp, created_at, id)`, oldest first.
//!
//! **Nothing here touches Tauri, and nothing here writes storage.** One item write — [`save_weight_entry`]
//! — goes through P0's reachability-gated [`SaveGate`]: an entry exists once the shared service has
//! accepted the write, so no local-only entry and no second write path can be produced here (D1.1,
//! SYNC-2). What this device holds afterwards is its **copy of the shared collection**, filled by the
//! read path (SYNC-3) — never by this module writing it.
//!
//! **A value is checked where the draft becomes a record.** [`validate_value`] is the one WT-4 rule —
//! a value of zero or less is refused, and a value above [`PLAUSIBILITY_BOUND_GRAMS`] is refused until
//! the draft's [`outlier_confirmed`](WeightEntryDraft::outlier_confirmed) says the person confirmed it
//! — and [`record`] applies it before anything is recorded. [`revise`] calls the same function, so a
//! correction and a create are judged by one rule rather than two. The bound is a confirmation
//! threshold, not a rejection (D1.9): a confirmed outlier is recorded at its true value, never clamped.
//!
//! **An edit replaces the editable set and keeps the target's identity.** [`revise`] takes the stored
//! entry and the submitted draft: the value, note and timestamp are replaced, an omitted timestamp
//! keeping the stored instant rather than restamping a back-dated entry, while the identity, rabbit,
//! photo references and `created_at` come from the stored entry — a correction never changes its
//! recorded creation order (WT-7). The derived alert flag is `false` on the revised value, because it
//! is re-derived on read rather than carried (WT-5, D1.13, D1.16).
//!
//! **The trend/history read is a window over the current entry set.** [`trend`] selects the entries
//! inside a [`WeightRange`] and carries the rabbit's threshold beside them. The window is applied
//! here, on the read, and never stored, so the trend and the weight-only history can hold different
//! ranges over the same rabbit without one influencing the other (WT-6, D1.4). Points are ordered by
//! [`ordered`], tombstones are excluded before the content check, and a live row whose content this
//! device cannot describe refuses the whole read rather than answering a partial set (D1.16, D1.17).
//!
//! **The drop alert is one derivation over the whole entry set.** [`alerts`] orders the rabbit's
//! entries by the one comparator and flags an entry whose drop against its predecessor exceeds the
//! rabbit's threshold, so a threshold change, an edit or a deletion re-evaluates the surrounding
//! entries without rewriting any stored one (WT-7; D1.3, D1.13). [`trend`] applies the window to the
//! **points only**: each flag is derived from the full set, so a point whose predecessor lies outside
//! the returned window is still judged against it (I3). The stored `has_weight_drop_alert` is never
//! read back (D1.16). The dashboard read (2.14) and the entry-detail read (2.15) call [`alerts`] the
//! same way rather than re-deriving the rule. Duplicate-entry detection and a deletion's tombstone
//! remain the write path's (D1.17).

use std::collections::HashMap;

use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use super::rabbit::SAVE_ITEM_RPC;
use crate::ipc::types::{
    CommandError, Grams, HouseholdCollection, HouseholdId, Instant, ItemId, ItemPayload, MemberId,
    PLAUSIBILITY_BOUND_GRAMS, Rabbit, RabbitId, RequestId, WeightEntry, WeightEntryDraft,
    WeightEntryId, WeightEntrySummary, WeightRange, WeightTrend,
};
use crate::storage::AbsoluteInstant;
use crate::sync::{SaveGate, SaveRequest, SharedService};

/// The entry identity one create operation mints (WT-1, D0.14).
///
/// A weight entry has no id of its own before its create, so the operation's identity is the entry's:
/// the frontend mints one opaque value per state-changing call and re-uses it on a retry, which is
/// what makes a replayed create the same entry rather than a second one. Nothing is parsed out of the
/// value — it is opaque, and its only property is that it is unique per operation (D1.17's minting
/// rule: no id may contain `/`, and the frontend's ids are 32 lowercase hex characters).
pub fn entry_id(request_id: &RequestId) -> WeightEntryId {
    WeightEntryId(request_id.0.clone())
}

/// The collection item id a weight entry is stored under (D0.14, D1.17).
///
/// The identity is the entry's opaque string: creation mints the entry id from its operation and
/// writes it as `item_id`, so every later write — an edit or a deletion in task 2.11 — addresses that
/// same row. Nothing else may derive an item id, which is what keeps one entry from becoming two rows.
pub fn item_id(entry_id: &WeightEntryId) -> ItemId {
    ItemId(entry_id.0.clone())
}

/// The one WT-4 value rule, applied wherever a draft becomes a recorded value.
///
/// A value of zero or less is refused (`validation`): it names no weighable rabbit and is inevitably a
/// typing error. A value **above** [`PLAUSIBILITY_BOUND_GRAMS`] is refused until
/// [`WeightEntryDraft::outlier_confirmed`] says the person confirmed it; exactly at the bound is
/// accepted without confirmation, so the bound itself is plausible and only a value beyond it is
/// questioned (D1.9). A confirmed outlier passes unchanged and is recorded at its true value — the
/// bound is a confirmation threshold, never a rejection.
///
/// It is public so task 2.11's edit path calls this same function for a correction: the value rule has
/// one implementation, not a create copy and an edit copy.
///
/// # Errors
///
/// `validation`, carrying the operation's identity, for a value of zero or less and for an unconfirmed
/// value above the bound. Nothing is sent by this function; it only answers whether the value may
/// become an entry.
pub fn validate_value(
    value_grams: Grams,
    outlier_confirmed: bool,
    request_id: &RequestId,
) -> Result<(), CommandError> {
    if value_grams <= Grams(0) {
        return Err(CommandError::validation(Some(request_id.clone())));
    }

    if value_grams > PLAUSIBILITY_BOUND_GRAMS && !outlier_confirmed {
        return Err(CommandError::validation(Some(request_id.clone())));
    }

    Ok(())
}

/// Record one weight entry for `rabbit_id` (WT-1, DM-1).
///
/// The supplied value is recorded in canonical grams exactly as it stands (D1.2, DM-1). The timestamp
/// is the supplied instant stored as that moment — never clamped to the present, so a back-dated
/// entry keeps its date (WT-1) — or `creation_instant` when the draft omitted it. `created_at` is
/// **always** `creation_instant`, even when the person supplied a timestamp: it is the entry's
/// recorded creation order, which the editable timestamp cannot carry (WT-7, D1.12). The note is
/// optional and recorded whole. The three facts creation fixes are the entry's identity ([`entry_id`],
/// the operation's identity), its `created_at`, and `has_weight_drop_alert: false`, because the alert
/// is derived on read and is never content (D1.16, D1.3). The entry has no photos yet: choosing one is
/// PHOTO-2 in P2 (D1.11) and [`WeightEntryDraft`] has no photo field.
///
/// `creation_instant` is the caller's, not this module's: reading the clock is the write path's job,
/// and minting it once per operation is what keeps a retried create byte-identical (D0.14). It is
/// called unconditionally, because every create records a `created_at`; the memo supplies the
/// operation's first instant on a retry whether or not the person supplied a timestamp.
///
/// # Errors
///
/// `validation` for a blank operation identity, because the entry's id is minted from it and a blank
/// one would name no item at all; for a value of zero or less, and for an unconfirmed value above the
/// plausibility bound (WT-4, via [`validate_value`]); and for a supplied timestamp that is not an
/// absolute instant — the unparseable text is refused rather than replaced with `now`, so a corrupt
/// input cannot masquerade as the moment of creation (DM-6). All carry the operation's identity, and
/// none sends anything.
pub fn record(
    rabbit_id: RabbitId,
    draft: WeightEntryDraft,
    creation_instant: impl FnOnce() -> Instant,
    request_id: &RequestId,
) -> Result<WeightEntry, CommandError> {
    if request_id.0.trim().is_empty() {
        return Err(CommandError::validation(Some(request_id.clone())));
    }

    let WeightEntryDraft {
        value_grams,
        timestamp,
        note,
        outlier_confirmed,
    } = draft;

    validate_value(value_grams, outlier_confirmed, request_id)?;

    // The creation instant is minted unconditionally: it is the entry's `created_at`, the ordering
    // key WT-7 needs even when the person supplied a timestamp of their own (D1.12). The caller
    // memoises it per operation, so a retried create re-sends the same value (D0.14).
    let created_at = creation_instant();

    let timestamp = match timestamp {
        // The moment is the point: an explicit past instant is a valid record of that moment and is
        // never moved. The storage boundary normalises the offset to UTC, as it does for every
        // absolute instant (DM-6), so two spellings of one moment are equal after a round trip.
        Some(supplied) => AbsoluteInstant::parse(&supplied)
            .map_err(|_| CommandError::validation(Some(request_id.clone())))?
            .to_wire(),
        // The defaulted timestamp is the creation moment itself; the two coincide, but they remain
        // two recorded facts so a correction can move one without moving the other.
        None => created_at.clone(),
    };

    Ok(WeightEntry {
        id: entry_id(request_id),
        rabbit_id,
        value_grams,
        timestamp,
        created_at,
        note,
        photo_refs: Vec::new(),
        has_weight_drop_alert: false,
    })
}

/// Revise one stored entry, replacing exactly its editable fields (WT-5, WT-7; D1.13, D1.16).
///
/// [`WeightEntryDraft`] is the whole editable set — the value, the timestamp and the optional note —
/// and it is submitted whole by the pre-filled form, so this function replaces all three and needs no
/// "unchanged" versus "cleared" sentinel. Four facts come from the **target** instead: the identity,
/// the rabbit, the photo references and `created_at`. An edit may not move an entry to another rabbit
/// or another item, may not change when it was created (WT-7, D1.12), and [`WeightEntryDraft`] has no
/// photo or creation field (choosing a photo is PHOTO-2 in P2, D1.11).
///
/// **`timestamp: None` keeps the stored instant.** The frozen draft uses `None` for "now" on create
/// and "the entry's existing timestamp" on update, so an edit of a back-dated entry never restamps it
/// (design, `WeightEntryDraft`). A supplied instant is parsed and normalised to UTC by the storage
/// boundary exactly as [`record`] does (DM-6), and an unparseable one is refused rather than replaced
/// with the stored value.
///
/// The value is judged by [`validate_value`], the one WT-4 rule: zero or less is refused, and a value
/// above the plausibility bound is refused until the draft confirms it (D1.9). The returned
/// `has_weight_drop_alert` is `false`; the flag is derived from the current entry set on every read
/// and never persisted (D1.3, D1.13), and [`save_weight_entry`] normalises it again on the way out.
///
/// # Errors
///
/// `validation` for a blank operation identity, a value of zero or less, an unconfirmed value above
/// the plausibility bound (WT-4), and a timestamp that is not an absolute instant (DM-6). Each carries
/// the operation's identity, and none sends anything.
pub fn revise(
    stored: &WeightEntry,
    draft: WeightEntryDraft,
    request_id: &RequestId,
) -> Result<WeightEntry, CommandError> {
    if request_id.0.trim().is_empty() {
        return Err(CommandError::validation(Some(request_id.clone())));
    }

    let WeightEntryDraft {
        value_grams,
        timestamp,
        note,
        outlier_confirmed,
    } = draft;

    validate_value(value_grams, outlier_confirmed, request_id)?;

    let timestamp = match timestamp {
        Some(supplied) => AbsoluteInstant::parse(&supplied)
            .map_err(|_| CommandError::validation(Some(request_id.clone())))?
            .to_wire(),
        // The frozen draft's `None` means "keep the stored instant" on an edit: a back-dated entry is
        // never restamped by a correction (WT-5).
        None => stored.timestamp.clone(),
    };

    Ok(WeightEntry {
        id: stored.id.clone(),
        rabbit_id: stored.rabbit_id.clone(),
        value_grams,
        timestamp,
        // Creation order is recorded once and never moves: WT-5's correction changes what was
        // measured and when it happened, not when the entry was made (WT-7, D1.12).
        created_at: stored.created_at.clone(),
        note,
        photo_refs: stored.photo_refs.clone(),
        // Derived on read, never content: the write boundary states the constant (D1.16).
        has_weight_drop_alert: false,
    })
}

/// The one total order WT-7 uses for a rabbit's entries: `(timestamp, created_at, id)` (D1.12).
///
/// It lives here, beside the entry model, so the order a reader presents and the order a future
/// derivation takes a predecessor from cannot drift into two rules. `created_at` is the tie key; the
/// entry id closes a same-instant tie and is never consulted when the creation instants differ.
///
/// A stored instant that does not parse is corrupt content, so building a key refuses rather than
/// silently sorting by its wire text.
pub fn entry_order(entry: &WeightEntry) -> Result<EntryOrder, CommandError> {
    Ok(EntryOrder {
        timestamp: parse_instant(&entry.timestamp)?,
        created_at: parse_instant(&entry.created_at)?,
        id: entry.id.0.clone(),
    })
}

/// Order `entries` by the one comparator ([`entry_order`], D1.12), oldest first.
///
/// Every reader that presents or derives from an ordered series uses this function, so the trend and
/// the drop-alert derivation of task 2.13 cannot disagree about which entry precedes which. All keys
/// are built before anything is reordered, so a corrupt instant refuses without a partial order.
pub fn ordered(entries: Vec<WeightEntry>) -> Result<Vec<WeightEntry>, CommandError> {
    let mut keyed = Vec::with_capacity(entries.len());

    for entry in entries {
        keyed.push((entry_order(&entry)?, entry));
    }

    keyed.sort_by(|(left, _), (right, _)| left.cmp(right));

    Ok(keyed.into_iter().map(|(_, entry)| entry).collect())
}

/// The sort key [`ordered`] uses, with the field order that is the total order (D1.12).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntryOrder {
    /// The entry's reading timestamp.
    timestamp: OffsetDateTime,
    /// The entry's recorded creation instant, which an edit never moves.
    created_at: OffsetDateTime,
    /// The entry id, closing a same-instant tie deterministically across devices.
    id: String,
}

/// WT-7's documented default drop threshold, in percent (RAB-1, RAB-5).
///
/// The wire type makes the threshold a required `f64` on every rabbit
/// ([`Rabbit::weight_drop_threshold_percent`]), so the derivation never receives "no threshold": the
/// default is what a rabbit profile carries unless the person changes it, and it is recorded once on
/// the rabbit — never on an entry. No alert is derived here; [`alerts`] takes the stored value.
pub const DEFAULT_DROP_THRESHOLD_PERCENT: f64 = 5.0;

/// One entry's derived drop alert, in the shared order (WT-7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryAlert {
    /// The entry the flag belongs to.
    pub id: WeightEntryId,
    /// Whether the entry's drop against its predecessor exceeds the rabbit's threshold.
    pub has_weight_drop_alert: bool,
}

/// Derive the drop alert of every entry of one rabbit (WT-7; D1.3, D1.12, D1.13).
///
/// The entries are ordered by [`ordered`] — `(timestamp, created_at, id)` — so the predecessor is the
/// one WT-7 names, and two entries sharing a timestamp are judged by their recorded creation order
/// rather than by insertion or id. The first entry has **no predecessor and is never flagged**.
///
/// An entry is flagged when its drop against the predecessor **exceeds** `threshold_percent`, as a
/// percentage of the predecessor's value. The comparison is strict: a drop exactly at the threshold
/// is not a drop *beyond* it and raises no alert. It is evaluated without a division — `(previous −
/// current) · 100 > threshold · previous` — so no `NaN` or infinity can arise and the equality
/// boundary is decided on the exact gram values rather than on a rounded quotient. Because both sides
/// are computed identically on every device, the same entries and threshold yield the same flags
/// everywhere (WT-7).
///
/// **Nothing is stored and nothing is rewritten.** The function reads the entries it is given and
/// returns one flag per entry; a threshold change is another call with another number, and an edit or
/// deletion is another call over the changed set. The stored `has_weight_drop_alert` is never read
/// (D1.16).
///
/// Callers pass the rabbit's **full** entry set, never a window: `get_weight_entries` derives here
/// first and only then applies its range, and the dashboard row (2.14) and the entry detail (2.15)
/// call this same function for their flags (I3, R2).
///
/// # Errors
///
/// `unknown` with no operation identity when a stored timestamp or creation instant is not an
/// absolute instant, because the order the derivation depends on cannot be built (DM-6).
pub fn alerts(
    entries: &[WeightEntry],
    threshold_percent: f64,
) -> Result<Vec<EntryAlert>, CommandError> {
    let mut derived = Vec::with_capacity(entries.len());
    let mut previous: Option<Grams> = None;

    for entry in ordered(entries.to_vec())? {
        let has_weight_drop_alert = previous
            .is_some_and(|previous| drop_exceeds(previous, entry.value_grams, threshold_percent));

        previous = Some(entry.value_grams);
        derived.push(EntryAlert {
            id: entry.id,
            has_weight_drop_alert,
        });
    }

    Ok(derived)
}

/// Whether the drop from `previous` to `current` exceeds `threshold_percent` (WT-7).
///
/// One line of arithmetic with no division: `(previous − current) · 100 > threshold · previous`.
/// The left side is exact for integer grams and the right side is one rounding of the stored
/// threshold, so an exactly-at-threshold drop compares equal and does **not** flag. A gain is a
/// negative drop and cannot flag.
///
/// The guard makes the function total on corrupt input: a **non-positive predecessor** names no
/// percentage (WT-4 refuses it on write), and a **negative or non-finite threshold** names no bound —
/// read as an inequality, either would let a gain or an equal-value pair satisfy the comparison. Both
/// raise no alert; the profile boundary refuses such a threshold as `validation`
/// ([`crate::domain::rabbit`]), so this is the second of two safeguards rather than the primary one.
fn drop_exceeds(previous: Grams, current: Grams, threshold_percent: f64) -> bool {
    if previous <= Grams(0) || !threshold_percent.is_finite() || threshold_percent < 0.0 {
        return false;
    }

    let dropped = i64::from(previous.0) - i64::from(current.0);

    dropped as f64 * 100.0 > threshold_percent * f64::from(previous.0)
}

/// One entry's detail: the stored entry with its drop-alert flag re-derived (WT-8; D1.13, I2, I3).
///
/// The entry's own fields are returned unchanged — the canonical grams, the timestamp, the note
/// **whole** (nothing on this path truncates it) and the photo references in their stored order,
/// which is empty when the entry has none, so no pager is ever invented (WT-8). Only
/// [`WeightEntry::has_weight_drop_alert`] is replaced: it comes from [`alerts`] over the rabbit's
/// **full** entry set, found by the entry's own id, so the detail can never disagree with the trend
/// or the dashboard, and a stored value is ignored (D1.16, I2, I3).
///
/// The entry set is [`entries_of`]'s — the one live-row scan the trend/history read shares — so the
/// tombstone and unknown-content rules cannot drift between the two reads that must agree (I3, R2).
/// The rabbit's threshold is read from the rabbit's row in the same collection, because the flag
/// needs it and no derivation may be computed from a subset (D1.16). An archived rabbit is not a
/// refusal: RAB-8 presents its timeline read-only and the frontend withholds Edit and Delete.
///
/// The value is **not** converted to a display unit: grams are the only weight on this path and
/// `src/features/weight/api.ts` formats them for display (D1.2, I1).
///
/// # Errors
///
/// `unknown` with the unknown-content key when a live row has no content this device can describe
/// (D1.16); `validation` when the collection holds no live rabbit for the entry, which names an
/// orphaned entry; and `unknown` when a stored timestamp or creation instant is not an absolute
/// instant. A read carries no operation identity, so every failure answers `request_id: None`.
pub fn detail(
    collection: &HouseholdCollection,
    entry: &WeightEntry,
) -> Result<WeightEntry, CommandError> {
    let entries = entries_of(collection, &entry.rabbit_id)?;

    // `entries_of` has already refused every live row this device cannot describe, so the rabbit row
    // is resolved from the same known-content collection (D1.16).
    let rabbit = collection
        .items
        .iter()
        .filter(|item| !item.deleted)
        .find_map(|item| match &item.payload {
            Some(ItemPayload::Rabbit { rabbit }) if rabbit.id == entry.rabbit_id => Some(rabbit),
            _ => None,
        })
        .ok_or_else(|| CommandError::validation(None))?;

    let has_weight_drop_alert = alerts(&entries, rabbit.weight_drop_threshold_percent)?
        .into_iter()
        .find(|alert| alert.id == entry.id)
        .is_some_and(|alert| alert.has_weight_drop_alert);

    Ok(WeightEntry {
        has_weight_drop_alert,
        ..entry.clone()
    })
}

/// The live weight entries of `rabbit_id` in `collection`, in collection order (WT-6, WT-8; D1.16,
/// D1.17).
///
/// The **one** live-row scan both weight read derivations use — the trend/history window and the
/// entry detail — so the tombstone rule and the unknown-content refusal cannot drift between reads
/// that must never disagree (I3, R2; D1.16). A tombstone is excluded **before** the content check,
/// because a tombstone's content need not be on this device and it names an entry that is already
/// removed (D1.17). A live row whose content is not on this device refuses the read with the
/// unknown-content key: it could be one of this rabbit's entries, and a subset is not the rabbit's
/// set (D1.16). A rabbit row — this one or another — is not an entry of this series.
///
/// # Errors
///
/// `unknown` with the unknown-content key when a live row of the collection has no content this
/// device can describe (D1.16). A read carries no operation identity, so the refusal answers
/// `request_id: None`.
fn entries_of(
    collection: &HouseholdCollection,
    rabbit_id: &RabbitId,
) -> Result<Vec<WeightEntry>, CommandError> {
    let mut entries = Vec::new();

    for item in &collection.items {
        if item.deleted {
            continue;
        }

        match &item.payload {
            Some(ItemPayload::WeightEntry { weight_entry })
                if weight_entry.rabbit_id == *rabbit_id =>
            {
                entries.push(weight_entry.clone());
            }
            Some(_) => {}
            None => return Err(CommandError::household_content_unloaded(None)),
        }
    }

    Ok(entries)
}

/// One rabbit's weight entries inside a range window, oldest first, with the rabbit's threshold
/// alongside (WT-6, WT-7; D1.4, D1.16, D1.17).
///
/// **The window is read-side only.** It is `[now - N days, now]` for a bounded [`WeightRange`], both
/// edges inclusive, and unbounded on both sides for `All` — the full history is not a computed period
/// (D1.4). Nothing is stored per range and this function holds no state, so two calls with different
/// ranges over the same rabbit cannot influence each other.
///
/// **Both edges are meaningful.** An entry whose timestamp is exactly `now - N days` is inside the
/// window and one a second earlier is outside; an entry stamped after `now` is outside too, because
/// WT-6 presents "the entries of the last 30 days", not every entry that is merely not old enough
/// (the write path accepts any parseable instant, so a future timestamp is reachable). `All` has no
/// bounds at all, so the full history includes an entry stamped in the future (D1.4).
///
/// The points are ordered by [`ordered`] — `(timestamp, created_at, id)`, the one comparator the
/// previous-entry derivation also uses (D1.12) — oldest first. The entry set is [`entries_of`]'s, the
/// same scan the detail read uses: a tombstone is excluded **before** the content check, so a removed
/// entry never becomes an unreadable row (D1.17), and a live row whose content is not on this device
/// refuses the read with the unknown-content key because it could be one of this rabbit's entries (or
/// an entry that precedes one of them), and a subset is not the rabbit's set (D1.16).
///
/// Each point's `has_weight_drop_alert` is [`alerts`]'s derivation over the rabbit's **full** entry
/// set, never over the window: a point whose predecessor lies outside the returned points is still
/// judged against it (I3, R2). The stored payload's flag is ignored on every read (D1.16).
///
/// # Errors
///
/// `unknown` with the unknown-content key when a live row of the collection has no content this
/// device can describe (D1.16), and `unknown` when a stored timestamp or creation instant is not an
/// absolute instant. A read carries no operation identity, so both answer `request_id: None`.
pub fn trend(
    collection: &HouseholdCollection,
    rabbit: &Rabbit,
    range: WeightRange,
    now: &Instant,
) -> Result<WeightTrend, CommandError> {
    let (lower_bound, upper_bound) = window(range, now)?;
    // The same live-entry scan the detail read uses, so neither can classify a row differently.
    let entries = entries_of(collection, &rabbit.id)?;

    // The flags are derived from the full set before the window is applied, so trimming the points
    // can never re-base an alert on a different predecessor (I3).
    let flags: HashMap<WeightEntryId, bool> =
        alerts(&entries, rabbit.weight_drop_threshold_percent)?
            .into_iter()
            .map(|alert| (alert.id, alert.has_weight_drop_alert))
            .collect();

    let mut in_window = Vec::new();

    for entry in entries {
        let at = parse_instant(&entry.timestamp)?;

        if inside(at, lower_bound, upper_bound) {
            in_window.push(entry);
        }
    }

    let points = ordered(in_window)?
        .into_iter()
        .map(|entry| {
            let id = entry.id;

            WeightEntrySummary {
                has_weight_drop_alert: flags.get(&id).copied().unwrap_or(false),
                id,
                value_grams: entry.value_grams,
                timestamp: entry.timestamp,
            }
        })
        .collect();

    Ok(WeightTrend {
        range,
        points,
        drop_threshold_percent: rabbit.weight_drop_threshold_percent,
    })
}

/// The two bounds of a range window, either of which may be absent (WT-6, D1.4).
///
/// A bounded range is `[now - N days, now]`, both edges inclusive. `All` is `(None, None)`: the full
/// history is the absence of a window, not a computed period, so it includes a future-dated entry.
/// Returning the upper edge is what keeps a future timestamp out of 7/30/90 — "the entries of the last
/// N days" (WT-6) and "only the entries within that range" (DASH-7) are not satisfied by a lower bound
/// alone, and the write path accepts any parseable instant.
fn window(
    range: WeightRange,
    now: &Instant,
) -> Result<(Option<OffsetDateTime>, Option<OffsetDateTime>), CommandError> {
    let days = match range {
        WeightRange::Days7 => 7,
        WeightRange::Days30 => 30,
        WeightRange::Days90 => 90,
        WeightRange::All => return Ok((None, None)),
    };
    let now = parse_instant(now)?;

    Ok((Some(now - Duration::days(days)), Some(now)))
}

/// Whether `at` lies inside the window's bounds; an absent bound is unbounded on that side.
fn inside(
    at: OffsetDateTime,
    lower_bound: Option<OffsetDateTime>,
    upper_bound: Option<OffsetDateTime>,
) -> bool {
    lower_bound.is_none_or(|bound| at >= bound) && upper_bound.is_none_or(|bound| at <= bound)
}

/// Parse a stored or supplied instant for window arithmetic.
///
/// [`AbsoluteInstant`] is the boundary's canonical representation, but comparison against a computed
/// cut-off needs the underlying moment; a value the formatter could read is therefore parsed a second
/// time here. A stored timestamp that does not parse is a corrupt row, so it refuses the read rather
/// than being silently treated as outside the window.
fn parse_instant(text: &Instant) -> Result<OffsetDateTime, CommandError> {
    OffsetDateTime::parse(&text.0, &Rfc3339).map_err(|_| CommandError::unknown(None))
}

/// Write one recorded weight entry as an item of its household's shared collection (WT-1, D1.1).
///
/// The entry is written to the **household the caller named**, which is the household that claimed the
/// entry's rabbit when the write boundary resolved it — an entry never chooses its collection from the
/// person's active household, because that is a per-device selection rather than a scope the backend
/// may infer.
///
/// `writer` and `marker` are the two per-operation facts `save_item` takes beside the item itself, and
/// both come from the caller for the same reason a rabbit write takes them: the writer is the member
/// the person is in that household, and the marker is minted once per operation so a retry re-sends a
/// byte-identical body (D0.7 criteria 3 and 6, D0.14).
///
/// The payload's `has_weight_drop_alert` is normalised to `false` on the way out: it is derived on
/// read and never content, and the constant value keeps a re-issued write byte-identical (D1.16).
///
/// # Errors
///
/// The gate's own causes for the attempt — `connectivity`, `no_account`, `allowance_exceeded`,
/// `rate_limited`, `timeout`, `validation` and `unknown` — and `validation` for a blank operation
/// identity, which [`SaveRequest::new`] refuses because a blank key would break the service's
/// deduplication (D0.14). No failure is ever reported as a saved entry.
pub async fn save_weight_entry(
    service: &dyn SharedService,
    household_id: &HouseholdId,
    entry: &WeightEntry,
    writer: &MemberId,
    marker: &Instant,
    request_id: RequestId,
) -> Result<(), CommandError> {
    let body = save_body(household_id, entry, writer, marker, &request_id);
    let request = SaveRequest::new(request_id, SAVE_ITEM_RPC, body)?;

    let _accepted = SaveGate::new(service).save(request).await?;

    Ok(())
}

/// The body the shared service's `save_item` receives for one weight entry (D1.16).
///
/// It is the function's own argument list: the collection and item it writes, the version marker
/// last-write-wins compares, the writing member, the operation's identity and the payload. The payload
/// is the entry under the frozen `weight_entry` tag, so what this device writes and what a read later
/// resolves are one representation.
///
/// **The body is a function of its inputs alone.** It reads no clock and derives nothing from the
/// environment, so a retry that supplies the same entry, writer, marker and identity submits the same
/// bytes — which is what the service's request ledger requires of a re-issued operation (D0.7
/// criterion 6, D0.14). That is also why the creation instant was resolved before this call rather
/// than read here.
///
/// The removal flag is **omitted**, not sent `false`: `save_item`'s `deleted` defaults to `false`
/// (D1.17), so a non-removal write keeps the six-argument body and the service's default is the only
/// place that state is stated.
fn save_body(
    household_id: &HouseholdId,
    entry: &WeightEntry,
    writer: &MemberId,
    marker: &Instant,
    request_id: &RequestId,
) -> Value {
    let payload = ItemPayload::WeightEntry {
        weight_entry: WeightEntry {
            // Derived on read, never content: the write boundary states the constant (D1.16).
            has_weight_drop_alert: false,
            ..entry.clone()
        },
    };

    json!({
        "request_id": request_id.0,
        "household_id": household_id.0,
        "item_id": item_id(&entry.id).0,
        "updated_at": marker.0,
        "writer_id": writer.0,
        "payload": payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::types::Grams;
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::double::{RecordedCall, ScriptedService};

    fn request_id(value: &str) -> RequestId {
        RequestId(value.to_owned())
    }

    fn rabbit(value: &str) -> RabbitId {
        RabbitId(value.to_owned())
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

    /// The creation instant the caller minted when the draft omits a timestamp.
    fn creation_instant() -> Instant {
        at("2026-10-22T09:31:00Z")
    }

    /// A draft supplying only the value: no timestamp, no note, nothing confirmed.
    fn draft() -> WeightEntryDraft {
        WeightEntryDraft {
            value_grams: Grams(1_800),
            timestamp: None,
            note: None,
            outlier_confirmed: false,
        }
    }

    /// Record one entry exactly as the command path does, with the creation instant supplied.
    fn record_with(
        draft: WeightEntryDraft,
        request_id: RequestId,
    ) -> Result<WeightEntry, CommandError> {
        record(rabbit("rabbit-1"), draft, creation_instant, &request_id)
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

    /// The one write the transport was asked to submit.
    fn written_body(service: &ScriptedService) -> Value {
        let writes = written_bodies(service);

        assert_eq!(writes.len(), 1, "one create submits exactly one write");

        writes[0].clone()
    }

    /// WT-4, D1.9: the bound is inclusive. Exactly 10 kg is plausible, so it needs no confirmation.
    #[test]
    fn a_value_exactly_at_the_plausibility_bound_is_accepted_without_confirmation() {
        validate_value(PLAUSIBILITY_BOUND_GRAMS, false, &request_id("operation-1"))
            .expect("the bound itself is a plausible value");
    }

    /// WT-4, D1.9: one gram above the bound is a *confirmation* threshold — refused until confirmed,
    /// and accepted once it is, never clamped to the bound.
    #[test]
    fn one_gram_above_the_plausibility_bound_requires_confirmation() {
        let above = Grams(PLAUSIBILITY_BOUND_GRAMS.0 + 1);
        let operation = request_id("operation-1");

        let error = validate_value(above, false, &operation)
            .expect_err("an unconfirmed outlier is refused");

        assert_eq!(error.cause, crate::ipc::types::SaveCause::Validation);
        assert_eq!(error.request_id, Some(operation.clone()));

        validate_value(above, true, &operation).expect("a confirmed outlier is accepted");
    }

    /// WT-1, "An entry is recorded with the current time": an omitted timestamp records the creation
    /// instant the caller minted, not an invented one.
    #[test]
    fn an_omitted_timestamp_records_the_creation_instant() {
        let entry =
            record_with(draft(), request_id("operation-1")).expect("the draft records an entry");

        assert_eq!(entry.timestamp, creation_instant());
    }

    /// WT-1, "An entry records an edited timestamp": a supplied instant in the past is stored as that
    /// moment — in the returned entry **and on the wire** — and is never clamped to the present.
    #[test]
    fn a_supplied_past_timestamp_is_recorded_unchanged() {
        let service = ScriptedService::new();
        let supplied = at("2026-01-02T03:04:05Z");
        let operation = request_id("operation-1");
        let entry = record_with(
            WeightEntryDraft {
                timestamp: Some(supplied.clone()),
                ..draft()
            },
            operation.clone(),
        )
        .expect("the draft records an entry");
        let one = household("household-1");

        block_on_ready(save_weight_entry(
            &service,
            &one,
            &entry,
            &writer(),
            &marker(),
            operation,
        ))
        .expect("the double accepted the write");

        assert_eq!(entry.timestamp, supplied, "the past moment is kept");
        assert_ne!(
            entry.timestamp,
            creation_instant(),
            "a supplied instant is never replaced by the creation moment"
        );
        assert_eq!(
            written_body(&service)["payload"]["weight_entry"]["timestamp"],
            json!(supplied.0),
            "the stored payload carries the supplied moment, not the creation instant"
        );
    }

    /// DM-6: a timestamp is the moment, so two spellings of one instant are one stored timestamp.
    #[test]
    fn a_supplied_offset_names_the_same_moment_once_stored() {
        let entry = record_with(
            WeightEntryDraft {
                timestamp: Some(at("2026-01-02T05:04:05+02:00")),
                ..draft()
            },
            request_id("operation-1"),
        )
        .expect("the draft records an entry");

        assert_eq!(entry.timestamp, at("2026-01-02T03:04:05Z"));
    }

    /// WT-1, "The note is optional": no note is saved as no note, never as an empty string.
    #[test]
    fn an_entry_saved_without_a_note_carries_none() {
        let entry =
            record_with(draft(), request_id("operation-1")).expect("the draft records an entry");

        assert_eq!(entry.note, None);
    }

    /// WT-8's untruncated note: a supplied note is kept whole, with its line breaks and spacing.
    #[test]
    fn a_supplied_note_is_recorded_whole() {
        let note = "  weighed before breakfast\nand after the run  ";
        let entry = record_with(
            WeightEntryDraft {
                note: Some(note.to_owned()),
                ..draft()
            },
            request_id("operation-1"),
        )
        .expect("the draft records an entry");

        assert_eq!(entry.note, Some(note.to_owned()));
    }

    /// DM-1, "Every stored weight is in grams": the stored value is the canonical number the draft
    /// carried, and nothing converts it.
    #[test]
    fn every_stored_weight_is_grams() {
        let service = ScriptedService::new();
        let entry = record_with(
            WeightEntryDraft {
                value_grams: Grams(1_234),
                ..draft()
            },
            request_id("operation-1"),
        )
        .expect("the draft records an entry");
        let one = household("household-1");

        block_on_ready(save_weight_entry(
            &service,
            &one,
            &entry,
            &writer(),
            &marker(),
            request_id("operation-1"),
        ))
        .expect("the double accepted the write");

        let payload = written_body(&service)["payload"].clone();

        assert_eq!(payload["kind"], "weight_entry");
        assert_eq!(
            payload["weight_entry"]["value_grams"],
            json!(1_234),
            "the stored value is the gram value the draft carried"
        );
    }

    /// DM-1, "A weight entry carries no unit of its own": the stored payload's field set is exactly
    /// the entry's, and no unit field exists anywhere in it.
    #[test]
    fn a_stored_entry_carries_no_unit_of_its_own() {
        let service = ScriptedService::new();
        let entry =
            record_with(draft(), request_id("operation-1")).expect("the draft records an entry");
        let one = household("household-1");

        block_on_ready(save_weight_entry(
            &service,
            &one,
            &entry,
            &writer(),
            &marker(),
            request_id("operation-1"),
        ))
        .expect("the double accepted the write");

        let body = written_body(&service);
        let stored = body["payload"]["weight_entry"]
            .as_object()
            .expect("the payload carries the entry as an object");
        let mut keys: Vec<&str> = stored.keys().map(String::as_str).collect();
        keys.sort_unstable();

        assert_eq!(
            keys,
            vec![
                "created_at",
                "has_weight_drop_alert",
                "id",
                "note",
                "photo_refs",
                "rabbit_id",
                "timestamp",
                "value_grams",
            ],
            "no unit field is stored beside the grams value"
        );
        assert!(
            !body.to_string().contains("\"unit\""),
            "the write body names no unit at all"
        );
    }

    /// D1.16: the derived alert is written `false` and never carried in, even when the caller's value
    /// says otherwise — the stored payload can never state an alert the derivation would not.
    #[test]
    fn the_stored_payload_writes_the_derived_alert_as_false() {
        let service = ScriptedService::new();
        let entry = WeightEntry {
            has_weight_drop_alert: true,
            ..record_with(draft(), request_id("operation-1")).expect("the draft records an entry")
        };
        let one = household("household-1");

        block_on_ready(save_weight_entry(
            &service,
            &one,
            &entry,
            &writer(),
            &marker(),
            request_id("operation-1"),
        ))
        .expect("the double accepted the write");

        assert_eq!(
            written_body(&service)["payload"]["weight_entry"]["has_weight_drop_alert"],
            json!(false)
        );
    }

    /// D0.14, D1.17: an entry has no id before its create, so its id — and the item id it is stored
    /// under — are the operation's identity, and a retried create can therefore only be the same row.
    #[test]
    fn the_entry_and_its_item_are_the_create_operations_identity() {
        let operation = request_id("operation-1");
        let entry = record_with(draft(), operation.clone()).expect("the draft records an entry");

        assert_eq!(entry_id(&operation), entry.id);
        assert_eq!(item_id(&entry.id), ItemId(operation.0.clone()));
    }

    /// A blank operation identity names no item, so it is refused as `validation` before anything is
    /// recorded.
    #[test]
    fn a_blank_operation_identity_is_refused() {
        let operation = request_id("  ");
        let error =
            record_with(draft(), operation.clone()).expect_err("a blank identity names no item");

        assert_eq!(error.cause, crate::ipc::types::SaveCause::Validation);
        assert_eq!(error.request_id, Some(operation));
    }

    /// An unparseable timestamp is refused rather than replaced with the creation instant: a corrupt
    /// input must not masquerade as the moment of creation (DM-6).
    #[test]
    fn an_unparseable_timestamp_is_refused() {
        let operation = request_id("operation-1");
        let error = record_with(
            WeightEntryDraft {
                timestamp: Some(at("2026-01-02 03:04:05")),
                ..draft()
            },
            operation.clone(),
        )
        .expect_err("the text names no absolute instant");

        assert_eq!(error.cause, crate::ipc::types::SaveCause::Validation);
        assert_eq!(error.request_id, Some(operation));
    }

    /// The write body names the entry's item in the household the caller resolved, and carries the
    /// operation's marker and writer — the three facts the service's ledger and last-write-wins tuple
    /// read from the call (D0.7 criteria 3 and 6).
    #[test]
    fn the_write_body_names_the_entry_its_household_and_the_operations_marker() {
        let service = ScriptedService::new();
        let operation = request_id("operation-1");
        let entry = record_with(draft(), operation.clone()).expect("the draft records an entry");
        let one = household("household-1");

        block_on_ready(save_weight_entry(
            &service,
            &one,
            &entry,
            &writer(),
            &marker(),
            operation.clone(),
        ))
        .expect("the double accepted the write");

        let body = written_body(&service);

        assert_eq!(body["request_id"], json!("operation-1"));
        assert_eq!(body["household_id"], json!("household-1"));
        assert_eq!(body["item_id"], json!("operation-1"));
        assert_eq!(body["updated_at"], json!("2026-10-22T09:30:00Z"));
        assert_eq!(body["writer_id"], json!("member-1"));
        assert!(
            body.get("deleted").is_none(),
            "a create omits the removal flag and lets the service default it"
        );
    }

    /// Two creates with different operation identities are two entries, even when every recorded
    /// value is identical: content never names an item (RAB-3's rule, applied to an entry).
    #[test]
    fn two_operations_record_two_entries_with_the_same_content() {
        let first = record_with(draft(), request_id("operation-1")).expect("recorded");
        let second = record_with(draft(), request_id("operation-2")).expect("recorded");

        assert_ne!(first.id, second.id);
        assert_eq!(first.value_grams, second.value_grams);
    }

    /// WT-5: an edit that omits the timestamp keeps the entry's stored instant. A back-dated entry is
    /// never restamped by a correction, which is the frozen draft's `None` on update.
    #[test]
    fn an_edit_that_omits_the_timestamp_keeps_the_stored_instant() {
        let stored = record_with(
            WeightEntryDraft {
                timestamp: Some(at("2026-01-02T03:04:05Z")),
                ..draft()
            },
            request_id("operation-1"),
        )
        .expect("the draft records an entry");

        let revised =
            revise(&stored, draft(), &request_id("operation-2")).expect("the revision is accepted");

        assert_eq!(revised.timestamp, at("2026-01-02T03:04:05Z"));
        assert_eq!(revised.timestamp, stored.timestamp);
    }

    /// WT-5: a supplied timestamp replaces the stored one, and an offset spelling is the same moment
    /// once the storage boundary normalises it (DM-6).
    #[test]
    fn an_edit_applies_the_supplied_timestamp() {
        let stored = record_with(draft(), request_id("operation-1")).expect("recorded");

        let revised = revise(
            &stored,
            WeightEntryDraft {
                timestamp: Some(at("2026-01-02T05:04:05+02:00")),
                ..draft()
            },
            &request_id("operation-2"),
        )
        .expect("the revision is accepted");

        assert_eq!(revised.timestamp, at("2026-01-02T03:04:05Z"));
    }

    /// WT-5: the editable set is replaced — value and note — while the identity, the rabbit and the
    /// photos come from the stored entry and cannot be changed by a draft.
    #[test]
    fn an_edit_replaces_the_value_and_note_and_keeps_the_identity_rabbit_and_photos() {
        let mut stored = record_with(
            WeightEntryDraft {
                note: Some("before correction".to_owned()),
                ..draft()
            },
            request_id("operation-1"),
        )
        .expect("recorded");
        stored.photo_refs = vec!["photo-1".to_owned()];

        let revised = revise(
            &stored,
            WeightEntryDraft {
                value_grams: Grams(1_500),
                note: Some("corrected".to_owned()),
                ..draft()
            },
            &request_id("operation-2"),
        )
        .expect("the revision is accepted");

        assert_eq!(revised.id, stored.id, "an edit never renames the item");
        assert_eq!(revised.rabbit_id, stored.rabbit_id);
        assert_eq!(revised.photo_refs, stored.photo_refs);
        assert_eq!(revised.value_grams, Grams(1_500));
        assert_eq!(revised.note.as_deref(), Some("corrected"));
    }

    /// WT-5: a blank note in the pre-filled form is no note, not the stored one (the draft is the
    /// whole editable set).
    #[test]
    fn an_edit_with_no_note_clears_the_stored_note() {
        let stored = record_with(
            WeightEntryDraft {
                note: Some("morning".to_owned()),
                ..draft()
            },
            request_id("operation-1"),
        )
        .expect("recorded");

        let revised =
            revise(&stored, draft(), &request_id("operation-2")).expect("the revision is accepted");

        assert_eq!(revised.note, None);
    }

    /// D1.16: the derived alert is written `false` even when the stored value says otherwise — it is
    /// re-derived on read, never carried into a write.
    #[test]
    fn an_edit_normalises_the_derived_alert_to_false() {
        let stored = WeightEntry {
            has_weight_drop_alert: true,
            ..record_with(draft(), request_id("operation-1")).expect("recorded")
        };

        let revised =
            revise(&stored, draft(), &request_id("operation-2")).expect("the revision is accepted");

        assert!(!revised.has_weight_drop_alert);
    }

    /// WT-4, D1.9: an edit applies the same value rule as a create — zero is refused, an unconfirmed
    /// outlier is refused, and a confirmed outlier is accepted at its true value.
    #[test]
    fn an_edit_applies_the_same_value_rules_as_a_create() {
        let stored = record_with(draft(), request_id("operation-1")).expect("recorded");
        let operation = request_id("operation-2");

        for refused in [
            WeightEntryDraft {
                value_grams: Grams(0),
                ..draft()
            },
            WeightEntryDraft {
                value_grams: Grams(PLAUSIBILITY_BOUND_GRAMS.0 + 1),
                outlier_confirmed: false,
                ..draft()
            },
        ] {
            let error = revise(&stored, refused, &operation).expect_err("the value is refused");
            assert_eq!(error.cause, crate::ipc::types::SaveCause::Validation);
            assert_eq!(error.request_id, Some(operation.clone()));
        }

        let confirmed = revise(
            &stored,
            WeightEntryDraft {
                value_grams: Grams(30_000),
                outlier_confirmed: true,
                ..draft()
            },
            &operation,
        )
        .expect("a confirmed outlier is accepted");

        assert_eq!(confirmed.value_grams, Grams(30_000), "never clamped");
    }

    /// A blank operation identity names no operation, so an edit is refused before anything could be
    /// composed.
    #[test]
    fn an_edit_with_a_blank_operation_identity_is_refused() {
        let stored = record_with(draft(), request_id("operation-1")).expect("recorded");
        let operation = request_id("  ");

        let error = revise(&stored, draft(), &operation).expect_err("a blank identity is refused");

        assert_eq!(error.cause, crate::ipc::types::SaveCause::Validation);
        assert_eq!(error.request_id, Some(operation));
    }

    /// An unparseable timestamp is refused rather than silently replaced with the stored value: a
    /// corrupt input must not masquerade as the entry's existing moment (DM-6).
    #[test]
    fn an_edit_with_an_unparseable_timestamp_is_refused() {
        let stored = record_with(draft(), request_id("operation-1")).expect("recorded");
        let operation = request_id("operation-2");

        let error = revise(
            &stored,
            WeightEntryDraft {
                timestamp: Some(at("2026-01-02 03:04:05")),
                ..draft()
            },
            &operation,
        )
        .expect_err("the text names no absolute instant");

        assert_eq!(error.cause, crate::ipc::types::SaveCause::Validation);
        assert_eq!(error.request_id, Some(operation));
    }

    /// The rabbit a trend is read for, with the default 5% threshold (RAB-1, WT-7).
    fn stored_rabbit() -> Rabbit {
        Rabbit {
            id: rabbit("rabbit-1"),
            household_id: household("household-1"),
            name: "Pebble".to_owned(),
            photo_ref: None,
            breed: None,
            dob: None,
            dob_unknown: true,
            sex: crate::ipc::types::Sex::Female,
            neuter_status: crate::ipc::types::NeuterStatus::Spayed,
            adoption_date: None,
            is_archived: false,
            weight_drop_threshold_percent: DEFAULT_DROP_THRESHOLD_PERCENT,
        }
    }

    /// One stored entry of `rabbit` at the instant and value given, created at that same instant.
    fn stored_entry(rabbit: &Rabbit, id: &str, value: i32, timestamp: &str) -> WeightEntry {
        created_entry(rabbit, id, value, timestamp, timestamp)
    }

    /// One stored entry with its reading timestamp and its recorded creation instant stated
    /// separately — the tie case WT-7's order exists for (D1.12).
    fn created_entry(
        rabbit: &Rabbit,
        id: &str,
        value: i32,
        timestamp: &str,
        created_at: &str,
    ) -> WeightEntry {
        WeightEntry {
            id: WeightEntryId(id.to_owned()),
            rabbit_id: rabbit.id.clone(),
            value_grams: Grams(value),
            timestamp: at(timestamp),
            created_at: at(created_at),
            note: None,
            photo_refs: Vec::new(),
            has_weight_drop_alert: false,
        }
    }

    /// One live row carrying the entry's content (D1.16).
    fn entry_row(entry: &WeightEntry) -> crate::ipc::types::CollectionItem {
        row_at(entry, &entry.timestamp)
    }

    /// One live row carrying the rabbit's content, so the detail read can read its threshold (WT-8).
    fn rabbit_row(rabbit: &Rabbit) -> crate::ipc::types::CollectionItem {
        crate::ipc::types::CollectionItem {
            item_id: ItemId(rabbit.id.0.clone()),
            updated_at: at("2026-10-22T09:30:00Z"),
            deleted: false,
            payload: Some(ItemPayload::Rabbit {
                rabbit: rabbit.clone(),
            }),
        }
    }

    /// One live entry row whose LWW `updated_at` is stated separately from the entry's content, so a
    /// test can show that version marker plays no part in ordering (D1.12).
    fn row_at(entry: &WeightEntry, updated_at: &Instant) -> crate::ipc::types::CollectionItem {
        crate::ipc::types::CollectionItem {
            item_id: ItemId(entry.id.0.clone()),
            updated_at: updated_at.clone(),
            deleted: false,
            payload: Some(ItemPayload::WeightEntry {
                weight_entry: entry.clone(),
            }),
        }
    }

    /// The collection one rabbit and its entry rows form.
    fn collection(
        rabbit: &Rabbit,
        items: Vec<crate::ipc::types::CollectionItem>,
    ) -> HouseholdCollection {
        HouseholdCollection {
            household_id: rabbit.household_id.clone(),
            items,
            source: crate::ipc::types::CollectionSource::Local,
            loaded_at: None,
        }
    }

    /// WT-6, "Selecting a range limits the trend to that window": entries spanning six months yield
    /// only the entries of the last 30 days, oldest first.
    #[test]
    fn a_thirty_day_window_returns_only_the_entries_inside_it_oldest_first() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-1", 1_500, "2026-04-23T00:00:00Z"),
            stored_entry(&rabbit, "entry-2", 1_600, "2026-09-23T00:00:00Z"),
            stored_entry(&rabbit, "entry-3", 1_700, "2026-10-01T00:00:00Z"),
            stored_entry(&rabbit, "entry-4", 1_800, "2026-10-17T00:00:00Z"),
            stored_entry(&rabbit, "entry-5", 1_900, "2026-10-22T00:00:00Z"),
        ];
        let rows = entries.iter().map(entry_row).collect();
        let collection = collection(&rabbit, rows);

        let trend = trend(
            &collection,
            &rabbit,
            WeightRange::Days30,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect("the collection is readable");

        assert_eq!(trend.range, WeightRange::Days30);
        assert_eq!(
            trend
                .points
                .iter()
                .map(|point| point.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-2", "entry-3", "entry-4", "entry-5"],
            "the April entry is outside the 30-day window and the rest are oldest first"
        );
    }

    /// WT-6, "The full-history range presents every entry": `All` is the absence of a lower bound, so
    /// every entry of the rabbit is returned.
    #[test]
    fn the_full_history_range_returns_every_entry() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-1", 1_500, "2026-01-01T00:00:00Z"),
            stored_entry(&rabbit, "entry-2", 1_700, "2026-10-01T00:00:00Z"),
            stored_entry(&rabbit, "entry-3", 1_900, "2026-10-22T00:00:00Z"),
        ];
        let rows = entries.iter().map(entry_row).collect();
        let collection = collection(&rabbit, rows);

        let trend = trend(
            &collection,
            &rabbit,
            WeightRange::All,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect("the collection is readable");

        assert_eq!(trend.points.len(), 3);
        assert_eq!(trend.points[0].id.0, "entry-1");
        assert_eq!(trend.points[2].id.0, "entry-3");
    }

    /// WT-7, D1.12: a supplied reading timestamp does not stand in for creation order, so the entry
    /// records the operation's creation instant beside it.
    #[test]
    fn an_entry_records_its_creation_instant_even_when_the_timestamp_is_supplied() {
        let supplied = at("2026-01-02T03:04:05Z");
        let entry = record_with(
            WeightEntryDraft {
                timestamp: Some(supplied.clone()),
                ..draft()
            },
            request_id("operation-1"),
        )
        .expect("the draft records an entry");

        assert_eq!(entry.timestamp, supplied, "the reading moment is kept");
        assert_eq!(
            entry.created_at,
            creation_instant(),
            "creation order is the operation's instant, not the supplied timestamp"
        );
        assert_ne!(entry.created_at, entry.timestamp);
    }

    /// WT-5, WT-7, D1.12: a correction replaces the editable set and leaves the entry's place in the
    /// journal alone — `created_at` comes from the target, exactly as the identity does.
    #[test]
    fn a_correction_preserves_the_recorded_creation_instant() {
        let rabbit = stored_rabbit();
        let stored = created_entry(
            &rabbit,
            "entry-1",
            1_800,
            "2026-10-01T00:00:00Z",
            "2026-09-01T00:00:00Z",
        );

        let revised = revise(
            &stored,
            WeightEntryDraft {
                value_grams: Grams(1_700),
                timestamp: Some(at("2026-10-02T00:00:00Z")),
                ..draft()
            },
            &request_id("operation-2"),
        )
        .expect("the revision is accepted");

        assert_eq!(revised.timestamp, at("2026-10-02T00:00:00Z"));
        assert_eq!(
            revised.created_at, stored.created_at,
            "an edit moves the value and timestamp, never the creation order"
        );
    }

    /// WT-7, D1.12: two entries sharing a timestamp are ordered by their recorded creation instant,
    /// not by the id — the ids here sort opposite to creation order on purpose.
    #[test]
    fn entries_sharing_a_timestamp_are_ordered_by_creation_instant() {
        let rabbit = stored_rabbit();
        let earlier = created_entry(
            &rabbit,
            "entry-z",
            1_900,
            "2026-10-22T00:00:00Z",
            "2026-10-22T09:00:00Z",
        );
        let later = created_entry(
            &rabbit,
            "entry-a",
            1_800,
            "2026-10-22T00:00:00Z",
            "2026-10-22T10:00:00Z",
        );
        let collection = collection(&rabbit, vec![entry_row(&later), entry_row(&earlier)]);

        let trend = trend(
            &collection,
            &rabbit,
            WeightRange::All,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect("the collection is readable");

        assert_eq!(
            trend
                .points
                .iter()
                .map(|point| point.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-z", "entry-a"],
            "the earlier-created entry precedes the later one even though its id sorts later"
        );
    }

    /// WT-7, D1.12's discriminating case: the earlier-created entry was corrected *after* the later
    /// one, so its LWW `updated_at` is the newest of the two. Creation order is not `updated_at`, so
    /// the order is unchanged.
    #[test]
    fn a_correction_after_the_later_entry_does_not_reorder_the_series() {
        let rabbit = stored_rabbit();
        let earlier = created_entry(
            &rabbit,
            "entry-z",
            1_900,
            "2026-10-22T00:00:00Z",
            "2026-10-22T09:00:00Z",
        );
        let later = created_entry(
            &rabbit,
            "entry-a",
            1_800,
            "2026-10-22T00:00:00Z",
            "2026-10-22T10:00:00Z",
        );
        let rows = vec![
            // `entry-z` was edited most recently; an `updated_at` rule would put it last.
            row_at(&earlier, &at("2026-10-23T12:00:00Z")),
            row_at(&later, &at("2026-10-22T10:05:00Z")),
        ];
        let collection = collection(&rabbit, rows);

        let trend = trend(
            &collection,
            &rabbit,
            WeightRange::All,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect("the collection is readable");

        assert_eq!(
            trend
                .points
                .iter()
                .map(|point| point.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-z", "entry-a"],
            "creation order survives a correction made after the later entry"
        );
    }

    /// WT-7, D1.12: when the timestamp and the creation instant are both equal, the id closes the
    /// order. The rows are inserted in descending id order so an unsorted or insertion-ordered result
    /// would fail; `entry-a` must come first.
    #[test]
    fn equal_timestamps_and_creation_instants_are_ordered_by_id() {
        let rabbit = stored_rabbit();
        let later_id = created_entry(
            &rabbit,
            "entry-z",
            1_900,
            "2026-10-22T00:00:00Z",
            "2026-10-22T09:00:00Z",
        );
        let earlier_id = created_entry(
            &rabbit,
            "entry-a",
            1_800,
            "2026-10-22T00:00:00Z",
            "2026-10-22T09:00:00Z",
        );
        let collection = collection(&rabbit, vec![entry_row(&later_id), entry_row(&earlier_id)]);

        let trend = trend(
            &collection,
            &rabbit,
            WeightRange::All,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect("the collection is readable");

        assert_eq!(
            trend
                .points
                .iter()
                .map(|point| point.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-a", "entry-z"],
            "a same-instant tie falls back to the entry id, never to insertion order"
        );
    }

    /// The derived flags of `entries`, in order, as `(id, flagged)` pairs.
    fn alert_flags(rabbit: &Rabbit, entries: &[WeightEntry]) -> Vec<(String, bool)> {
        alerts(entries, rabbit.weight_drop_threshold_percent)
            .expect("the entries order")
            .into_iter()
            .map(|alert| (alert.id.0, alert.has_weight_drop_alert))
            .collect()
    }

    /// WT-7, "A drop exceeding the threshold is flagged": 2000 g → 1800 g at 5% is a 10% drop.
    #[test]
    fn a_drop_beyond_the_threshold_is_flagged() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z"),
            stored_entry(&rabbit, "entry-2", 1_800, "2026-10-21T00:00:00Z"),
        ];

        assert_eq!(
            alert_flags(&rabbit, &entries),
            vec![("entry-1".to_owned(), false), ("entry-2".to_owned(), true)]
        );
    }

    /// WT-7, "A drop within the threshold is not flagged": 2000 g → 1950 g at 5% is a 2.5% drop.
    #[test]
    fn a_drop_within_the_threshold_is_not_flagged() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z"),
            stored_entry(&rabbit, "entry-2", 1_950, "2026-10-21T00:00:00Z"),
        ];

        assert_eq!(
            alert_flags(&rabbit, &entries),
            vec![("entry-1".to_owned(), false), ("entry-2".to_owned(), false)]
        );
    }

    /// WT-7's word is "exceeds", so the comparison is strict: a drop exactly at the threshold is not
    /// flagged, and one gram more is. 2000 → 1900 is exactly 5%; 2000 → 1899 is 5.05%.
    #[test]
    fn a_drop_exactly_at_the_threshold_is_not_flagged_and_one_gram_beyond_is() {
        let rabbit = stored_rabbit();
        let at_threshold = [
            stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z"),
            stored_entry(&rabbit, "entry-2", 1_900, "2026-10-21T00:00:00Z"),
        ];
        let beyond = [
            stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z"),
            stored_entry(&rabbit, "entry-2", 1_899, "2026-10-21T00:00:00Z"),
        ];

        assert!(
            !alert_flags(&rabbit, &at_threshold)[1].1,
            "equality is not `exceeds`"
        );
        assert!(alert_flags(&rabbit, &beyond)[1].1);
    }

    /// WT-7: the derivation's second safeguard. A negative or non-finite threshold should never reach
    /// it — the profile boundary refuses one — but if it did, it must not flag a gain or an
    /// equal-value pair: neither is a drop.
    #[test]
    fn a_negative_or_non_finite_threshold_flags_no_entry() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z"),
            stored_entry(&rabbit, "entry-2", 2_000, "2026-10-21T00:00:00Z"),
            stored_entry(&rabbit, "entry-3", 2_100, "2026-10-22T00:00:00Z"),
        ];

        for threshold in [-5.0, f64::NEG_INFINITY, f64::INFINITY, f64::NAN] {
            let flags = alerts(&entries, threshold).expect("the entries order");

            assert!(
                flags.iter().all(|alert| !alert.has_weight_drop_alert),
                "{threshold} flags no entry: an equal value and a gain are not drops"
            );
        }
    }

    /// WT-7: the first entry has no predecessor, so it is never flagged — whatever its value.
    #[test]
    fn the_first_entry_is_never_flagged() {
        let rabbit = stored_rabbit();
        let entries = [stored_entry(
            &rabbit,
            "entry-1",
            1_000,
            "2026-10-20T00:00:00Z",
        )];

        assert_eq!(
            alert_flags(&rabbit, &entries),
            vec![("entry-1".to_owned(), false)]
        );
    }

    /// WT-7, "Raising the threshold clears an existing flag" and "Lowering the threshold raises a
    /// flag": the same entries are re-derived against three thresholds, and no entry is rewritten.
    /// `entry-2` drops 10%; `entry-3` drops 2.5% against it.
    #[test]
    fn a_threshold_change_re_derives_the_flags_without_rewriting_an_entry() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z"),
            stored_entry(&rabbit, "entry-2", 1_800, "2026-10-21T00:00:00Z"),
            stored_entry(&rabbit, "entry-3", 1_755, "2026-10-22T00:00:00Z"),
        ];
        let as_stored = entries.clone();

        assert_eq!(
            alert_flags(&rabbit, &entries),
            vec![
                ("entry-1".to_owned(), false),
                ("entry-2".to_owned(), true),
                ("entry-3".to_owned(), false),
            ],
            "at 5% the 10% drop is flagged and the 2.5% drop is not"
        );

        let raised = alerts(&entries, 15.0).expect("the entries order");
        assert_eq!(
            raised
                .iter()
                .filter(|alert| alert.has_weight_drop_alert)
                .count(),
            0,
            "raising the threshold above every drop clears the flags"
        );

        let lowered = alerts(&entries, 2.0).expect("the entries order");
        assert_eq!(
            lowered
                .iter()
                .filter(|alert| alert.has_weight_drop_alert)
                .count(),
            2,
            "lowering it below the 2.5% drop raises the flag"
        );

        assert_eq!(
            entries, as_stored,
            "a threshold change derives flags only; no stored entry is rewritten"
        );
        assert!(
            entries.iter().all(|entry| !entry.has_weight_drop_alert),
            "every stored entry still carries the write boundary's constant `false`"
        );
    }

    /// WT-7, "Entries sharing a timestamp are ordered by creation": the earlier-created entry is the
    /// predecessor, and the later-created one's drop is measured against it. The ids sort opposite to
    /// creation order, so an id-based tie rule would flip the result.
    #[test]
    fn entries_sharing_a_timestamp_are_judged_against_the_earlier_created_entry() {
        let rabbit = stored_rabbit();
        let earlier = created_entry(
            &rabbit,
            "entry-z",
            2_000,
            "2026-10-22T00:00:00Z",
            "2026-10-22T09:00:00Z",
        );
        let later = created_entry(
            &rabbit,
            "entry-a",
            1_800,
            "2026-10-22T00:00:00Z",
            "2026-10-22T10:00:00Z",
        );

        assert_eq!(
            alert_flags(&rabbit, &[later, earlier]),
            vec![("entry-z".to_owned(), false), ("entry-a".to_owned(), true)],
            "the later-created entry is judged against the earlier-created one"
        );
    }

    /// WT-7, "Deleting an entry re-derives the following entry's alert": with the middle entry gone,
    /// the remaining later entry is compared against the one that now precedes it.
    #[test]
    fn deleting_an_entry_re_derives_the_following_entry_against_its_new_predecessor() {
        let rabbit = stored_rabbit();
        let first = stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z");
        let middle = stored_entry(&rabbit, "entry-2", 1_500, "2026-10-21T00:00:00Z");
        let following = stored_entry(&rabbit, "entry-3", 1_600, "2026-10-22T00:00:00Z");

        assert!(
            !alert_flags(&rabbit, &[first.clone(), middle, following.clone()])[2].1,
            "against the 1500 g entry the 1600 g reading is a gain"
        );
        assert!(
            alert_flags(&rabbit, &[first, following])[1].1,
            "against the 2000 g entry the 1600 g reading is a 20% drop"
        );
    }

    /// WT-7, "Editing an entry re-derives the alerts around it": the edited entry's own flag and the
    /// following entry's flag both change with the corrected value.
    #[test]
    fn editing_an_entry_re_derives_its_own_flag_and_the_following_entrys() {
        let rabbit = stored_rabbit();
        let first = stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z");
        let edited = stored_entry(&rabbit, "entry-2", 1_900, "2026-10-21T00:00:00Z");
        let following = stored_entry(&rabbit, "entry-3", 1_800, "2026-10-22T00:00:00Z");

        let before = alert_flags(&rabbit, &[first.clone(), edited.clone(), following.clone()]);
        assert_eq!(
            before,
            vec![
                ("entry-1".to_owned(), false),
                ("entry-2".to_owned(), false),
                ("entry-3".to_owned(), true),
            ],
            "the 5% drop is at the threshold; the 5.26% drop beyond it is flagged"
        );

        let corrected = stored_entry(&rabbit, "entry-2", 1_750, "2026-10-21T00:00:00Z");
        let after = alert_flags(&rabbit, &[first, corrected, following]);

        assert_eq!(
            after,
            vec![
                ("entry-1".to_owned(), false),
                ("entry-2".to_owned(), true),
                ("entry-3".to_owned(), false),
            ],
            "the corrected entry is now flagged and its follower no longer is"
        );
    }

    /// I3: the window limits the points, never the derivation. Entries outside a window are still the
    /// predecessors of its earliest points, and every returned flag equals the one the full set
    /// produced for the same entry.
    #[test]
    fn a_point_whose_predecessor_is_outside_the_window_is_judged_against_it() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-old", 2_000, "2026-01-01T00:00:00Z"),
            stored_entry(&rabbit, "entry-mid", 1_700, "2026-09-01T00:00:00Z"),
            stored_entry(&rabbit, "entry-recent", 1_600, "2026-10-22T00:00:00Z"),
        ];
        let rows = entries.iter().map(entry_row).collect();
        let collection = collection(&rabbit, rows);
        let now = at("2026-10-23T00:00:00Z");

        let full: HashMap<WeightEntryId, bool> = alerts(&entries, DEFAULT_DROP_THRESHOLD_PERCENT)
            .expect("the entries order")
            .into_iter()
            .map(|alert| (alert.id, alert.has_weight_drop_alert))
            .collect();

        for (range, expected) in [
            (WeightRange::Days30, vec![("entry-recent", true)]),
            (
                WeightRange::Days90,
                vec![("entry-mid", true), ("entry-recent", true)],
            ),
            (
                WeightRange::All,
                vec![
                    ("entry-old", false),
                    ("entry-mid", true),
                    ("entry-recent", true),
                ],
            ),
        ] {
            let trend =
                trend(&collection, &rabbit, range, &now).expect("the collection is readable");

            assert_eq!(
                trend
                    .points
                    .iter()
                    .map(|point| (point.id.0.as_str(), point.has_weight_drop_alert))
                    .collect::<Vec<_>>(),
                expected,
                "each point is judged against its full-set predecessor in {range:?}"
            );

            for point in &trend.points {
                assert_eq!(
                    Some(&point.has_weight_drop_alert),
                    full.get(&point.id),
                    "a window's flag equals the full set's flag for the same entry"
                );
            }
        }
    }

    /// WT-7, D1.3, D1.16: the rabbit's threshold travels with the series so a point is judged against
    /// the same number that produced it, and a stored payload's `true` is ignored: the flag is derived
    /// on read, and a lone entry has no predecessor to be flagged against.
    #[test]
    fn the_trend_carries_the_rabbits_threshold_and_ignores_a_stored_alert() {
        let rabbit = Rabbit {
            weight_drop_threshold_percent: 8.5,
            ..stored_rabbit()
        };
        let entry = WeightEntry {
            has_weight_drop_alert: true,
            ..stored_entry(&rabbit, "entry-1", 1_800, "2026-10-22T00:00:00Z")
        };
        let collection = collection(&rabbit, vec![entry_row(&entry)]);

        let trend = trend(
            &collection,
            &rabbit,
            WeightRange::All,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect("the collection is readable");

        assert_eq!(trend.drop_threshold_percent, 8.5);
        assert!(
            !trend.points[0].has_weight_drop_alert,
            "the stored flag is never read back; the derivation has no predecessor to flag against"
        );
    }

    /// WT-6's 90-day window is exercised directly, so a wrong day count in that arm cannot pass the
    /// suite. `2026-07-25T00:00:00Z` is exactly 90 days before `2026-10-23T00:00:00Z`.
    #[test]
    fn the_ninety_day_window_has_its_own_cutoff() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-early", 1_500, "2026-07-24T23:59:59Z"),
            stored_entry(&rabbit, "entry-edge", 1_600, "2026-07-25T00:00:00Z"),
            stored_entry(&rabbit, "entry-late", 1_700, "2026-10-22T00:00:00Z"),
        ];
        let rows = entries.iter().map(entry_row).collect();
        let collection = collection(&rabbit, rows);

        let trend = trend(
            &collection,
            &rabbit,
            WeightRange::Days90,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect("the collection is readable");

        assert_eq!(
            trend
                .points
                .iter()
                .map(|point| point.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-edge", "entry-late"],
            "90 days, inclusive at the cut-off; 89 days and 23:59:59 is outside"
        );
    }

    /// WT-6, DASH-7: a bounded range presents "the entries of the last N days", so an entry stamped in
    /// the future is outside it. `All` has no upper bound and still presents that entry (D1.4).
    #[test]
    fn a_future_timestamp_is_outside_a_bounded_range_but_inside_the_full_history() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-now", 1_800, "2026-10-23T00:00:00Z"),
            stored_entry(&rabbit, "entry-future", 1_700, "2026-12-01T00:00:00Z"),
        ];
        let rows = entries.iter().map(entry_row).collect();
        let collection = collection(&rabbit, rows);
        let now = at("2026-10-23T00:00:00Z");

        let thirty = trend(&collection, &rabbit, WeightRange::Days30, &now)
            .expect("the collection is readable");
        let all = trend(&collection, &rabbit, WeightRange::All, &now)
            .expect("the collection is readable");

        assert_eq!(
            thirty
                .points
                .iter()
                .map(|point| point.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-now"],
            "the upper edge is `now`; the future entry is not in the last 30 days"
        );
        assert_eq!(
            all.points
                .iter()
                .map(|point| point.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-now", "entry-future"],
            "the full history has no upper bound"
        );
    }

    /// WT-6, "The weight history range is independent of the trend range": two reads over the same
    /// rabbit, with different ranges, each answer their own window and neither changes the other.
    #[test]
    fn two_ranges_over_the_same_rabbit_answer_independently() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-1", 1_500, "2026-08-01T00:00:00Z"),
            stored_entry(&rabbit, "entry-2", 1_600, "2026-10-01T00:00:00Z"),
            stored_entry(&rabbit, "entry-3", 1_800, "2026-10-21T00:00:00Z"),
        ];
        let rows = entries.iter().map(entry_row).collect();
        let collection = collection(&rabbit, rows);
        let now = at("2026-10-23T00:00:00Z");

        let thirty = trend(&collection, &rabbit, WeightRange::Days30, &now)
            .expect("the collection is readable");
        let seven = trend(&collection, &rabbit, WeightRange::Days7, &now)
            .expect("the collection is readable");
        let thirty_again = trend(&collection, &rabbit, WeightRange::Days30, &now)
            .expect("the collection is readable");

        assert_eq!(
            thirty
                .points
                .iter()
                .map(|point| point.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-2", "entry-3"],
            "the 30-day read keeps its own window"
        );
        assert_eq!(
            seven
                .points
                .iter()
                .map(|point| point.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-3"],
            "the 7-day read is not narrowed or widened by the other"
        );
        assert_eq!(
            thirty_again.points, thirty.points,
            "the earlier range is unchanged by the later read"
        );
    }

    /// WT-6's boundary: the cut-off is inclusive. An entry exactly seven days before `now` is inside
    /// the window; an entry one second earlier is outside.
    #[test]
    fn an_entry_exactly_at_the_seven_day_cutoff_is_inside_and_a_second_earlier_is_outside() {
        let rabbit = stored_rabbit();
        let entries = [
            stored_entry(&rabbit, "entry-early", 1_500, "2026-10-15T23:59:59Z"),
            stored_entry(&rabbit, "entry-edge", 1_600, "2026-10-16T00:00:00Z"),
            stored_entry(&rabbit, "entry-late", 1_700, "2026-10-22T00:00:00Z"),
        ];
        let rows = entries.iter().map(entry_row).collect();
        let collection = collection(&rabbit, rows);

        let trend = trend(
            &collection,
            &rabbit,
            WeightRange::Days7,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect("the collection is readable");

        assert_eq!(
            trend
                .points
                .iter()
                .map(|point| point.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-edge", "entry-late"],
            "the entry exactly at the cut-off is inside; one second earlier is outside"
        );
    }

    /// D1.17: a tombstone is excluded before the content check, so a removed entry neither appears nor
    /// refuses the read even though its retained content was dropped at decode.
    #[test]
    fn a_tombstoned_entry_is_excluded_before_the_content_check() {
        let rabbit = stored_rabbit();
        let live = stored_entry(&rabbit, "entry-1", 1_800, "2026-10-22T00:00:00Z");
        let rows = vec![
            entry_row(&live),
            crate::ipc::types::CollectionItem {
                item_id: ItemId("entry-2".to_owned()),
                updated_at: at("2026-10-22T00:00:00Z"),
                deleted: true,
                payload: None,
            },
        ];
        let collection = collection(&rabbit, rows);

        let trend = trend(
            &collection,
            &rabbit,
            WeightRange::All,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect("a tombstone is not unreadable content");

        assert_eq!(trend.points.len(), 1);
        assert_eq!(trend.points[0].id.0, "entry-1");
    }

    /// D1.16: a live row whose content is not on this device could be an entry of this rabbit (or its
    /// predecessor), so the read refuses rather than answering a subset.
    #[test]
    fn a_live_row_with_unknown_content_refuses_the_read() {
        let rabbit = stored_rabbit();
        let live = stored_entry(&rabbit, "entry-1", 1_800, "2026-10-22T00:00:00Z");
        let rows = vec![
            entry_row(&live),
            crate::ipc::types::CollectionItem {
                item_id: ItemId("entry-2".to_owned()),
                updated_at: at("2026-10-22T00:00:00Z"),
                deleted: false,
                payload: None,
            },
        ];
        let collection = collection(&rabbit, rows);

        let error = trend(
            &collection,
            &rabbit,
            WeightRange::All,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect_err("an unreadable row leaves the entry set partial");

        assert_eq!(error.cause, crate::ipc::types::SaveCause::Unknown);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_eq!(error.request_id, None);
    }

    /// Only the named rabbit's entries are in its series; another rabbit's entry in the same
    /// collection is not a point of this trend.
    #[test]
    fn another_rabbits_entry_is_not_part_of_the_series() {
        let rabbit = stored_rabbit();
        let mut other = stored_rabbit();
        other.id = RabbitId("rabbit-2".to_owned());
        let mine = stored_entry(&rabbit, "entry-1", 1_800, "2026-10-22T00:00:00Z");
        let theirs = stored_entry(&other, "entry-2", 1_600, "2026-10-22T00:00:00Z");
        let collection = collection(&rabbit, vec![entry_row(&mine), entry_row(&theirs)]);

        let trend = trend(
            &collection,
            &rabbit,
            WeightRange::All,
            &at("2026-10-23T00:00:00Z"),
        )
        .expect("the collection is readable");

        assert_eq!(trend.points.len(), 1);
        assert_eq!(trend.points[0].id.0, "entry-1");
    }

    /// WT-8, "The detail shows the value in the display unit and the untruncated note": the read
    /// returns the stored grams, the timestamp and the note whole — newlines, spacing and length
    /// included — with the photo references in their stored order. The display unit is not applied
    /// here (I1); `api.ts` formats the grams.
    #[test]
    fn the_detail_returns_the_stored_grams_timestamp_whole_note_and_photos() {
        let rabbit = stored_rabbit();
        let note = "weighed after the long run\n\
                    and before breakfast  \n\
                    the vet asked us to keep a weekly log, so this note is deliberately long \
                    enough that any truncation shorter than the whole text would be visible in \
                    the assertion below";
        let entry = WeightEntry {
            note: Some(note.to_owned()),
            photo_refs: vec!["photo-2".to_owned(), "photo-1".to_owned()],
            ..stored_entry(&rabbit, "entry-1", 1_842, "2026-10-22T07:15:00Z")
        };
        let collection = collection(&rabbit, vec![rabbit_row(&rabbit), entry_row(&entry)]);

        let detail = detail(&collection, &entry).expect("the collection is readable");

        assert_eq!(
            detail.value_grams,
            Grams(1_842),
            "the value is the stored grams"
        );
        assert_eq!(detail.timestamp, at("2026-10-22T07:15:00Z"));
        assert_eq!(
            detail.note.as_deref(),
            Some(note),
            "the note is returned whole, with its newlines, spacing and full length"
        );
        assert_eq!(
            detail.photo_refs,
            vec!["photo-2".to_owned(), "photo-1".to_owned()],
            "the photo references keep their stored order"
        );
    }

    /// WT-8, "The drop-alert flag appears in the detail when it applies", and D1.16: the flag is
    /// derived over the rabbit's full entry set and any stored value is ignored — the first entry's
    /// stored `true` cannot survive the derivation, and the second entry's drop is derived.
    #[test]
    fn the_detail_derives_the_flag_over_the_full_entry_set_and_ignores_the_stored_value() {
        let rabbit = stored_rabbit();
        let first = WeightEntry {
            has_weight_drop_alert: true,
            ..stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z")
        };
        let second = WeightEntry {
            has_weight_drop_alert: false,
            ..stored_entry(&rabbit, "entry-2", 1_800, "2026-10-21T00:00:00Z")
        };
        let collection = collection(
            &rabbit,
            vec![rabbit_row(&rabbit), entry_row(&first), entry_row(&second)],
        );

        let flagged = detail(&collection, &second).expect("the collection is readable");
        let unflagged = detail(&collection, &first).expect("the collection is readable");

        assert!(
            flagged.has_weight_drop_alert,
            "2000 g to 1800 g is a 10% drop beyond the 5% threshold"
        );
        assert!(
            !unflagged.has_weight_drop_alert,
            "the stored `true` is ignored: the first entry has no predecessor to drop from"
        );
    }

    /// WT-8, "An entry with no photos shows no pager": an entry with no photo references answers an
    /// empty list, so nothing can invent a pager.
    #[test]
    fn an_entry_with_no_photos_answers_an_empty_photo_reference_list() {
        let rabbit = stored_rabbit();
        let entry = stored_entry(&rabbit, "entry-1", 1_800, "2026-10-22T00:00:00Z");
        let collection = collection(&rabbit, vec![rabbit_row(&rabbit), entry_row(&entry)]);

        let detail = detail(&collection, &entry).expect("the collection is readable");

        assert!(
            detail.photo_refs.is_empty(),
            "no pager is invented for an entry without photos"
        );
    }

    /// WT-8, "An archived rabbit's entry detail is read-only": archived is a read-only presentation,
    /// not a refusal, and the flag is still derived from the archived rabbit's threshold.
    #[test]
    fn an_archived_rabbits_entry_detail_is_readable_and_still_derives_its_flag() {
        let rabbit = Rabbit {
            is_archived: true,
            ..stored_rabbit()
        };
        let first = stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z");
        let second = stored_entry(&rabbit, "entry-2", 1_800, "2026-10-21T00:00:00Z");
        let collection = collection(
            &rabbit,
            vec![rabbit_row(&rabbit), entry_row(&first), entry_row(&second)],
        );

        let detail =
            detail(&collection, &second).expect("an archived rabbit's entry is read, not refused");

        assert_eq!(detail.value_grams, Grams(1_800));
        assert!(
            detail.has_weight_drop_alert,
            "the derivation needs only the rabbit's threshold, which an archived rabbit still has"
        );
    }

    /// D1.16: a live row whose content is not on this device could be an entry of this rabbit (or its
    /// predecessor), so the detail refuses rather than answering a partial derivation.
    #[test]
    fn a_live_row_with_unknown_content_refuses_the_detail() {
        let rabbit = stored_rabbit();
        let entry = stored_entry(&rabbit, "entry-1", 1_800, "2026-10-22T00:00:00Z");
        let rows = vec![
            rabbit_row(&rabbit),
            entry_row(&entry),
            crate::ipc::types::CollectionItem {
                item_id: ItemId("entry-2".to_owned()),
                updated_at: at("2026-10-22T00:00:00Z"),
                deleted: false,
                payload: None,
            },
        ];
        let collection = collection(&rabbit, rows);

        let error = detail(&collection, &entry)
            .expect_err("an unreadable row leaves the entry set partial");

        assert_eq!(error.cause, crate::ipc::types::SaveCause::Unknown);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_eq!(error.request_id, None);
    }

    /// D1.17: a tombstone is excluded before the content check, so a removed entry neither refuses the
    /// detail nor takes part in the derivation — its content need not be on this device.
    #[test]
    fn a_tombstoned_row_is_excluded_from_the_details_derivation() {
        let rabbit = stored_rabbit();
        let first = stored_entry(&rabbit, "entry-1", 2_000, "2026-10-20T00:00:00Z");
        let second = stored_entry(&rabbit, "entry-2", 1_900, "2026-10-21T00:00:00Z");
        let rows = vec![
            rabbit_row(&rabbit),
            entry_row(&first),
            // If the removed 1000 g entry were part of the derivation, `entry-2` would show a 90%
            // drop and be flagged; the tombstone's content is deliberately absent too.
            crate::ipc::types::CollectionItem {
                item_id: ItemId("entry-removed".to_owned()),
                updated_at: at("2026-10-22T00:00:00Z"),
                deleted: true,
                payload: None,
            },
            entry_row(&second),
        ];
        let collection = collection(&rabbit, rows);

        let detail = detail(&collection, &second).expect("a tombstone is not unreadable content");

        assert!(
            !detail.has_weight_drop_alert,
            "the removed entry is not a predecessor: 2000 g to 1900 g is exactly at the threshold"
        );
    }

    /// The detail needs the rabbit's threshold, so an entry whose rabbit row the collection does not
    /// hold cannot be judged; it is refused rather than flagged against an invented value.
    #[test]
    fn an_entry_whose_rabbit_row_is_absent_is_refused() {
        let rabbit = stored_rabbit();
        let entry = stored_entry(&rabbit, "entry-1", 1_800, "2026-10-22T00:00:00Z");
        let collection = collection(&rabbit, vec![entry_row(&entry)]);

        let error = detail(&collection, &entry).expect_err("no rabbit row means no threshold");

        assert_eq!(error.cause, crate::ipc::types::SaveCause::Validation);
        assert_eq!(error.request_id, None);
    }
}
