//! Integration tests for diagnostics (task 5.9): NFR-3's content exclusion, the shipped
//! configuration's lack of an analytics transport, and the content-shaped save failure.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-shared-household-foundation/specs/product-quality/spec.md` states for
//! NFR-3, and each test is named after the scenario it covers; the content-shaped case has its own
//! test beside them.
//!
//! # The boundary these tests drive
//!
//! [`DiagnosticSink`] is the sink the process uses, [`DiagnosticRecord::save_failed`] is the
//! constructor the save path uses, and [`SaveGate`] is the write path that produces the failure.
//! The payload really is part of the attempt: it travels in the [`SaveRequest`] body to the
//! transport, and the failure is driven through the gate — so the test has the content in hand
//! before it asserts that nothing content-shaped reached the record or the summary.
//!
//! **No production call site records into the sink yet.** The module documentation names the tasks
//! that own the recording sites (the save path, a load, a probe), but P0's code calls none of them,
//! so "a failed save records exactly one record" cannot be asserted end to end through a command.
//! What these tests compose is the two halves the product will compose — the failed attempt and the
//! record its constructor builds — and they assert the recorded value, its serialised form and the
//! summary. The sink used for the counts is a local [`DiagnosticSink`] with the production
//! capacity, not the process-wide static, so the counts are exact and the tests are independent.
//!
//! # Why the value is serialised
//!
//! A typed-field assertion cannot catch a field added later: the record's serialisation is what a
//! new `String` or `serde_json::Value` field would show up in, both as a value and as a key. So
//! the tests scan the serialised record and summary for the payload's values and hold the record's
//! whole key set to the vocabulary the crate's unit tests declare. Rendering is not involved, and
//! nothing here is platform-specific (TOOL-1: iOS and macOS).

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use app_lib::diagnostics::{
    DiagnosticOutcome, DiagnosticRecord, DiagnosticSink, DiagnosticSubject, SINK_CAPACITY,
};
use app_lib::ipc::types::{Instant, RequestId, SaveCause};
use app_lib::settings::unit::GRAMS_PER_KILOGRAM;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::double::{RecordedCall, ScriptedService};
use app_lib::sync::{Answer, SaveGate, SaveRequest, ServiceReply};

/// The instant the record is made at.
const RECORDED_AT: &str = "2026-10-19T10:00:00Z";

/// The rabbit's name in the health-shaped payload.
const RABBIT_NAME: &str = "Clover";

/// The note text in the health-shaped payload.
const NOTE: &str = "sore paw, keep an eye on it";

/// Photo content in the health-shaped payload: a tiny PNG, base64-encoded.
const PHOTO: &str = concat!(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAY",
    "AAAAFcSJAAAADUlEQVR42mP8z8BQDwAE",
    "hQGAhKmMIQAAAABJRU5ErkJggg==",
);

/// The weight in the health-shaped payload, in kilograms as a person would enter it.
const WEIGHT_KG: f64 = 2.437;

/// The analytics transports the shipped configuration must not declare.
///
/// The token list mirrors the crate's unit-level guard (`diagnostics::mod`'s test) and adds
/// `"plausible"`, another web-analytics vendor that would arrive through `package.json` like the
/// rest. What is new here is the *third file*: `tauri.conf.json` joins the two manifests, because a
/// transport can also arrive as a script or a URL rather than as a dependency.
const ANALYTICS_TRANSPORTS: [&str; 19] = [
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
    "plausible",
];

/// The key vocabulary a save-failure record declares.
///
/// It mirrors the crate's `SAVE_FAILED_KEYS`, which is `#[cfg(test)]` inside the library and so not
/// visible from an integration test; a new field on the record changes this assertion.
const DECLARED_SAVE_FAILURE_KEYS: [&str; 6] = ["at", "cause", "id", "kind", "outcome", "subject"];

fn at(value: &str) -> Instant {
    Instant(value.to_owned())
}

fn request_id(value: &str) -> RequestId {
    RequestId(value.to_owned())
}

/// A health entry of the shape P1 will save: a name, a note, photo bytes and a weight.
fn health_payload() -> Value {
    json!({
        "rabbit_name": RABBIT_NAME,
        "note": NOTE,
        "photo_base64": PHOTO,
        "weight_kg": WEIGHT_KG,
    })
}

/// Everything in the payload that a diagnostic value must never carry.
///
/// Both weight spellings are derived from [`WEIGHT_KG`], so changing the payload cannot silently
/// stop covering the weight: the value as entered, and its gram equivalent as the canonical
/// (integer) form a weight would be stored in.
fn content_values() -> Vec<String> {
    vec![
        RABBIT_NAME.to_owned(),
        NOTE.to_owned(),
        PHOTO.to_owned(),
        format!("{WEIGHT_KG}"),
        format!("{}", (WEIGHT_KG * GRAMS_PER_KILOGRAM) as u64),
    ]
}

