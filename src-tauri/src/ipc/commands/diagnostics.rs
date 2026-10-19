//! The diagnostics command: a redacted summary of the local diagnostics sink (NFR-3).
//!
//! Task 2.12 owns this body. The summary is identifiers, counts and timestamps only — never rabbit
//! names, note text, photo content or weight values — so nothing it returns can carry content out of
//! the product. P0 defines it for support diagnostics; no P0 screen shows it, and P3 reuses it
//! unchanged.
//!
//! The body is still a **placeholder** that fails through [`CommandError::not_implemented`]: the
//! cause `unknown`, `retryable: false` and the placeholder key `error.not_implemented`, with
//! `request_id: None`, because this command takes no `RequestId`. That key says "this command has no
//! implementation yet"; a genuine unclassifiable failure is [`CommandError::unknown`] and names
//! `error.unknown` instead (see the parent module's docs).

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;

/// A redacted summary of the diagnostics sink: counts and timestamps, never content (NFR-3).
///
/// P0 defines it for support diagnostics only — no P0 screen shows it — and P3 reuses it unchanged.
///
/// # Errors
///
/// `unknown` for an unclassifiable internal failure. The stub returns `unknown` until task 2.12.
#[tauri::command]
#[specta::specta]
pub async fn get_diagnostics_summary() -> Result<DiagnosticsSummary, CommandError> {
    Err(CommandError::not_implemented(None))
}
