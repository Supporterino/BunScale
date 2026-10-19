//! Settings-area commands: the per-device display weight unit and its setter (SET-4).
//!
//! Task 2.10 owns both bodies. The pair is local to the device and changes no stored value (D0.4):
//! stored weights stay in grams and conversion happens where a weight is entered or displayed, so
//! `set_display_weight_unit` is state-changing — it carries a `RequestId` (D0.14) — without touching
//! the shared service. Its failures are therefore local (a storage fault, or `unknown`), not the
//! write path's causes.
//!
//! Both bodies are still **placeholders** that fail through [`CommandError::not_implemented`]: the
//! cause `unknown`, `retryable: false` and the placeholder key `error.not_implemented`, with the
//! command's `RequestId` carried through where it has one. That key says "this command has no
//! implementation yet"; a genuine unclassifiable failure is [`CommandError::unknown`] and names
//! `error.unknown` instead (see the parent module's docs).

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;

/// The effective per-device display unit (SET-4).
///
/// The person's own choice when there is one, otherwise the unit derived from the device's regional
/// settings (task 2.10). It is a display preference only: no stored value is read or written in
/// that unit (D0.4).
///
/// # Errors
///
/// `unknown` for an unclassifiable local-storage failure. The stub returns `unknown` until task
/// 2.10.
#[tauri::command]
#[specta::specta]
pub async fn get_display_weight_unit() -> Result<WeightUnit, CommandError> {
    Err(CommandError::not_implemented(None))
}

/// Store the per-device display unit and return the effective unit (SET-4).
///
/// The change is local to this device and never rewrites a stored value (D0.4): conversion happens
/// only where a weight is entered or displayed. `request_id` is the operation's identity, re-used
/// if the surface offers a retry (D0.14).
///
/// # Errors
///
/// `unknown` for an unclassifiable local-storage failure. The stub returns `unknown` until task
/// 2.10.
#[tauri::command]
#[specta::specta]
pub async fn set_display_weight_unit(
    unit: WeightUnit,
    request_id: RequestId,
) -> Result<WeightUnit, CommandError> {
    let _ = unit;
    Err(CommandError::not_implemented(Some(request_id)))
}
