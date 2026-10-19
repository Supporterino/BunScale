//! The identity command: the account id and the display name Settings & Profile shows (SET-2).
//!
//! Task 2.9 owns this body. An unresolved identity is **not** a failure here: [`Identity`]'s
//! `resolved` and `display_name` carry SET-2's fallback chain's outcome and the command returns
//! normally, because an absent name never blocks a screen. The chain's last tier — a localized
//! "You"-style placeholder — is frontend copy (task 3.7) and is deliberately not invented on this
//! side.
//!
//! The body is still a **placeholder** that fails through [`CommandError::not_implemented`]: the
//! cause `unknown`, `retryable: false` and the placeholder key `error.not_implemented`, with
//! `request_id: None`, because this command takes no `RequestId`. That key says "this command has no
//! implementation yet"; a genuine unclassifiable failure is [`CommandError::unknown`] and names
//! `error.unknown` instead (see the parent module's docs).

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;

/// The current person's account id and name, resolved with SET-2's fallback chain (task 2.9).
///
/// An unresolved identity returns normally (`resolved: false`, `display_name: None`): it must not
/// fail the command or block the rest of Settings (SET-2's scenario).
///
/// # Errors
///
/// `unknown` for an unclassifiable internal failure. The stub returns `unknown` until task 2.9.
#[tauri::command]
#[specta::specta]
pub async fn get_identity() -> Result<Identity, CommandError> {
    Err(CommandError::not_implemented(None))
}
