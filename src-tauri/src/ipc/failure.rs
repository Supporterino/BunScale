//! The one place a [`CommandError`] is built: the cause → (message key, retryable) policy, and one
//! constructor per cause (task 2.11, NFR-8, D0.6, D0.14).
//!
//! [`CommandError`] and [`SaveCause`] themselves are frozen in [`super::types`]; what is decided
//! here is the policy that type leaves open — which copy a cause renders through, whether a retry
//! can clear it, and the operation identity the failure carries — so that no command assembles a
//! failure field by field and no cause can be reported through a shared generic message.
//!
//! # One key per cause, with two documented exceptions
//!
//! [`policy`] is a total function over the frozen, exhaustive cause list: every cause has its own
//! non-empty message key, no two causes share one, and a new variant cannot be added without making
//! the function non-exhaustive. That is what makes NFR-8's "no bare failure messages" a property of
//! the code rather than a review checklist: a failure cannot reach the frontend without naming a
//! specific cause, and the cause selects exactly one key — except the two keys documented below.
//!
//! The keys are owned here; the copy behind them is the i18n scaffold's (task 3.7), and the UI
//! renders the key rather than a message, because the backend must not author user-facing text — and
//! must never carry a path, a token or content across this boundary (NFR-3).
//!
//! | Cause | Message key | Retryable |
//! |---|---|---|
//! | `connectivity` | `error.connectivity` | yes |
//! | `no_account` | `error.no_account` | yes |
//! | `allowance_exceeded` | `error.allowance_exceeded` | yes |
//! | `rate_limited` | `error.rate_limited` | yes |
//! | `timeout` | `error.timeout` | yes |
//! | `validation` | `error.validation` | no |
//! | `unknown` | `error.unknown` | no |
//!
//! **Two keys belong to no cause, and both keep the frozen cause list intact.** One cause naming
//! more than one key is deliberate and narrowly bounded; neither key replaces a cause's own, and a
//! later edit must not "tidy" either into `error.unknown`:
//!
//! * `error.not_implemented` ([`NOT_IMPLEMENTED_MESSAGE_KEY`]) — the placeholder a not-yet-
//!   implemented command returns through [`CommandError::not_implemented`]. It is a state of the
//!   surface, not a cause: the frozen list admits no "not implemented" variant (D0.6).
//! * `error.household_content_unloaded` ([`HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY`]) — the
//!   unknown-content refusal D1.16 defines for a collection row whose content this device cannot
//!   describe, returned through [`CommandError::household_content_unloaded`]. It keeps the cause
//!   `unknown` and `retryable: false` — nothing is known about the failure and nothing can be
//!   promised about a retry — but it names *this* condition rather than borrowing `error.unknown`'s
//!   operation-neutral copy: rows were loaded and one content was not, which is a fact the next
//!   successful refresh repairs, not a generic failure.
//!
//! The three keys are asserted to differ, so a placeholder, a genuine unclassifiable failure and
//! the unknown-content refusal stay distinguishable on the surface.
//!
//! # The retryable policy
//!
//! `retryable` answers exactly one question: **can re-issuing this same command, with the same
//! [`RequestId`], succeed?** It is the only signal the frontend has for offering a retry at all
//! (3.4 holds the retry state, 4.5 renders the action), so it has to be honest in both directions: a
//! retry offered for a failure no retry can clear is the misleading, success-shaped surface NFR-8
//! forbids, and a retry withheld from a failure a retry *can* clear strands the person with a
//! condition they could have resolved.
//!
//! A retry can clear these, so they are retryable:
//!
//! * `connectivity` — what failed is the transport (DNS, TLS, connect, read), never the request
//!   itself; SYNC-2's scenario names connectivity among the causes the retry is offered for.
//! * `no_account` — the account is missing or invalid *at the moment of the attempt*, and a fresh
//!   token or a repaired account makes the very same request valid (D0.7 criterion 4 reads the
//!   `401` this way). SYNC-2 names it too.
//! * `allowance_exceeded` — the owner's exhausted allowance is a state that changes outside this
//!   request, and SYNC-2 requires the same retry to be offered for it as for the other three; the
//!   copy has to name the condition (D0.12), not the flag.
//! * `rate_limited` — the gateway's `429` carries `Retry-After`: the refusal is explicitly
//!   time-bounded, which is the clearest case of "a later attempt can succeed".
//! * `timeout` — no outcome arrived inside D0.13's bound, so the service may already have applied
//!   the operation. That is precisely the case D0.14's request ledger exists for: re-issuing with
//!   the same [`RequestId`] returns the stored outcome instead of applying the change twice, so the
//!   retry is both safe and effective, and R6's duplicated household stays closed.
//!
//! A retry cannot clear these, so they are not retryable:
//!
//! * `validation` — the refusal is a property of the input: the same command with the same values is
//!   refused again, so the request has to change for the outcome to change. Offering a retry would
//!   promise something the surface cannot deliver.
//! * `unknown` — nothing is known about the failure, so nothing can be promised about a second
//!   attempt either. The cause is still named specifically through its own key, which is what NFR-8
//!   requires; the surface simply does not offer a retry on top of it.
//!
//! The placeholder failure is not retryable either, for the same reason: a retry cannot make a
//! missing implementation appear.
//!
//! **`allowance_exceeded` is `true`, and [`CommandError`]'s own contract comment states the same
//! rule.** The flag is the only signal the surface carries for offering a retry, and SYNC-2 requires
//! the retry to be offered for exactly the four service-reported conditions — connectivity, no
//! account, allowance and rate limiting — so a `false` there would withhold a retry the requirement
//! promises. What the flag encodes is therefore "a later attempt **can** succeed" — once the named
//! condition clears — not "it succeeds without anything changing first"; the key's copy is what has
//! to explain the condition (D0.12).
//!
//! # The operation's identity
//!
//! Every constructor takes the command's `RequestId` and passes it through unchanged (D0.14): it is
//! the identity the shared service deduplicates on, so it is exactly what makes the retry the
//! surface offers safe, and it must never be regenerated, invented or replaced here. `None` is for
//! the failures that have no operation identity to deduplicate — a read's or the reachability
//! probe's — whose retry is a fresh call rather than a re-issued one; a state-changing command
//! always passes `Some`.
//!
//! [`CommandError`]: super::types::CommandError
//! [`SaveCause`]: super::types::SaveCause
//! [`RequestId`]: super::types::RequestId

