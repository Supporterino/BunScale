//! Reachability re-evaluation at the platform lifecycle's prompts (NFR-1, D0.13; task 2.13).
//!
//! The product re-evaluates whether the shared service can be reached when the person comes back to
//! it: a **foreground return on iOS** and a **window that regains focus on macOS**, where the product
//! kept running. Both are only *triggers*; neither has any logic of its own. Every trigger — and
//! `check_service_reachability` — goes through [`re_evaluate`], so there is exactly one sequence in
//! the product:
//!
//! 1. take a **fresh** probe (task 2.4's [`super::probe`]), never a remembered status (D0.2, R3);
//! 2. record its outcome on the [`StatusTracker`] with this device's clock (task 2.8's
//!    [`AbsoluteInstant::now`]);
//! 3. publish the resulting [`ServiceStatus`] as `service:status-changed`;
//! 4. return it to the caller.
//!
//! # The trigger also refreshes the active household's copy (D0.2, D0.23)
//!
//! When a **lifecycle trigger's** fresh outcome is `reachable` with a valid account and this process
//! holds an active household, the trigger additionally publishes `household:changed` with kind
//! `refreshed` for that household. That is the producer the frozen event always promised: the
//! frontend's existing `refreshed` policy invalidates the household's collection query, which calls
//! `load_household` and therefore performs the reload D0.2 describes for a start or foreground
//! return. `created` and `switched` come from the household commands instead.
//!
//! The publication is **not** part of [`re_evaluate`] and **not** part of `load_household`.
//! `check_service_reachability` calls [`re_evaluate`] too, and asking for a status must not
//! invalidate household data; and because the frontend's `refreshed` policy reloads the collection,
//! an emit from the load would refetch in a loop. Only the trigger path (`Coalescer::run`) emits it.
//! Structural guarantee, not a convention: neither [`re_evaluate`] nor
//! [`ReadPath::load`](crate::sync::ReadPath::load) takes a [`HouseholdChangePublisher`] at all, so
//! the load path cannot publish `household:changed` even by mistake.
//!
//! The trigger's own gate is the **fresh** outcome, not the status it leaves behind. A probe that
//! got no answer does not prove connectivity: below the offline threshold the tracker deliberately
//! retains the previous status, which can still be `reachable` with a valid account. Publishing a
//! refresh from that retained value would claim a connection this trigger never had, so the gate
//! also requires the probe to have answered. The trigger still publishes the retained status as
//! `service:status-changed`; only the household refresh is withheld.
//!
//! # The bounds are applied once, in the tracker (D0.13)
//!
//! [`super::probe`] gives each of its two checks [`super::REACHABILITY_PROBE_TIMEOUT`] (5 s), so a
//! check that gets no answer counts as a failed probe ([`ProbeOutcome::is_failed_probe`]). The
//! threshold itself lives in [`StatusTracker::observe`]: a run of failed probes is reported as
//! `unreachable` only at [`super::OFFLINE_FAILURE_THRESHOLD`] (3), and any answered probe — a plain
//! success, or a `401`/`429` that says the service is there but the account is not usable — resets
//! the count. This module does not re-derive either rule; it only feeds the tracker fresh outcomes.
//!
//! **Nothing here survives the process.** A tracker is created empty on every launch (see
//! `state::AppState`), so no trigger can surface a previous session's status as current (R3).
//!
//! # What each platform's trigger observes — and what it does not
//!
//! **iOS.** The proven foreground signal on the pinned stack is the window-level
//! [`tauri::WindowEvent::Resumed`]: tao 0.37.1's iOS backend maps both
//! `applicationWillEnterForeground` and `sceneWillEnterForeground` to it. It is the event this module
//! triggers on. `RunEvent::Resumed` is
//! deliberately **not** a trigger: on tauri 2.12.4 / tauri-runtime-wry 2.12.2 it is produced only
//! from wry's `StartCause::Poll`, an event-loop wake-up rather than the platform's foreground
//! callback, so treating it as "the person came back" would record an observation at a moment this
//! module cannot justify.
//!
//! **macOS.** The trigger is a window gaining key focus (`WindowEvent::Focused(true)`,
//! `windowDidBecomeKey`). That covers the person returning to the product, but it does **not** cover
//! connectivity returning while the product stays focused. NFR-1's `[macos]` "connectivity is
//! restored while the product remains running" is therefore observed at the next focus/activation or
//! on an explicit [`check_service_reachability`](crate::ipc::commands::sync::check_service_reachability),
//! not at the instant connectivity returns. P0 implements no bounded periodic re-check: a polling
//! policy would need its own interval and back-off, and inventing one here would be a policy this
//! change does not own. The same limit applies to the iOS trigger: it fires on a lifecycle return,
//! not when the network changes underneath a product that never left the foreground.
//!
//! # One trigger-driven re-evaluation at a time
//!
//! A trigger is a request to look, not a command to measure once per event. If a second trigger
//! arrives while a trigger-driven re-evaluation is still running, it does not start a second probe:
//! it sets a rerun flag, and the running task performs exactly one follow-up when it finishes. This
//! is what keeps completion order equal to start order, so an older probe cannot increment the
//! failure count after a newer success has reset it. The explicit
//! [`check_service_reachability`](crate::ipc::commands::sync::check_service_reachability) command is
//! not coalesced: it is the person asking for a probe *now*, and its own result is what it returns
//! and publishes.
//!
//! # Why the trigger decision is separate from the re-evaluation
//!
//! [`is_re_evaluation_trigger`] is a pure predicate over [`tauri::RunEvent`], `cfg`-gated per
//! platform. On iOS the foreground return is the window-level `WindowEvent::Resumed`. On macOS it is
//! a `WindowEvent::Focused(true)` for a window that the person brought back. Every other platform —
//! and every other event — answers `false`, so a platform with no such lifecycle event compiles and
//! simply never fires a trigger; no later platform has to define a variant this module names.
//!
//! [`ProbeOutcome::is_failed_probe`]: super::ProbeOutcome::is_failed_probe
//! [`StatusTracker::observe`]: super::StatusTracker::observe
//! [`tauri::RunEvent`]: tauri::RunEvent

