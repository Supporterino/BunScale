# Tasks — add-household-sharing-hardening (P3)

One task = one commit = one owner. Format: `- [ ] <n>.<m> [agent] <imperative>`.
Order: contract and native surface → backend → frontend-logic → frontend-ui → tests → review.
Tick a task only after the `definition-of-done` gate passes for it.
Commit message: `feat(add-household-sharing-hardening): <task>`.
Platform scope is stated on every task that is not universal: iOS, macOS, or both.
UI tasks capture screenshots at 390×844 and 1280×800 into `.artifacts/screens/`.
Native platform behaviour — the notification surface, background delivery and the sharing options — is
verified manually on **both iOS and macOS** (task 5.15), because `testing-strategy` records that native
lifecycle behaviour cannot be driven by `tauri-driver` there.

**Carried from P0 (Q25, decided 2026-10-20):** this change owns the account sign-in/acquisition flow.
P0 deliberately ships no sign-in screen, so its running app answers `no_account` on every
state-changing command; the mechanism is already decided by P0's D0.7 (Supabase Auth/GoTrue, email OTP,
the refresh token in the platform Keychain) and the first live account is provisioned by the
deployment task. Before implementation, add the flow to this plan — the contract commands (request and
verify an OTP against GoTrue, session/token handling), the screens, and the tests — and do not reopen
the mechanism decision.

## 1. Contract and native surface

- [ ] 1.1 [architect] Extend the P0 IPC contract in `src-tauri/src/ipc/`: the invitation lifecycle
  (create, reuse, accept), the pending-adoption state, member-list resolution, owner-gated deletion,
  live-update subscription, the owner-attributed storage usage and the notification authorization —
  `request_notification_authorization()`, `get_notification_authorization()` and the
  `NotificationAuthorization` state (`unknown | not_determined | granted | denied`) that
  `add-vet-export-and-reminders` consumes — and the sharing policy values, in the P0 naming style. Freeze the names for
  this phase and add typed stubs returning a `not implemented` error. Carry the settled retry model —
  a client-generated `request_id` on every state-changing command, which the shared service deduplicates
  on — as a parameter of the retryable commands (invitation creation, household deletion), and record
  how the `timeout` cause is returned. Carry the attribution criterion this phase adds to the mechanism's
  requirements (D-H16): the shared service MUST attribute a contribution made by a non-owner to the
  household **owner's** allowance end to end and MUST expose the household's owner-attributed usage, so a
  candidate that cannot is rejected; confirm the mechanism P0 selected (`docs/adr/0002-shared-service-mechanism.md`)
  against it and, if it fails, report a blocker (project hard rule 7) rather than degrading to a generic
  failure message (NFR-8).
- [ ] 1.2 [architect] Record this phase's decisions in `docs/adr/0003-sharing-hardening.md`: the
  single-attempt bounded-time rule and where it is enforced (D-H1), one-invitation-per-household with
  reuse (D-H2), local-first member-list construction (D-H4), the owner gate at the storage boundary with
  an irreversible cascade and no restore path (D-H5), background delivery as a native-to-Rust handoff
  (D-H6), HS-13 as a carried unmitigated risk (D-H10), the values and retry model consumed from P0's
  design rather than re-derived (D-H12: 15 s attempt bound, 5 s reachability-probe bound, 3 consecutive
  failures before "offline", `request_id` deduplication), the membership-level cap and the refused
  acceptance into a full household (D-H13), the visible notification naming the household only and the
  one notification authorization P4 consumes (`request_notification_authorization()` /
  `get_notification_authorization()` and the `NotificationAuthorization` state, D-H14), the
  offline-deletion window (D-H15), and the
  attribution criterion (D-H16).
- [ ] 1.3 [tauri-security] Declare the capability, CSP, scope, entitlement and background-mode changes
  this phase needs on **both iOS and macOS**: the background-delivery mode/backing task the selected
  mechanism requires, whatever scope presenting an invitation link requires, and the notification
  authorization this phase introduces (D-H14). Record explicitly where a platform requires **nothing**
  rather than adding a speculative key: notification authorization is requested at runtime on both
  platforms, so **no `Info.plist` usage-description string is added for it on iOS** (no such
  usage-description key exists for notifications — state that rather than inventing one) and **no
  entitlement is added on either platform** for local notifications; `aps-environment` and the
  `remote-notification` background mode are declared only if the selected mechanism delivers changes
  remotely, with their one-line justification. Keep it least-privilege, add no permission the phase does
  not use, keep the iOS and macOS configurations in step, and keep the config `cfg`-clean for later
  targets.
