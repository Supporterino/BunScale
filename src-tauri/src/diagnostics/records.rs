//! The redacted diagnostic record: identifiers, counts, timestamps and outcomes only (NFR-3).
//!
//! This module is where NFR-3's exclusion list is enforced by the *shape* of the data rather than by
//! the discipline of every future call site. [`DiagnosticRecord`] declares three fields and no more;
//! [`DiagnosticSubject`] admits only opaque identifiers taken from the frozen contract; and
//! [`DiagnosticOutcome`] is a closed enum carrying a frozen cause, a frozen service state or a count.
//! Every leaf below those three is therefore one of exactly four things — an opaque identifier, a
//! count, an absolute instant, or a closed enum built from the frozen `SaveCause`/`ServiceState`
//! types — and there is no free-form `String`, no byte buffer and no numeric weight anywhere in the
//! shape. A rabbit name, note text, photo content or weight value therefore has **no content-shaped
//! field to occupy and no content-shaped parameter to arrive through**: every constructor takes an
//! identifier, a count, a cause, a state or an instant, and none of those is where content belongs.
//!
//! **What that guarantee is, and what it is not.** It is a guarantee about the *shape*: a record with
//! a spare `String` or a `serde_json::Value` field would be exactly as redacted as the code that fills
//! it, and nothing would catch the day a call site starts filling it with a note — here that field
//! does not exist, so it is not a field a later call site can start using. It is **not** a guarantee
//! that the string inside an identifier carries nothing: [`RequestId`](crate::ipc::types::RequestId)
//! and [`HouseholdId`](crate::ipc::types::HouseholdId) are transparent `pub String` newtypes frozen by
//! the contract, so `DiagnosticRecord::save_failed(at, RequestId(rabbit_name.to_owned()), cause)`
//! compiles. An identifier stays opaque by the contract's own discipline — the frontend generates a
//! `RequestId` per operation (D0.14, task 3.1), and a `HouseholdId` is an opaque id (D0.7
//! criterion 1) — and never by anything these types enforce. What *is* fixed is that there is no
//! other place a payload could sit: the tests below guard that shape by regression rather than
//! demonstrate the redaction of a value that has no path in.
//!
//! **Records are written only through named constructors.** Each constructor names the operation it
//! describes — a save that succeeded, a save that failed, a reachability probe, a collection load —
//! so a call site states the outcome it observed instead of assembling fields, and the fields stay
//! private to this module. The identifier a record carries is the operation's own: the
//! [`RequestId`](crate::ipc::types::RequestId) the shared service deduplicates on (D0.14) or the
//! [`HouseholdId`](crate::ipc::types::HouseholdId) the operation addressed (D0.7 criterion 1).
//!
//! **Why `Serialize` is derived although nothing serialises a record.** The derived representation is
//! what lets a test assert each constructor's exact key set: a content-bearing field that is
//! *populated* adds a key, and the regression guard fails rather than the content leaking quietly. The
//! guard's one edge — an optional field absent from every record the tests construct, which serialises
//! to no key at all, exactly as `subject` does for a probe — is stated in that test rather than left
//! implied, and the field-type argument above is what covers it. A record is not registered with the
//! IPC builder — [`DiagnosticsSummary`](crate::ipc::types::DiagnosticsSummary) stays the only
//! diagnostics value the frontend sees — so this derives no wire surface.

use serde::Serialize;

use crate::ipc::types::{HouseholdId, Instant, RequestId, SaveCause, ServiceState};

/// The opaque identifier a record may name: never content (NFR-3).
///
/// Each variant holds one of the frozen contract's identifier newtypes, so the only strings a record
/// can carry are values the contract already treats as opaque — a household ([`HouseholdId`]) or an
/// operation ([`RequestId`]). There is deliberately no name-shaped variant: a rabbit's name, a
/// member's display name and a note's text are content, and a variant that could hold one would undo
/// the redaction this module exists to guarantee. A member's *id* is an identifier (D0.3); a
/// display name is not, which is why none is representable here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum DiagnosticSubject {
    /// The household an operation addressed (D0.7 criterion 1).
    Household(HouseholdId),
    /// The client-generated identity of one operation (D0.14).
    Request(RequestId),
}

/// What one record reports: a closed set of outcomes, never a message (NFR-3, NFR-8).
///
/// The list is exhaustive at compile time and every variant carries only frozen values — a
/// [`SaveCause`](crate::ipc::types::SaveCause), a [`ServiceState`](crate::ipc::types::ServiceState)
/// or an item count — so no outcome can smuggle a detail string, a name or a weight into a record.
/// "The cause" rather than "the message" is the point: the cause is a token from the frozen taxonomy
/// and the message is the frontend's copy (tasks 2.11 and 3.4), so a record can name a failure
/// without ever naming its content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum DiagnosticOutcome {
    /// A save attempt completed and the shared service accepted it (SYNC-2).
    SaveSucceeded,
    /// A save attempt failed with its specific cause (NFR-8).
    SaveFailed {
        /// The cause the single failure surface reported. A frozen enum, never a message or a path.
        cause: SaveCause,
    },
    /// A reachability probe produced a fresh outcome (NFR-1, D0.13).
    ReachabilityProbed {
        /// The service state the probe observed. A frozen enum, never a detail string.
        state: ServiceState,
    },
    /// A household collection was loaded, and how many items the load resolved (SYNC-1).
    CollectionLoaded {
        /// How many items the load resolved. A count is a permitted diagnostic value; an item's
        /// content is not, and no content of those items is anywhere near this field.
        item_count: u32,
    },
}