use super::types::{CommandError, RequestId, SaveCause};

/// The message key of the **placeholder** failure a command with no implementation yet returns.
///
/// It is deliberately not one of the causes' keys (see the module documentation): the cause list is
/// frozen and admits no "not implemented" variant (D0.6), so the placeholder is a state of the
/// surface rather than a cause, and `error.unknown` stays what a genuine unclassifiable failure
/// names.
pub const NOT_IMPLEMENTED_MESSAGE_KEY: &str = "error.not_implemented";

/// The message key of the **unknown-content** refusal a read or a write answers when a row of this
/// device's collection has no content it can describe (D1.16; task 2.5).
///
/// It keeps the cause `unknown` and the retryable `false` that cause carries — the state is answered,
/// never guessed — but it names the condition itself rather than reusing `error.unknown`'s
/// operation-neutral copy (see the module documentation for why one cause may own two keys).
/// `src/features/i18n` owns the copy behind it (task 3.8).
pub const HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY: &str = "error.household_content_unloaded";

/// One cause's own copy key and retry policy: the whole content of the mapping in [`policy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CausePolicy {
    /// The i18n key of this cause's own copy. Non-empty, and distinct per cause.
    pub message_key: &'static str,
    /// Whether re-issuing the failed operation with the same `RequestId` can succeed (see the module
    /// documentation for the policy this encodes).
    pub retryable: bool,
}

