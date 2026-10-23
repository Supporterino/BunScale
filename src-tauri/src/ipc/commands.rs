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
//! return. `error.not_implemented` is a surface state, and `error.household_content_unloaded` is the
//! one documented second key for `unknown` (D1.16); see [`failure`](super::failure)'s module doc.
//!
//! **Three of the thirteen P1 bodies are stubs; `create_rabbit` (task 2.1), `update_rabbit` (task
//! 2.4), `list_rabbits` and `archive_rabbit` (task 2.5), `restore_rabbit` (task 2.6), `delete_rabbit`
//! (task 2.7), `create_weight_entry` (task 2.9), `update_weight_entry` and `delete_weight_entry` (task
//! 2.11) and `get_weight_entries` (task 2.12) are implemented.** Each remaining stub returns
//! [`CommandError::not_implemented`] — cause `unknown`, `retryable: false`, key
//! `error.not_implemented` — carrying `None` on the reading commands, so a frozen signature can never
//! be mistaken for implemented behaviour. The remaining reads are `get_rabbit` (still awaiting an
//! owner in `tasks.md`), `get_weight_entry` (2.15) and `get_dashboard` (2.14). Replacing a body needs
//! no `src/bindings.ts` change — the state a command reaches the app through is a `tauri::State`
//! parameter, which the exporter does not emit.
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

    fn rabbit() -> RabbitId {
        RabbitId("rabbit-1".to_owned())
    }

    fn entry() -> WeightEntryId {
        WeightEntryId("entry-1".to_owned())
    }

    fn household() -> HouseholdId {
        HouseholdId("household-1".to_owned())
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

    /// The three remaining P1 read stubs are idempotent, so their failure carries no operation
    /// identity (D0.14). `list_rabbits` left this list when task 2.5 implemented its body and
    /// `get_weight_entries` left it when task 2.12 did; both are covered by their tasks' own tests.
    /// The two write stubs left when task 2.11 implemented `update_weight_entry` and
    /// `delete_weight_entry`.
    ///
    /// Each body is polled once through the double's helper: a stub has nothing to wait for, so a
    /// future that is not immediately ready is itself the defect.
    #[test]
    fn every_p1_read_stub_is_not_implemented_and_carries_no_operation_identity() {
        let failures = [
            block_on_ready(rabbit::get_rabbit(rabbit())).expect_err("get_rabbit is a stub"),
            block_on_ready(weight::get_weight_entry(entry()))
                .expect_err("get_weight_entry is a stub"),
            block_on_ready(dashboard::get_dashboard(household()))
                .expect_err("get_dashboard is a stub"),
        ];

        assert_eq!(failures.len(), 3, "every remaining P1 read is covered");

        for failure in &failures {
            assert_not_implemented(failure, None);
        }
    }
}
