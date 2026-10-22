//! The command surface, split by area so one body can be replaced per task (task 2.11).
//!
//! P0's eleven commands (the frozen contract of task 1.3) are grouped by the area that owns their
//! behaviour — [`household`], [`sync`], [`settings`], [`identity`], [`diagnostics`]. P1 adds thirteen
//! more, frozen by task 1.1 and grouped the same way: [`rabbit`]'s seven, [`weight`]'s five and
//! [`dashboard`]'s one. Each area file owns its commands' doc comments, imports and bodies; this file
//! owns the grouping and the re-exports below, which exist for Rust callers so that
//! `ipc::commands::<name>` keeps resolving.
//!
//! The IPC builder does **not** use that path: it registers every command through the defining area
//! module, because `collect_commands!` resolves a hidden wrapper macro at the path it is handed —
//! [`super::builder`] records why, and why a re-export path cannot serve. That split is what lets a
//! section-2 task (2.1 and 2.2 the household commands, 2.5 the collection read, 2.9 identity, 2.10
//! the display unit, 2.12 diagnostics, 2.13 the two status commands) edit one file without touching
//! a sibling's.
//!
//! **All eleven P0 bodies are implemented.** [`settings`]'s pair (2.10), [`diagnostics`]'s command
//! (2.12), [`household`]'s four (2.1, 2.2), [`sync`]'s three (2.5, 2.13) and [`identity`]'s command
//! (2.9) are written and never return the placeholder key. Failures still reach the frontend only
//! through the contract's single failure surface, [`CommandError`], built by
//! [`failure`](super::failure)'s constructors and never field by field. A genuine unclassifiable
//! failure is [`CommandError::unknown`] and names `error.unknown`; the placeholder
//! [`CommandError::not_implemented`] — the cause `unknown`, `retryable: false` and the key
//! `error.not_implemented` — remains the distinct state a future not-yet-implemented command would
//! return. It is not a second key for `unknown`: one cause owns one key (NFR-8).
//!
//! **Twelve of the thirteen P1 bodies are stubs; `create_rabbit`'s is implemented (task 2.1).** Each
//! remaining stub returns [`CommandError::not_implemented`] — cause `unknown`, `retryable: false`, key
//! `error.not_implemented` — carrying the command's [`RequestId`] on the writing commands and `None` on
//! the reading ones, so a frozen signature can never be mistaken for implemented behaviour. Section 2
//! replaces them; the signatures, the argument names the bindings export and this module split are what
//! task 1.1 froze, so a replacement body need not change `src/bindings.ts` — the state a command reaches
//! the app through is a `tauri::State` parameter, which the exporter does not emit.
//!
//! **The frontend reaches these commands only through `src/features/*/api.ts`** — wrappers over the
//! generated `src/bindings.ts` (project hard rule 2). No route, component or hook imports the
//! bindings or calls `invoke` directly, and `api.ts` is also where a state-changing command's
//! [`RequestId`] is generated and re-used on retry (task 3.1, D0.14).
//!
//! Every command is `async`: what sits behind it does network and storage I/O, so the async
//! signature is part of the contract rather than a detail of the placeholder. Every command that
//! changes product data takes a [`RequestId`]; the reads and the reachability probe take none, so
//! their failure carries `request_id: None` (they are idempotent — see
//! [`CommandError::request_id`]).
//!
//! **Why the area files take the shared types as a glob.** `use super::super::types::*` in an area
//! file does two things that both outlive section 2: the area's implementation can name any contract
//! type without an import edit that could race its siblings, and the frozen command doc comments
//! link names their signature does not use — the household reads link [`ServiceStatus`] — which
//! rustdoc can only resolve from a name that is in scope.
//!
//! [`CommandError`]: super::types::CommandError
//! [`CommandError::not_implemented`]: super::types::CommandError::not_implemented
//! [`CommandError::request_id`]: super::types::CommandError::request_id
//! [`CommandError::unknown`]: super::types::CommandError::unknown
//! [`RequestId`]: super::types::RequestId
//! [`ServiceStatus`]: super::types::ServiceStatus

pub mod dashboard;
pub mod diagnostics;
pub mod household;
pub mod identity;
pub mod rabbit;
pub mod settings;
pub mod sync;
pub mod weight;

pub use dashboard::get_dashboard;
pub use diagnostics::get_diagnostics_summary;
pub use household::{
    create_household, get_active_household, list_households, set_active_household,
};
pub use identity::get_identity;
pub use rabbit::{
    archive_rabbit, create_rabbit, delete_rabbit, get_rabbit, list_rabbits, restore_rabbit,
    update_rabbit,
};
pub use settings::{get_display_weight_unit, set_display_weight_unit};
pub use sync::{check_service_reachability, get_service_status, load_household};
pub use weight::{
    create_weight_entry, delete_weight_entry, get_weight_entries, get_weight_entry,
    update_weight_entry,
};

