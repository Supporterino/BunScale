//! The real shared-service client: HTTPS from Rust with the pinned `reqwest` and rustls (D0.7,
//! D0.8).
//!
//! # What it is, and what it is not
//!
//! It is a **thin adapter** over [`SharedService`]: it sends what a call describes and reports the
//! HTTP status and body that came back, and it classifies nothing. That split is what keeps the
//! mapping from a status or an envelope to a `SaveCause` testable, because this type is never
//! exercised by the gate: P0 has no live deployment and nothing here boots the service, so a
//! classification written into this adapter would ship untested. The classification lives in
//! [`super::probe`] and [`super::gate`], where the scripted double drives it.
//!
//! **It is not constructed anywhere the tests run.** No `lib.rs`, command or test builds an
//! `HttpService`: the app mounts it when a deployment exists (the live-deployment task D0.7 records
//! as a precondition of the first real write), and the gate never does. The type exists now so that
//! the transport's shape is fixed — and compiled for `aarch64-apple-ios` and macOS — before the
//! deployment arrives.
//!
//! # The deployment's shape it assumes
//!
//! The paths below are the Supabase stack's (D0.7): a health endpoint for the service check, the
//! account service's user endpoint for the account check, and a PostgREST function for each service
//! call (the collection read and the write). They are constants in one place because they are
//! **deployment configuration**, not mechanism: the deployment task that first writes through these
//! functions is the one that verifies the routes, and it changes a path here rather than hunting
//! literals.
//!
//! Every request carries the person's JWT and nothing else — no service-role key, no API key the app
//! holds (D0.7 criterion 1). The token arrives through [`AccessTokenSource`] rather than being stored
//! here, because the credential's storage is the auth task's (the platform Keychain, D0.7 criterion
//! 4) and this boundary must not become a second place a credential lives.
//!
//! # The bound
//!
//! Each call's limit is handed to `reqwest`'s per-request timeout (its `RequestBuilder::timeout`),
//! so [`super::SAVE_ATTEMPT_TIMEOUT`], [`super::READ_REFRESH_TIMEOUT`] and
//! [`super::REACHABILITY_PROBE_TIMEOUT`] are enforced by the transport that is actually waiting, and
//! a call that passes its bound reports [`ServiceReply::NoOutcome`] — the `timeout` condition
//! (D0.13). The bound covers the **body read** as well as the response: a body that runs out of time
//! is that same condition, not an answer with an unreadable body, because a deadline is a condition
//! of the attempt rather than of its content. No timer is added to this crate for it.

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use super::service::{
    Answer, HTTP_UNAUTHORIZED, SaveRequest, ServiceFuture, ServiceReply, SharedService,
};
use crate::ipc::types::HouseholdId;

/// The access token the next call carries, or `None` while no account token is available.
///
/// A **supplier** rather than a token: the credential lives in the platform Keychain (D0.7 criterion
/// 4), the auth task owns its storage and refresh, and asking per call is what lets a refreshed token
/// take effect without rebuilding the client. `Send + Sync` because one client serves every task.
pub type AccessTokenSource = Arc<dyn Fn() -> Option<String> + Send + Sync>;

/// The path of the service's own health check (D0.7 criterion 4).
const HEALTH_PATH: &str = "/health";

/// The path of the account check (D0.7 criterion 4): the account service's view of the token.
const ACCOUNT_PATH: &str = "/auth/v1/user";

/// The path prefix of a service function (D0.7 criteria 1, 3 and 5): a function called with the
/// person's JWT.
const RPC_PREFIX: &str = "/rest/v1/rpc/";

/// The PostgREST function that reads a household's collection and its changes (ADR 0002).
const HOUSEHOLD_CHANGES_RPC: &str = "household_changes";

/// The real transport: the pinned `reqwest` client, talking to one deployment over HTTPS (D0.7).
///
/// It holds the HTTP client, the deployment's base URL and the token supplier — nothing else, and in
/// particular no cache, no queue and no status (D0.1, D0.2).
pub struct HttpService {
    client: reqwest::Client,
    base_url: String,
    access_token: AccessTokenSource,
}