/// The cause → (message key, retryable) mapping, as a total function over the frozen cause list.
///
/// This is the single source of both fields: the constructors below route through it, so a call
/// site chooses a cause and cannot choose a key. See the module documentation for the table and for
/// the retryable reasoning.
pub const fn policy(cause: SaveCause) -> CausePolicy {
    match cause {
        SaveCause::Connectivity => CausePolicy {
            message_key: "error.connectivity",
            retryable: true,
        },
        SaveCause::NoAccount => CausePolicy {
            message_key: "error.no_account",
            retryable: true,
        },
        SaveCause::AllowanceExceeded => CausePolicy {
            message_key: "error.allowance_exceeded",
            retryable: true,
        },
        SaveCause::RateLimited => CausePolicy {
            message_key: "error.rate_limited",
            retryable: true,
        },
        SaveCause::Timeout => CausePolicy {
            message_key: "error.timeout",
            retryable: true,
        },
        SaveCause::Validation => CausePolicy {
            message_key: "error.validation",
            retryable: false,
        },
        SaveCause::Unknown => CausePolicy {
            message_key: "error.unknown",
            retryable: false,
        },
    }
}

/// The constructors: one per `SaveCause`, each applying that cause's own copy and retry flag.
///
/// A caller names the condition it observed and passes the operation's `RequestId` when it has one;
/// everything else is the mapping's. Nothing outside this module builds a [`CommandError`] by hand,
/// which is what keeps the cause list, the keys and the retry flags from drifting apart.
impl CommandError {
    /// No HTTP response at all: DNS, TLS, connect or read failed (D0.7 criterion 4).
    ///
    /// Retryable: the transport is what failed, and it is not a property of the request.
    pub fn connectivity(request_id: Option<RequestId>) -> Self {
        Self::for_cause(SaveCause::Connectivity, request_id)
    }

    /// The service answered `401`: the account is missing or invalid (D0.7 criterion 4).
    ///
    /// Retryable: a fresh token or a repaired account makes the same request valid.
    pub fn no_account(request_id: Option<RequestId>) -> Self {
        Self::for_cause(SaveCause::NoAccount, request_id)
    }

    /// The household owner's storage allowance is exhausted, as the service reported it (D0.12).
    ///
    /// This is its own condition, never a generic write error mapped after the fact. Retryable: the
    /// allowance is a state that changes outside this request, and SYNC-2 offers the retry for it.
    pub fn allowance_exceeded(request_id: Option<RequestId>) -> Self {
        Self::for_cause(SaveCause::AllowanceExceeded, request_id)
    }

    /// The gateway answered `429` with `Retry-After` (D0.7 criterion 4).
    ///
    /// Retryable: the refusal is explicitly time-bounded.
    pub fn rate_limited(request_id: Option<RequestId>) -> Self {
        Self::for_cause(SaveCause::RateLimited, request_id)
    }

    /// No outcome arrived inside the operation's bound (D0.13): `SAVE_ATTEMPT_TIMEOUT` for a save
    /// attempt, `REACHABILITY_PROBE_TIMEOUT` for a probe.
    ///
    /// Retryable: the service may already have applied the operation, and re-issuing with the same
    /// `RequestId` returns the stored outcome rather than applying it twice (D0.14).
    pub fn timeout(request_id: Option<RequestId>) -> Self {
        Self::for_cause(SaveCause::Timeout, request_id)
    }

    /// The input was refused as invalid: a blank household name in P0 (HS-1), RAB-2's and WT-4's
    /// refusals in P1.
    ///
    /// Not retryable: the same input is refused again, so the request itself has to change.
    pub fn validation(request_id: Option<RequestId>) -> Self {
        Self::for_cause(SaveCause::Validation, request_id)
    }

    /// The failure could not be classified — an honest last resort, never a fallback that hides a
    /// cause the product already knows (D0.6).
    ///
    /// Not retryable: nothing is known about the failure, so nothing can be promised about a second
    /// attempt. The cause is still named through its own key, so this is a specific report and not a
    /// bare "something went wrong".
    ///
    /// A command that simply has no implementation yet does not come through here: it returns
    /// [`CommandError::not_implemented`], the same cause and flag with the placeholder key, and the
    /// unknown-content refusal has its own key through [`CommandError::household_content_unloaded`]
    /// while keeping this cause and flag (D1.16).
    pub fn unknown(request_id: Option<RequestId>) -> Self {
        Self::for_cause(SaveCause::Unknown, request_id)
    }