- [ ] 1.4 [apple-native] Implement the presentation of the standard sharing options for an invitation
  link on **iOS** (`UIActivityViewController`, `cfg(target_os = "ios")`) eligible for `bun run tauri ios
  dev`, without adding a dependency that does not support both Apple platforms (HS-5).
- [ ] 1.5 [apple-native] Implement the presentation of the standard sharing options on **macOS**
  (`cfg(target_os = "macos")`), reusing one shared contract-facing entry point with the iOS
  implementation (HS-5).
- [ ] 1.6 [apple-native] Implement background delivery of shared-data changes on **iOS**: register the
  background delivery, hand a delivered change to the Rust sync layer, and raise the notification for
  that delivery through the notification surface of task 1.9 — naming the household **only**, carrying no
  health content (no rabbit name, symptom, note text, weight or medication name) and nothing else taken
  from the household's data (D-H14) — `cfg(target_os = "ios")`, keeping the native surface thin and
  `cfg`-clean (HS-11).
- [ ] 1.7 [apple-native] Implement background delivery of shared-data changes on **macOS**: deliver a
  change while the product is not frontmost, hand it to the Rust sync layer, and raise the same
  household-naming, health-content-free notification through the surface of task 1.9
  (`cfg(target_os = "macos")`) (HS-11, D-H14).
- [ ] 1.8 [apple-native] Unit-test the iOS and macOS native bridges for the sharing options, the
  background handoff and the notification surface — covers "A background-delivered change notifies the
  household without revealing its content" and "A change is still delivered when a notification cannot be
  raised", asserting that the notification names the household and carries no health content, and that a
  delivery still applies when no notification can be raised; also record the manual simulator steps for
  both platforms (testing-strategy: native lifecycle behaviour is not drivable by `tauri-driver` on
  iOS/macOS) (HS-5, HS-11, D-H14).
- [ ] 1.9 [apple-native] Implement the notification surface this phase introduces, on **both iOS and
  macOS** behind one shared contract-facing entry point (`cfg`-gated, `cfg(target_os = "ios")` /
  `cfg(target_os = "macos")`): implement `request_notification_authorization()` — asking at the first
  delivery that needs a notification rather than at launch — and `get_notification_authorization()`, which
  observes the state and returns it as `NotificationAuthorization`
  (`unknown | not_determined | granted | denied`), so `add-vet-export-and-reminders` consumes that one
  surface instead of declaring a second authorization (D-H14), and raise a local notification naming the
  household only. It lands before 1.6–1.7 raise anything
  through it; `UNUserNotificationCenter` requires macOS 10.14, far below P0's shipped deployment floor
  (iOS 16.4 / macOS 13.3, D0.16), so no target change follows (HS-11).

## 2. Backend (Rust)

- [ ] 2.1 [rust-backend] Implement invitation creation: verify availability before attempting, reuse an
  existing invitation instead of creating a second, carry the client-generated `request_id` so a retry is
  deduplicated rather than creating a duplicate, and refuse creation when the household already holds six
  members — the cap is a property of the household's membership (D-H13), so the refusal here is the same
  one acceptance at the cap gives — with the limit explained (HS-5, HS-7).
- [ ] 2.2 [rust-backend] Implement the single-attempt bounded-time rule for sharing operations: every
  invitation create and invitation accept resolves as success or as a `timeout` cause within
  `SharingPolicy.attempt_timeout_ms` (15 s), the pre-attempt reachability check within
  `SharingPolicy.reachability_timeout_ms` (5 s), with nothing queued and nothing completing later
  (HS-5, HS-6, D1, D-H12).
- [ ] 2.3 [rust-backend] Implement invitation acceptance: resolve the invitation, make the recipient a
  member of that household with read/write access, including on a fresh setup with no household, and
  write a redacted diagnostic record when acceptance does not complete (identifiers and outcomes only)
  (HS-6).
- [ ] 2.4 [rust-backend] Implement the adoption-in-progress state: distinguish "holds no household" from
  "acceptance unfinished", keep the adopted household active once its data arrives within the bound
  without further action, keep a household created during the wait active, and clear the state when the
  attempt resolves (HS-6).
- [ ] 2.5 [rust-backend] Implement member-cap enforcement at membership level (D-H13): six members
  including the creator, checked inside the attempt that would admit a member, so an invitation or an
  acceptance that would exceed the cap is refused with the same six-member limit explanation and the
  person is not admitted (HS-7).
- [ ] 2.6 [rust-backend] Implement the member-list resolution path: always include the current person
  from local data with a "You"-capable label key, resolve display names account-first, fall back to the
  locally known name and then to a placeholder, and keep locally known members with the cause surfaced
  when a participant refresh fails (HS-9).