#[cfg(any(target_os = "ios", target_os = "macos"))]
use tauri::WindowEvent;
use tauri::{AppHandle, Manager, RunEvent};

use std::sync::{Mutex, MutexGuard, OnceLock};

use super::StatusTracker;
use super::probe::{ProbeOutcome, probe};
use crate::ipc::events::{HouseholdChangeKind, HouseholdChangePublisher, HouseholdChanged};
use crate::ipc::types::{ServiceState, ServiceStatus};
use crate::state::AppState;
use crate::storage::AbsoluteInstant;

/// Where one re-evaluation publishes its fresh outcome.
///
/// It exists so the sequence can be exercised without building a Tauri app: the running app's
/// implementation is [`tauri::AppHandle`], which emits the frozen `service:status-changed` event, and
/// the unit tests use a recorder. This is a seam over *publication only* — the probe and the
/// tracker update are the same in both cases.
pub trait StatusPublisher {
    /// Publish `status` to whoever is listening.
    fn publish(&self, status: &ServiceStatus);
}

/// Publish through Tauri's own event bus (task 1.3's `service:status-changed`).
///
/// The event was mounted by `ipc::builder().mount_events(app)` in the app's single `setup`, so
/// [`tauri_specta::Event::emit`] resolves the frozen name from the registry instead of building one
/// by hand. A publication failure is deliberately not a status failure: the observation has already
/// happened and is what the caller returns, so a listener that could not be reached does not make
/// the status less true.
impl StatusPublisher for AppHandle {
    fn publish(&self, status: &ServiceStatus) {
        let _published = tauri_specta::Event::emit(status, self);
    }
}

/// The last observed status this process holds (the `get_service_status` behaviour).
///
/// Before any fresh outcome it is `state: unknown` with every other field absent — the tracker's
/// own never-observed value, which is never a defaulted `observed_at` or `account_valid` (R3). This
/// call reads no clock and makes no network request, so it cannot change what it reports.
pub fn current_status(state: &AppState) -> ServiceStatus {
    state.status().status()
}