    /// The **placeholder** failure a command with no implementation yet returns: the cause
    /// `unknown`, `retryable: false`, and [`NOT_IMPLEMENTED_MESSAGE_KEY`] instead of the cause's own
    /// key (design "Every body is a typed stub", section 2 replaces the callers one task at a time).
    ///
    /// The cause and the flag are the classified `unknown` failure's — nothing is known and nothing
    /// can be promised — while the key says *why* nothing could be answered, so a placeholder is
    /// never mistaken for a production failure path. It is the only constructor here that does not
    /// use its cause's key, and the module documentation records why that is a placeholder and not a
    /// second key for `unknown`.
    pub fn not_implemented(request_id: Option<RequestId>) -> Self {
        let classified_unknown = policy(SaveCause::Unknown);

        Self {
            cause: SaveCause::Unknown,
            retryable: classified_unknown.retryable,
            message_key: NOT_IMPLEMENTED_MESSAGE_KEY.to_owned(),
            request_id,
        }
    }

    /// A collection row this command needs has no content this version can describe (D1.16).
    ///
    /// The cause is `unknown` and the retry flag is that cause's: the state is answered, never
    /// guessed, and re-issuing the same command cannot repair it because the refusal is a property of
    /// the local copy, not of the request. The key is **not** `error.unknown`: P0's copy there is
    /// deliberately operation-neutral ("That did not work. Nothing was changed."), and this refusal
    /// has its own explanation — the device holds the item but could not read its details, which the
    /// next refresh fills (D1.16, task 3.8). A read passes `None`, a state-changing command passes its
    /// [`RequestId`] like every other failure of that command.
    pub fn household_content_unloaded(request_id: Option<RequestId>) -> Self {
        let classified_unknown = policy(SaveCause::Unknown);

        Self {
            cause: SaveCause::Unknown,
            retryable: classified_unknown.retryable,
            message_key: HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY.to_owned(),
            request_id,
        }
    }

