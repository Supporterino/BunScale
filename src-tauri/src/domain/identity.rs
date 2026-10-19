//! Identity resolution: the name Settings & Profile shows, and what "unresolved" means (SET-2).
//!
//! SET-2 fixes a chain with three tiers. The frozen [`Identity`] shape carries its outcome:
//!
//! 1. **The account's display name**, when the account tier resolves. This is the only tier that
//!    produces `resolved: true`; the name is the one the account service reports for the account
//!    (D0.7 follow-up 2 reads it from the sign-in flow's user metadata).
//! 2. **The locally known member name**, when the account tier did not produce a name. It is the
//!    display name recorded on a member of this device's household book whose account is the
//!    current person's — see [`local_member_name`]. A name recorded for a different account is
//!    never borrowed: showing someone else's name as the current person's would be a worse outcome
//!    than showing none.
//! 3. **The unresolved marker**: `display_name: None` and `resolved: false`; with no account at all,
//!    `account_user_id` is `None` as well. It is a normal result, not a failure.
//!    [`resolve_identity`] returns an [`Identity`] rather than a `Result`, so "no identity could be
//!    resolved" cannot be mistaken for an error by a caller, and the Settings screen can render the
//!    rest of its content regardless (SET-2's scenario).
//!
//! **The third tier is the frontend's, not this module's.** A localized "You"-style placeholder is
//! copy (task 3.7); this module reports "no name known" and never invents a user-facing string.
//!
//! # `resolved` means the account tier, not the presence of a name
//!
//! The flag answers "did the account tier produce the name?", never "is there any name at all?".
//! `resolved: false` with `display_name: Some(..)` is therefore the local fallback, and
//! `resolved: false` with `display_name: None` is the unresolved marker. An account whose name is
//! absent still reports its id in `account_user_id`: the identity is partially known even when the
//! name is not, and dropping the id would make the contract claim the account itself was missing.
//!
//! A blank name is **not** a name. An account display name or a member display name that is empty
//! after trimming is treated as absent, because the screen would otherwise show an empty label
//! where the chain promises a fallback.
//!
//! # P0 has no account, and this module says so rather than inventing one (Q25)
//!
//! P0 ships **no sign-in or account-acquisition flow** (Q25), so the running app's state is built
//! with no account at all. The account tier cannot resolve, and there is no account to attribute a
//! member to either: the local fallback is not read from an arbitrary household member, because
//! choosing one would be exactly the fabricated identity SET-2's unresolved tier exists to avoid.
//! In P0 the command therefore answers the unresolved marker, or the local name once a later change
//! supplies an account. No dependency is added for identity, and no display name is fabricated from
//! the account id.
//!
//! **Nothing here touches Tauri.** The command in [`crate::ipc`] is the adapter that reads the
//! process's account and household book; this module is the chain itself and is testable without an
//! app, a service or a clock.

use crate::ipc::types::{AccountUserId, Identity, Member};

/// Resolve SET-2's chain into the frozen [`Identity`].
///
/// The three arguments are the chain's inputs, in order: the current account id (if any), the
/// display name the account service reports for it (if any), and the locally known member name the
/// caller has attributed to that account (if any). A blank account or member name is treated as
/// absent, and the local name is ignored when there is no account to attribute it to.
///
/// The result is total: every combination of inputs has an answer, and an unresolved identity is
/// the marker rather than an error. `resolved` is `true` exactly when the account tier produced the
/// name, so `resolved: false` with `display_name: Some(..)` is the local fallback and
/// `resolved: false` with `display_name: None` is unresolved.
///
/// **The local name is only used beside an account.** "The locally known member name" means the
/// name of the current account's member, so a caller cannot supply one for a person the app has no
/// account for: with no account the result is the unresolved marker. That keeps the attribution
/// rule here rather than merely in the command, and means an unattributed name can never become
/// someone's identity through this API.
pub fn resolve_identity(
    account: Option<&AccountUserId>,
    account_display_name: Option<&str>,
    local_name: Option<&str>,
) -> Identity {
    // Neither a display name nor a member name is meaningful without the identity it belongs to: a
    // name without an account is not this person's account tier and cannot be attributed locally.
    let account_name = account.and_then(|_| non_blank(account_display_name));
    let attributed_local_name = account.and_then(|_| non_blank(local_name));
    let display_name = account_name.or(attributed_local_name);

    Identity {
        account_user_id: account.cloned(),
        display_name: display_name.map(str::to_owned),
        resolved: account_name.is_some(),
    }
}