- [ ] 2.7 [rust-backend] Implement owner-gated household deletion at the storage boundary: refuse a
  non-owner with a cause, carry the client-generated `request_id` so a retry is deduplicated instead of
  deleting twice, then delete the household with its rabbits, weight and health entries, photos,
  medications, doses, members and tags in one irreversible operation, and withdraw access from every
  member at the first moment each member's device reaches the shared service: a member unreachable at
  deletion keeps reading the previously loaded data until that reach, when the cached copy is dropped and
  access is gone, and the product does not attempt to erase another device's cache while it is
  unreachable (D-H15) (HS-10, NFR-10).
- [ ] 2.8 [rust-backend] Implement the deletion outcomes: make the newest remaining household active or
  leave none active, share the deletion with the household's members, and leave the household and all of
  its data intact with a cause when the deletion does not take effect (HS-10, DM-3, NFR-8).
- [ ] 2.9 [rust-backend] Implement the per-household live-update subscription and re-establish it
  whenever a household becomes held mid-session — created, or joined by an accepted invitation — with no
  restart required (SYNC-6, HS-11).
- [ ] 2.10 [rust-backend] Implement the background-delivery handoff in the sync layer: accept a change
  delivered by the native layer for a household, apply it to the loaded collection, and surface it as a
  refresh of the affected household when the product is next foregrounded (HS-11).
- [ ] 2.11 [rust-backend] Implement owner-attributed storage accounting: charge content saved into a
  household to that household's owner — including a contribution made by a non-owner, which is charged to
  the owner and **not** to the contributing member — expose the household's owner-attributed usage so the
  attribution is observable from the product (D-H16), and map an exhausted owner allowance to the
  storage-allowance cause rather than a generic failure (HS-12, SYNC-2).
- [ ] 2.12 [rust-backend] Implement set-up-again recovery: on a start on the same account with
  connectivity, recover the household from the shared service as the source of truth and present it
  instead of an empty household or onboarding; show the connectivity-required state when nothing was
  loaded and the service is unreachable (SYNC-7, SYNC-3).
- [ ] 2.13 [rust-backend] Implement the honest-status rules: keep the status not-yet-determined until a
  fresh outcome, do not present offline after a single failure, present offline only after three
  consecutive failures or a direct check, collapse rapid updates into one syncing state, and expose the
  consecutive-failure count (NFR-6).
- [ ] 2.14 [rust-backend] Implement the sharing policy configuration: `attempt_timeout_ms` (15 s),
  `reachability_timeout_ms` (5 s) and `offline_failure_threshold` (3) as per-device values kept out of
  the household collection, consumed from P0's design with no value re-derived or guessed (HS-5, HS-6,
  NFR-6, D-H12).

## 3. Frontend logic

- [ ] 3.1 [frontend-logic] Run `bun run gen:bindings` against the frozen contract and add
  `src/features/sharing/api.ts` and `src/features/members/api.ts`, extending
  `src/features/households/api.ts` (delete flow and owner-attributed usage) and `src/features/sync/api.ts`
  (status and the notification authorization — `get_notification_authorization()` /
  `request_notification_authorization()` — the contract exposes for
  `add-vet-export-and-reminders`); no `invoke` call outside these files
  (HS-5, HS-6, HS-9, HS-10, HS-12).
- [ ] 3.2 [frontend-logic] Implement the invitation query and mutation state: pre-attempt availability
  check, progress state, reuse path, cause-specific failure with retry, and timeout resolution that
  never leaves the surface stuck (HS-5).
- [ ] 3.3 [frontend-logic] Implement acceptance state: adopt on accept, keep the creation onboarding
  suppressed while an adoption is unresolved, keep a household created during the wait active, and
  surface a timeout with its cause (HS-6).
- [ ] 3.4 [frontend-logic] Implement the member-list query: current person injected first, the
  display-name fallback chain, retention of locally known members with the cause surfaced on a failed
  refresh, and a refresh when participants resolve (HS-9).
- [ ] 3.5 [frontend-logic] Implement the deletion flow state: availability only for the owner, a
  confirmation gate that deletes nothing when dismissed, and the post-deletion transition to the newest
  remaining household or to onboarding (HS-10).
- [ ] 3.6 [frontend-logic] Subscribe to `invitation:changed`, `household:members-changed`,
  `household:deleted` and the extended `service:status-changed`, and invalidate only the affected
  household queries, including after a mid-session create or join (SYNC-6, HS-11, HS-10).
- [ ] 3.7 [frontend-logic] Extend the i18n keys for this phase: invitation copy per cause, the localized
  "You" label and placeholders, the deletion consequence copy, the member-cap explanation, and the
  status states including "syncing" and not-yet-determined (HS-5, HS-7, HS-9, HS-10, NFR-6).