/// Assert the payload really carries the content values the scans look for.
///
/// The kilogram spelling is derived from [`WEIGHT_KG`] rather than typed, so this keeps the
/// derivation honest: if the payload's own rendering stopped matching, a scan would look for a
/// string the payload never contains and pass vacuously. The gram spelling is the same weight
/// through the product's canonical factor and follows the constant, so it needs no payload field.
fn assert_payload_carries_content(payload: &Value) {
    let serialised = serde_json::to_string(payload).expect("the payload serialises");

    for content in [
        RABBIT_NAME.to_owned(),
        NOTE.to_owned(),
        PHOTO.to_owned(),
        format!("{WEIGHT_KG}"),
    ] {
        assert!(
            serialised.contains(&content),
            "the payload does not carry {content:?}"
        );
    }
}

/// Every write the transport was asked to submit, as the body it carried.
fn write_bodies(service: &ScriptedService) -> Vec<Value> {
    service
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            RecordedCall::Save { request, .. } => Some(request.body().clone()),
            _ => None,
        })
        .collect()
}

/// Every object key in `value`, at any depth, sorted and deduplicated.
///
/// A nested object counts too, so a content-bearing field inside the outcome would be caught.
fn keys_of(value: &Value) -> Vec<String> {
    fn collect(value: &Value, keys: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, nested) in map {
                    keys.push(key.clone());
                    collect(nested, keys);
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect(item, keys);
                }
            }
            _ => {}
        }
    }

    let mut keys = Vec::new();
    collect(value, &mut keys);
    keys.sort();
    keys.dedup();

    keys
}

/// One shipped file, read from the crate root and returned lowercased for a containment check.
fn lowercased(name: &str) -> (String, String) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));

    (name.to_owned(), contents.to_ascii_lowercase())
}

/// NFR-3, "Diagnostics carry no content".
///
/// **WHEN** a diagnostic record is produced **THEN** it contains identifiers, counts and outcomes
/// only, and no rabbit name, note text, photo content or weight value.
#[test]
fn diagnostics_carry_no_content() {
    let operation = request_id("operation-1");
    let payload = health_payload();
    assert_payload_carries_content(&payload);

    // The payload is really part of the attempted write: the save path hands it to the transport.
    let service = ScriptedService::new().on_write([ServiceReply::NoResponse]);
    let request = SaveRequest::new(operation.clone(), "save_health_entry", payload.clone())
        .expect("a well-formed write");
    let failure = block_on_ready(SaveGate::new(&service).save(request))
        .expect_err("an unreachable service refuses the save");

    assert_eq!(failure.cause, SaveCause::Connectivity);
    assert_eq!(
        service.write_count(),
        1,
        "the payload was part of the attempt"
    );
    assert_eq!(
        write_bodies(&service),
        vec![payload],
        "the transport really received the health payload"
    );

    // What the save path records for that failure: the moment, the operation's identity, the cause.
    let entry = DiagnosticRecord::save_failed(at(RECORDED_AT), operation.clone(), failure.cause);
    let mut sink = DiagnosticSink::new(SINK_CAPACITY);
    sink.record(entry);

    // THEN the failure surface itself carries no content ...
    let error_text = serde_json::to_string(&failure).expect("the failure surface serialises");
    for content in content_values() {
        assert!(
            !error_text.contains(&content),
            "the failure carried {content:?}"
        );
    }

    // ... and neither does the record, value by value and key by key.
    let record = sink.records().next().expect("the record is held");
    let serialised = serde_json::to_string(record).expect("a record serialises");
    for content in content_values() {
        assert!(
            !serialised.contains(&content),
            "the record carries {content:?}"
        );
    }

    let value = serde_json::to_value(record).expect("a record serialises");
    assert_eq!(
        keys_of(&value),
        DECLARED_SAVE_FAILURE_KEYS,
        "a field outside the declared vocabulary reached the record"
    );

    // AND the typed record is the identifier, cause and instant it was given, and nothing else.
    assert_eq!(record.at(), &at(RECORDED_AT));
    assert_eq!(
        record.subject(),
        Some(&DiagnosticSubject::Request(operation))
    );
    assert_eq!(
        record.outcome(),
        &DiagnosticOutcome::SaveFailed {
            cause: SaveCause::Connectivity,
        }
    );

    // The summary of that sink counts the one record and names its instant; no content is in it.
    let summary = sink.summary();
    assert_eq!(summary.record_count, 1);
    assert_eq!(summary.oldest_at, Some(at(RECORDED_AT)));
    assert_eq!(summary.newest_at, Some(at(RECORDED_AT)));

    let summary_text = serde_json::to_string(&summary).expect("the summary serialises");
    for content in content_values() {
        assert!(
            !summary_text.contains(&content),
            "the summary carries {content:?}"
        );
    }
}