/// Re-evaluate reachability: one fresh probe, recorded, published and returned (NFR-1, D0.13).
///
/// The observation instant is taken here, from this device's clock, after the probe returned; the
/// tracker decides whether that outcome replaces the status or counts towards the offline threshold.
/// See the module documentation for the sequence and for where the bounds are applied.
pub async fn re_evaluate(state: &AppState, publisher: &impl StatusPublisher) -> ServiceStatus {
    observe_fresh(state, publisher).await.1
}

/// One fresh probe, recorded and published, together with the outcome it observed.
///
/// [`re_evaluate`] returns only the recorded status, but the trigger path also needs the outcome:
/// the status can be a retained one, so it cannot say by itself whether this probe answered (see
/// [`publish_active_refresh`]).
async fn observe_fresh(
    state: &AppState,
    publisher: &impl StatusPublisher,
) -> (ProbeOutcome, ServiceStatus) {
    let outcome = probe(state.service()).await;
    let observed_at = AbsoluteInstant::now().to_wire();

    // The guard is released before publication: `observe` is synchronous, so no lock is held across
    // an await, and nothing that a publisher does can run while the tracker is borrowed.
    let status = {
        let mut tracker: std::sync::MutexGuard<'_, StatusTracker> = state.status();

        tracker.observe(outcome, observed_at)
    };

    publisher.publish(&status);

    (outcome, status)
}

/// Handle one run-loop event, re-evaluating when it is this platform's lifecycle trigger.
///
/// The spawn keeps the event-loop callback short: the probe is network I/O, and the callback runs on
/// the main thread. The task reads the one managed [`AppState`] from the handle, so it observes the
/// same tracker and transport the commands use.
///
/// Triggers are coalesced: a trigger that arrives while a trigger-driven re-evaluation is running
/// does not start a second task. It sets the rerun flag, and the running task performs exactly one
/// follow-up (see the module documentation). If the state is not installed yet (an event that
/// arrives before `setup`), nothing is probed, and the gate is released so a later trigger starts
/// cleanly.
pub fn handle_run_event(app: &AppHandle, event: &RunEvent) {
    if !is_re_evaluation_trigger(event) {
        return;
    }

    let coalescer = trigger_coalescer();
    if !coalescer.begin() {
        // One trigger-driven re-evaluation is already running; it will follow up once.
        return;
    }

    let app = app.clone();

    let _task = tauri::async_runtime::spawn(async move {
        let Some(state) = app.try_state::<AppState>() else {
            loop {
                if !coalescer.finish() {
                    break;
                }
            }

            return;
        };

        coalescer.run(&state, &app, &app).await;
    });
}

/// The process-wide coalescer for trigger-driven re-evaluations.
///
/// One process runs one app and one tracker, so the gate is process-wide for the same reason the
/// tracker is: two triggers must not race each other into recording observations out of order.
fn trigger_coalescer() -> &'static Coalescer {
    static COALESCER: OnceLock<Coalescer> = OnceLock::new();

    COALESCER.get_or_init(Coalescer::new)
}

/// One trigger-driven re-evaluation in flight, and whether another trigger arrived while it ran.
#[derive(Debug, Default)]
struct Coalescer {
    /// The in-flight/rerun state, guarded so the event-loop thread and the probe task agree.
    state: Mutex<CoalesceState>,
}

/// What [`Coalescer`] guards.
#[derive(Debug, Default)]
struct CoalesceState {
    /// Whether a trigger-driven re-evaluation is running.
    in_flight: bool,
    /// Whether a trigger arrived while the running one was in flight.
    rerun: bool,
}

impl Coalescer {
    /// A coalescer with nothing in flight.
    fn new() -> Self {
        Self::default()
    }

    /// Ask to start a trigger-driven re-evaluation.
    ///
    /// `true` means the caller now owns the run and must call [`Coalescer::finish`] when it ends;
    /// `false` means one was already in flight and this trigger has been folded into its one
    /// follow-up.
    fn begin(&self) -> bool {
        let mut state = lock(&self.state);

        if state.in_flight {
            state.rerun = true;

            false
        } else {
            state.in_flight = true;

            true
        }
    }

