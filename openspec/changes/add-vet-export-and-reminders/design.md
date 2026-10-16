# Design — add-vet-export-and-reminders (P4)

## Context

This phase delivers after **P5** (see `proposal.md`): the medication and `Schedule` model, the
human-readable schedule summary (MED-4) and the record-a-dose flow with its pre-filled
confirmation (MED-5, MED-8) already exist when this change starts. MED-9 … MED-11 are specified
here because decision **D3** makes reminders *one* mechanism, so medication reminders are
configurations of the same mechanism as weigh reminders rather than a second subsystem (IMP-5).

Three of the four things this phase adds are surfaces on the *device* rather than on the shared
household record: reminders, the language preference, and the notification authorization state.
Export is the fourth: it *reads* the household record and produces files locally. So the phase's
central design question is not "what data do we store" but **where the boundary between
device-local configuration, household data and produced files runs**, and how the product stays
honest about two device states it does not control (notification authorization, the product not
running).

Platforms: iOS (primary) and macOS (Phase 1) — baselines 390×844 first, 1280×800 second. Android,
Windows and Linux are later, so everything stays `cfg`-clean.

Stack: Tauri v2, Rust in `src-tauri/`, React + TypeScript + Vite in `src/`, TanStack Router +
Query, generated `src/bindings.ts` (never hand-edited), bun.

## Decisions

### D4.1 — The one reminder mechanism (implementation of D3)

One device-local store, one schedule form, one occurrence model, one follow-up path. The two kinds
are configurations of it:

| | Weigh reminder | Medication reminder |
|---|---|---|
| Target | a rabbit, or the household | a medication (P5's entity) |
| Schedule | user-defined recurring day(s) + time | derived from the medication's P5 schedule for every type that schedules doses |
| Follow-up default | off | **on** (MED-10) |
| Actions | open the product at the target | record the dose as given (opens P5's pre-filled confirmation, MED-8) or snooze |

Everything else — creation, edit, enable/disable, delete, persistence, scheduling, delivery,
cancellation, the follow-up decision, per-device/per-person scoping — is shared code with one
implementation. Consequences:

- an occurrence is the unit the mechanism schedules and reasons about: `{ reminder_id, due_at,
  state }` with `state ∈ pending | fired | snoozed | satisfied | missed`. "Satisfied" is decided by
  the kind's own evidence — a weight entry for the target (weigh), a recorded dose for the slot
  (medication) — so the two kinds plug into one follow-up rule instead of duplicating it;
- MED-11 is expressed as configuration, not as a branch in a scheduler: a `prn` medication has no
  occurrences, so it has nothing to schedule. There is no code path that must remember to exclude
  PRN;
- the store is **device-local and outside the household collection**, exactly as P0 fixed for
  per-device preferences. Reminders are therefore never seen by another member's device, which is
  REM-2 without a sharing rule to maintain;
- because the mechanism is device-local, configuring a reminder is **not** a save to the shared
  service and the reachability gate (D1) does not apply to it. It is a local write. This is stated
  here because the tempting wrong reading — "every write needs connectivity" — would make
  reminders fail on a plane, which is the opposite of their purpose.

The one open coordination cost: the store exists on each device, so the same person on two devices
has two independent sets of reminders (REM-2's scenario "A reminder does not follow the person to
another device"). That is the documented product model (§5) and is accepted, not a bug.

### D4.2 — Medication occurrence derivation uses P5's schedule, not a copy of it

An occurrence is computed from the medication's schedule by the same rules P5 already implements
for "what is due" (MED-3, MED-4), and the medication's *active* state (MED-2) decides whether any
occurrence exists at all. This design adds no second schedule interpreter: if P5's summary says
"every 3 days from 12 Oct at 09:00", the occurrences are exactly those instants. That is what makes
EXP-4's "export succeeds for any schedule type" and MED-9's "slots derived for every schedule type"
the same underlying question, answered once.

### D4.3 — Follow-up evaluation: schedule ahead, cancel on evidence, reconcile on launch

Neither platform will run product code on a schedule the person did not ask for, and this design
deliberately does **not** request a background mode (see D4.5). The follow-up is therefore handled
in two halves:

1. **Schedule ahead.** When a reminder occurrence is scheduled, its follow-up is scheduled with it,
   for the following day (weigh) or the kind's follow-up moment (medication). This is what makes the
   follow-up possible at all when the product is not running.
2. **Cancel on evidence, reconcile on launch/foreground.** Every launch and foreground return
   reconciles outstanding occurrences against the locally kept copy of the household data (P0's
   loaded-data read path): a weigh occurrence with a weight entry logged for its target after it
   fired is marked `satisfied` and its follow-up cancelled; a medication occurrence with a dose
   recorded for its slot is `satisfied` likewise. Cancellation is a local operation on the device's
   own scheduled notifications, so it needs no connectivity.

Accepted limitation, recorded so it is not discovered later as a bug: a device that never runs the
product again between the occurrence and the follow-up will not have the chance to cancel, so a
follow-up can be raised for a target that was, in fact, weighed/dosed elsewhere — and conversely a
follow-up evaluated only on launch cannot be raised if the device never runs again. The scenario
"a weigh-in after the reminder suppresses the follow-up" is therefore asserted for the case where
the device does run before the follow-up falls due, and `questions.md` records the residual gap.

### D4.4 — Document generation: Rust-side, then a native handoff

**Decision: the vet-readable document is generated in Rust**, from the selected history, and the
produced files are then handed to the platform's standard sharing options by a small native bridge.
The alternative — rendering the document natively (UIKit's PDF renderer on iOS, an AppKit print
operation on macOS) — was rejected:

| | Rust-side generation (chosen) | Native renderer per platform (rejected) |
|---|---|---|
| Implementations of the report layout | one | two, one per platform, that will drift |
| Testability | content asserted in Rust tests with no device and no simulator | needs a device/simulator per platform; content asserted through rendered pixels or a PDF reader |
| German coverage | one font/locale path to get right once | two |
| Future targets (Android, Windows, Linux) | nothing to add but a new handoff bridge | a third and fourth renderer |
| Platform-specific risk | font embedding and file location | two full renderers, plus the layout risk anyway |

Consequences recorded now, because they are where this choice can go wrong:

- **Font and language.** The generated document must present German text correctly (umlauts and ß).
  The dependency must therefore either embed a font covering German or use an encoding that does.
  Getting this wrong is a silent corruption of a document a vet reads, so it is a test task
  (5.11/5.18 export a German-tagged history) rather than a build-time hope.
- **Thread.** Generation runs off the main thread, so a large range cannot freeze the interface; the
  UI shows a working state until the files are ready.
- **File location and lifetime.** Files are written to a device-local export location with a defined
  lifetime (cleared after the handoff completes or on the next launch). Nothing is written into the
  household collection and nothing is uploaded (EXP-3).
- **`cfg`-cleanliness.** Selection, layout and serialisation live in a platform-neutral module; only
  the file location and the handoff are platform-gated, so a future target adds a bridge and not a
  report.
- **Dependency.** Exactly one document-generation dependency is added, with a one-line justification
  and a check that it supports both iOS and macOS (project hard rule 6, task 1.6). It must be a
  pure-Rust library — no system library to link, no runtime to embed — otherwise the iOS build
  breaks in exactly the way P0's R5 recorded for the sharing mechanism.

### D4.5 — No background mode, no new always-on surface

Nothing in this phase requires the product to run in the background: occurrence scheduling is handed
to the device's own notification scheduler, and reconciliation (D4.3) happens on launch and
foreground return. Requesting a background mode would widen the security surface for a convenience
the requirements do not ask for, so the `tauri-security` task explicitly adds none and records that
no entitlement is required beyond notification authorization and the file location.

### D4.6 — The accessible chart summary is derived, never stored

The summary is computed from the **same weight series the chart renders** — latest value in the
display unit, and the trend direction derived from the same values — and exposed as the chart's
accessible label. There is no second query, no stored summary and no second source of chart data, so
the summary cannot disagree with what is on screen; it is recomputed when the series changes
(NFR-4's "no second data source" scenario). The definition of "trend direction" and the no-value
wording are recorded in `questions.md`; the requirement itself only says the summary exists and does
not over-claim.

### D4.7 — Device states the product must not pretend about

Two device states can make a reminder silently useless, and both are surfaced rather than assumed:

- **Notification authorization.** A device that has not granted it cannot raise reminders. The
  product presents the authorization state where reminders are configured and does not present a
  configured reminder as if it will certainly fire. This is the same honesty stance P0 fixed for
  service status (NFR-6's requirement belongs to P3; the stance is reused, not restated).
- **The product not running.** Covered by D4.3's accepted limitation.

### D4.8 — Language override: resolve once at launch, never mid-session

The stored preference (System / Deutsch / English) is read **at launch** and resolves to a language
once; `System` resolves to the device's language at that moment. Changing the setting writes the new
value and shows a next-launch notice; the running session's language is fixed. This makes SET-3's
"applies on the next launch" a property of one resolution point rather than a rule every component
must respect, and it is why no component may read the language preference reactively.

### D4.9 — Export reads; it does not gate

Export reads the rabbit's history through P0's read path: already-loaded data is sufficient, so an
export works while the service is unreachable, and it adds no write, no queue and no reachability
check of its own. A failure to produce or hand over the files is reported through P0's
cause-carrying error surface (NFR-8 names export explicitly) and presents no file, so a failed export
can never look like a successful one.

## State and boundaries

```
Rust (src-tauri/)                                  Frontend (src/)
  src-tauri/src/ipc/          contract: reminder, export and language commands;
                              events; shared types            → src/bindings.ts (generated)

  src-tauri/src/reminders/    ONE device-local store outside the household
                              collection: reminders (both kinds), occurrences,
                              follow-up scheduling and reconciliation (D4.1–D4.3)
  src-tauri/src/export/       selection from the rabbit's history, PDF
                              generation, raw CSV files (platform-neutral, D4.4)
  src-tauri/src/settings/     language preference beside P0's display unit (D4.8)

  src-tauri/src/platform/     cfg-gated: file location + standard sharing options;
                              notification authorization, scheduling and actions
                              (iOS / macOS; future targets add a bridge)

  household collection (P0)   UNCHANGED. Nothing in this change writes to it.
                                                   → src/features/reminders|export/api.ts + hooks
                                                   → src/features/settings/ language hook
                                                   → src/routes/ reminders, weigh-reminder form,
                                                     export flow, reminder-action landing
                                                   → chart + control components: accessible summary,
                                                     screen-reader labels, text scaling
                                                   → i18n string tables: German + English complete
```

Rules held here:

- **Reminders, the language preference and the notification state are device-local.** They are never
  written into the household collection, never shared and never exported (REM-2, SET-3, D4.1).
- **Export produces files, not records.** The produced files are device-local artifacts with a
  defined lifetime; the product sends them nowhere (EXP-3, D4.4).
- **One mechanism.** No medication-specific scheduler, follow-up loop or persistence shape exists;
  the difference between the kinds is configuration (D4.1, D4.2). This is a review finding if broken.
- **One schedule interpretation.** Occurrence derivation reuses P5's schedule semantics and its
  summary (MED-4) rather than a second interpreter (D4.2).
- **The frontend never calls `invoke` directly**, only `src/features/*/api.ts` over generated
  bindings (project hard rule 2).
- **No background mode** and no always-on surface (D4.5).

## IPC contract — to be defined by architect

**Status: proposal only. `architect` owns this section and the typed stubs in
`src-tauri/src/ipc/` (tasks 1.1–1.7). Nothing may be implemented against these names until 1.1 is
done and `bun run gen:bindings` regenerates `src/bindings.ts`.**

The naming style follows P0: `snake_case` commands with explicit parameters and no implicit
context; events named `<area>:changed` or `<area>:<verb>`; opaque ids as strings; instants as P0's
`Instant`.

### Commands the frontend needs in this phase

| Command (proposed) | Purpose | Requirements |
|---|---|---|
| `list_reminders()` | The one list of reminders held for **this device**, both kinds, with their targets, schedules, follow-up and enabled state | D3, REM-1, REM-2 |
| `create_weigh_reminder(target, schedule, follow_up_enabled)` | `target = household \| rabbit(rabbit_id)`; user-defined recurring schedule | REM-1 |
| `create_medication_reminder(medication_id, follow_up_enabled)` | Occurrences derived from the medication's P5 schedule; follow-up defaults to enabled | MED-9, MED-10 |
| `update_reminder(reminder_id, …)` | Change schedule, target, follow-up or enabled state; re-schedules | REM-1, MED-10 |
| `delete_reminder(reminder_id)` | Remove a reminder and its pending occurrences | REM-1, REM-2 |
| `get_reminder_authorization()` | This device's notification authorization state, freshly observed | D4.7 |
| `request_reminder_authorization()` | Ask the device for it; the outcome is reported honestly | D4.7 |
| `snooze_reminder(reminder_id, occurrence_at)` | Defer an occurrence without recording anything; the slot stays unsatisfied | MED-9 |
| `resolve_reminder_slot(reminder_id, occurrence_at)` | Mark an occurrence satisfied after P5's confirmation recorded the dose | MED-9 |
| `reconcile_reminders()` | Reconcile occurrences against loaded data on launch/foreground; cancel follow-ups whose evidence exists | REM-3, MED-10, D4.3 |
| `export_rabbit_history(rabbit_id, range, format)` | Produce the export; `format = pdf \| csv`; returns the produced artifacts (one for `pdf`, several for `csv`) | EXP-1, EXP-2, EXP-4 |
| `get_export_span(rabbit_id)` | The earliest and latest instants the rabbit has history for, so the range control can default sensibly | EXP-1 (see `questions.md` Q2) |
| `present_export_sharing(export_ids)` | Hand the produced files to the device's standard sharing options | EXP-3 |
| `get_language_preference()` / `set_language_preference(preference)` | The per-device language preference; setting it does not change the running session | SET-3 |

`architect` decisions required in 1.1: whether `export_rabbit_history` streams progress for a large
range or merely returns when done (a `[mobile]` working state is required either way), and whether
the produced artifacts are addressed by an opaque export id (as sketched) or by a path handle —
choosing the id keeps the file location an implementation detail, which D4.4 prefers.

### Events the frontend needs in this phase

| Event (proposed) | Payload | Purpose |
|---|---|---|
| `reminders:changed` | `{ kind: created \| updated \| deleted \| reconciled, reminder_id? }` | Invalidate the device's reminder list after any change, including a reconciliation |
| `reminder:action` | `{ reminder_id, occurrence_at, action: record \| snooze }` | Delivered when the person acts on a raised reminder, so the frontend can open P5's pre-filled confirmation (`record`) or confirm the deferral (`snooze`) |
| `reminder:notification-authorization-changed` | `{ state }` | Keep the authorization surface truthful when it changes while the product runs |
| `export:state-changed` | `{ export_id, state: generating \| ready \| failed }` | Drive the working state and the produced-file list |

Events are additive to P0's `household:changed` and `service:status-changed`, which are unchanged.
Because reminders are device-local, `reminders:changed` is deliberately **not** household-scoped.

### Shared types

| Type (proposed) | Shape | Notes |
|---|---|---|
| `ReminderId` | opaque, stable string | device-local identity |
| `ReminderKind` | `weigh \| medication` | D3; the only difference between the two kinds |
| `ReminderTarget` | `{ kind: household \| rabbit \| medication, id? }` | `id` absent for household-wide (REM-1) |
| `ReminderSchedule` | `{ kind: recurring_days, weekdays, time_of_day }` | the weigh kind's user-defined schedule; a medication reminder derives its occurrences from P5's `Schedule` instead of carrying one (D4.2) |
| `ReminderConfig` | `{ id, kind, target, schedule?, medication_id?, created_by_member_id, follow_up_enabled, enabled, authorization? }` | one shape for both kinds (D4.1) |
| `ReminderOccurrence` | `{ reminder_id, due_at: Instant, state: pending \| fired \| snoozed \| satisfied \| missed }` | the unit the mechanism schedules and reconciles (D4.1) |
| `ReminderAuthorization` | `{ state: unknown \| not_determined \| granted \| denied, observed_at }` | D4.7; honest rather than assumed |
| `ReminderAction` | `record \| snooze` | MED-9's two offered actions |
| `ExportFormat` | `pdf \| csv` | EXP-1, EXP-2 |
| `ExportRange` | `{ from: Instant, to: Instant }` | the selectable range (EXP-1) |
| `ExportArtifact` | `{ export_id, format, file_name, role: weight \| health \| medication \| dose \| document, created_at }` | `role` is what makes EXP-2's dedicated medication and dose files assertable |
| `ExportBundle` | `{ rabbit_id, range, artifacts: [ExportArtifact] }` | one export, however many files it produced |
| `ExportState` | `generating \| ready \| failed` | drives the working state (D4.4) |
| `LanguagePreference` | `system \| de \| en` | SET-3, per device |

Reused, not redefined: P0's `CommandError`, `SaveCause`, `Instant`, `MemberId`, `HouseholdId`;
P1's `RabbitId` and weight-entry types; **P5's `MedicationId`, `DoseId`, `Medication`, `Schedule`
and `Dose`** — this change consumes P5's model and adds no parallel medication or schedule type.

## Traceability: user stories → scenarios

| Story | Requirement(s) | Capability / scenario |
|---|---|---|
| — (D3, IMP-5) | unified mechanism | reminders: "Both kinds of reminder are created and listed through one mechanism", "Both kinds carry the same configuration and the same day-to-day operations", "Reminders live on the device that created them and survive a relaunch", "A reminder fires while the product is not open `[ios]`", "A reminder fires while the product stays running `[macos]`" |
| US-REM-01 | REM-1 | reminders: "A reminder configured for a recurring day and time fires on schedule", "An edited reminder is honoured with its new schedule", "A disabled or deleted reminder raises nothing" |
| US-REM-02 | REM-1 | reminders: "A household-wide reminder covers the household rather than one rabbit" |
| US-REM-03 | REM-2 | reminders: "Only the creating person's device is reminded", "Another member is never notified by a reminder they did not create", "A second member configures an independent reminder", "One person's reminder change leaves another member's reminder untouched", "A reminder does not follow the person to another device" |
| US-REM-04 | REM-3 | reminders: all four follow-up scenarios |
| US-MED-17 | MED-9 | medication-tracking: "A reminder is raised for each upcoming slot of an active medication", "Upcoming slots are derived for every schedule type that schedules doses", "The reminder offers recording the dose as given and snoozing", "Snoozing records no dose and leaves the slot unsatisfied" |
| US-MED-18 | MED-9 | medication-tracking: "Only the device that configured the reminder is reminded" |
| US-MED-19 | MED-9 | medication-tracking: "Recording from the reminder opens the pre-filled confirmation", "The confirmed dose satisfies the slot" |
| US-MED-20 | MED-10 | medication-tracking: all four MED-10 scenarios |
| US-MED-21 | MED-11 | medication-tracking: all three MED-11 scenarios |
| US-EXP-01 | EXP-1 | export: "A selected range produces a formatted PDF covering weight, health and medications", "The selected range determines which entries are presented" |
| US-EXP-02 | EXP-1 | export: "Finished courses are presented alongside active ones", "Each medication is presented with its full dose history" |
| US-EXP-03 | EXP-2 | export: "A range produces raw delimited files for the entries" |
| US-EXP-04 | EXP-2 | export: "Medications and doses are written to dedicated files", "The dose file carries timestamp, amount and giver for every dose", "Raw files cover finished courses as well as active ones" |
| US-EXP-05 | EXP-3 | export: all four sharing scenarios |
| US-EXP-06 | EXP-4 | export: all six schedule-type scenarios |
| US-SET-04 | SET-3 (override) + NFR-5 (coverage) | settings-profile: "The chosen language is stored on the device and used on the next launch", "System follows the device's language"; product-quality: "The whole product is presented in German when German is the product language" |
| US-SET-05 | SET-3 | settings-profile: "A language change applies on the next launch, not in the running session", "The person is told the change applies on the next launch" |
| US-QUAL-01 | NFR-4 | product-quality: the two text-size scenarios (`[mobile]`, `[desktop]`) |
| US-QUAL-02 | NFR-4 | product-quality: "The weight chart exposes a summary of its latest value and trend direction", "The chart summary is read when the chart receives focus" |
| US-QUAL-03 | NFR-5 | product-quality: "The whole product is presented in German when German is the product language", "System-supplied labels are localized", "Metric units are the default" |
| RAB-8 (export clause, P1) | EXP-1 | export: "Export remains available for an archived rabbit" |
| NFR-8 (P0, export named) | referenced | export: "An export that cannot be produced presents no file" — the *cause-specific message* is P0's requirement, referenced and not restated |

Stories with no scenario in this change: **none** among the requirements this change owns.
US-MED-23 (offline dose logging) is **dropped** by decision D1 (P0) and has no scenario here.
US-SET-06 (display weight unit) is SET-4 and P0's. US-QUAL-04 … US-QUAL-09 belong to P3 (NFR-6),
P6 (NFR-7), P1 (NFR-9), P0 (NFR-2/NFR-3/NFR-10) and this change restates none of them.

Platform tagging: only where behaviour genuinely differs. The reminder mechanism carries the
`[ios]`/`[macos]` pair, because a reminder falling due while the product is not open is a different
lifecycle situation from one falling due while it stays running. The sharing handoff carries an
`[ios]`/`[macos]` pair, because the device's standard sharing surface differs between them. Export's
range/format behaviour is form-factor independent, so no `[mobile]`/`[desktop]` scenario is added;
the layout consequence (the range control at 390×844 and at 1280×800) is UI work in task 4.5, and
the one place where form factor does change behaviour — text scaling without breaking layout — is
tagged in NFR-4.

## Risks

| # | Risk | Mitigation in this change |
|---|---|---|
| R1 | **Reminders silently never fire** because notification authorization was denied. The person trusts a reminder that cannot be raised. | D4.7: the authorization state is a first-class, freshly observed value, presented where reminders are configured; task 5.13 asserts the surface, and no surface presents a configured reminder as guaranteed. |
| R2 | **A second reminder subsystem reappears (IMP-5)** — a medication-specific scheduler, store or follow-up loop, because medication slots are the harder case. | D4.1/D4.2 make kind the only difference; the delta states one mechanism as its own requirement; task 6.1 reviews specifically for a second persistence shape or a second follow-up path. |
| R3 | **A follow-up is raised for a target that was actually weighed/dosed, or is missed entirely.** Follow-up evaluation depends on the product running (D4.3). | The cancellation path is defined and asserted (tasks 5.4, 5.6); the residual gap is stated in D4.3 and recorded in `questions.md` Q6 rather than papered over. |
| R4 | **Reminders leak into shared data**, so one member's reminder notifies another (REM-2) or survives a household switch. | Reminders are device-local by construction and the delta asserts it (mechanism scenario "it is not part of the data any household member loads"); task 6.1 checks no household write carries a reminder. |
| R5 | **The generated document is wrong where it matters** — German umlauts corrupted, a course omitted, a dose history silently truncated. | D4.4 records the font/encoding requirement; tasks 5.9–5.11 assert content, among them a German-tagged export (5.11), and 5.19 takes the export through the sharing handoff end to end. |
| R6 | **A large range freezes the interface or exhausts memory.** | Generation runs off the main thread with a working state (D4.4); task 4.5 gives the UI that state, and the default range is a question (Q2) rather than an unbounded "all time" guess. |
| R7 | **The export leaves the device without the person choosing a destination** — a silent upload dressed up as a share. | EXP-3 states the files are handed to the device's standard sharing options and sent nowhere the person did not choose; the design forbids the product from having an upload path at all; task 6.1 checks no outbound call exists on the export path. |
| R8 | **The P4/P5 ordering is misread as duplication** — MED-9 … MED-11 specified twice, or the medication model restated here. | `proposal.md` explains the swap and the relocation; the `medication-tracking` delta header states P5's ownership and that it is ADDED-only; task 6.2 checks no P5 requirement is restated, modified or removed and that no `## MODIFIED Requirements` section exists anywhere in this change. |
| R9 | **Localization is claimed complete but is partial.** NFR-5 is a completion gate over every phase archived before this one (P0 … P3, P5). | Task 3.7 completes the tables across all those surfaces and task 5.16 asserts that neither language falls back to the other or to a key, so a missing string fails a test rather than shipping; task 4.9 checks German layout at both viewports. |
| R10 | **The accessible chart summary duplicates chart data or drifts from it.** | D4.6 derives it from the same series with no second query; task 5.16 asserts the summary follows the series with no additional data request. |
| R11 | **Archive order.** Landing this change before P5 would leave `medication-tracking` holding reminder requirements for an entity that does not exist. | Stated in `proposal.md` (archive order P0 → P1 → P2 → P3 → P5 → P4 → P6). |
| R12 | **Security surface drift** — a background mode or entitlement added "for reminders" that the phase does not need. | D4.5 forbids a background mode; task 1.7 must record where no entitlement is required and add none; task 6.1 checks the diff against it. |

## Verification

- Specs: `openspec validate add-vet-export-and-reminders --strict`.
- Gate (per `definition-of-done`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` (no diff in
  `src/bindings.ts`), `git diff --exit-code src/routeTree.gen.ts` after a build,
  `bun run typecheck`, `bun run lint`, `bun run test`, `bun run test:e2e`.
- Every requirement in this change has at least one scenario named by a test task in `tasks.md`.
- EXP-4 has an explicit test covering **all four** schedule types, with and without recorded doses
  (task 5.10).
- The accessible chart summary has a screen-reader scenario that is **tested**, not only
  implemented (task 5.21), including the no-value case.
- Cross-check at review: the delta contains exactly EXP-1 … EXP-4, REM-1 … REM-3, the unified
  mechanism requirement, MED-9 … MED-11, SET-3, NFR-4 and NFR-5 — ADDED only, with no
  `## MODIFIED Requirements` section, no P5 requirement restated and no requirement from another
  phase restated.