    /// Build the failure for a cause computed at run time, applying the same one-to-one mapping.
    ///
    /// The save path (2.4) and the reachability probe (2.13) classify an outcome — a status code, a
    /// transport error, a passed deadline — before they know which constructor applies, and this is
    /// how such a classification reaches the surface without re-deriving a key or a flag at the call
    /// site. Where the cause is a literal, the named constructors above are still the better form:
    /// they make the call site say which condition it observed.
    pub fn for_cause(cause: SaveCause, request_id: Option<RequestId>) -> Self {
        let cause_policy = policy(cause);

        Self {
            cause,
            retryable: cause_policy.retryable,
            message_key: cause_policy.message_key.to_owned(),
            request_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Every variant of the frozen cause list (D0.6).
    ///
    /// The mapping is total over it at compile time — a new variant makes [`policy`] non-exhaustive
    /// — so this list cannot silently fall behind the type it stands for.
    const ALL_CAUSES: [SaveCause; 7] = [
        SaveCause::Connectivity,
        SaveCause::NoAccount,
        SaveCause::AllowanceExceeded,
        SaveCause::RateLimited,
        SaveCause::Timeout,
        SaveCause::Validation,
        SaveCause::Unknown,
    ];

    #[test]
    fn every_cause_reports_through_its_own_message_key() {
        let mut keys = HashSet::new();

        for cause in ALL_CAUSES {
            let key = policy(cause).message_key;

            assert!(!key.is_empty(), "{cause:?} has no message key");
            assert!(key.starts_with("error."), "{cause:?} has a non-error key");
            assert!(keys.insert(key), "{cause:?} shares the key {key}");
        }

        assert_eq!(keys.len(), ALL_CAUSES.len());
    }

    #[test]
    fn every_named_constructor_applies_its_own_causes_policy() {
        let request_id = RequestId("operation-1".to_owned());

        for cause in ALL_CAUSES {
            let error = named_constructor(cause, request_id.clone());

            assert_eq!(error.cause, cause);
            assert_eq!(
                error,
                CommandError::for_cause(cause, Some(request_id.clone()))
            );
            assert_eq!(error.request_id.as_ref(), Some(&request_id));
        }
    }

    /// The named constructor of one cause, so the test above exercises the real entry points.
    ///
    /// A constructor wired to the wrong cause is caught by the `for_cause` comparison, which is the
    /// mapping's own answer for that cause.
    fn named_constructor(cause: SaveCause, request_id: RequestId) -> CommandError {
        match cause {
            SaveCause::Connectivity => CommandError::connectivity(Some(request_id)),
            SaveCause::NoAccount => CommandError::no_account(Some(request_id)),
            SaveCause::AllowanceExceeded => CommandError::allowance_exceeded(Some(request_id)),
            SaveCause::RateLimited => CommandError::rate_limited(Some(request_id)),
            SaveCause::Timeout => CommandError::timeout(Some(request_id)),
            SaveCause::Validation => CommandError::validation(Some(request_id)),
            SaveCause::Unknown => CommandError::unknown(Some(request_id)),
        }
    }

    #[test]
    fn the_retryable_policy_is_the_documented_one() {
        // The four conditions SYNC-2 offers a retry for, plus the deadline D0.14's ledger exists
        // for.
        for cause in [
            SaveCause::Connectivity,
            SaveCause::NoAccount,
            SaveCause::AllowanceExceeded,
            SaveCause::RateLimited,
            SaveCause::Timeout,
        ] {
            assert!(
                policy(cause).retryable,
                "{cause:?} can be cleared by a retry"
            );
        }

        for cause in [SaveCause::Validation, SaveCause::Unknown] {
            assert!(!policy(cause).retryable, "{cause:?} resists a retry");
        }
    }

    #[test]
    fn a_stub_failure_names_the_placeholder_key_and_a_classified_unknown_does_not() {
        let request_id = RequestId("operation-1".to_owned());

        let stub = CommandError::not_implemented(Some(request_id.clone()));
        let classified = CommandError::unknown(Some(request_id.clone()));

        assert_eq!(stub.cause, SaveCause::Unknown);
        assert!(!stub.retryable);
        assert_eq!(stub.message_key, NOT_IMPLEMENTED_MESSAGE_KEY);
        assert_eq!(stub.request_id.as_ref(), Some(&request_id));

        // The placeholder is not a second key for `unknown`: a failure the code could not classify
        // keeps the cause's own key, and the two paths stay distinguishable.
        assert_eq!(classified.message_key, "error.unknown");
        assert_ne!(stub.message_key, classified.message_key);
    }

    #[test]
    fn an_unknown_content_refusal_keeps_its_cause_and_names_its_own_key() {
        let request_id = RequestId("operation-1".to_owned());

        let refusal = CommandError::household_content_unloaded(Some(request_id.clone()));

        // The state is answered as `unknown`, with the retry the cause withholds (D1.16).
        assert_eq!(refusal.cause, SaveCause::Unknown);
        assert!(!refusal.retryable);
        assert_eq!(refusal.request_id.as_ref(), Some(&request_id));

        // The key is the condition's own: not `error.unknown`'s operation-neutral copy, and not the
        // placeholder a command with no implementation names.
        assert_eq!(refusal.message_key, HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY);
        assert_ne!(refusal.message_key, "error.unknown");
        assert_ne!(refusal.message_key, NOT_IMPLEMENTED_MESSAGE_KEY);
        assert_ne!(refusal, CommandError::unknown(Some(request_id.clone())));
        assert_ne!(refusal, CommandError::not_implemented(Some(request_id)));
    }

    #[test]
    fn a_reads_or_probes_failure_carries_no_operation_identity() {
        assert_eq!(CommandError::unknown(None).request_id, None);
        assert_eq!(CommandError::not_implemented(None).request_id, None);
        assert_eq!(
            CommandError::household_content_unloaded(None).request_id,
            None
        );
        assert_eq!(CommandError::timeout(None).request_id, None);
    }
}
