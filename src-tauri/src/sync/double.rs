//! The scripted double: the second implementation of [`SharedService`] the seam exists for.
//!
//! P0 has no live deployment and the gate never boots the shared service, so the write path's
//! behaviour can only be tested against a transport whose every answer is chosen by the test. This
//! module is that transport. It is a **published** double rather than a `#[cfg(test)]` one because
//! it is the seam's other side: the later tasks that build on the gate (2.5's read path, 2.13's
//! status commands) and the section-5 tests reach for it the same way this task's unit tests do.
//! `test_support::block_on_ready` is published for the same reason — a test in `src-tauri/tests/`
//! drives the double with no async runtime and no new dependency, exactly as a unit test here does.
//!
//! Three properties make it usable as evidence rather than as decoration:
//!
//! 1. **Every answer is scripted, and the script is consumed in order.** `replies` given to
//!    `on_service_check`/`on_account_check`/`on_write` are handed out one per call, and the last one
//!    is repeated for every further call, so "the first three probes fail and everything after them
//!    succeeds" is one line. An unscripted call gets the healthy default: the service answers `200`,
//!    the account is valid, and a write comes back with the completed-write envelope.
//! 2. **Every call is recorded, at call time.** A call the code under test never makes is absent
//!    from [`ScriptedService::calls`], which is exactly how a test proves that a refused write was
//!    never *sent* — the observable half of "nothing is written as if it were saved" (D0.1, SYNC-2).
//! 3. **Nothing in it awaits anything.** Each call hands back a future that is ready on its first
//!    poll, so the tests need no async runtime and therefore no runtime dependency the manifest does
//!    not already pin (see `test_support::block_on_ready`). The one thing the double therefore cannot
//!    show is a genuinely stalled call: it reports `ServiceReply::NoOutcome` instead, which is the
//!    transport condition a bound having passed produces (D0.13).

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde_json::Value;

use super::service::{
    Answer, OUTCOME_APPLIED, SaveRequest, ServiceFuture, ServiceReply, SharedService,
};

/// One call the double answered, in the order it answered them.
///
/// The bound is recorded with the call because the bound is the caller's, not the transport's: a
/// test asserts that the probe used [`super::REACHABILITY_PROBE_TIMEOUT`] and that a save attempt
/// used [`super::SAVE_ATTEMPT_TIMEOUT`] (D0.13) by reading it back here.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordedCall {
    /// The service check, with the bound it was given.
    CheckService {
        /// The limit the caller put on the call.
        bound: Duration,
    },
    /// The account check, with the bound it was given.
    CheckAccount {
        /// The limit the caller put on the call.
        bound: Duration,
    },
    /// A write, with the bound it was given and the write it was asked to submit.
    Save {
        /// The limit the caller put on the attempt.
        bound: Duration,
        /// The write as the caller described it — the operation identity included, so a test can
        /// assert the identity a failure carries is the one the command was given (D0.14).
        request: SaveRequest,
    },
}

/// The double's script: the replies still to hand out, and the calls already answered.
#[derive(Debug)]
struct Script {
    service: VecDeque<ServiceReply>,
    account: VecDeque<ServiceReply>,
    writes: VecDeque<ServiceReply>,
    calls: Vec<RecordedCall>,
}

/// A shared service whose every answer a test scripts (see the module documentation).
#[derive(Debug)]
pub struct ScriptedService {
    script: Mutex<Script>,
}

impl ScriptedService {
    /// A service that answers as a healthy one: the service check answers `200`, the account check
    /// answers `200`, and a write answers `200` with the completed-write envelope.
    ///
    /// The healthy default is deliberate: a test states the one thing it is about with a builder
    /// below and leaves the rest of the transport alone, so a test that means to refuse a write
    /// cannot pass by accident because it also left the account check unscripted.
    pub fn new() -> Self {
        Self {
            script: Mutex::new(Script {
                service: VecDeque::new(),
                account: VecDeque::new(),
                writes: VecDeque::new(),
                calls: Vec::new(),
            }),
        }
    }

    /// Script the service check's replies: handed out one per call, the last one repeated.
    pub fn on_service_check(mut self, replies: impl IntoIterator<Item = ServiceReply>) -> Self {
        self.script_mut().service = replies.into_iter().collect();
        self
    }

    /// Script the account check's replies: handed out one per call, the last one repeated.
    pub fn on_account_check(mut self, replies: impl IntoIterator<Item = ServiceReply>) -> Self {
        self.script_mut().account = replies.into_iter().collect();
        self
    }

    /// Script the writes' replies: handed out one per call, the last one repeated.
    pub fn on_write(mut self, replies: impl IntoIterator<Item = ServiceReply>) -> Self {
        self.script_mut().writes = replies.into_iter().collect();
        self
    }

    /// Every call this double has answered, in order.
    pub fn calls(&self) -> Vec<RecordedCall> {
        self.script().calls.clone()
    }