    /// Finish the running re-evaluation and report whether it must run exactly one follow-up.
    ///
    /// A `true` answer keeps the run in flight: the caller loops and owns the follow-up. A `false`
    /// answer releases it.
    fn finish(&self) -> bool {
        let mut state = lock(&self.state);

        if state.rerun {
            state.rerun = false;

            true
        } else {
            state.in_flight = false;

            false
        }
    }

    /// Run trigger-driven re-evaluations until no trigger arrived while one was running.
    ///
    /// Each pass is the same fresh probe, observation and status publication the command and the
    /// first trigger use: the probe is outside the tracker lock, and publication follows the
    /// observation through [`observe_fresh`]. After a pass whose **own probe answered** with a usable
    /// account, the trigger additionally publishes `household:changed` with kind `refreshed` for the
    /// active household, which is how a start or foreground return reloads the copy through the
    /// frontend's existing policy (D0.2, D0.23). That publication lives here, not in
    /// [`re_evaluate`] (which
    /// [`check_service_reachability`](crate::ipc::commands::sync::check_service_reachability) also
    /// calls) and never in `load_household`: the frontend invalidates the collection on `refreshed`,
    /// so an emit from the load would refetch in a loop.
    async fn run<S: StatusPublisher, H: HouseholdChangePublisher>(
        &self,
        state: &AppState,
        statuses: &S,
        households: &H,
    ) {
        loop {
            let (outcome, status) = observe_fresh(state, statuses).await;
            publish_active_refresh(state, outcome, &status, households);

            if !self.finish() {
                break;
            }
        }
    }
}

/// Report the active household's collection as stale after a trigger's fresh probe found the
/// service usable.
///
/// This is the **trigger path's** extra step, after [`observe_fresh`] recorded and published the
/// status. It is deliberately not inside [`re_evaluate`]: the explicit `check_service_reachability`
/// command calls that function too, and asking for a status must not invalidate household data. It
/// is also never part of `load_household`/[`ReadPath::load`](crate::sync::ReadPath::load): that path
/// does not even receive a publisher, and the frontend's `refreshed` policy invalidates the
/// collection, whose query calls `load_household`, so an emit from the load would refetch in a loop.
///
/// Nothing is published when the probe **got no answer**, when the outcome is not a usable service
/// (`reachable` **and** a valid account), or when this process holds no active household. The fresh
/// outcome matters as well as the status: below the offline threshold the tracker retains the
/// previous status on an unanswered probe, and a retained `reachable` does not prove that this
/// trigger reached anything — so connectivity is gated on the probe having answered, not on what the
/// tracker left behind.
fn publish_active_refresh(
    state: &AppState,
    outcome: ProbeOutcome,
    status: &ServiceStatus,
    publisher: &impl HouseholdChangePublisher,
) {
    if outcome.is_failed_probe() {
        return;
    }

    if status.state != ServiceState::Reachable || status.account_valid != Some(true) {
        return;
    }

    // Clone the id and drop the book's guard before publishing, so no lock is held while an event
    // listener runs.
    let active = {
        let households = state.households();

        households.active_id().cloned()
    };

    if let Some(household_id) = active {
        publisher.publish(&HouseholdChanged {
            household_id,
            kind: HouseholdChangeKind::Refreshed,
        });
    }
}

