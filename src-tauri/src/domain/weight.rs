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
//! — and [`record`] applies it before anything is recorded. It is a separate function so task 2.11's
//! edit path can apply the identical rule to a correction rather than growing a second copy. The bound
//! is a confirmation threshold, not a rejection (D1.9): a confirmed outlier is recorded at its true
//! value, never clamped.
//!
//! **What this module does not decide.** Duplicate-entry detection, editing and deletion, the reads
//! and the drop-alert derivation belong to tasks 2.11–2.15.

use serde_json::{Value, json};

use super::rabbit::SAVE_ITEM_RPC;
use crate::ipc::types::{
    CommandError, Grams, HouseholdId, Instant, ItemId, ItemPayload, MemberId,
    PLAUSIBILITY_BOUND_GRAMS, RabbitId, RequestId, WeightEntry, WeightEntryDraft, WeightEntryId,
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
/// entry keeps its date (WT-1) — or `creation_instant` when the draft omitted it. The note is optional
/// and recorded whole. The two facts creation fixes are the entry's identity ([`entry_id`], the
/// operation's identity) and `has_weight_drop_alert: false`, because the alert is derived on read and
/// is never content (D1.16, D1.3). The entry has no photos yet: choosing one is PHOTO-2 in P2 (D1.11)
/// and [`WeightEntryDraft`] has no photo field.
///
/// `creation_instant` is the caller's, not this module's: reading the clock is the write path's job,
/// and minting it once per operation is what keeps a retried create byte-identical (D0.14). It is
/// called **only** when the draft omitted the timestamp, so a supplied instant never touches the
/// operation's memo.
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

    let timestamp = match timestamp {
        // The moment is the point: an explicit past instant is a valid record of that moment and is
        // never moved. The storage boundary normalises the offset to UTC, as it does for every
        // absolute instant (DM-6), so two spellings of one moment are equal after a round trip.
        Some(supplied) => AbsoluteInstant::parse(&supplied)
            .map_err(|_| CommandError::validation(Some(request_id.clone())))?
            .to_wire(),
        None => creation_instant(),
    };

    Ok(WeightEntry {
        id: entry_id(request_id),
        rabbit_id,
        value_grams,
        timestamp,
        note,
        photo_refs: Vec::new(),
        has_weight_drop_alert: false,
    })
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
}