/// One redacted diagnostic record (NFR-3, task 2.12).
///
/// The three fields are the whole shape, and none of them can hold content: the instant is an
/// absolute one (D0.5), the subject is one of the frozen opaque identifiers and the outcome is the
/// closed enum above. The fields are private and every instance comes from one of the named
/// constructors, so a record cannot be assembled with an extra field, and a caller that wanted to
/// record a rabbit name would find neither field nor parameter to put it in (see the module
/// documentation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticRecord {
    /// When the record was made, as an absolute instant (D0.5). It is supplied by the recorder, which
    /// is the only place that knows the moment; this module reads no clock.
    at: Instant,
    /// The opaque identifier the operation had, when it had one. A reachability probe has none, so
    /// this is absent rather than defaulted.
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<DiagnosticSubject>,
    /// What happened — an outcome token, never a message or a detail.
    outcome: DiagnosticOutcome,
}

impl DiagnosticRecord {
    /// A save attempt the shared service accepted (SYNC-2).
    ///
    /// The operation's identity is the [`RequestId`] the command carried (D0.14); it is recorded as
    /// the operation's own value and never regenerated here.
    pub fn save_succeeded(at: Instant, request_id: RequestId) -> Self {
        Self {
            at,
            subject: Some(DiagnosticSubject::Request(request_id)),
            outcome: DiagnosticOutcome::SaveSucceeded,
        }
    }

    /// A save attempt that failed with `cause` (NFR-8).
    ///
    /// This is the constructor the save path uses, and its parameters are the whole of what a failed
    /// save may say: the moment, the operation's identity and the cause. The value the save was
    /// attempting to write is deliberately not a parameter — there is nowhere to put it (NFR-3) — so a
    /// save that fails while holding a rabbit name, a note, a photo or a weight records the cause and
    /// the identifier and nothing else.
    pub fn save_failed(at: Instant, request_id: RequestId, cause: SaveCause) -> Self {
        Self {
            at,
            subject: Some(DiagnosticSubject::Request(request_id)),
            outcome: DiagnosticOutcome::SaveFailed { cause },
        }
    }

    /// A reachability probe's fresh outcome (NFR-1).
    ///
    /// A probe addresses no household and carries no operation identity, so the record names only
    /// when it ran and what it observed.
    pub fn reachability_probed(at: Instant, state: ServiceState) -> Self {
        Self {
            at,
            subject: None,
            outcome: DiagnosticOutcome::ReachabilityProbed { state },
        }
    }

    /// A household's collection was loaded, with the number of items it resolved (SYNC-1).
    ///
    /// The household is an identifier; the items themselves are not here in any form — only how many
    /// there were, which is a count and not content.
    pub fn collection_loaded(at: Instant, household: HouseholdId, item_count: u32) -> Self {
        Self {
            at,
            subject: Some(DiagnosticSubject::Household(household)),
            outcome: DiagnosticOutcome::CollectionLoaded { item_count },
        }
    }

    /// When the record was made (D0.5).
    pub fn at(&self) -> &Instant {
        &self.at
    }

    /// The opaque identifier the record names, when the operation had one.
    pub fn subject(&self) -> Option<&DiagnosticSubject> {
        self.subject.as_ref()
    }

    /// What happened.
    pub fn outcome(&self) -> &DiagnosticOutcome {
        &self.outcome
    }
}

/// The keys a serialised record may declare, sorted: the declared vocabulary the serialization guards
/// in this module and in [`sink`](super::sink) hold each constructor's record to.
///
/// It is test-only — nothing in production serialises a record — and it exists so an expected key set
/// is checked against one declared list instead of being restated ad hoc.
#[cfg(test)]
pub(crate) const DECLARED_KEYS: [&str; 8] = [
    "at",
    "cause",
    "id",
    "item_count",
    "kind",
    "outcome",
    "state",
    "subject",
];

/// The keys a save-failure record declares, sorted.
///
/// It is the one shape with two guards — the per-constructor one here and the recorded-value one in
/// [`sink`](super::sink) — so both read this list and cannot drift apart.
#[cfg(test)]
pub(crate) const SAVE_FAILED_KEYS: [&str; 6] = ["at", "cause", "id", "kind", "outcome", "subject"];

/// Every object key in `value`, at any depth: the serialized form's whole key set, not just its top
/// level, so a nested field cannot slip in unobserved.
#[cfg(test)]
pub(crate) fn serialised_keys(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Object(map) => map
            .iter()
            .flat_map(|(key, nested)| {
                let mut keys = vec![key.clone()];

                keys.extend(serialised_keys(nested));
                keys
            })
            .collect(),
        serde_json::Value::Array(items) => items.iter().flat_map(serialised_keys).collect(),
        _ => Vec::new(),
    }
}

