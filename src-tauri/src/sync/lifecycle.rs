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
use super::probe::probe;
use crate::ipc::types::ServiceStatus;
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
    let outcome = probe(state.service()).await;
    let observed_at = AbsoluteInstant::now().to_wire();

    // The guard is released before publication: `observe` is synchronous, so no lock is held across
    // an await, and nothing that a publisher does can run while the tracker is borrowed.
    let status = {
        let mut tracker: std::sync::MutexGuard<'_, StatusTracker> = state.status();

        tracker.observe(outcome, observed_at)
    };

    publisher.publish(&status);

    status
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

        coalescer.run(&state, &app).await;
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
    /// Each pass is the same [`re_evaluate`] the command and the first trigger use: the probe is
    /// outside the tracker lock, and publication follows the observation inside `re_evaluate`.
    async fn run<P: StatusPublisher>(&self, state: &AppState, publisher: &P) {
        loop {
            let _status = re_evaluate(state, publisher).await;

            if !self.finish() {
                break;
            }
        }
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
    use crate::ipc::types::{SaveCause, ServiceState};
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::double::{RecordedCall, ScriptedService};
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
        block_on_ready(coalescer.run(&state, &publisher));

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
