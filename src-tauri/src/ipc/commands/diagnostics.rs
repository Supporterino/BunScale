//! The diagnostics command: a redacted summary of the local diagnostics sink (NFR-3).
//!
//! Task 2.12 owns this body. The summary is identifiers, counts and timestamps only — never rabbit
//! names, note text, photo content or weight values — so nothing it returns can carry content out of
//! the product. P0 defines it for support diagnostics; no P0 screen shows it, and P3 reuses it
//! unchanged.
//!
//! The body reads the process-wide sink of [`crate::diagnostics`] and returns its frozen
//! [`DiagnosticsSummary`]: how many records it holds and when the oldest and newest were recorded,
//! both `None` while the sink is empty, so "nothing has been recorded" is reported honestly rather
//! than as a defaulted instant. The sink is in memory and bounded — `SINK_CAPACITY` records, the
//! oldest dropped — so the summary describes a recent window, and reading it cannot grow the
//! process's memory.
//!
//! The records themselves never cross this boundary: only the summary is a frozen contract type, and
//! a record has no content-bearing field for a rabbit name, a note, a photo or a weight to occupy
//! (NFR-3). The sink has no transport of any kind, so there is nowhere for a record to travel.
//!
//! The one failure the body can report is an unclassifiable internal one — the sink's lock left
//! poisoned by a panic in another thread — and it goes through the failure surface's constructor,
//! [`CommandError::unknown`], never field by field: `request_id: None`, because this command takes no
//! `RequestId` and is idempotent, so its retry is a fresh call rather than a re-issued one.

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;

/// A redacted summary of the diagnostics sink: counts and timestamps, never content (NFR-3).
///
/// P0 defines it for support diagnostics only — no P0 screen shows it — and P3 reuses it unchanged.
/// It reports how many records the sink holds and when its oldest and newest held records were
/// recorded; an empty sink answers `None` for both instants, never a defaulted one. No rabbit name,
/// note text, photo content or weight value can appear here (NFR-3).
///
/// # Errors
///
/// `unknown` only when the sink itself cannot be read — its lock was left poisoned by a panic in
/// another thread — reported through the single failure surface. This read is idempotent and takes
/// no `RequestId`, so the failure carries `request_id: None`.
#[tauri::command]
#[specta::specta]
pub async fn get_diagnostics_summary() -> Result<DiagnosticsSummary, CommandError> {
    crate::diagnostics::summary().map_err(|_| CommandError::unknown(None))
}