/// NFR-3, "No third-party analytics transport is configured".
///
/// **WHEN** the app is built and run **THEN** it sends health and weight data only to its own
/// shared service, and no third-party analytics service receives it.
#[test]
fn no_third_party_analytics_transport_is_configured() {
    // The three files the app actually ships: the Rust manifest, the webview's dependency manifest
    // and the app configuration. The first two mirror the crate's unit-level guard; the
    // configuration is the integration-side addition, because a transport can also arrive as a
    // script or a URL.
    let shipped = [
        lowercased("Cargo.toml"),
        lowercased("../package.json"),
        lowercased("tauri.conf.json"),
    ];

    for transport in ANALYTICS_TRANSPORTS {
        for (name, contents) in &shipped {
            assert!(
                !contents.contains(transport),
                "{name} declares the analytics transport {transport}"
            );
        }
    }

    // The webview's CSP is the transport half of the same fact: it may connect only to the Tauri
    // IPC, so no analytics endpoint is reachable even from a bundled script. P0 has no service
    // origin yet (D0.7: the deployment is outside this repository), so any third-party origin here
    // would be a new decision rather than an inherited default.
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let config: Value = serde_json::from_str(
        &fs::read_to_string(&config_path).expect("the app configuration is readable"),
    )
    .expect("the app configuration is JSON");
    let connect = config["app"]["security"]["csp"]["connect-src"]
        .as_array()
        .expect("connect-src is declared in the app configuration");
    let origins: Vec<&str> = connect.iter().filter_map(Value::as_str).collect();
    assert_eq!(
        origins,
        ["ipc:", "http://ipc.localhost"],
        "the webview may connect only to the Tauri IPC"
    );

    // And the summary is the only diagnostics value the frontend sees: the generated bindings
    // export the one command and its summary type, and no record type.
    let (_, bindings) = lowercased("../src/bindings.ts");
    assert!(
        bindings.contains("getdiagnosticssummary"),
        "the diagnostics command is not in the generated bindings"
    );
    assert!(
        bindings.contains("export type diagnosticssummary"),
        "the summary type is not exported to the frontend"
    );
    assert!(
        !bindings.contains("diagnosticrecord"),
        "a record type reached the generated bindings"
    );
    assert!(
        !bindings.contains("diagnostic_record"),
        "a record type reached the generated bindings"
    );
}

/// The content-shaped input case: a save failure carrying a health payload leaves the sink's
/// summary with counts and timestamps only.
///
/// The failure here is the service's own allowance report rather than an unreachable transport: a
/// second cause path, whose recorded *summary* is this test's subject. The record's own text is the
/// first test's.
#[test]
fn a_health_payload_shaped_save_failure_leaves_only_counts_and_timestamps() {
    let operation = request_id("operation-2");
    let payload = health_payload();
    assert_payload_carries_content(&payload);

    let service = ScriptedService::new().on_write([ServiceReply::Answered(Answer::with_body(
        200,
        json!({ "outcome": "allowance_exceeded" }),
    ))]);
    let request = SaveRequest::new(operation.clone(), "save_health_entry", payload.clone())
        .expect("a well-formed write");
    let failure = block_on_ready(SaveGate::new(&service).save(request))
        .expect_err("the household's allowance is exhausted");

    assert_eq!(failure.cause, SaveCause::AllowanceExceeded);
    assert_eq!(
        write_bodies(&service),
        vec![payload],
        "the payload was part of the attempt"
    );

    let mut sink = DiagnosticSink::new(SINK_CAPACITY);
    sink.record(DiagnosticRecord::save_failed(
        at(RECORDED_AT),
        operation,
        failure.cause,
    ));

    // THEN the sink holds one record, and the summary is the count and the recorded instants only.
    let summary = sink.summary();
    assert_eq!(sink.len(), 1, "exactly one record was recorded");
    assert_eq!(summary.record_count, 1);
    assert_eq!(summary.oldest_at, Some(at(RECORDED_AT)));
    assert_eq!(summary.newest_at, Some(at(RECORDED_AT)));

    let value = serde_json::to_value(&summary).expect("the summary serialises");
    assert_eq!(
        keys_of(&value),
        ["newest_at", "oldest_at", "record_count"],
        "the summary grew a field"
    );

    let serialised = serde_json::to_string(&summary).expect("the summary serialises");
    for content in content_values() {
        assert!(
            !serialised.contains(&content),
            "the summary carries {content:?}"
        );
    }
}
