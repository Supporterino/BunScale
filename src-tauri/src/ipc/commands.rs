//! The P0 command surface, split by area so section 2 can replace one body per task (task 2.11).
//!
//! The eleven commands of the frozen contract (task 1.3) are grouped by the area that owns their
//! behaviour — [`household`], [`sync`], [`settings`], [`identity`], [`diagnostics`]. Each area file
//! owns its commands' doc comments, imports and bodies; this file owns the grouping and the
//! re-exports below, which exist for Rust callers so that `ipc::commands::<name>` keeps resolving.
//! The IPC builder does **not** use that path: it registers every command through the defining area
//! module, because `collect_commands!` resolves a hidden wrapper macro at the path it is handed —
//! [`super::builder`] records why, and why a re-export path cannot serve. That split is what lets a
//! section-2 task (2.1 and 2.2 the household commands, 2.5 the collection read, 2.9 identity, 2.10
//! the display unit, 2.12 diagnostics, 2.13 the two status commands) edit one file without touching
//! a sibling's.
//!
//! **Three of the eleven bodies are implemented; the other eight are placeholders.** [`settings`]'s
//! pair (2.10) and [`diagnostics`]'s command (2.12) are written and never return the placeholder
//! key, while every body in [`household`] (2.1, 2.2), [`sync`] (2.5, 2.13) and [`identity`] (2.9)
//! still returns `Err` through the contract's single failure surface, [`CommandError`], built by
//! [`failure`](super::failure)'s constructors and never field by field. Such a failure is
//! [`CommandError::not_implemented`] — the cause `unknown`, `retryable: false` and the
//! **placeholder key** `error.not_implemented` — carrying the command's `RequestId` where the
//! command has one. `unknown` is the only honest cause a stub can name, because the frozen list
//! admits no "not implemented" variant (D0.6); the placeholder key is what keeps the stub distinct
//! from a *classified* `unknown` failure, which is [`CommandError::unknown`] and names
//! `error.unknown`. The placeholder is a state of the surface, not a second cause — one cause
//! owns one key (NFR-8) — and no P0 screen can see a stub, because section 4 builds the
//! surfaces against section 2's implementations.
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

pub mod diagnostics;
pub mod household;
pub mod identity;
pub mod settings;
pub mod sync;

pub use diagnostics::get_diagnostics_summary;
pub use household::{
    create_household, get_active_household, list_households, set_active_household,
};
pub use identity::get_identity;
pub use settings::{get_display_weight_unit, set_display_weight_unit};
pub use sync::{check_service_reachability, get_service_status, load_household};
