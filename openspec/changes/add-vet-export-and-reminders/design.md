# Design — add-vet-export-and-reminders (P4)

## Context

This phase delivers after **P5** (see `proposal.md`): the medication and `Schedule` model, the
human-readable schedule summary (MED-4) and the record-a-dose flow with its pre-filled
confirmation (MED-5, MED-8) already exist when this change starts, and with them MED-13's
correction path (referenced, not restated) — so a course can be finished and later be active again,
and the reminder consequences of both directions are stated here rather than in P5. MED-9 … MED-11
are specified here because decision **D3** makes reminders *one* mechanism, so medication reminders are
configurations of the same mechanism as weigh reminders rather than a second subsystem (IMP-5).

Three of the four things this phase adds are surfaces on the *device* rather than on the shared
household record: reminders, the language preference, and the notification authorization state.
Export is the fourth: it *reads* the household record and produces files locally. So the phase's
central design question is not "what data do we store" but **where the boundary between
device-local configuration, household data and produced files runs**, and how the product stays
honest about two device states it does not control (notification authorization, and whether the
platform grants it background execution at all — D4.3/D4.7/D4.15).

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
| Schedule | user-defined recurring day(s) + time | derived from the medication's P5 schedule for every type that schedules doses, for as long as the medication is active — re-derived if it is finished and then becomes active again (D4.2) |
| Follow-up default | off | **on** (MED-10) |
| Actions | open the product at the target | record the dose as given (opens P5's pre-filled confirmation, MED-8) or snooze (a fixed 10-minute deferral, repeatable, never satisfying the slot — D4.10) |

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
occurrence exists at all. **That status is read, never remembered:** MED-13 keeps a finished
course's end date correctable, and correcting it — to a future date, or away — makes the medication
active again, so finishing a course cancels its occurrences for as long as it stays finished and
its return to active schedules them again for its upcoming slots (MED-9). No stored `finished` flag
and no one-way cancellation may leave an active medication silently reminder-less. This design adds
no second schedule interpreter: if P5's summary says "every 3 days from 12 Oct at 09:00", the
occurrences are exactly those instants. That is what makes
EXP-4's "export succeeds for any schedule type" and MED-9's "slots derived for every schedule type"
the same underlying question, answered once.

### D4.3 — Follow-up evaluation: schedule ahead, reconcile in a background window, cancel on evidence

**A background mode IS requested.** This **reverses an earlier decision of this change**: the first
draft of this document asked for no background mode and accepted the resulting limitation instead, and
the background-mode question formerly recorded in `questions.md` recorded whether that acceptance was
final. It is not final — the product owner has settled it (that question is answered and closed, and
`questions.md` now records no open question), so the product asks for background execution precisely so
it can reconcile and cancel pending follow-ups **while it is suspended**. The two rejected alternatives
are recorded here so they are not reintroduced: accepting the limitation with no background mode at
all, and raising follow-ups opportunistically only when the product happens to be running.

**It is not a second background subsystem.** `add-household-sharing-hardening` (P3) already specifies
background delivery for HS-11 — "changes reach every member without the product being opened manually" —
with its own platform story: the same background-mode/backing-task declaration and entitlement
(`tauri-security` task 1.3), the same native plugin and its registration (`apple-native` tasks 1.6 and
1.7, unit-tested at 1.8 and manually verified at 5.15), and the same Rust handoff into the sync layer
(`rust-backend` task 2.10, design D-H6). P3 archives **before** this change, so this change **consumes
and extends** that mechanism: **same entitlement, same native plugin, same platform story**. The
extension is one seam — the window handler that P3's registration already runs also invokes this
change's reconciliation — and it is deliberately not a second registration, a second mode, a second
entitlement, a second plugin or a second reconciliation path (D4.5, R12). Tasks 1.13–1.15 add that
extension; 2.9/2.16 and 3.9 add the reconciliation that runs inside it.

The follow-up is therefore handled in three parts:

1. **Schedule ahead.** When a reminder occurrence is scheduled, its follow-up is scheduled with it, for
   the following day (weigh) or the kind's follow-up moment (medication). This is what makes the
   follow-up possible at all when the product is not running, and it is also what still raises the
   follow-up when the platform grants no background execution at all: the follow-up sits on the
   device's own notification scheduler, not on product code.
2. **Reconcile in a background execution window.** When the platform grants the product a background
   execution window, the product reconciles outstanding occurrences against the locally kept copy of
   the household data (P0's loaded-data read path): a weigh occurrence with a weight entry logged for
   its target after it fired is marked `satisfied` and its follow-up cancelled; a medication occurrence
   with a dose recorded for its slot is `satisfied` likewise. Cancellation is a local operation on the
   device's own scheduled notifications, so it needs no connectivity. This is the half the earlier,
   no-background-mode design could not do while suspended, and it is the only thing *this change* uses
   the window for; P3's own use of it is unchanged.
3. **Cancel on evidence, reconcile on launch/foreground.** The **same** reconciliation runs on every
   launch and foreground return — one implementation, three triggers, no second path.

**The limit, stated plainly.** iOS decides whether and when a granted background execution window
actually runs, and it may not run at all: the grant is an opportunity, not a schedule and not a
guarantee. So this **narrows** the gap between "a follow-up should fire" and "it reliably fires" — a
follow-up whose window was satisfied on another device is now cancelled without the person opening the
product in most cases — but it does **not** close it. A device that is never granted a window, or that
is not run again before the follow-up falls due, can still raise a follow-up for a window that was in
fact satisfied elsewhere. That residual gap stays in the risk register (**R3**) and is stated in
D4.7 rather than papered over; no surface presents a follow-up as guaranteed, and the requirement's
scenario carries the qualifier *subject to the platform granting background execution*.

**macOS is different, and is described differently.** macOS does not suspend the product the way iOS
does: it keeps running, so reconciliation happens on the ordinary launch/foreground path and while it
runs. On macOS the background window is a convenience, not the mechanism, and the delta says so instead
of claiming for macOS what is really a claim about iOS.

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

### D4.5 — One background mechanism, consumed from P3, and no second always-on surface

The background execution this phase needs for reminder reconciliation (D4.3) is **P3's background
mechanism, not a new one**. `add-household-sharing-hardening` declares the background mode, the backing
task and the `Info.plist`/entitlement configuration for HS-11 and implements the native registration
that runs a window handler (`tauri-security` task 1.3; `apple-native` tasks 1.6–1.7, unit-tested at 1.8,
manually verified at 5.15). This change extends that one seam: the same declaration and the same
registration gain the reconciliation entry point of D4.3 and nothing else. `tauri-security` therefore
adds **no** second background mode, **no** second entitlement and **no** plugin beyond P3's; task 1.13
confirms the extension and task 6.1 reviews the diff for a
second background subsystem or a second reconciliation path. That is why the background-mode question is closed by consuming P3
rather than by declaring new platform work.

Two things stay deliberately absent. Nothing keeps the product running continuously: there is no
always-on surface, no polling loop and no long-lived background task, and the background window is
used only to reconcile (D4.3). And no code path assumes a window will ever be granted — the
schedule-ahead half of D4.3 stands whether or not one arrives, which is why the platform limit is a
risk (R3) and not a premise.

**Dependencies: none added.** No new dependency and no new plugin is introduced beyond what this change
already names (the document-generation dependency of D4.4 and the notification and file-handoff
bridges); the background mechanism is P3's, so project hard rule 6 needs no new justification here.

### D4.6 — The accessible chart summary is derived, never stored

The summary is computed from the **same weight series the chart renders** — latest value in the
display unit, and the trend direction derived from the same values — and exposed as the chart's
accessible label. There is no second query, no stored summary and no second source of chart data, so
the summary cannot disagree with what is on screen; it is recomputed when the series changes
(NFR-4's "no second data source" scenario). The definition of "trend direction" and the no-value and
one-entry wording are recorded as **D4.14**; the requirement itself says the summary exists, that the
trend is direction-only, and that it never invents a direction it cannot support.

### D4.7 — Device states the product must not pretend about

Three device states can make a reminder silently useless, and all are surfaced rather than assumed:

- **Notification authorization.** A device that has not granted it cannot raise reminders. The
  product presents the authorization state where reminders are configured and does not present a
  configured reminder as if it will certainly fire, and the configuration stays usable (D4.15). This
  is the same honesty stance P0 fixed for service status (NFR-6's requirement belongs to P3; the
  stance is reused, not restated) — **extended from the service to notifications**, stated in the
  requirement rather than only here. The authorization itself is **P3's** one surface, observed
  through `get_notification_authorization()` and requested through
  `request_notification_authorization()` as the one `NotificationAuthorization` state, consumed and
  not re-declared (D4.15).
- **Background execution.** The product asks for it (D4.3/D4.5), but iOS decides whether and when a
  granted window actually runs, and may never run one. Reconciliation therefore happens when it
  happens, and the product presents no follow-up as guaranteed to reflect evidence recorded
  elsewhere — the same stance as the authorization state above. The residual gap is carried as R3.
  macOS keeps the product running, so this state does not arise there in the same form.
- **The product not running.** The schedule-ahead half of D4.3 covers what can be covered without the
  product running, and D4.3's reconciliation covers the rest only while a window is granted.

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

### D4.10 — Snooze defers by a fixed 10 minutes, may repeat, and never satisfies the slot

**Settled by the product owner** (former `questions.md` Q1). A snooze defers the occurrence by a
**fixed 10 minutes** and may be snoozed **repeatedly**; it records nothing and **never satisfies the
slot**. MED-9's rule that a snooze is not a dose is therefore preserved in full, and a snooze that
runs past the slot's dose window leaves the slot **missed**, so MED-10's missed-dose follow-up still
applies to it. The occurrence state set already carries `snoozed` and `missed` (D4.1), so this is a
rule about transitions, not a new state: `pending → fired → snoozed → fired … → missed` when the
snooze crosses the window, and never `satisfied` without a recorded dose. The delay is a named
constant, so a test can assert it; the weigh kind has no snooze action today and is untouched.

### D4.11 — The export range opens on everything the rabbit has

**Settled by the product owner** (former Q2). The range control opens on **everything the rabbit
has** — from its earliest history to its latest — and offers the weight chart's **7-day, 30-day,
90-day and all-history** presets (WT-6, P1) alongside the rabbit's full span. The span comes from
`get_export_span(rabbit_id)`; the presets are the chart's own set rather than an independent one, so
the product has one vocabulary of ranges. This is the default, not a cap: the person can still narrow
the range, and R6's "a large range must not freeze the interface" is handled by D4.4's off-main-thread
generation and the working state rather than by a smaller default.

### D4.12 — The range selects entries and courses; a presented course reports every dose (confirmed)

**Confirmed by the product owner** (former Q3) as the reading this delta already took, so it is
recorded as a decision rather than an assumption. The selected range decides **which entries and
which courses are presented**; a course that is presented reports **every dose it has**. "Full" is
therefore not narrowed by the range: a vet reading "Baytril, 12 doses" is never handed a dose list
that silently omits the out-of-range ones. The alternative reading — "full within the range" — is
rejected, because a silently shortened dose history is a worse defect than a longer list.

### D4.13 — Raw files carry the canonical stored values; the PDF stays formatted (confirmed)

**Confirmed by the product owner** (former Q4). The raw CSV carries the **canonical stored values** —
grams, amounts and units exactly as recorded — with **no presentation formatting** and no conversion
to the person's display unit. The **PDF stays formatted** in the person's display unit, as any screen
would, because it is a document a vet reads rather than a data interchange format. The display-unit
boundary (SET-4, P0) therefore applies to the presentation boundary the PDF is, and deliberately not
to the raw path, which is what makes the two formats complementary rather than duplicates.

### D4.14 — Trend direction is latest-versus-previous, direction only, and never invented

**Settled by the product owner** (former Q5). The accessible summary's trend compares the **latest
entry with the previous one** and states **direction only**, claiming **no magnitude** — so no
percentage, no rate and no threshold appear in the wording. With **one** entry, the summary reports
that value and says there is **no trend yet**; with **none**, it says there is nothing to summarise
using NFR-5's **"never weighed"** vocabulary established for the no-value case. A direction is never
invented where the data cannot support one. The question behind this decision offered "a comparison
with the range's first entry" and "a threshold" as alternatives; both are rejected — the first would
claim a trend from two points that do not describe one, and the second would need a magnitude the
product does not own.

### D4.15 — Notification authorization is presented honestly; the configuration stays usable

**Settled by the product owner** (former Q6). When the device's notification authorization is denied,
not yet determined or **revoked**, the product presents that state honestly and keeps the reminder
configuration **usable**: a reminder can still be configured, edited, enabled, disabled and deleted,
but it is never presented as one that will fire. The state it presents is P3's `NotificationAuthorization`
— `unknown` (not yet observed), `not_determined`, `granted` or `denied` — so a state that has not yet
been observed is never presented as a granted or a denied one. This is the same honesty stance P0 fixed
for service status (NFR-6 is P3's requirement; the stance is reused, not restated), now **extended from
the service to notifications** — the requirement states this rather than leaving it here, so it is
testable. The rejected alternative is refusing to save a reminder that cannot fire, which would make
the configuration hostage to a state the person can change at any time.

**This change declares no authorization surface of its own.** The notification authorization — the
permission and the request/observation entry points, introduced by `add-household-sharing-hardening`
for its visible background-delivery notifications (its `tauri-security` task 1.3 declaration; HS-11's
background delivery, D-H6) — is **P3's**. It is a runtime authorization request on both iOS and macOS,
so it carries **no `Info.plist` usage-description string and no entitlement** (P3's D-H14): none is
declared here and none is invented. P3's names are used here unchanged, because they are the same fact
on the same platform: `request_notification_authorization()` asks, `get_notification_authorization()`
observes, and `NotificationAuthorization`
(`{ state: unknown | not_determined | granted | denied, observed_at }`) is the one state both phases
present. This change **consumes** that one authorization, so there is one permission, one
authorization state and one observation path across both phases, and no reminder-named authorization
command or type of its own; task 1.8 declares no second permission, no second usage description and no
entitlement, and task 6.1 reviews for exactly that. Scheduling and cancelling local notifications,
registering the record and snooze actions and delivering an action taken while the product is not open
are this change's own bridge (tasks 1.9/1.10) — that is delivery of this phase's
notifications, not a second authorization.

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
                              notification authorization (P3's: request_notification_authorization() /
                              get_notification_authorization()), scheduling and actions;
                              the background execution window that P3 declares and
                              registers, extended with the reminder reconciliation
                              entry point (iOS / macOS; future targets add a bridge)

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
- **One background mechanism, P3's.** The background execution window used for reminder
  reconciliation (D4.3) is the declaration, the plugin and the registration P3 establishes for HS-11,
  extended with one reconciliation entry point — never a second background subsystem (D4.5, R12).
- **One notification authorization, P3's.** The notification permission and the authorization state
  are P3's single surface — `request_notification_authorization()` and
  `get_notification_authorization()` over the one `NotificationAuthorization` state — consumed here and
  never re-declared; no second permission, no second usage description and no entitlement is declared
  here, because P3's authorization is a runtime request requiring neither (D-H14); this change adds
  only the scheduling, cancellation and action delivery of its own notifications (D4.15, R12).

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
| `get_notification_authorization()` | **Consumed from P3** (`add-household-sharing-hardening`, D-H14), not declared here: this device's notification authorization state, freshly observed through P3's one observation path and presented honestly (D4.15) | D4.7, D4.15 |
| `request_notification_authorization()` | **Consumed from P3**, not declared here: ask for notification authorization through P3's one request path (no second permission, no second usage description and no entitlement declared here — P3's authorization is a runtime request needing neither, D-H14); the outcome is reported honestly | D4.7, D4.15 |
| `snooze_reminder(reminder_id, occurrence_at)` | Defer an occurrence by a fixed 10 minutes (repeatable) without recording anything; the slot stays unsatisfied, and a snooze past the window leaves it missed | MED-9, D4.10 |
| `resolve_reminder_slot(reminder_id, occurrence_at)` | Mark an occurrence satisfied after P5's confirmation recorded the dose | MED-9 |
| `reconcile_reminders()` | Reconcile occurrences against loaded data on launch, on foreground return and inside P3's background execution window; cancel follow-ups whose evidence exists | REM-3, MED-10, D4.3 |
| `export_rabbit_history(rabbit_id, range, format)` | Produce the export; `format = pdf \| csv`; returns the produced artifacts (one for `pdf`, several for `csv`) | EXP-1, EXP-2, EXP-4 |
| `get_export_span(rabbit_id)` | The earliest and latest instants the rabbit has history for, so the range control defaults to everything the rabbit has | EXP-1 (D4.11) |
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
| `reminder:notification-authorization-changed` | `{ state }` — the `state` member of P3's `NotificationAuthorization` | Keep the authorization surface truthful when it changes while the product runs |
| `export:state-changed` | `{ export_id, state: generating \| ready \| failed }` | Drive the working state and the produced-file list |

Events are additive to P0's `household:changed` and `service:status-changed`, which are unchanged.
Because reminders are device-local, `reminders:changed` is deliberately **not** household-scoped. A
reconciliation performed inside a background execution window (D4.3) is the same reconciliation and
emits the same `reconciled` kind; the background path therefore adds **no command and no event of its
own**, and the native window handler reaches the Rust layer without new IPC surface — exactly as P3's
background-delivered change does (P3 D-H6).

### Shared types

| Type (proposed) | Shape | Notes |
|---|---|---|
| `ReminderId` | opaque, stable string | device-local identity |
| `ReminderKind` | `weigh \| medication` | D3; the only difference between the two kinds |
| `ReminderTarget` | `{ kind: household \| rabbit \| medication, id? }` | `id` absent for household-wide (REM-1) |
| `ReminderSchedule` | `{ kind: recurring_days, weekdays, time_of_day }` | the weigh kind's user-defined schedule; a medication reminder derives its occurrences from P5's `Schedule` instead of carrying one (D4.2) |
| `ReminderConfig` | `{ id, kind, target, schedule?, medication_id?, created_by_member_id, follow_up_enabled, enabled, authorization? }` | one shape for both kinds (D4.1) |
| `ReminderOccurrence` | `{ reminder_id, due_at: Instant, state: pending \| fired \| snoozed \| satisfied \| missed }` | the unit the mechanism schedules and reconciles (D4.1) |
| `NotificationAuthorization` | `{ state: unknown \| not_determined \| granted \| denied, observed_at }` | **Consumed from P3**, not declared here (D-H14); this change presents the state honestly rather than assuming it. `unknown` is "not yet observed" — the state a status starts in — and is distinct from `not_determined`, an observed outcome (the person has not been asked); D4.7/D4.15 |
| `ReminderAction` | `record \| snooze` | MED-9's two offered actions |
| `ExportFormat` | `pdf \| csv` | EXP-1, EXP-2 |
| `ExportRange` | `{ from: Instant, to: Instant }` | the selectable range (EXP-1) |
| `ExportArtifact` | `{ export_id, format, file_name, role: weight \| health \| medication \| dose \| document, created_at }` | `role` is what makes EXP-2's dedicated medication and dose files assertable |
| `ExportBundle` | `{ rabbit_id, range, artifacts: [ExportArtifact] }` | one export, however many files it produced |
| `ExportState` | `generating \| ready \| failed` | drives the working state (D4.4) |
| `LanguagePreference` | `system \| de \| en` | SET-3, per device |

Reused, not redefined: P0's `CommandError`, `SaveCause`, `Instant`, `MemberId`, `HouseholdId`;
P1's `RabbitId` and weight-entry types; **P5's `MedicationId`, `DoseId`, `Medication`, `Schedule`
and `Dose`** — this change consumes P5's model and adds no parallel medication or schedule type; and
**P3's `NotificationAuthorization` with its `get_notification_authorization()` and
`request_notification_authorization()` commands** — consumed as they are, with no reminder-named
authorization command or type declared here (D4.15).

## Traceability: user stories → scenarios

| Story | Requirement(s) | Capability / scenario |
|---|---|---|
| — (D3, IMP-5) | unified mechanism | reminders: "Both kinds of reminder are created and listed through one mechanism", "Both kinds carry the same configuration and the same day-to-day operations", "Reminders live on the device that created them and survive a relaunch", "A reminder fires while the product is not open `[ios]`", "A reminder fires while the product stays running `[macos]`" |
| US-REM-01 | REM-1 | reminders: "A reminder configured for a recurring day and time fires on schedule", "An edited reminder is honoured with its new schedule", "A disabled or deleted reminder raises nothing" |
| US-REM-02 | REM-1 | reminders: "A household-wide reminder covers the household rather than one rabbit" |
| US-REM-03 | REM-2 | reminders: "Only the creating person's device is reminded", "Another member is never notified by a reminder they did not create", "A second member configures an independent reminder", "One person's reminder change leaves another member's reminder untouched", "A reminder does not follow the person to another device" |
| US-REM-04 | REM-3 | reminders: all seven follow-up scenarios, including the `[ios]`/`[macos]` reconciliation scenarios |
| US-MED-17 | MED-9 | medication-tracking: "A reminder is raised for each upcoming slot of an active medication", "Upcoming slots are derived for every schedule type that schedules doses", "The reminder offers recording the dose as given and snoozing", "Snoozing records no dose and leaves the slot unsatisfied", "A snooze may be repeated without satisfying the slot", "A snooze that runs past the dose window leaves the slot missed" (D4.10), "A reminder states honestly that notifications are not authorized", "Revoking authorization leaves the reminder configuration usable" (D4.15) |
| US-MED-18 | MED-9 | medication-tracking: "Only the device that configured the reminder is reminded" |
| US-MED-19 | MED-9 | medication-tracking: "Recording from the reminder opens the pre-filled confirmation", "The confirmed dose satisfies the slot" |
| US-MED-20 | MED-10 | medication-tracking: all seven MED-10 scenarios, including the `[ios]`/`[macos]` reconciliation scenarios |
| US-MED-21 | MED-11 | medication-tracking: all three MED-11 scenarios |
| US-EXP-01 | EXP-1 | export: "A selected range produces a formatted PDF covering weight, health and medications", "The selected range determines which entries and courses are presented", "The range control opens on everything the rabbit has" (D4.11) |
| US-EXP-02 | EXP-1 | export: "Finished courses are presented alongside active ones", "Each medication is presented with its full dose history" |
| US-EXP-03 | EXP-2 | export: "A range produces raw CSV files for the entries", "The raw files carry stored values while the PDF is formatted" (D4.13) |
| US-EXP-04 | EXP-2 | export: "Medications and doses are written to dedicated files", "The dose file carries timestamp, amount and giver for every dose", "Raw files cover finished courses as well as active ones" |
| US-EXP-05 | EXP-3 | export: all four sharing scenarios |
| US-EXP-06 | EXP-4 | export: all six schedule-type scenarios |
| US-SET-04 | SET-3 (override) + NFR-5 (coverage) | settings-profile: "The chosen language is stored on the device and used on the next launch", "System follows the device's language"; product-quality: "The whole product is presented in German when German is the product language" |
| US-SET-05 | SET-3 | settings-profile: "A language change applies on the next launch, not in the running session", "The person is told the change applies on the next launch" |
| US-QUAL-01 | NFR-4 | product-quality: the two text-size scenarios (`[mobile]`, `[desktop]`) |
| US-QUAL-02 | NFR-4 | product-quality: "The weight chart exposes a summary of its latest value and trend direction", "A chart with a single entry reports no trend yet", "A chart with no values summarises that instead of a value", "The chart summary is read when the chart receives focus" |
| US-QUAL-03 | NFR-5 | product-quality: "The whole product is presented in German when German is the product language", "System-supplied labels are localized", "Metric units are the default" |
| RAB-8 (export clause, P1) | EXP-1 | export: "Export remains available for an archived rabbit" |
| NFR-8 (P0, export named) | referenced | export: "An export that cannot be produced presents no file" — the *cause-specific message* is P0's requirement, referenced and not restated |

Stories with no scenario in this change: **none** among the requirements this change owns.
US-MED-23 (offline dose logging) is **dropped** by decision D1 (P0) and has no scenario here.
US-SET-06 (display weight unit) is SET-4 and P0's. US-QUAL-04 … US-QUAL-09 belong to P3 (NFR-6),
P6 (NFR-7), P1 (NFR-9), P0 (NFR-2/NFR-3/NFR-10) and this change restates none of them.

Platform tagging: only where behaviour genuinely differs. The reminder mechanism carries the
`[ios]`/`[macos]` pair, because a reminder falling due while the product is not open is a different
lifecycle situation from one falling due while it stays running. The follow-up reconciliation carries
the same pair for the same reason — on iOS it runs inside a background execution window that the
platform may or may not grant (D4.3), while macOS keeps the product running and reconciles as it goes —
so no scenario claims for macOS what is really a claim about iOS. The sharing handoff carries an
`[ios]`/`[macos]` pair, because the device's standard sharing surface differs between them. Export's
range/format behaviour is form-factor independent, so no `[mobile]`/`[desktop]` scenario is added;
the layout consequence (the range control at 390×844 and at 1280×800) is UI work in task 4.5, and
the one place where form factor does change behaviour — text scaling without breaking layout — is
tagged in NFR-4.

## Risks

| # | Risk | Mitigation in this change |
|---|---|---|
| R1 | **Reminders silently never fire** because notification authorization was denied or later revoked. The person trusts a configured reminder that cannot be raised. | **Q6 is settled (D4.15)**: the authorization is **P3's** one surface — `get_notification_authorization()` observing the one `NotificationAuthorization` state, `request_notification_authorization()` asking for it — observed freshly and presented where reminders are configured, and the configuration stays usable while no reminder is presented as one that will fire — the honesty P0 fixed for service status extended from the service to notifications. The requirement and its scenarios state it (MED-9: "A reminder states honestly that notifications are not authorized", "Revoking authorization leaves the reminder configuration usable"), task 5.13 asserts the surface and tasks 3.2/4.1 implement it; no surface presents a configured reminder as guaranteed. This narrows the risk to a person who ignores a state the product shows plainly; it is not eliminated, because the product does not control the granted state. |
| R2 | **A second reminder subsystem reappears (IMP-5)** — a medication-specific scheduler, store or follow-up loop, because medication slots are the harder case. | D4.1/D4.2 make kind the only difference; the delta states one mechanism as its own requirement; task 6.1 reviews specifically for a second persistence shape or a second follow-up path. |
| R3 | **A follow-up is raised for a window that was satisfied elsewhere, or is missed entirely.** The requested background mode (D4.3) **narrows** this gap but does not close it: iOS decides whether and when a granted background execution window actually runs and may never run one, so a follow-up already scheduled can fall due before any reconciliation, and a device that is never granted a window never reconciles while suspended. | The cancellation path is defined once and asserted — schedule-ahead plus reconciliation from three triggers (tasks 2.9/2.16, 3.9) — and covered by 5.4/5.6 plus the background reconciliation tests (5.22) and the manual simulator step (5.24); the residual gap is stated in D4.3 and D4.7 and **carried here in this register** rather than papered over, and no surface presents a follow-up as guaranteed (task 4.1's honesty requirement, D4.7). |
| R4 | **Reminders leak into shared data**, so one member's reminder notifies another (REM-2) or survives a household switch. | Reminders are device-local by construction and the delta asserts it (mechanism scenario "it is not part of the data any household member loads"); task 6.1 checks no household write carries a reminder. |
| R5 | **The generated document is wrong where it matters** — German umlauts corrupted, a course omitted, a dose history silently truncated. | D4.4 records the font/encoding requirement; tasks 5.9–5.11 assert content, among them a German-tagged export (5.11), and 5.19 takes the export through the sharing handoff end to end. |
| R6 | **A large range freezes the interface or exhausts memory.** | Generation runs off the main thread with a working state (D4.4); task 4.5 gives the UI that state, and the default range is settled (D4.11: everything the rabbit has, offered with the chart's 7d/30d/90d/all presets) rather than an unbounded "all time" guess — the working state, not a smaller default, is what keeps a large range usable. |
| R7 | **The export leaves the device without the person choosing a destination** — a silent upload dressed up as a share. | EXP-3 states the files are handed to the device's standard sharing options and sent nowhere the person did not choose; the design forbids the product from having an upload path at all; task 6.1 checks no outbound call exists on the export path. |
| R8 | **The P4/P5 ordering is misread as duplication** — MED-9 … MED-11 specified twice, or the medication model restated here. | `proposal.md` explains the swap and the relocation; the `medication-tracking` delta header states P5's ownership and that it is ADDED-only; task 6.2 checks no P5 requirement is restated, modified or removed and that no `## MODIFIED Requirements` section exists anywhere in this change. |
| R9 | **Localization is claimed complete but is partial.** NFR-5 is a completion gate over every phase archived before this one (P0 … P3, P5). | Task 3.7 completes the tables across all those surfaces and task 5.16 asserts that neither language falls back to the other or to a key, so a missing string fails a test rather than shipping; task 4.9 checks German layout at both viewports. |
| R10 | **The accessible chart summary duplicates chart data or drifts from it.** | D4.6 derives it from the same series with no second query; task 5.16 asserts the summary follows the series with no additional data request. |
| R11 | **Archive order.** Landing this change before P5 would leave `medication-tracking` holding reminder requirements for an entity that does not exist. | Stated in `proposal.md` (archive order P0 → P1 → P2 → P3 → P5 → P4 → P6). |
| R12 | **A second background subsystem or entitlement appears "for reminders"** instead of consuming P3's mechanism — a second background mode, a second plugin, a second registration or a second reconciliation path; or a **second notification authorization** appears instead of consuming P3's one surface. | D4.5 states that the background mechanism is P3's and names what may be extended (the window handler entry point) and what may not (mode, entitlement, plugin, registration, reconciliation path); D4.15 states that the notification authorization, its permission and its two commands over the one `NotificationAuthorization` state (`get_notification_authorization()` / `request_notification_authorization()`) are P3's single surface — a runtime authorization request carrying no usage-description string and no entitlement (P3's D-H14) — and that no reminder-named authorization command or type is declared here; task 1.13 declares the background extension and adds nothing new, task 1.8 declares no second notification permission, usage description or entitlement, task 2.16 reuses the one reconciliation implementation; task 6.1 checks the diff for a second background subsystem, a second entitlement, a second notification authorization and a second follow-up path. |
| R13 | **A medication returned to active stays silent.** Finishing a course cancels its occurrences, and MED-13 lets a finished course's end date be corrected so the course becomes active again; if the cancellation is treated as one-way, or occurrences are derived from a remembered `finished` state, the medication raises nothing at the slots it is scheduled for while every surface shows it as active. | MED-9 scopes the cancellation to **as long as the course remains finished** and requires the occurrences to be scheduled again on return to active; D4.2 derives occurrences from the medication's **current** status on every evaluation with no stored `finished` flag; task 5.5 covers "A course returned to active resumes its upcoming reminders" alongside the finished-course scenarios, and task 6.2 checks MED-13 is referenced rather than restated here. |

## Verification

- Specs: `openspec validate add-vet-export-and-reminders --strict`.
- Gate (per `definition-of-done`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo check --target aarch64-apple-ios`, `bun run gen:bindings` (no diff in
  `src/bindings.ts`), `git diff --exit-code src/routeTree.gen.ts` after a build,
  `bun run typecheck`, `bun run lint`, `bun run test`, `bun run test:e2e`.
- Every requirement in this change has at least one scenario named by a test task in `tasks.md`.
- The six settled decisions are testable: the snooze delay, repeatability and the missed slot (task
  5.8, D4.10); the default range and its presets (tasks 5.9/5.14, D4.11); the range-versus-doses reading
  (task 5.9, D4.12); the canonical raw values against the formatted PDF (task 5.11, D4.13); the trend
  definition and the one/zero-entry wording (tasks 5.16/5.21, D4.14); and the authorization surface
  consumed from P3 (`NotificationAuthorization`, `get_notification_authorization()` /
  `request_notification_authorization()`, task 5.13, D4.15).
- EXP-4 has an explicit test covering **all four** schedule types, with and without recorded doses
  (task 5.10).
- The accessible chart summary has a screen-reader scenario that is **tested**, not only
  implemented (task 5.21), including the no-value case and the single-entry no-trend case (D4.14).
- The background path has no automated native coverage at phase 1 (`testing-strategy`: native
  lifecycle behaviour is not drivable by `tauri-driver` on iOS/macOS). It is covered by Rust-level
  tests of the reconciliation entry point and the follow-up scenarios it decides (task 5.22) plus a
  **manual simulator step on iOS and macOS** (task 5.24), in addition to P3's own manual step (5.15)
  for the mechanism it reuses.
- Cross-check at review: the delta contains exactly EXP-1 … EXP-4, REM-1 … REM-3, the unified
  mechanism requirement, MED-9 … MED-11, SET-3, NFR-4 and NFR-5 — ADDED only, with no
  `## MODIFIED Requirements` section, no P5 requirement restated and no requirement from another
  phase restated; and **no second notification authorization or permission appears anywhere** — the
  notification authorization consumed here is P3's single surface, a runtime request that carries no
  usage-description string and no entitlement (D4.15).