#[cfg(test)]
mod tests {
    use super::super::failure::NOT_IMPLEMENTED_MESSAGE_KEY;
    use super::super::types::*;
    use super::*;
    use crate::sync::double::test_support::block_on_ready;

    /// One operation identity, re-used so a failure's carried id is checked against a known value.
    fn request_id() -> RequestId {
        RequestId("operation-1".to_owned())
    }

    fn rabbit() -> RabbitId {
        RabbitId("rabbit-1".to_owned())
    }

    fn entry() -> WeightEntryId {
        WeightEntryId("entry-1".to_owned())
    }

    fn household() -> HouseholdId {
        HouseholdId("household-1".to_owned())
    }

    /// A profile with every field set, so the argument is a complete value rather than a default.
    fn profile() -> RabbitProfile {
        RabbitProfile {
            name: "Pebble".to_owned(),
            photo_ref: None,
            breed: None,
            dob: None,
            dob_unknown: true,
            sex: Sex::Unknown,
            neuter_status: NeuterStatus::Unknown,
            adoption_date: None,
            weight_drop_threshold_percent: 5.0,
        }
    }

    fn draft() -> WeightEntryDraft {
        WeightEntryDraft {
            value_grams: Grams(1_800),
            timestamp: None,
            note: None,
            outlier_confirmed: false,
        }
    }

    /// The one placeholder failure shape (design I5): classified as `unknown` with its retry withheld,
    /// and named by the placeholder key rather than by `error.unknown`.
    fn assert_not_implemented(error: &CommandError, expected: Option<RequestId>) {
        assert_eq!(error.cause, SaveCause::Unknown);
        assert!(!error.retryable);
        assert_eq!(error.message_key, NOT_IMPLEMENTED_MESSAGE_KEY);
        assert_ne!(
            error.message_key, "error.unknown",
            "a placeholder is not a second key for the classified cause"
        );
        assert_eq!(error.request_id, expected);
    }

    /// The seven remaining state-changing P1 stubs carry the operation identity, so the retry the
    /// failure surface offers has something to re-issue (D0.14, design I5). `create_rabbit` left this
    /// list when task 2.1 implemented its body; it is covered by that task's own tests.
    ///
    /// Each body is polled once through the double's helper: a stub has nothing to wait for, so a
    /// future that is not immediately ready is itself the defect.
    #[test]
    fn every_p1_write_stub_is_not_implemented_and_carries_the_operations_identity() {
        let id = request_id();

        let failures = [
            block_on_ready(rabbit::update_rabbit(rabbit(), profile(), id.clone()))
                .expect_err("update_rabbit is a stub"),
            block_on_ready(rabbit::archive_rabbit(rabbit(), id.clone()))
                .expect_err("archive_rabbit is a stub"),
            block_on_ready(rabbit::restore_rabbit(rabbit(), id.clone()))
                .expect_err("restore_rabbit is a stub"),
            block_on_ready(rabbit::delete_rabbit(rabbit(), id.clone()))
                .expect_err("delete_rabbit is a stub"),
            block_on_ready(weight::create_weight_entry(rabbit(), draft(), id.clone()))
                .expect_err("create_weight_entry is a stub"),
            block_on_ready(weight::update_weight_entry(entry(), draft(), id.clone()))
                .expect_err("update_weight_entry is a stub"),
            block_on_ready(weight::delete_weight_entry(entry(), id.clone()))
                .expect_err("delete_weight_entry is a stub"),
        ];

        assert_eq!(failures.len(), 7, "every remaining P1 write is covered");

        for failure in &failures {
            assert_not_implemented(failure, Some(id.clone()));
        }
    }

    /// The five P1 reads are idempotent, so their failure carries no operation identity (D0.14).
    #[test]
    fn every_p1_read_stub_is_not_implemented_and_carries_no_operation_identity() {
        let failures = [
            block_on_ready(rabbit::list_rabbits(household(), true))
                .expect_err("list_rabbits is a stub"),
            block_on_ready(rabbit::get_rabbit(rabbit())).expect_err("get_rabbit is a stub"),
            block_on_ready(weight::get_weight_entries(rabbit(), WeightRange::All))
                .expect_err("get_weight_entries is a stub"),
            block_on_ready(weight::get_weight_entry(entry()))
                .expect_err("get_weight_entry is a stub"),
            block_on_ready(dashboard::get_dashboard(household()))
                .expect_err("get_dashboard is a stub"),
        ];

        assert_eq!(failures.len(), 5, "every P1 read is covered");

        for failure in &failures {
            assert_not_implemented(failure, None);
        }
    }
}
