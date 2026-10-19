//! The redacted diagnostics sink: identifiers, counts, timestamps and outcomes only (NFR-3).
//!
//! NFR-3 allows a diagnostic record to hold **identifiers, counts and outcomes** and forbids it to
//! hold **a rabbit name, note text, photo content or a weight value**. That line is held here by the
//! shape of the types rather than by a reviewer's discipline. [`DiagnosticRecord`] has three fields —
//! an absolute instant (D0.5), an optional [`DiagnosticSubject`] and a [`DiagnosticOutcome`] — and
//! every leaf below them is one of exactly four kinds: an opaque identifier newtype taken from the
//! frozen contract, a count, an absolute instant, or a closed enum built from the frozen
//! [`SaveCause`](crate::ipc::types::SaveCause) and
//! [`ServiceState`](crate::ipc::types::ServiceState). There is no free-form `String`, no byte buffer
//! and no numeric weight; the named constructors that build a record take the identifier, the count,
//! the cause and the instant and nothing else. A rabbit name, a note, a photo or a weight therefore
//! has **no content-shaped field to go in and no content-shaped parameter to arrive through** — with
//! the one qualification `records` states in full: an identifier newtype is a transparent `pub String`
//! frozen by the contract, so an identifier stays opaque by the contract's own discipline (a
//! `RequestId` is generated per operation, D0.14) and never by anything these types can enforce.
//! [`DiagnosticSubject`] is where the limit is stated most plainly: it admits a household id and an
//! operation id, and no name-shaped variant, because a display name (let alone a rabbit's) is content.
//!
//! **The sink is bounded, and the bound is stated rather than implied.** [`DiagnosticSink`] is a ring
//! of at most [`SINK_CAPACITY`] = 64 records; recording into a full ring drops its **oldest** record.
//! Sixty-four is the deliberate middle between two bad ends: far more than a session's ordinary
//! handful of save failures, probes and loads, so the summary describes a recent window rather than a
//! single event, and small enough that the whole sink is a few kilobytes — no reason to persist it, to
//! evict by age, or to worry about it on a suspended mobile process. Records are appended as they
//! happen, so the ring's front is the oldest survivor and its back the newest; that is how the
//! summary's `oldest_at` and `newest_at` are read without ordering two instants, a job that belongs to
//! task 2.8.
//!
//! **Records never leave the device.** The sink lives in this process's memory: it opens no file,
//! holds no socket and has no transport of any kind, and the only network calls this crate makes are
//! the shared service's own (ADR 0002), which carry household data rather than diagnostics. NFR-3's
//! "no third-party analytics" is therefore a capability this module **lacks** rather than a policy it
//! honours: no analytics or telemetry SDK is in the crate to be configured, and the test below holds
//! the dependency manifest to that.
//!
//! **What is deliberately absent, and who owns it.** Persisting the sink, keeping records across a
//! launch and any retention policy for a stored copy are not P0's, and nothing here schedules,
//! aggregates or uploads. Only one view of the sink is frozen —
//! [`DiagnosticsSummary`](crate::ipc::types::DiagnosticsSummary) with its count and two timestamps,
//! the value `get_diagnostics_summary` returns (task 1.3) — so the records themselves never cross the
//! IPC boundary. The call sites that record are the tasks that observe the outcomes: the save path
//! (2.4), a collection load (2.5) and the reachability probe (2.13); this module owns the shape they
//! record into, the bound they record under and the summary the command returns.

pub mod records;
pub mod sink;

pub use records::{DiagnosticOutcome, DiagnosticRecord, DiagnosticSubject};
pub use sink::{DiagnosticSink, SINK_CAPACITY, SinkError, record, summary};

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    /// The two dependency manifests a third-party analytics transport could arrive through, each
    /// lowercased once so a check is a plain containment test: the crate's `Cargo.toml` and the app's
    /// root `package.json`.
    ///
    /// Both are read because a Tauri app has two dependency surfaces — a Rust SDK and a
    /// JavaScript/webview SDK — and the JavaScript one is at least as likely to gain an analytics
    /// package while no Rust manifest changes. `Cargo.lock` is deliberately **not** read: it lists
    /// every transitive crate, where a token like `segment` false-positives on `unicode-segmentation`.
    ///
    /// The paths are resolved from the crate root (`CARGO_MANIFEST_DIR`) rather than from the process
    /// working directory — the same rule the bindings export follows — so `..` reaches the repository
    /// root whatever directory the test binary runs from.
    fn lowercased_manifests() -> [(&'static str, String); 2] {
        let names = ["Cargo.toml", "../package.json"];

        names.map(|name| {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
            let contents = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));

            (name, contents.to_ascii_lowercase())
        })
    }

    #[test]
    fn neither_manifest_configures_a_third_party_analytics_transport() {
        // NFR-3 forbids health and weight data reaching a third-party analytics service. This crate
        // has no analytics transport at all: the diagnostics sink is in memory and has no network
        // surface, and the only client P0 adds is the shared service's own (ADR 0002). This test is the
        // module-level guard for that, and it is deliberately a *guard*, not a proof: the proof is the
        // shape of the types (`DiagnosticRecord` has no content-bearing field) and of the sink (no
        // transport field, no file, no socket). What a guard catches is the realistic regression — an
        // SDK riding in as a dependency — and it has to watch both surfaces this app has, because the
        // webview's `package.json` is a dependency file no Rust manifest would show.
        let manifests = lowercased_manifests();

        for transport in [
            // The Rust-side names.
            "sentry",
            "posthog",
            "amplitude",
            "mixpanel",
            "segment",
            "datadog",
            "newrelic",
            "new-relic",
            "firebase",
            "google-analytics",
            "opentelemetry-otlp",
            "bugsnag",
            "instabug",
            "appcenter",
            // The JavaScript/webview-side names.
            "@sentry/",
            "logrocket",
            "heap",
            "countly",
        ] {
            for (manifest, contents) in &manifests {
                assert!(
                    !contents.contains(transport),
                    "{manifest} declares the analytics transport {transport}"
                );
            }
        }
    }
}