/// Lock a mutex, recovering a poisoned one rather than cascading the panic.
///
/// The critical section only moves two booleans and reads them back, so it cannot leave the
/// protected value inconsistent; the recovery mirrors `state::AppState`'s own locking rule.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Whether this run-loop event is the platform's re-evaluation trigger (NFR-1).
///
/// iOS: the window-level [`tauri::WindowEvent::Resumed`], which tao 0.37.1's iOS backend sends for
/// `applicationWillEnterForeground` / `sceneWillEnterForeground` — the proven foreground signal.
/// The app-level `RunEvent::Resumed` is **not** accepted: on the pinned tauri/tauri-runtime-wry it
/// comes only from wry's `StartCause::Poll`, not from the platform's foreground callback.
///
/// macOS: a window that regains focus while the product keeps running, which is
/// `WindowEvent::Focused(true)`; there is no process suspend/resume to hook on the desktop, and a
/// focus change is what "the person came back" is observable as. Connectivity returning while the
/// product stays focused is not observed (see the module documentation); the next focus change or an
/// explicit `check_service_reachability` is when it is noticed.
///
/// Any other platform, and any other event, is not a trigger.
fn is_re_evaluation_trigger(event: &RunEvent) -> bool {
    #[cfg(target_os = "ios")]
    {
        matches!(event, RunEvent::WindowEvent { event, .. } if window_resumed(event))
    }

    #[cfg(target_os = "macos")]
    {
        matches!(
            event,
            RunEvent::WindowEvent { event, .. } if window_focus_regained(event)
        )
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        let _ = event;

        false
    }
}

/// Whether `event` is a window resuming, the iOS foreground form tao emits.
#[cfg(target_os = "ios")]
fn window_resumed(event: &WindowEvent) -> bool {
    matches!(event, WindowEvent::Resumed)
}