/// The keys `record` serialises to, sorted and deduplicated.
#[cfg(test)]
pub(crate) fn record_keys(record: &DiagnosticRecord) -> Vec<String> {
    let value = serde_json::to_value(record).expect("a record serialises");
    let mut keys = serialised_keys(&value);

    keys.sort();
    keys.dedup();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The instants are opaque to this module — ordering two of them is task 2.8's — so a test only
    /// ever compares a record's instant with the value it supplied.
    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn request(value: &str) -> RequestId {
        RequestId(value.to_owned())
    }

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    /// One record of the operation identity the shared service deduplicates on (D0.14).
    fn succeeded(moment: &Instant) -> DiagnosticRecord {
        DiagnosticRecord::save_succeeded(moment.clone(), request("request-1"))
    }

    fn failed(moment: &Instant, cause: SaveCause) -> DiagnosticRecord {
        DiagnosticRecord::save_failed(moment.clone(), request("request-1"), cause)
    }

    fn probed(moment: &Instant, state: ServiceState) -> DiagnosticRecord {
        DiagnosticRecord::reachability_probed(moment.clone(), state)
    }

    fn loaded(moment: &Instant, household_id: &str, item_count: u32) -> DiagnosticRecord {
        DiagnosticRecord::collection_loaded(moment.clone(), household(household_id), item_count)
    }

    fn request_subject(value: &str) -> DiagnosticSubject {
        DiagnosticSubject::Request(request(value))
    }

    fn household_subject(value: &str) -> DiagnosticSubject {
        DiagnosticSubject::Household(household(value))
    }

    #[test]
    fn every_constructor_records_the_outcome_and_identifier_it_names() {
        let first = at("2026-10-19T10:00:00Z");
        let second = at("2026-10-19T10:01:00Z");
        let third = at("2026-10-19T10:02:00Z");
        let fourth = at("2026-10-19T10:03:00Z");

        let saved = succeeded(&first);
        assert_eq!(saved.at(), &first);
        assert_eq!(saved.subject(), Some(&request_subject("request-1")));
        assert_eq!(saved.outcome(), &DiagnosticOutcome::SaveSucceeded);

        let expected = request_subject("request-1");
        let save_failure = failed(&second, SaveCause::AllowanceExceeded);
        assert_eq!(save_failure.at(), &second);
        assert_eq!(save_failure.subject(), Some(&expected));
        assert_eq!(
            save_failure.outcome(),
            &DiagnosticOutcome::SaveFailed {
                cause: SaveCause::AllowanceExceeded,
            }
        );

        let probe = probed(&third, ServiceState::Unreachable);
        assert_eq!(probe.at(), &third);
        assert_eq!(probe.subject(), None);
        assert_eq!(
            probe.outcome(),
            &DiagnosticOutcome::ReachabilityProbed {
                state: ServiceState::Unreachable,
            }
        );

        let load = loaded(&fourth, "household-1", 7);
        assert_eq!(load.at(), &fourth);
        assert_eq!(load.subject(), Some(&household_subject("household-1")));
        assert_eq!(
            load.outcome(),
            &DiagnosticOutcome::CollectionLoaded { item_count: 7 }
        );
    }

    #[test]
    fn every_constructor_serialises_exactly_its_declared_keys() {
        // Each constructor's key set is asserted **exactly**, not as a union, so a content-bearing
        // field that is populated fails this test instead of slipping through as one more key.
        //
        // The residual limit, recorded rather than implied: an *optional* field that is absent from
        // every record constructed here serialises to no key at all — exactly how `subject` is skipped
        // for a probe — so a new `Option`-shaped content field that is `None` in these four records
        // would add no key and keep this guard green. That is why the primary guarantee is the field
        // *types* (a content-bearing type has no place in `DiagnosticRecord`) and this key check is the
        // regression guard for a field that is actually populated.
        let moment = at("2026-10-19T10:00:00Z");

        let saved = succeeded(&moment);
        let saved_keys = ["at", "id", "kind", "outcome", "subject"];
        let failure = failed(&moment, SaveCause::AllowanceExceeded);
        let probe = probed(&moment, ServiceState::Reachable);
        let probe_keys = ["at", "outcome", "state"];
        let load = loaded(&moment, "household-1", 3);
        let load_keys = ["at", "id", "item_count", "kind", "outcome", "subject"];

        assert_eq!(record_keys(&saved), saved_keys);
        assert_eq!(record_keys(&failure), SAVE_FAILED_KEYS);
        assert_eq!(record_keys(&probe), probe_keys);
        assert_eq!(record_keys(&load), load_keys);

        // And nothing outside the declared vocabulary may appear on any of them.
        for record in [&saved, &failure, &probe, &load] {
            for key in record_keys(record) {
                assert!(
                    DECLARED_KEYS.contains(&key.as_str()),
                    "the undeclared key {key} reached a record"
                );
            }
        }
    }
}