    /// How many writes this double has been asked to submit.
    ///
    /// It is the direct observable of "a write is attempted only while the service is reachable and
    /// the account is valid" (SYNC-2): a refused attempt leaves this at zero, because the write was
    /// never described to the transport at all.
    pub fn write_count(&self) -> usize {
        self.script()
            .calls
            .iter()
            .filter(|call| matches!(call, RecordedCall::Save { .. }))
            .count()
    }

    /// The script, for a builder to change.
    ///
    /// A poisoned lock is recovered rather than propagated: this double holds no state a panic can
    /// leave inconsistent (its methods only move a queued reply and append a record), so a test that
    /// panicked elsewhere should not cascade into a second panic here.
    fn script_mut(&mut self) -> &mut Script {
        match self.script.get_mut() {
            Ok(script) => script,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// The script, for a call to read and consume.
    fn script(&self) -> MutexGuard<'_, Script> {
        match self.script.lock() {
            Ok(script) => script,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl Default for ScriptedService {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedService for ScriptedService {
    /// Answer the service check from the script, recording the call.
    ///
    /// The record is written when the call is **made**, not when its future is polled: the double's
    /// futures are ready immediately, so the two cannot diverge, and a call the code under test does
    /// not make never appears in [`ScriptedService::calls`].
    fn check_service(&self, bound: Duration) -> ServiceFuture<'_, ServiceReply> {
        let reply = {
            let mut script = self.script();

            script.calls.push(RecordedCall::CheckService { bound });
            next(&mut script.service, healthy_reply)
        };

        Box::pin(async move { reply })
    }

    /// Answer the account check from the script, recording the call.
    fn check_account(&self, bound: Duration) -> ServiceFuture<'_, ServiceReply> {
        let reply = {
            let mut script = self.script();

            script.calls.push(RecordedCall::CheckAccount { bound });
            next(&mut script.account, healthy_reply)
        };

        Box::pin(async move { reply })
    }

    /// Answer the write from the script, recording the call and the write it was given.
    fn save<'a>(
        &'a self,
        request: &'a SaveRequest,
        bound: Duration,
    ) -> ServiceFuture<'a, ServiceReply> {
        let reply = {
            let mut script = self.script();

            script.calls.push(RecordedCall::Save {
                bound,
                request: request.clone(),
            });
            next(&mut script.writes, completed_write_reply)
        };

        Box::pin(async move { reply })
    }
}

/// The next scripted reply: the queue is consumed from the front, and its last reply stands for
/// every call after it, so a one-element script applies to every call.
fn next(queue: &mut VecDeque<ServiceReply>, fallback: fn() -> ServiceReply) -> ServiceReply {
    let reply = if queue.len() > 1 {
        queue.pop_front()
    } else {
        queue.front().cloned()
    };

    match reply {
        Some(reply) => reply,
        None => fallback(),
    }
}

/// The reply of a call the service answered successfully, and the double's default.
fn healthy_reply() -> ServiceReply {
    ServiceReply::Answered(Answer::status_only(200))
}

/// The reply of a write the service completed, and the double's default.
fn completed_write_reply() -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(200, envelope(OUTCOME_APPLIED)))
}

/// One write envelope carrying `outcome`, as the service's own report carries it.
///
/// The envelope is built field by field rather than by a JSON macro so that the double's default is
/// a value the tests' `serde_json` pin already covers, with nothing to parse.
fn envelope(outcome: &str) -> Value {
    let mut envelope = serde_json::Map::new();
    envelope.insert("outcome".to_owned(), Value::String(outcome.to_owned()));

    Value::Object(envelope)
}

/// A minimal executor for the futures this double hands out, and only for those.
///
/// The double's calls never await anything, so their futures are ready on the first poll and a
/// `Pending` result is a bug in the test rather than something to wait for. Making that explicit
/// keeps the gate free of a runtime dependency it would otherwise have to add just to test its own
/// transport (`rust-conventions`: a testable boundary that needs no app), and it keeps a hang
/// impossible: the panic says what went wrong instead of blocking a gate run.
///
/// **It is a test seam, not product API**, which is why it is hidden from the documentation but
/// **published rather than `#[cfg(test)]`**: a test in `src-tauri/tests/` compiles the library
/// without `cfg(test)`, so a unit-test-only helper would leave section 5's write-path tests — and
/// any later task's — unable to poll the double at all. Nothing in the app calls it; it exists so
/// that the tests which *must* exercise the seam can reach it.
#[doc(hidden)]
pub mod test_support {
    use std::future::Future;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    /// Poll `future` once and return its output, panicking if it is not ready.
    ///
    /// It is deliberately not a general executor: it never yields, so it cannot drive a future that
    /// really waits, and one that needs a genuine wait is reporting that it is not the double's.
    pub fn block_on_ready<F: Future>(future: F) -> F::Output {
        let mut future = Box::pin(future);
        let waker = Waker::from(Arc::new(DoNothing));
        let mut context = Context::from_waker(&waker);

        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("a scripted call is ready on its first poll"),
        }
    }

    /// The waker of [`block_on_ready`]: nothing is ever scheduled, so nothing has to be woken.
    struct DoNothing;

    impl Wake for DoNothing {
        fn wake(self: Arc<Self>) {}
    }
}
