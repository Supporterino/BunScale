//! The rabbit domain: one rabbit's recorded field set, and the one write path a rabbit is created
//! through (RAB-1, RAB-2, RAB-3; D1.1, D1.14, I8).
//!
//! The contract in [`crate::ipc::types`] fixes the *shape* that crosses the boundary — [`Rabbit`],
//! [`RabbitProfile`], [`Sex`], [`NeuterStatus`]. This module fixes what that shape may not violate:
//!
//! * **A rabbit's name is non-blank.** [`validate_name`] refuses `""` and a name that is entirely
//!   whitespace as `validation` before anything is sent, and it is the one rule both this module's
//!   create path and the update path that follows it call. A name with any non-whitespace character
//!   is kept exactly as supplied — the rule decides whether a name exists, it does not edit one
//!   (RAB-2).
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
//! * **The write names its writer and its version marker.** `save_item` takes both, because the
//!   service's last-write-wins tuple is `(updated_at, writer_id, request_id)` (ADR 0002 criteria 3
//!   and 6). The writer is the member the person is in that household, resolved by the caller from
//!   its book; the marker is minted **once per operation** and re-sent unchanged by a retry, so a
//!   re-issued write is byte-identical and the service's request ledger recognises it as the same
//!   operation rather than refusing it as a changed payload.
//!
//! **Nothing here touches Tauri, and nothing here writes storage.** [`save_rabbit`] is the one write
//! path, and it goes through P0's reachability-gated [`SaveGate`]: a rabbit exists once the shared
//! service has accepted the write, so no local-only rabbit and no second write path can be produced
//! here (D1.1, D0.1). What this device holds afterwards is its **copy of the shared collection**,
//! filled by the read path (SYNC-3) — never by this module writing it.

use serde_json::{Value, json};

use crate::ipc::types::{
    CommandError, HouseholdId, Instant, MemberId, NeuterStatus, Rabbit, RabbitId, RabbitProfile,
    RequestId, Sex,
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
fn rabbit_id(request_id: &RequestId) -> RabbitId {
    RabbitId(request_id.0.clone())
}

/// Write one recorded rabbit as an item of its household's shared collection (RAB-1, D1.1).
///
/// The write goes through P0's reachability gate: the attempt is refused before anything leaves the
/// device when the service cannot be reached or the account is not usable, and a failure is never
/// presented as a created rabbit (SYNC-2). A receipt is proof the service accepted the write and
/// nothing more — the operation's own answer schema carries no field this path needs, because the
/// rabbit is fully determined by the request (its id is the operation's identity, and every other
/// field is the one the person supplied).
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
/// deduplication (D0.14). No failure is ever reported as a created rabbit.
pub async fn save_rabbit(
    service: &dyn SharedService,
    record: &RabbitRecord,
    writer: &MemberId,
    marker: &Instant,
    request_id: RequestId,
) -> Result<(), CommandError> {
    let body = save_body(record, writer, marker, &request_id);
    let request = SaveRequest::new(request_id, SAVE_ITEM_RPC, body)?;

    let _accepted = SaveGate::new(service).save(request).await?;

    Ok(())
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
/// environment, so a retry that supplies the same record, writer, marker and identity submits the
/// same bytes — which is what the service's request ledger requires of a re-issued operation (D0.7
/// criterion 6, D0.14).
fn save_body(
    record: &RabbitRecord,
    writer: &MemberId,
    marker: &Instant,
    request_id: &RequestId,
) -> Value {
    json!({
        "request_id": request_id.0,
        "household_id": record.household_id.0,
        "item_id": record.id.0,
        "updated_at": marker.0,
        "writer_id": writer.0,
        "payload": {
            "kind": "rabbit",
            "rabbit": record.to_wire(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::types::SaveCause;
    use crate::sync::ServiceReply;
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::double::{RecordedCall, ScriptedService};

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
}