/// Whether `event` is a window gaining focus.
///
/// Extracted from [`is_re_evaluation_trigger`] because a `RunEvent::WindowEvent` cannot be
/// constructed outside Tauri (its variant is `#[non_exhaustive]`), while this half can be tested
/// directly on the event the trigger is about.
#[cfg(target_os = "macos")]
fn window_focus_regained(event: &WindowEvent) -> bool {
    matches!(event, WindowEvent::Focused(true))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::ipc::events::test_support::RecordingHouseholdPublisher;
    use crate::ipc::events::{HouseholdChangeKind, HouseholdChanged};
    use crate::ipc::types::{AccountUserId, HouseholdId, RequestId, SaveCause, ServiceState};
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::double::{RecordedCall, ScriptedService};
    use crate::sync::probe::AccountOutcome;
    use crate::sync::{Answer, REACHABILITY_PROBE_TIMEOUT, ServiceReply};

    /// The publication seam's test side: it records what the app would have emitted.
    #[derive(Debug, Default)]
    struct RecordingPublisher {
        published: Mutex<Vec<ServiceStatus>>,
    }

    impl RecordingPublisher {
        fn published(&self) -> Vec<ServiceStatus> {
            self.published
                .lock()
                .expect("the records are readable")
                .clone()
        }
    }

    impl StatusPublisher for RecordingPublisher {
        fn publish(&self, status: &ServiceStatus) {
            self.published
                .lock()
                .expect("the records are writable")
                .push(status.clone());
        }
    }

    /// A transcript of one re-evaluation, with the app state it observed.
    fn evaluate(service: Arc<ScriptedService>) -> (AppState, RecordingPublisher, ServiceStatus) {
        let state = AppState::new(service, None);
        let publisher = RecordingPublisher::default();
        let status = block_on_ready(re_evaluate(&state, &publisher));

        (state, publisher, status)
    }

    fn account() -> AccountUserId {
        AccountUserId("account-1".to_owned())
    }

    fn request_id(value: &str) -> RequestId {
        RequestId(value.to_owned())
    }

    /// One accepted create answer the domain can read.
    fn accepted_create(household_id: &str) -> ServiceReply {
        ServiceReply::Answered(Answer::with_body(
            200,
            serde_json::json!({
                "outcome": "applied",
                "household": {
                    "id": household_id,
                    "owner_member_id": "member-1",
                    "created_at": "2026-10-19T10:00:00Z",
                },
            }),
        ))
    }

    #[test]
    fn the_status_is_unknown_before_any_fresh_outcome() {
        let state = AppState::new(Arc::new(ScriptedService::new()), None);

        let status = current_status(&state);

        assert_eq!(status.state, ServiceState::Unknown);
        assert_eq!(status.account_valid, None);
        assert_eq!(status.cause, None);
        assert_eq!(status.observed_at, None, "no instant is defaulted");
    }

    #[test]
    fn a_probe_that_gets_no_answer_counts_towards_the_threshold() {
        let service = Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
        let (state, _publisher, status) = evaluate(service);

        // The first failure is not yet an outage, and it carries no fabricated observation time.
        assert_eq!(status.state, ServiceState::Unknown);
        assert_eq!(status.observed_at, None);
        assert_eq!(state.status().consecutive_failures(), 1);
    }

    #[test]
    fn three_failures_without_a_success_report_unreachable() {
        let service = Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
        let state = AppState::new(service, None);
        let publisher = RecordingPublisher::default();

        for failures in 1..=3 {
            let status = block_on_ready(re_evaluate(&state, &publisher));

            assert_eq!(state.status().consecutive_failures(), failures);

            if failures < super::super::OFFLINE_FAILURE_THRESHOLD {
                assert_eq!(status.state, ServiceState::Unknown);
            } else {
                assert_eq!(status.state, ServiceState::Unreachable);
                assert_eq!(status.account_valid, None);
                assert_eq!(status.cause, Some(SaveCause::Connectivity));
                assert!(status.observed_at.is_some(), "the threshold probe is timed");
            }
        }
    }

    #[test]
    fn an_answered_probe_resets_the_count_even_when_the_answer_is_401_or_429() {
        // Two unanswered probes, then the service answers `401`, then `429`. Each answer proves the
        // service is reachable, so each resets the run rather than extending it (D0.13).
        let service = Arc::new(
            ScriptedService::new()
                .on_service_check([
                    ServiceReply::NoResponse,
                    ServiceReply::NoResponse,
                    ServiceReply::Answered(Answer::status_only(200)),
                    ServiceReply::Answered(Answer::status_only(200)),
                ])
                .on_account_check([
                    ServiceReply::Answered(Answer::status_only(401)),
                    ServiceReply::Answered(Answer::status_only(429)),
                ]),
        );
        let state = AppState::new(service, None);
        let publisher = RecordingPublisher::default();

        block_on_ready(re_evaluate(&state, &publisher));
        block_on_ready(re_evaluate(&state, &publisher));
        assert_eq!(state.status().consecutive_failures(), 2);

        let unauthorised = block_on_ready(re_evaluate(&state, &publisher));
        assert_eq!(unauthorised.state, ServiceState::Reachable);
        assert_eq!(unauthorised.account_valid, Some(false));
        assert_eq!(unauthorised.cause, Some(SaveCause::NoAccount));
        assert_eq!(state.status().consecutive_failures(), 0);

        let limited = block_on_ready(re_evaluate(&state, &publisher));
        assert_eq!(limited.state, ServiceState::Reachable);
        assert_eq!(limited.cause, Some(SaveCause::RateLimited));
        assert_eq!(state.status().consecutive_failures(), 0);
    }

    #[test]
    fn a_re_evaluation_publishes_the_fresh_status_it_returns() {
        let (state, publisher, status) = evaluate(Arc::new(ScriptedService::new()));

        assert_eq!(status.state, ServiceState::Reachable);
        assert_eq!(
            publisher.published(),
            vec![status.clone()],
            "the listener sees exactly the value the caller receives"
        );
        assert_eq!(current_status(&state), status, "and the tracker holds it");
    }

    #[test]
    fn a_re_evaluation_probes_with_the_reachability_bound() {
        let recording = Arc::new(ScriptedService::new());
        let state = AppState::new(recording.clone(), None);
        let publisher = RecordingPublisher::default();

        block_on_ready(re_evaluate(&state, &publisher));

        assert_eq!(
            recording.calls(),
            vec![
                RecordedCall::CheckService {
                    bound: REACHABILITY_PROBE_TIMEOUT,
                },
                RecordedCall::CheckAccount {
                    bound: REACHABILITY_PROBE_TIMEOUT,
                },
            ]
        );
    }

    /// A state holding one created household as its active one, over a transport the test scripts.
    fn state_with_active_household(service: Arc<ScriptedService>) -> (AppState, HouseholdId) {
        let state = AppState::new(service, Some(account()));
        let created =
            block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
                .expect("the double accepted the create");

        (state, created.id)
    }

    #[test]
    fn a_trigger_with_a_reachable_service_and_an_active_household_publishes_one_refreshed() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_create("household-1")]));
        let (state, active) = state_with_active_household(service);

        let coalescer = Coalescer::new();
        let statuses = RecordingPublisher::default();
        let households = RecordingHouseholdPublisher::default();

        assert!(coalescer.begin(), "the trigger starts the run");
        block_on_ready(coalescer.run(&state, &statuses, &households));

        assert_eq!(
            statuses.published(),
            vec![current_status(&state)],
            "the status is still published once per pass"
        );
        assert_eq!(
            households.published(),
            vec![HouseholdChanged {
                household_id: active,
                kind: HouseholdChangeKind::Refreshed,
            }],
            "exactly one refresh for the active household"
        );
    }

    #[test]
    fn a_trigger_with_no_active_household_publishes_no_household_change() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service, None);
        let coalescer = Coalescer::new();
        let statuses = RecordingPublisher::default();
        let households = RecordingHouseholdPublisher::default();

        assert!(coalescer.begin());
        block_on_ready(coalescer.run(&state, &statuses, &households));
        let status = current_status(&state);

        // The status is usable; there is simply no household whose copy could be refreshed.
        assert_eq!(status.state, ServiceState::Reachable);
        assert_eq!(status.account_valid, Some(true));
        assert!(households.published().is_empty());
    }

    #[test]
    fn a_trigger_with_an_invalid_account_publishes_no_household_change() {
        // The create's probe is answered with a valid account; the trigger's probe is answered `401`.
        let service = Arc::new(
            ScriptedService::new()
                .on_write([accepted_create("household-1")])
                .on_account_check([
                    ServiceReply::Answered(Answer::status_only(200)),
                    ServiceReply::Answered(Answer::status_only(401)),
                ]),
        );
        let (state, _active) = state_with_active_household(service);

        let coalescer = Coalescer::new();
        let statuses = RecordingPublisher::default();
        let households = RecordingHouseholdPublisher::default();

        assert!(coalescer.begin());
        block_on_ready(coalescer.run(&state, &statuses, &households));
        let status = current_status(&state);

        assert_eq!(status.state, ServiceState::Reachable);
        assert_eq!(status.account_valid, Some(false));
        assert!(
            households.published().is_empty(),
            "an unusable account cannot refresh a copy"
        );
    }

    /// A trigger probe that got no answer must not refresh on a status it did not observe.
    ///
    /// Below the offline threshold the tracker deliberately retains the previous status, so the
    /// status can still say `reachable` with a valid account even though this trigger reached
    /// nothing. The refresh is gated on the probe's own answered outcome, so nothing is published
    /// for the household; the retained status is still published as the status fact.
    #[test]
    fn a_trigger_whose_probe_gets_no_answer_publishes_no_refresh_for_a_retained_status() {
        // The create's probe is answered; the trigger's probe then gets no answer.
        let service = Arc::new(
            ScriptedService::new()
                .on_write([accepted_create("household-1")])
                .on_service_check([
                    ServiceReply::Answered(Answer::status_only(200)),
                    ServiceReply::NoResponse,
                ]),
        );
        let (state, active) = state_with_active_household(service);

        // A previous fresh probe left the status `reachable` with a valid account; the trigger's
        // unanswered probe is below the threshold, so the tracker retains exactly that.
        {
            let mut tracker = state.status();

            tracker.observe(
                ProbeOutcome::Answered(AccountOutcome::Identified),
                AbsoluteInstant::now().to_wire(),
            );
        }

        let coalescer = Coalescer::new();
        let statuses = RecordingPublisher::default();
        let households = RecordingHouseholdPublisher::default();

        assert!(coalescer.begin());
        block_on_ready(coalescer.run(&state, &statuses, &households));

        let retained = current_status(&state);
        assert_eq!(retained.state, ServiceState::Reachable);
        assert_eq!(
            retained.account_valid,
            Some(true),
            "the retained status is the one that would have allowed a refresh"
        );
        assert_eq!(state.status().consecutive_failures(), 1);
        assert_eq!(state.households().active_id(), Some(&active));

        assert!(
            households.published().is_empty(),
            "an unanswered probe proves no connectivity, so the copy is not refreshed"
        );
        assert_eq!(
            statuses.published().last(),
            Some(&retained),
            "the retained status is still published as the status fact"
        );
    }

    /// A publisher that also reports triggers arriving while its re-evaluation is still in flight.
    ///
    /// `finish` has not run when the publisher is called, so a `begin` here is a trigger arriving
    /// during the run rather than after it.
    struct TriggeringPublisher<'a> {
        coalescer: &'a Coalescer,
        published: Mutex<Vec<ServiceStatus>>,
        triggered: AtomicBool,
    }

    impl TriggeringPublisher<'_> {
        fn new(coalescer: &Coalescer) -> TriggeringPublisher<'_> {
            TriggeringPublisher {
                coalescer,
                published: Mutex::new(Vec::new()),
                triggered: AtomicBool::new(false),
            }
        }
    }

    impl StatusPublisher for TriggeringPublisher<'_> {
        fn publish(&self, status: &ServiceStatus) {
            self.published
                .lock()
                .expect("the records are writable")
                .push(status.clone());

            if !self.triggered.swap(true, Ordering::SeqCst) {
                // Two triggers arrive during this run. Both fold into a single follow-up.
                assert!(!self.coalescer.begin());
                assert!(!self.coalescer.begin());
            }
        }
    }

    #[test]
    fn many_triggers_during_one_run_collapse_into_one_follow_up() {
        let coalescer = Coalescer::new();

        assert!(coalescer.begin(), "the first trigger starts the run");
        assert!(!coalescer.begin());
        assert!(!coalescer.begin());
        assert!(coalescer.finish(), "one follow-up is owed");
        assert!(!coalescer.finish(), "and only one");
    }

    #[test]
    fn a_trigger_during_an_in_flight_re_evaluation_runs_exactly_one_follow_up() {
        let coalescer = Coalescer::new();
        // The first probe gets no answer; the follow-up's probe succeeds, so where the tracker ends
        // shows which probe recorded last.
        let service = Arc::new(ScriptedService::new().on_service_check([
            ServiceReply::NoResponse,
            ServiceReply::Answered(Answer::status_only(200)),
        ]));
        let state = AppState::new(service.clone(), None);
        let publisher = TriggeringPublisher::new(&coalescer);

        assert!(coalescer.begin(), "the first trigger starts the run");
        block_on_ready(coalescer.run(&state, &publisher, &RecordingHouseholdPublisher::default()));

        let probes = service
            .calls()
            .into_iter()
            .filter(|call| matches!(call, RecordedCall::CheckService { .. }))
            .count();
        assert_eq!(probes, 2, "the original probe and exactly one follow-up");

        // Completion order is start order because only one trigger-driven probe runs at a time: the
        // failed first probe cannot land after the successful follow-up.
        assert_eq!(state.status().consecutive_failures(), 0);
        let final_status = current_status(&state);
        assert_eq!(final_status.state, ServiceState::Reachable);

        let published = publisher
            .published
            .lock()
            .expect("the records are readable");
        assert_eq!(published.len(), 2);
        assert_eq!(
            published.last(),
            Some(&final_status),
            "the tracker ends on the last probe's result, which is also the last publication"
        );
    }

    #[cfg(target_os = "ios")]
    #[test]
    fn a_foreground_return_is_the_ios_trigger() {
        // The window-level event is the proven foreground signal; the app-level event-loop wake is
        // deliberately not a trigger on this pinned stack.
        assert!(window_resumed(&WindowEvent::Resumed));
        assert!(!window_resumed(&WindowEvent::Suspended));
        assert!(
            !is_re_evaluation_trigger(&RunEvent::Resumed),
            "StartCause::Poll is not the platform's foreground callback"
        );
        assert!(!is_re_evaluation_trigger(&RunEvent::Ready));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_window_gaining_focus_is_the_macos_trigger() {
        assert!(window_focus_regained(&WindowEvent::Focused(true)));
        assert!(!window_focus_regained(&WindowEvent::Focused(false)));
    }
}
