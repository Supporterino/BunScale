//! The identity command: the account id and the display name Settings & Profile shows (SET-2).
//!
//! Task 2.9 owns this body. The chain itself is [`crate::domain::identity`]; this adapter only
//! supplies what the process actually knows — the account the state was built with and the locally
//! known members in its household book — and returns the frozen [`Identity`]. An unresolved
//! identity is **not** a failure here: the command answers `Ok` with `resolved: false` and
//! `display_name: None`, because an absent name never blocks a screen (SET-2's scenario). The last
//! tier of the chain — a localized "You"-style placeholder — is frontend copy (task 3.7) and is
//! deliberately not invented on this side.
//!
//! **P0 has no account, so its account tier cannot resolve (Q25).** The running app is constructed
//! with no account (there is no sign-in or account-acquisition flow in this change), and P0 holds
//! no account display name either: D0.7 follow-up 2 takes that from the sign-in flow's user
//! metadata, which does not exist yet. The adapter therefore passes no account display name, and
//! with no account id it cannot attribute a local member to the person. The honest P0 outcome is
//! the unresolved marker — not a name derived from an account id, not an arbitrary household
//! member, and not a local-only identity invented to fill the screen. The arguments are already in
//! the chain's shape, so the change that supplies an account and its name changes what the adapter
//! reads, not the rule that decides the name.
//!
//! The local fallback is read from members whose `account_user_id` is the current account. A name
//! recorded for a different member is never borrowed, and a book with no such member contributes
//! nothing. That is what keeps "no local name" a legitimate answer rather than a failure.
//!
//! `src/bindings.ts` carries the command's doc comment below verbatim (D0.9); its change is the
//! deliberate doc-only diff of the implemented body (D0.18).

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;
use crate::domain::identity::{local_member_name, resolve_identity};
use crate::state::AppState;

/// The command body: SET-2's chain over the process's state, in the contract's result type.
///
/// It returns the same `Result<Identity, CommandError>` as [`get_identity`], and no path in it
/// produces `Err`: an unresolved identity is the normal outcome (see the command's doc comment).
/// This is the outermost layer a unit test can call. `tauri::State` has no public constructor
/// outside a running app, and the mock runtime that could supply one is behind `tauri`'s `test`
/// feature, which this crate does not enable; [`get_identity`] is therefore the one-line adapter
/// over this function that the handler invokes.
fn current_identity(state: &AppState) -> Result<Identity, CommandError> {
    let account = state.account();

    let local_name = account.and_then(|account| {
        let households = state.households();

        local_member_name(
            households.held().flat_map(|household| household.members()),
            account,
        )
        .map(str::to_owned)
    });

    Ok(resolve_identity(account, None, local_name.as_deref()))
}

/// The current person's account id and name, resolved with SET-2's fallback chain (task 2.9).
///
/// The account's display name when the account tier resolves, otherwise the locally known member
/// name for that account, otherwise no name. The account id is the one the app's state was built
/// with; P0 has no sign-in flow (Q25), so the running app has none and this read answers the
/// unresolved marker. An account known without a display name still reports its id and falls back
/// to the local name, with `resolved: false`.
///
/// An unresolved identity returns normally (`resolved: false`, `display_name: None`): it must not
/// fail the command or block the rest of Settings (SET-2's scenario). The final "You"-style
/// placeholder is frontend copy (task 3.7), so this command reports "no name known" instead of
/// inventing one.
///
/// # Errors
///
/// None is reachable: the read is local, and an unresolved identity is the contract's normal
/// outcome rather than a failure. The signature keeps the shared `CommandError` surface for the
/// frozen contract, but no path in this body produces one.
#[tauri::command]
#[specta::specta]
pub async fn get_identity(state: tauri::State<'_, AppState>) -> Result<Identity, CommandError> {
    current_identity(state.inner())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::domain::HouseholdRecord;
    use crate::ipc::types::{HouseholdId, Instant, Member, MemberId, RequestId};
    use crate::sync::ScriptedService;

    fn account(value: &str) -> AccountUserId {
        AccountUserId(value.to_owned())
    }

    fn state_with(account: Option<AccountUserId>) -> AppState {
        AppState::new(Arc::new(ScriptedService::new()), account)
    }

    /// Put one household in the state's book, whose owner member carries `display_name`.
    fn seed_member(state: &AppState, account: &AccountUserId, display_name: Option<&str>) {
        let record = HouseholdRecord::restore(
            HouseholdId("household-1".to_owned()),
            "Home".to_owned(),
            Instant("2026-10-19T10:00:00Z".to_owned()),
            MemberId("member-1".to_owned()),
            vec![Member {
                id: MemberId("member-1".to_owned()),
                account_user_id: account.clone(),
                display_name: display_name.map(str::to_owned),
            }],
        )
        .expect("the seeded record satisfies the domain's invariants");

        state
            .households()
            .record_created(RequestId("operation-1".to_owned()), record);
    }

    #[test]
    fn the_command_body_answers_ok_with_the_unresolved_marker_without_an_account() {
        let state = state_with(None);

        // The body returns the contract's `Result`; an unresolved identity is `Ok`, never `Err`
        // (SET-2). The `Ok` assertion is the point of this test: the chain's result is not a
        // failure surface.
        let identity =
            current_identity(&state).expect("an unresolved identity is a normal Ok result");

        assert_eq!(identity.account_user_id, None);
        assert_eq!(identity.display_name, None);
        assert!(!identity.resolved);
    }

    /// Holds the exported command's shape at compile time without needing a `tauri::State` value.
    ///
    /// `tauri::State` has no public constructor outside an app, so the test above reaches the same
    /// `Result` through the body the command delegates to; this pins that the exported
    /// `get_identity` still takes exactly that state and yields exactly that output.
    fn assert_command_shape<'a, F, Fut>(_: F)
    where
        F: Fn(tauri::State<'a, AppState>) -> Fut,
        Fut: std::future::Future<Output = Result<Identity, CommandError>> + 'a,
    {
    }

    #[test]
    fn the_exported_command_is_the_tested_body_behind_tauri_state() {
        assert_command_shape(get_identity);
    }

    #[test]
    fn without_an_account_a_local_member_name_is_not_borrowed() {
        let state = state_with(None);
        seed_member(&state, &account("account-1"), Some("Ada"));

        let identity =
            current_identity(&state).expect("an unattributed local name is not a failure");

        assert_eq!(identity.account_user_id, None);
        assert_eq!(
            identity.display_name, None,
            "P0 cannot attribute the member to a person, so it must not guess one"
        );
        assert!(!identity.resolved);
    }

    #[test]
    fn a_locally_known_member_name_is_the_fallback_for_a_known_account() {
        let state = state_with(Some(account("account-1")));
        seed_member(&state, &account("account-1"), Some("Ada"));

        let identity = current_identity(&state).expect("the local fallback is a normal Ok result");

        assert_eq!(identity.account_user_id, Some(account("account-1")));
        assert_eq!(identity.display_name.as_deref(), Some("Ada"));
        assert!(
            !identity.resolved,
            "P0 has no account display name, so the name is the local fallback"
        );
    }

    #[test]
    fn a_member_of_another_account_is_never_the_persons_name() {
        let state = state_with(Some(account("account-2")));
        seed_member(&state, &account("account-1"), Some("Ada"));

        let identity = current_identity(&state)
            .expect("another account's member contributes no name and no failure");

        assert_eq!(identity.account_user_id, Some(account("account-2")));
        assert_eq!(identity.display_name, None);
        assert!(!identity.resolved);
    }
}