/// The display name this device's book knows for `account`, if any.
///
/// Only members whose `account_user_id` is `account` are considered, so a name recorded for another
/// member of the household cannot become the current person's name. Blank names are skipped, and
/// the first named match wins in the caller's order (the household book serves acquisition order),
/// which keeps the outcome deterministic across reads.
///
/// `None` means no member for that account carries a usable name — the next tier of SET-2's chain
/// is the caller's unresolved marker. The function never invents a name and never reads outside the
/// members it is handed.
pub fn local_member_name<'a>(
    members: impl IntoIterator<Item = &'a Member>,
    account: &AccountUserId,
) -> Option<&'a str> {
    members
        .into_iter()
        .filter(|member| &member.account_user_id == account)
        .find_map(|member| non_blank(member.display_name.as_deref()))
}

/// `value` when it is a name, `None` when it is absent or blank.
///
/// The name is returned as supplied — trimming would change what the person or the account service
/// wrote — but a value that is entirely whitespace is not a name at all.
fn non_blank(value: Option<&str>) -> Option<&str> {
    value.filter(|candidate| !candidate.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::types::MemberId;

    fn account(value: &str) -> AccountUserId {
        AccountUserId(value.to_owned())
    }

    fn member(id: &str, account: &str, display_name: Option<&str>) -> Member {
        Member {
            id: MemberId(id.to_owned()),
            account_user_id: self::account(account),
            display_name: display_name.map(str::to_owned),
        }
    }

    #[test]
    fn the_account_display_name_is_shown_when_the_account_tier_resolves() {
        let identity = resolve_identity(
            Some(&account("account-1")),
            Some("Ada Lovelace"),
            Some("Ada (local)"),
        );

        assert_eq!(identity.account_user_id, Some(account("account-1")));
        assert_eq!(identity.display_name.as_deref(), Some("Ada Lovelace"));
        assert!(identity.resolved, "the account tier produced the name");
    }

    #[test]
    fn the_local_member_name_is_the_first_fallback_when_the_account_has_no_name() {
        let identity = resolve_identity(Some(&account("account-1")), None, Some("Ada (local)"));

        assert_eq!(identity.account_user_id, Some(account("account-1")));
        assert_eq!(identity.display_name.as_deref(), Some("Ada (local)"));
        assert!(
            !identity.resolved,
            "the name came from the local member, not the account"
        );
    }

    #[test]
    fn an_unresolved_identity_is_the_marker_and_not_a_failure() {
        // The call returns an Identity, not a Result: there is no error variant to fail with, which
        // is what makes "unresolved never disturbs callers" a property of the chain's type.
        let identity = resolve_identity(None, None, None);

        assert_eq!(identity.account_user_id, None);
        assert_eq!(identity.display_name, None);
        assert!(!identity.resolved);
    }

    #[test]
    fn a_local_name_without_an_account_is_not_attributable_and_the_identity_is_unresolved() {
        let identity = resolve_identity(None, None, Some("Ada (local)"));

        assert_eq!(identity.account_user_id, None);
        assert_eq!(
            identity.display_name, None,
            "the local name is only the current account's member, so without an account it is not used"
        );
        assert!(!identity.resolved);
    }

    #[test]
    fn an_account_name_without_an_account_is_not_used() {
        let identity = resolve_identity(None, Some("Ada Lovelace"), None);

        assert_eq!(identity.account_user_id, None);
        assert_eq!(identity.display_name, None);
        assert!(!identity.resolved);
    }

    #[test]
    fn a_blank_name_is_absent_and_the_chain_falls_through() {
        for blank in ["", " ", "\t\n"] {
            let identity = resolve_identity(
                Some(&account("account-1")),
                Some(blank),
                Some("Ada (local)"),
            );

            assert_eq!(identity.account_user_id, Some(account("account-1")));
            assert_eq!(
                identity.display_name.as_deref(),
                Some("Ada (local)"),
                "{blank:?} is not a name, so the local fallback applies"
            );
            assert!(!identity.resolved);
        }

        let nothing = resolve_identity(Some(&account("account-1")), Some("   "), Some(" \t"));
        assert_eq!(nothing.display_name, None);
        assert!(!nothing.resolved);
    }

    #[test]
    fn a_name_is_never_taken_from_another_account() {
        let members = [
            member("member-1", "account-1", Some("Ada")),
            member("member-2", "account-2", Some("Grace")),
        ];

        assert_eq!(
            local_member_name(members.iter(), &account("account-1")),
            Some("Ada")
        );
        assert_eq!(
            local_member_name(members.iter(), &account("account-2")),
            Some("Grace")
        );
        assert_eq!(
            local_member_name(members.iter(), &account("account-3")),
            None,
            "an unknown account is not shown someone else's name"
        );
    }

    #[test]
    fn the_first_named_member_for_the_account_wins_and_blank_names_are_skipped() {
        let members = [
            member("member-1", "account-1", None),
            member("member-2", "account-1", Some(" ")),
            member("member-3", "account-1", Some("  Ada  ")),
            member("member-4", "account-1", Some("Grace")),
        ];

        assert_eq!(
            local_member_name(members.iter(), &account("account-1")),
            Some("  Ada  "),
            "the name is kept as recorded, and a later duplicate does not replace it"
        );
    }
}