impl HttpService {
    /// Build the client for `base_url`, authenticating with the token `access_token` supplies.
    ///
    /// # Errors
    ///
    /// `reqwest::Error` when the client cannot be built at all — a TLS backend that fails to
    /// initialise is the case — which no later call could recover from, so the caller decides rather
    /// than a constructor panicking on it. It is **not** a product failure and never crosses the IPC
    /// boundary: the one failure surface a command reports through stays
    /// [`CommandError`](crate::ipc::types::CommandError) (D0.6).
    pub fn new(
        base_url: impl Into<String>,
        access_token: AccessTokenSource,
    ) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder().build()?;

        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            access_token,
        })
    }

    /// The absolute URL of `path` on this deployment.
    fn url(&self, path: &str) -> String {
        let mut url = String::with_capacity(self.base_url.len() + path.len());

        url.push_str(&self.base_url);
        url.push_str(path);

        url
    }

    /// The absolute URL of the service function `rpc`.
    fn rpc_url(&self, rpc: &str) -> String {
        let mut url = self.url(RPC_PREFIX);

        url.push_str(rpc);

        url
    }

    /// The account token the next call carries, when there is one.
    fn token(&self) -> Option<String> {
        (self.access_token)()
    }

    /// Send one prepared request and report what the transport produced (D0.7 criterion 4).
    ///
    /// The bound is classified before anything is discarded, and that is the point of the three
    /// arms below: a call that ran out of time reports [`ServiceReply::NoOutcome`] whether the
    /// deadline fell on the response or on the **body read**, because `timeout` is the condition of
    /// the attempt (D0.13) and reporting a deadline as a `2xx` with no body would turn the one cause
    /// D0.14's ledger exists to make retryable into an unclassifiable failure. Any *other* failure to
    /// read the body is reported as **no body**: whether that is a success depends on the call, and
    /// the write path reads it as "not a completed write" (SYNC-2) rather than this adapter deciding
    /// on its behalf.
    async fn send(request: reqwest::RequestBuilder, read_body: bool) -> ServiceReply {
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => return transport_reply(&error),
        };

        let status = response.status().as_u16();

        if !read_body {
            return ServiceReply::Answered(Answer::status_only(status));
        }

        match response.json::<Value>().await {
            Ok(body) => ServiceReply::Answered(Answer::with_body(status, body)),
            Err(error) if error.is_timeout() => ServiceReply::NoOutcome,
            Err(_) => ServiceReply::Answered(Answer::status_only(status)),
        }
    }
}

impl SharedService for HttpService {
    fn check_service(&self, bound: Duration) -> ServiceFuture<'_, ServiceReply> {
        Box::pin(async move {
            let request = self.client.get(self.url(HEALTH_PATH)).timeout(bound);

            Self::send(request, false).await
        })
    }

    fn check_account(&self, bound: Duration) -> ServiceFuture<'_, ServiceReply> {
        Box::pin(async move {
            let Some(token) = self.token() else {
                // No token means no account: the condition `401` names is already true, and a request
                // without an identity would be refused by the service anyway (D0.7 criterion 1).
                return ServiceReply::Answered(Answer::status_only(HTTP_UNAUTHORIZED));
            };

            let request = self
                .client
                .get(self.url(ACCOUNT_PATH))
                .timeout(bound)
                .bearer_auth(token);

            Self::send(request, false).await
        })
    }

    fn read<'a>(
        &'a self,
        household_id: &'a HouseholdId,
        since_change_seq: u64,
        bound: Duration,
    ) -> ServiceFuture<'a, ServiceReply> {
        Box::pin(async move {
            let Some(token) = self.token() else {
                // A read needs an identity for the same reason a write does (D0.7 criterion 1); the
                // read path treats this exactly like any other unreadable answer and keeps the copy.
                return ServiceReply::Answered(Answer::status_only(HTTP_UNAUTHORIZED));
            };

            let body = serde_json::json!({
                "household_id": household_id.0.as_str(),
                "since_change_seq": since_change_seq,
            });
            let sent = self
                .client
                .post(self.rpc_url(HOUSEHOLD_CHANGES_RPC))
                .timeout(bound)
                .bearer_auth(token)
                .json(&body);

            Self::send(sent, true).await
        })
    }

    fn save<'a>(
        &'a self,
        request: &'a SaveRequest,
        bound: Duration,
    ) -> ServiceFuture<'a, ServiceReply> {
        Box::pin(async move {
            let Some(token) = self.token() else {
                // The write is never sent without an identity (D0.7 criterion 1): the condition is the
                // one `401` names, reported as itself rather than as the failure of a call never made.
                return ServiceReply::Answered(Answer::status_only(HTTP_UNAUTHORIZED));
            };

            let sent = self
                .client
                .post(self.rpc_url(request.rpc()))
                .timeout(bound)
                .bearer_auth(token)
                .json(request.body());

            Self::send(sent, true).await
        })
    }
}

/// The reply a transport error becomes (D0.7 criterion 4).
///
/// A timeout reports [`ServiceReply::NoOutcome`], because the caller's bound is what ended the call —
/// the `timeout` condition D0.13 makes a real cause with a number. Every other transport failure is
/// "no response at all", which is the condition `connectivity` names.
fn transport_reply(error: &reqwest::Error) -> ServiceReply {
    if error.is_timeout() {
        ServiceReply::NoOutcome
    } else {
        ServiceReply::NoResponse
    }
}