- [ ] 3.8 [frontend-logic] Implement the status store rules on the client: never present a carried-over
  status as current before a fresh outcome, and collapse rapid updates into one stable syncing state
  (NFR-6).

## 4. Frontend UI

- [ ] 4.1 [frontend-ui] Build the invitation surface: the invite control on the household, progress
  feedback, the reuse path, cause-specific failure with a retry, and the hand-off to the standard sharing
  options. Screenshots at 390×844 and 1280×800 in `.artifacts/screens/` (HS-5).
- [ ] 4.2 [frontend-ui] Build the invitation acceptance surface: a joining state that never presents
  creation onboarding while the household's data is arriving, and a timeout failure showing its cause.
  Screenshots at 390×844 and 1280×800 in `.artifacts/screens/` (HS-6).
- [ ] 4.3 [frontend-ui] Build the member list: the current person first with the localized "You"-style
  label, display-name fallback rendering, and locally known members retained with the refresh cause
  shown. Screenshots at 390×844 and 1280×800 in `.artifacts/screens/` (HS-9).
- [ ] 4.4 [frontend-ui] Build the owner-only household delete control on the Settings row — directly
  visible, not a hidden gesture — with a confirmation stating that members lose access and that the data
  cannot be recovered, absent for a joined household. Screenshots at 390×844 and 1280×800 in
  `.artifacts/screens/` (HS-10, NFR-9).
- [ ] 4.5 [frontend-ui] Build the post-deletion landing and the member-cap explanation surfaces, at
  390×844 and 1280×800, with screenshots in `.artifacts/screens/` (HS-7, HS-10).
- [ ] 4.6 [frontend-ui] Build the honest service-status indicator: not-yet-determined, syncing, offline
  and cause presentations, with one stable syncing state that does not flicker. Screenshots at 390×844
  and 1280×800 in `.artifacts/screens/` (NFR-6).

## 5. Tests and manual verification

- [ ] 5.1 [test-engineer] Rust tests for invitation creation — covers "Inviting is refused before an
  attempt when the shared service is unavailable", "An invitation failure states its specific cause" and
  "An existing invitation is reused rather than duplicated" (HS-5), plus "An invitation beyond the member
  cap is blocked with an explanation" (HS-7).
- [ ] 5.2 [test-engineer] Rust tests for the bounded-time rule — covers "A stalled invitation attempt
  resolves with a timeout failure" (HS-5) and "An acceptance that cannot complete resolves with a timeout
  failure" (HS-6), asserting the attempt resolves at the settled 15 s attempt bound, that nothing is
  queued and that no attempt completes after the deadline.
- [ ] 5.3 [test-engineer] Rust tests for acceptance and adoption — covers "Accepting an invitation on a
  fresh setup grants membership with read/write access", "Acceptance does not fall back to onboarding
  while data is still arriving", "A household created while an invitation is still resolving stays
  active", "A delivered invitation is never discarded without a trace" (HS-6) and "Accepting an
  outstanding invitation cannot take the household beyond six members" (HS-7), asserting that an
  invitation issued while the household had room is refused into a household that already holds six,
  with the same six-member limit explanation a blocked creation gives and the person not admitted
  (D-H13).
- [ ] 5.4 [test-engineer] Rust tests for the member list — covers "The current person is listed with a
  localized You-style label", "An unresolved display name falls back without breaking the list", "The
  list renders before participants resolve and refreshes once they do" and "A failed participant refresh
  keeps locally known members and states the cause" (HS-9).
- [ ] 5.5 [test-engineer] Rust tests for owner-gated deletion — covers "Confirmed deletion removes the
  household and all of its data", "Confirmed deletion withdraws access from every member and reaches
  them", "The newest remaining household becomes active after deletion", "Onboarding is presented after
  the only household is deleted" and "A failed deletion leaves the household intact and states its
  cause" (HS-10), including a non-owner request being refused at the storage boundary.
- [ ] 5.6 [test-engineer] Rust tests for live updates and background handoff — covers "A household
  created mid-session receives live updates without a restart", "A household joined by accepting an
  invitation receives live updates without a restart" (SYNC-6), the handling of a native-delivered
  change, and "A change is still delivered when a notification cannot be raised" — asserting a delivery
  whose notification cannot be raised is still applied and present at the next show (HS-11, D-H14).
- [ ] 5.7 [test-engineer] Rust tests for attribution and recovery — covers "Content saved by a non-owner
  counts against the owner's allowance", asserting the contribution is charged to the owner's account
  rather than the saving member's and is visible in the household's owner-attributed usage, and "An
  exhausted owner allowance refuses a member's save with that cause", asserting the refusal follows from
  the household's consumption charged to the owner (HS-12, D-H16), plus all three set-up-again recovery
  scenarios (SYNC-7).
- [ ] 5.8 [test-engineer] Rust tests for honest status — covers "A single transient failure does not show
  offline", "Three consecutive failures show offline", "A direct check that the service or account is
  unavailable shows offline", "A status carried over from a previous session is not presented as current"
  and "Rapid consecutive updates show one stable syncing state" (NFR-6).
- [ ] 5.9 [test-engineer] Component tests at 390×844 and 1280×800 for the sharing surfaces — covers the
  invitation progress and cause scenarios (HS-5), both "The adopted household's dashboard appears without
  further action" and the onboarding-suppression case (HS-6), the member-list scenarios (HS-9), the
  visible delete control and its confirmation (HS-10) and the status scenarios (NFR-6).
- [ ] 5.10 [test-engineer] E2E with **two identities sharing one household** at 390×844 (mocked IPC):
  identity A invites → identity B accepts → both see the same rabbits and entries → A edits → B sees the
  edit without a manual refresh; covers the HS-5 invitation scenarios, "Accepting an invitation on a
  fresh setup grants membership with read/write access" and "The adopted household's dashboard appears
  without further action" (HS-6), "A change made by one member appears for another without a manual
  refresh" and "Every member of the household ends up with the same change" (HS-11).
- [ ] 5.11 [test-engineer] E2E on a fresh setup with no household: accept an invitation and assert that
  creation onboarding is never presented and the adopted household's dashboard appears; then the timeout
  path resolving with its cause and leaving no pending acceptance; covers the HS-6 fresh-setup and
  timeout scenarios.
- [ ] 5.12 [test-engineer] E2E: the owner deletes the shared household, the participant's Settings never
  offered a delete control, and the participant loses access and lands on the post-deletion state; then
  the same deletion with the participant's device unreachable, asserting that device keeps reading the
  loaded household until it next reaches the service, when the cached copy is dropped and access is gone,
  and that the owner's device does not attempt to erase it meanwhile (D-H15); covers
  "A joined household offers no delete control to a non-owner", "The confirmation states the full
  consequence", "Confirmed deletion withdraws access from every member and reaches them", "A member who is
  unreachable at the moment of deletion loses access at the next refresh" and "The newest
  remaining household becomes active after deletion" (HS-10).
- [ ] 5.13 [test-engineer] E2E: a household joined mid-session receives live updates without a restart
  (SYNC-6), and setting the product up again on the same account recovers the household rather than
  showing an empty state (SYNC-7).
- [ ] 5.14 [test-engineer] E2E for honest status: one transient failure does not present offline,
  repeated failures do, and a relaunch does not present the previous session's status as current; covers
  the NFR-6 scenarios at 390×844 and 1280×800.
- [ ] 5.15 [test-engineer] Manual simulator verification on **iOS and macOS**: background delivery of a
  change while the product is not open, the notification that delivery raises — naming the household
  only, with no rabbit name, symptom, note text, weight or medication name — the authorization prompt
  (`request_notification_authorization()`) and the denied path, where no notification appears and the
  state `get_notification_authorization()` reports is presented rather than assumed, while the change
  still arrives, and the platform sharing
  options (HS-5, HS-11, D-H14), performed with
  two accounts on two devices/simulators; record the observed outcomes in this change.
- [ ] 5.16 [orchestrator] Run the full gate across the change once (`definition-of-done`: every stage
  whose condition holds) and record stage → pass/fail in this change. Not per task, and no specialist
  runs it.

## 6. Review

- [ ] 6.1 [reviewer] Review the P3 diff for contract conformance: the frontend touches IPC only through
  generated bindings and `api.ts`, no `invoke` call exists outside those files, no local write queue or
  optimistic acceptance was introduced (D1), and no ownership-transfer or restore path appeared (D2).
  For the UI tasks (4.1–4.6), confirm touch targets, safe areas and the adaptive layout at 390×844 and
  1280×800, per `definition-of-done`.
- [ ] 6.2 [reviewer] Verify the delta contains exactly HS-5, HS-6, HS-7, HS-9, HS-10, HS-11, HS-12,
  SYNC-6, SYNC-7 and NFR-6 — no more, no less — that no requirement owned by P0, P1 or P2 was modified or
  redefined, that every requirement has at least one scenario named by a test task, that the sharing
  surfaces are honest about failure (NFR-8), and that HS-13 exists only as a recorded accepted risk.
