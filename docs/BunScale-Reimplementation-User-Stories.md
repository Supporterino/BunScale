# BunScale — Reimplementation User Stories

> **Companion to:** `BunScale-Reimplementation-Requirements.md`
> **Scope:** All shipped capabilities except developer-only demo tooling.
> **Purpose:** Backlog for a ground-up reimplementation, expressed as user stories with
> acceptance criteria, priorities, estimates, and traceability to requirement IDs.
> **Stance:** **Technology-agnostic.** No implementation choice, vendor, or integration is
> referenced. "The shared service" means whatever mechanism provides multi-person
> sharing — it is deliberately not a product name.

---

## How to read this document

**Story format**

```
### US-<AREA>-<n> — <Title>
As a <persona>, I want <capability>, so that <benefit>.
Priority: Must | Should | Could
Estimate: <points>   Traces: <REQ IDs>
Acceptance criteria:
- GIVEN … WHEN … THEN …
```

**Priority (MoSCoW used as delivery order, not scope cut)**
- **Must** — required for the product to be considered functional.
- **Should** — important, expect in a complete reimplementation.
- **Could** — valuable but deferrable.

**Estimates** are relative story points (1/2/3/5/8). They size the *story*, not the
reimplementation effort; treat them as ordering signal only.

**Personas:** *Owner* (primary caregiver, created the household), *Member* (co-owner /
partner with their own account), *User* (generic — either of the above).

**Legend:** `⚠` marks a story that is **blocked on an open decision** (see §13).

---

## Epic index

| Epic | Area | Requirements |
|---|---|---|
| EP-1 | Household & Sharing | HS-1 … HS-13 |
| EP-2 | Synchronization & Offline | SYNC-1 … SYNC-7 |
| EP-3 | Dashboard & Navigation | DASH-1 … DASH-9 |
| EP-4 | Rabbit Profiles & Lifecycle | RAB-1 … RAB-10 |
| EP-5 | Weight Tracking | WT-1 … WT-8 |
| EP-6 | Health Logging | HL-1 … HL-7 |
| EP-7 | Medication & Dosing | MED-1 … MED-12 |
| EP-8 | Photos | PHOTO-1 … PHOTO-5 |
| EP-9 | Reminders | REM-1 … REM-3 |
| EP-10 | Export | EXP-1 … EXP-4 |
| EP-11 | Settings & Profile | SET-1 … SET-4 |
| EP-12 | Cross-cutting Quality | NFR-1 … NFR-10 |

---

## EP-1 — Household & Sharing

> **Goal:** several people, each with their own account, jointly own and edit one set of
> rabbit data; a household can be created, shared, joined, switched, and deleted.

### US-HS-01 — Create the first household
**As a** first-time user, **I want** to create a named household on first launch, **so that** I have a home for my rabbits.
**Priority:** Must · **Estimate:** 3 · **Traces:** HS-1, HS-2
**Acceptance criteria:**
- GIVEN the product is launched with no household, WHEN I complete onboarding and enter a name, THEN a household is created, I am its first member, it becomes active, and the dashboard shows its (empty) rabbit list.
- GIVEN a household already exists, WHEN I launch, THEN the dashboard is shown directly without onboarding.

### US-HS-02 — Household creation requires a name
**As a** user, **I want** to be stopped from creating a nameless household, **so that** I can identify it later.
**Priority:** Must · **Estimate:** 1 · **Traces:** HS-1
**Acceptance criteria:**
- GIVEN I am creating a household, WHEN I submit a blank name, THEN no household is created and I am prompted for a name.

### US-HS-03 — Multiple households coexist
**As a** user, **I want** more than one household, **so that** I can keep separate sets of rabbits.
**Priority:** Should · **Estimate:** 3 · **Traces:** HS-3
**Acceptance criteria:**
- GIVEN a household with rabbits exists, WHEN I create a second household, THEN both coexist and their rabbits are never mixed.

### US-HS-04 — Switch the active household
**As a** user, **I want** to choose which household is active, **so that** the dashboard shows the right rabbits.
**Priority:** Should · **Estimate:** 2 · **Traces:** HS-3, HS-4
**Acceptance criteria:**
- GIVEN two households exist, WHEN I select the other one, THEN it becomes active and the dashboard shows only its rabbits.

### US-HS-05 — Create another household from Settings
**As a** user, **I want** to create a household from Settings without redoing onboarding, **so that** I can add one mid-use.
**Priority:** Should · **Estimate:** 2 · **Traces:** HS-4
**Acceptance criteria:**
- GIVEN a household exists and is active, WHEN I create a new one from Settings with a name, THEN it is created with me as first member and becomes active; blank names are refused.

### US-HS-06 — Invite a member via an invitation link
**As an** owner, **I want** to invite another person via a link, **so that** they can share care duties.
**Priority:** Must · **Estimate:** 5 · **Traces:** HS-5, HS-11
**Acceptance criteria:**
- GIVEN a household below the member cap and the shared service available, WHEN I choose to invite, THEN the product keeps working, shows progress, and presents the standard sharing options.
- GIVEN the recipient opens the link and accepts with their own account, THEN they become a member with read/write access to all rabbits.

### US-HS-07 — Reuse an existing invitation on re-invite
**As an** owner, **I want** a second invitation to reuse the existing one, **so that** inviting doesn't fail with "already shared".
**Priority:** Must · **Estimate:** 3 · **Traces:** HS-5
**Acceptance criteria:**
- GIVEN the household's invitation was created by a previous invite, WHEN I invite again, THEN the existing invitation's sharing options are presented and no duplicate is created.

### US-HS-08 — Invite blocked when the shared service is unavailable
**As an** owner, **I want** to be told when I can't invite because the service is down, **so that** I don't get a silent failure.
**Priority:** Should · **Estimate:** 2 · **Traces:** HS-5
**Acceptance criteria:**
- GIVEN the shared service is unavailable, WHEN I choose to invite, THEN I am told it is unavailable and no sharing options are attempted.

### US-HS-09 — Invite failures identify their cause
**As an** owner, **I want** invite failures to say *why*, **so that** I can act (storage allowance, rate limit, account).
**Priority:** Should · **Estimate:** 2 · **Traces:** HS-5
**Acceptance criteria:**
- GIVEN invitation creation fails for a specific reason, WHEN the failure surfaces, THEN the message identifies that reason.

### US-HS-10 — Stalled invite resolves with feedback
**As an** owner, **I want** a stuck invite to resolve rather than appear hung, **so that** I know the outcome.
**Priority:** Should · **Estimate:** 2 · **Traces:** HS-5
**Acceptance criteria:**
- GIVEN invitation creation does not complete promptly, WHEN I wait beyond a bounded time, THEN a timeout failure message is shown.

### US-HS-11 — Accept an invitation on a fresh setup
**As a** recipient with no household, **I want** accepting an invite to land me on the shared dashboard, **so that** I can start immediately.
**Priority:** Must · **Estimate:** 5 · **Traces:** HS-6
**Acceptance criteria:**
- GIVEN a fresh setup with no household, WHEN I accept a household invite, THEN the adopted household's dashboard appears once its data is available, without restarting the product.
- GIVEN acceptance completes while no background update is delivered, WHEN I wait, THEN the dashboard still appears within a bounded time after the data becomes available.

### US-HS-12 — Adoption is deterministic and never silently dropped
**As a** recipient, **I want** a delivered invite never to be lost, **so that** acceptance "just works".
**Priority:** Should · **Estimate:** 5 · **Traces:** HS-6
**Acceptance criteria:**
- GIVEN invitation details are delivered, WHEN acceptance does not complete, THEN the failure and cause are recorded in diagnostics (identifiers only) and the details are not discarded without a trace.
- GIVEN I create a new household while the invited one is still loading, WHEN the invite's data arrives, THEN my created household stays active.

### US-HS-13 — Member cap of six
**As an** owner, **I want** the household capped at six members, **so that** the sharing model stays workable.
**Priority:** Should · **Estimate:** 1 · **Traces:** HS-7
**Acceptance criteria:**
- GIVEN the household already has six members, WHEN someone tries to invite, THEN the invite is prevented and the limit is explained.

### US-HS-14 — Equal permissions for all members
**As a** member, **I want** the same read/write access as the owner, **so that** I can help without asking.
**Priority:** Must · **Estimate:** 2 · **Traces:** HS-8
**Acceptance criteria:**
- GIVEN two members share a household, WHEN one edits an entry created by the other, THEN the edit is saved and shared with all members.

### US-HS-15 — Member list always shows the current person
**As a** user, **I want** to see myself in the member list, **so that** I know I'm part of the household.
**Priority:** Should · **Estimate:** 2 · **Traces:** HS-9
**Acceptance criteria:**
- GIVEN a household that has never been shared, WHEN I open the member list, THEN I appear as a member labeled with a localized "You"-style label.

### US-HS-16 — Show the current person's display name
**As a** user, **I want** my account display name shown, **so that** the member list is meaningful.
**Priority:** Should · **Estimate:** 3 · **Traces:** HS-9
**Acceptance criteria:**
- GIVEN my account identity resolves, WHEN I open the member list, THEN my row shows the resolved name.
- GIVEN it cannot resolve, THEN my row still appears using the locally known name or a localized "You" placeholder.

### US-HS-17 — Member list survives a refresh failure
**As a** user, **I want** the member list to stay populated and explain failures, **so that** a connectivity blip doesn't blank it.
**Priority:** Should · **Estimate:** 2 · **Traces:** HS-9
**Acceptance criteria:**
- GIVEN locally known members exist, WHEN a participant refresh fails, THEN those members remain listed and the failure is surfaced with its cause.

### US-HS-18 — Member list renders before participants resolve
**As a** member who just joined, **I want** the member list populated immediately, **so that** I don't see an empty or erroneous list.
**Priority:** Should · **Estimate:** 2 · **Traces:** HS-9
**Acceptance criteria:**
- GIVEN participants are not yet resolved, WHEN I open the member list, THEN I see the current person and any known members, refreshed once participants resolve.

### US-HS-19 — Owner deletes a household
**As an** owner, **I want** to permanently delete a household and all its data, **so that** I can remove something I no longer want shared.
**Priority:** Should · **Estimate:** 5 · **Traces:** HS-10
**Acceptance criteria:**
- GIVEN I own a shared household with members and rabbits, WHEN I use the visibly placed Delete control on its Settings row and confirm, THEN the household, its rabbits, entries, photos, medications, doses, members and tags are permanently deleted, the invitation is withdrawn so members lose access, and the deletion is shared with them.
- GIVEN I have not confirmed, THEN nothing is deleted.

### US-HS-20 — Deletion confirmation states the full consequence
**As an** owner, **I want** the confirmation to spell out the impact, **so that** I don't delete a shared household by accident.
**Priority:** Should · **Estimate:** 1 · **Traces:** HS-10
**Acceptance criteria:**
- GIVEN I selected Delete for a shared household, WHEN the confirmation shows, THEN it states members will lose access and data cannot be recovered.

### US-HS-21 — Participants have no delete affordance
**As a** participant, **I want** no delete control for a household I joined, **so that** only the owner can destroy it.
**Priority:** Should · **Estimate:** 1 · **Traces:** HS-10
**Acceptance criteria:**
- GIVEN I joined via an accepted invite, WHEN I view the household in Settings, THEN no delete action is available.

### US-HS-22 — Deleting the active household falls back cleanly
**As a** user, **I want** a graceful landing after deleting the active household, **so that** the product stays usable.
**Priority:** Should · **Estimate:** 2 · **Traces:** HS-10
**Acceptance criteria:**
- GIVEN the active household is deleted and others exist, THEN the newest remaining household becomes active and its rabbits show.
- GIVEN it was the only household, THEN the creation onboarding is presented.

### US-HS-23 — Deletion failure is surfaced
**As an** owner, **I want** a failed deletion to leave things intact and tell me, **so that** I'm not misled.
**Priority:** Should · **Estimate:** 1 · **Traces:** HS-10, NFR-8
**Acceptance criteria:**
- GIVEN deletion fails to take effect, WHEN I attempt it, THEN the household remains and the failure is shown.

### US-HS-24 — Changes reach all members
**As a** member, **I want** others' changes to reach me automatically, **so that** we stay in step.
**Priority:** Must · **Estimate:** 5 · **Traces:** HS-11
**Acceptance criteria:**
- GIVEN two members are online, WHEN one logs an entry, THEN the other sees it without manual refresh.
- GIVEN the product is not open, WHEN shared data changes, THEN the change is delivered in the background so it appears without the user having to open the product.

### US-HS-25 — Shared content counts against the owner's storage
**As a** product owner, **I want** shared content to count against the household owner's storage, **so that** attribution is predictable.
**Priority:** Could · **Estimate:** 1 · **Traces:** HS-12
**Acceptance criteria:**
- GIVEN a non-owner attaches a photo, WHEN it is shared, THEN the storage it consumes counts against the owner's allowance.

---

## EP-2 — Synchronization & Offline

> **Goal:** one authoritative copy of each household's data, shared across members; reading
> works offline from what was already loaded; saving is honest about needing connectivity.

### US-SYNC-01 — One logical collection per household
**As a** member, **I want** a household's data to resolve to the same items for everyone, **so that** we never see duplicates.
**Priority:** Must · **Estimate:** 5 · **Traces:** SYNC-1
**Acceptance criteria:**
- GIVEN rabbits and entries created by multiple members, WHEN any member views the household, THEN the items are the same for everyone with no duplicate copies.

### US-SYNC-02 — Saves require connectivity, and failures are honest
**As a** user, **I want** a failed save to never look like a success, **so that** I don't trust data that wasn't kept.
**Priority:** Must · **Estimate:** 5 · **Traces:** SYNC-2, NFR-1, NFR-8
**Acceptance criteria:**
- GIVEN no connectivity, WHEN I log a weight entry, THEN it is NOT saved and I am told connectivity is required.
- GIVEN a valid account and connectivity, WHEN I log an entry, THEN it is saved and shared with members.

### US-SYNC-03 — Save failures identify cause and offer retry
**As a** user, **I want** to know why a save failed, **so that** I can fix it.
**Priority:** Should · **Estimate:** 3 · **Traces:** SYNC-2
**Acceptance criteria:**
- GIVEN a save fails because the storage allowance is exceeded, the service is rate-limiting, or the account is unavailable, WHEN surfaced, THEN the message names the cause and offers a retry.

### US-SYNC-04 — Browse previously loaded data while offline
**As a** user, **I want** previously loaded data to display offline, **so that** I can still reference it.
**Priority:** Must · **Estimate:** 5 · **Traces:** SYNC-3
**Acceptance criteria:**
- GIVEN a household was loaded while online, WHEN I go offline and open the product, THEN the previously loaded data is shown.

### US-SYNC-05 — Online launch refreshes what was loaded
**As a** user, **I want** fresh data when I launch online, **so that** I see the latest.
**Priority:** Must · **Estimate:** 3 · **Traces:** SYNC-3
**Acceptance criteria:**
- GIVEN connectivity at launch, WHEN I open the product, THEN data is refreshed.

### US-SYNC-06 — Nothing loaded, offline
**As a** user, **I want** an explicit connectivity-required state when nothing was ever loaded, **so that** I'm not shown misleading empty data.
**Priority:** Should · **Estimate:** 2 · **Traces:** SYNC-3
**Acceptance criteria:**
- GIVEN the household was never loaded and I am offline, WHEN I open the product, THEN I see a state explaining connectivity is required.

### US-SYNC-07 — Viewed photos remain viewable offline
**As a** user, **I want** photos I've viewed to remain viewable offline, **so that** I can reference them without connectivity.
**Priority:** Should · **Estimate:** 3 · **Traces:** SYNC-4
**Acceptance criteria:**
- GIVEN a photo was displayed at least once while online, WHEN I open its entry offline, THEN it is shown from the locally kept copy.

### US-SYNC-08 — Unviewed photo while offline shows a placeholder
**As a** user, **I want** a clear placeholder for photos I haven't viewed, **so that** I understand why it's missing.
**Priority:** Could · **Estimate:** 1 · **Traces:** SYNC-4
**Acceptance criteria:**
- GIVEN a photo was never displayed and I'm offline, WHEN I open its entry, THEN a "cannot load without connectivity" placeholder is shown.

### US-SYNC-09 — No duplication; concurrent edits converge
**As a** member, **I want** simultaneous edits to converge to one version, **so that** we don't end up with duplicates.
**Priority:** Must · **Estimate:** 5 · **Traces:** SYNC-5, NFR-2
**Acceptance criteria:**
- GIVEN two members edit the same entry offline, WHEN both reconnect, THEN a single entry remains reflecting the most recently saved edit.

### US-SYNC-10 — Live updates for mid-session households
**As a** user, **I want** a household created or joined mid-session to update live, **so that** I don't need to restart.
**Priority:** Should · **Estimate:** 3 · **Traces:** SYNC-6
**Acceptance criteria:**
- GIVEN I create or join a household while the product runs, WHEN it becomes active, THEN I receive live updates immediately.

### US-SYNC-11 — Set-up-again recovers data
**As a** user, **I want** my data back after setting the product up again, **so that** the product is not the only place it lives.
**Priority:** Should · **Estimate:** 3 · **Traces:** SYNC-7
**Acceptance criteria:**
- GIVEN I set the product up again on the same account, WHEN it loads, THEN the household's data is recovered and displayed.

---

## EP-3 — Dashboard & Navigation

### US-DASH-01 — Rabbit list on launch
**As a** user, **I want** to see my rabbits when I open the product, **so that** I get an at-a-glance view.
**Priority:** Must · **Estimate:** 3 · **Traces:** DASH-1
**Acceptance criteria:**
- GIVEN the household has two active rabbits, WHEN I open the product, THEN both show with name, thumbnail, and most recent weigh-in date.

### US-DASH-02 — Never-weighed indication
**As a** user, **I want** unweighed rabbits marked, **so that** I know who needs a reading.
**Priority:** Should · **Estimate:** 1 · **Traces:** DASH-2
**Acceptance criteria:**
- GIVEN a rabbit has no weight entries, WHEN the list shows, THEN its row indicates it has never been weighed.

### US-DASH-03 — Sort by most recent weigh-in
**As a** user, **I want** recently weighed rabbits on top, **so that** the list matches my attention.
**Priority:** Should · **Estimate:** 2 · **Traces:** DASH-3
**Acceptance criteria:**
- GIVEN rabbit A was weighed today and B a week ago, WHEN the list shows, THEN A is above B; a never-weighed rabbit sorts last regardless of creation order.

### US-DASH-04 — Attention flag for drops and severe health
**As a** user, **I want** at-risk rabbits flagged, **so that** I notice problems.
**Priority:** Must · **Estimate:** 3 · **Traces:** DASH-4, WT-7
**Acceptance criteria:**
- GIVEN a rabbit's latest entry triggers its drop threshold, THEN its row shows a warning indicator.
- GIVEN a rabbit has a Severe health entry within the last 7 days, THEN its row shows a warning indicator; a Severe entry older than 7 days does not flag it.

### US-DASH-05 — Attention flag clears when the threshold is raised
**As a** user, **I want** the flag to clear when it no longer applies, **so that** the signal stays trustworthy.
**Priority:** Should · **Estimate:** 1 · **Traces:** DASH-4
**Acceptance criteria:**
- GIVEN a rabbit is flagged by a drop, WHEN I raise its threshold so the drop no longer exceeds it, THEN the flag clears.

### US-DASH-06 — Active-medication badge
**As a** user, **I want** a quiet badge for rabbits on medication, **so that** I know without alarming styling.
**Priority:** Should · **Estimate:** 2 · **Traces:** DASH-5
**Acceptance criteria:**
- GIVEN a rabbit has an active medication, THEN its row shows an informational badge that is NOT the warning indicator.
- GIVEN all its medications are finished, THEN no badge shows.

### US-DASH-07 — Open the rabbit dashboard
**As a** user, **I want** to select a rabbit to see its detail, **so that** I can dig in.
**Priority:** Must · **Estimate:** 2 · **Traces:** DASH-6, DASH-7
**Acceptance criteria:**
- GIVEN the list is shown, WHEN I select a rabbit, THEN its dashboard opens with the weight chart and medications section.

### US-DASH-08 — Rabbit dashboard overview with history links
**As a** user, **I want** the dashboard to offer weight, health, and full history views, **so that** I can navigate by need.
**Priority:** Must · **Estimate:** 3 · **Traces:** DASH-7, WT-6, HL-3
**Acceptance criteria:**
- GIVEN the rabbit dashboard is shown, WHEN I view the History section, THEN I can open weight-only, health-only, and full combined history.
- GIVEN the rabbit is archived, THEN the dashboard and sub-views are shown without add/edit/delete actions.

### US-DASH-09 — Empty state prompts adding a first rabbit
**As a** new user, **I want** a clear prompt when there are no rabbits, **so that** I know how to start.
**Priority:** Must · **Estimate:** 1 · **Traces:** DASH-8, RAB-4
**Acceptance criteria:**
- GIVEN the household has zero rabbits, WHEN the dashboard shows, THEN it prompts me to add my first rabbit.

### US-DASH-10 — Per-row quick weigh control
**As a** user, **I want** a weigh control on each row, **so that** I can measure a specific rabbit fast.
**Priority:** Must · **Estimate:** 2 · **Traces:** DASH-9, WT-2
**Acceptance criteria:**
- GIVEN the list is shown, WHEN I use a row's weigh control, THEN the weight form opens pre-bound to that rabbit and saving records the entry.

---

## EP-4 — Rabbit Profiles & Lifecycle

### US-RAB-01 — Create a rabbit profile
**As a** user, **I want** to record a rabbit's identifying details, **so that** its history is anchored to it.
**Priority:** Must · **Estimate:** 3 · **Traces:** RAB-1
**Acceptance criteria:**
- GIVEN I am adding a rabbit, WHEN I enter a name and optionally photo, breed, DOB, sex, neuter status, and adoption date, THEN the profile is saved.

### US-RAB-02 — Date of birth may be unknown
**As a** user, **I want** to mark DOB as unknown, **so that** I can create a profile without a birth date.
**Priority:** Should · **Estimate:** 1 · **Traces:** RAB-1
**Acceptance criteria:**
- GIVEN I leave DOB unset and mark it unknown, THEN the profile saves without requiring a birth date.

### US-RAB-03 — Rabbit name is required
**As a** user, **I want** to be blocked from saving a nameless rabbit, **so that** profiles are identifiable.
**Priority:** Must · **Estimate:** 1 · **Traces:** RAB-2
**Acceptance criteria:**
- GIVEN I attempt to save with a blank name or a whitespace-only name, THEN the profile is not created and I am prompted for a name.

### US-RAB-04 — Multiple rabbits, no limit
**As a** user, **I want** as many rabbits as I need, **so that** the product fits my household.
**Priority:** Must · **Estimate:** 1 · **Traces:** RAB-3
**Acceptance criteria:**
- GIVEN two rabbits exist, WHEN I add a third, THEN it is created without restriction.

### US-RAB-05 — Add-rabbit entry point always available
**As a** user, **I want** to reach the add form from the dashboard any time, **so that** I can add a rabbit whenever.
**Priority:** Must · **Estimate:** 1 · **Traces:** RAB-4
**Acceptance criteria:**
- GIVEN the dashboard is shown, WHEN I use the add control, THEN the rabbit-creation form opens.

### US-RAB-06 — Edit an active rabbit's profile
**As a** member, **I want** to edit any profile field including the drop threshold, **so that** records stay accurate and alerts fit the rabbit.
**Priority:** Must · **Estimate:** 3 · **Traces:** RAB-5, WT-7
**Acceptance criteria:**
- GIVEN an active rabbit is shown, WHEN I use the directly visible edit control, THEN a pre-filled edit form opens and saving persists the change for all members.
- GIVEN the rabbit is archived, THEN the edit action is not available.

### US-RAB-07 — Archive a rabbit (soft delete)
**As a** user, **I want** to archive a deceased rabbit, **so that** it leaves the active list but keeps its history.
**Priority:** Must · **Estimate:** 3 · **Traces:** RAB-6, RAB-9
**Acceptance criteria:**
- GIVEN a rabbit with history, WHEN I archive it, THEN it leaves the active dashboard and its weight/health history remains retrievable from an archived list two years later.

### US-RAB-08 — Restore an archived rabbit
**As a** user, **I want** to restore an archived rabbit, **so that** I can correct a mistake.
**Priority:** Should · **Estimate:** 2 · **Traces:** RAB-7
**Acceptance criteria:**
- GIVEN a rabbit is archived, WHEN I restore it, THEN it reappears in the active list with unchanged history.

### US-RAB-09 — Archived timeline is read-only
**As a** user, **I want** an archived rabbit's history viewable but frozen, **so that** records are preserved.
**Priority:** Must · **Estimate:** 3 · **Traces:** RAB-8, MED-12
**Acceptance criteria:**
- GIVEN a rabbit is archived, WHEN I open its timeline, THEN the history is viewable and add/edit/delete actions are unavailable.
- GIVEN the archived timeline is open, THEN export and photo-gallery actions remain available.

### US-RAB-10 — Permanently delete a rabbit
**As a** user, **I want** to delete a rabbit and its data, **so that** I can remove it entirely on request.
**Priority:** Should · **Estimate:** 3 · **Traces:** RAB-10
**Acceptance criteria:**
- GIVEN an archived rabbit, WHEN I confirm permanent deletion, THEN the profile and all its weight/health entries and photos are removed and the deletion is shared with members.

---

## EP-5 — Weight Tracking

### US-WT-01 — Log a weight entry
**As a** user, **I want** to log a weight with a timestamp and note, **so that** I capture a reading.
**Priority:** Must · **Estimate:** 3 · **Traces:** WT-1
**Acceptance criteria:**
- GIVEN a rabbit exists, WHEN I enter a weight and save, THEN an entry is created timestamped now, or with the timestamp I edited.

### US-WT-02 — Three-tap quick weigh
**As a** user, **I want** to log a weight in ≤3 taps, **so that** it's effortless enough to do regularly.
**Priority:** Must · **Estimate:** 2 · **Traces:** WT-2
**Acceptance criteria:**
- GIVEN the product is freshly launched, WHEN I select a rabbit, enter a weight, and save, THEN the entry is recorded in no more than 3 taps.

### US-WT-03 — Quick-entry with rabbit picker
**As a** user, **I want** a quick-entry control that lets me pick a rabbit, **so that** I can weigh any rabbit from one place.
**Priority:** Should · **Estimate:** 2 · **Traces:** WT-2
**Acceptance criteria:**
- GIVEN the dashboard is shown, WHEN I use the quick-entry and pick a rabbit, THEN the form opens bound to that rabbit.

### US-WT-04 — Display unit per person, with regional default
**As a** user, **I want** weights shown in my preferred unit, **so that** I can read them naturally.
**Priority:** Must · **Estimate:** 2 · **Traces:** WT-3, SET-4
**Acceptance criteria:**
- GIVEN entries stored in grams, WHEN I set the unit to kilograms in Settings, THEN all values display in kg and the stored value is unchanged.
- GIVEN a metric-region setup on first use, THEN the display unit defaults to kilograms and the preference persists.

### US-WT-05 — Enter weights in the display unit, no picker
**As a** user, **I want** the form to use my chosen unit without a picker, **so that** entry is simpler.
**Priority:** Should · **Estimate:** 1 · **Traces:** WT-3
**Acceptance criteria:**
- GIVEN the display unit is kilograms, WHEN I enter a value and save, THEN it is interpreted and stored in grams and the form offers no unit picker.

### US-WT-06 — Reject non-positive values
**As a** user, **I want** invalid values refused, **so that** data stays clean.
**Priority:** Must · **Estimate:** 1 · **Traces:** WT-4
**Acceptance criteria:**
- GIVEN the form is open, WHEN I enter 0 or a negative value, THEN it is rejected and I am prompted for a valid value.

### US-WT-07 — Confirm implausible outliers
**As a** user, **I want** a confirmation for implausible values, **so that** typos are caught.
**Priority:** Should · **Estimate:** 1 · **Traces:** WT-4
**Acceptance criteria:**
- GIVEN I enter a value exceeding a plausibility bound (e.g. 30 kg), WHEN I save, THEN the product asks me to confirm first.

### US-WT-08 — Edit or delete a past entry
**As a** user, **I want** to correct or remove a past entry, **so that** data stays accurate.
**Priority:** Must · **Estimate:** 2 · **Traces:** WT-5
**Acceptance criteria:**
- GIVEN a past entry with a wrong value, WHEN I edit and save, THEN it updates and is shared with all members.

### US-WT-09 — Weight trend chart with ranges
**As a** user, **I want** a chart with 7/30/90/all ranges, **so that** I can spot trends.
**Priority:** Must · **Estimate:** 5 · **Traces:** WT-6
**Acceptance criteria:**
- GIVEN entries spanning six months, WHEN I select the 30-day range, THEN only the last 30 days display.

### US-WT-10 — Independent weight-history range
**As a** user, **I want** the weight-history list to have its own range, **so that** it doesn't follow the chart.
**Priority:** Should · **Estimate:** 2 · **Traces:** WT-6
**Acceptance criteria:**
- GIVEN the chart is set to 30 days and I set the history sub-view to 7 days, WHEN I open the history, THEN it lists only the last 7 days.

### US-WT-11 — Weight-drop alert
**As a** user, **I want** a warning when a drop exceeds a configurable threshold, **so that** I catch sudden decline.
**Priority:** Must · **Estimate:** 3 · **Traces:** WT-7, DASH-4
**Acceptance criteria:**
- GIVEN threshold 5% and previous 2000 g, WHEN I log 1800 g, THEN a visual warning appears on that entry and on the dashboard.
- GIVEN threshold 5% and previous 2000 g, WHEN I log 1950 g, THEN no warning appears.

### US-WT-12 — Threshold changes re-evaluate the alert
**As a** user, **I want** the alert to update when I change the threshold, **so that** it reflects my configuration.
**Priority:** Should · **Estimate:** 2 · **Traces:** WT-7, DASH-4
**Acceptance criteria:**
- GIVEN a drop of 10% flagged at threshold 5%, WHEN I raise the threshold to 15%, THEN the warning disappears.
- GIVEN a drop of 2.5% not flagged at threshold 5%, WHEN I lower it to 2%, THEN the warning appears.

### US-WT-13 — Weight entry detail view
**As a** user, **I want** to open an entry's full detail, **so that** I can see notes, alerts, and photos.
**Priority:** Must · **Estimate:** 3 · **Traces:** WT-8
**Acceptance criteria:**
- GIVEN an entry in the timeline, WHEN I open its row, THEN I see the value in the display unit, timestamp, full untruncated note, and a swipeable photo pager.

### US-WT-14 — Edit or delete from weight detail
**As a** user, **I want** edit/delete from the detail view, **so that** I can fix mistakes in context.
**Priority:** Should · **Estimate:** 2 · **Traces:** WT-8
**Acceptance criteria:**
- GIVEN the detail is open, WHEN I choose Edit and save, THEN the entry updates and the timeline reflects it.
- GIVEN the detail is open, WHEN I choose Delete and confirm, THEN the entry is removed and the timeline updates.

### US-WT-15 — Archived weight detail is read-only
**As a** user, **I want** an archived rabbit's entry detail without actions, **so that** history is preserved.
**Priority:** Should · **Estimate:** 1 · **Traces:** WT-8, RAB-8
**Acceptance criteria:**
- GIVEN the rabbit is archived, WHEN I open a weight entry's detail, THEN it shows the information with no Edit or Delete.

---

## EP-6 — Health Logging

### US-HL-01 — Log a structured health entry
**As a** user, **I want** to tag symptoms with severity, **so that** patterns become queryable.
**Priority:** Must · **Estimate:** 5 · **Traces:** HL-1
**Acceptance criteria:**
- GIVEN a rabbit exists, WHEN I select tags (Appetite, Activity/Behavior, …), set severity Moderate, and save, THEN an entry is created with those tags and severity.

### US-HL-02 — Free-text-only health entry
**As a** user, **I want** to save a note without tags, **so that** I can log anything quickly.
**Priority:** Should · **Estimate:** 1 · **Traces:** HL-2
**Acceptance criteria:**
- GIVEN I enter a note and no tags, WHEN I save, THEN the entry is accepted.

### US-HL-03 — Editable/back-dated timestamp
**As a** user, **I want** to set a past timestamp, **so that** I can log something noticed earlier.
**Priority:** Should · **Estimate:** 1 · **Traces:** HL-1
**Acceptance criteria:**
- GIVEN I set the timestamp to a past date, WHEN I save, THEN the entry uses the edited timestamp.

### US-HL-04 — Combined interleaved timeline
**As a** user, **I want** weight and health entries together, **so that** I can correlate symptom onset with weight.
**Priority:** Must · **Estimate:** 3 · **Traces:** HL-3
**Acceptance criteria:**
- GIVEN a rabbit has both types, WHEN I open full history, THEN they appear interleaved by timestamp.
- GIVEN a rabbit has only health entries, THEN they display without an empty-state prompt.

### US-HL-05 — Filter health history by tag
**As a** user, **I want** to filter by tag, **so that** I can review one symptom type.
**Priority:** Should · **Estimate:** 2 · **Traces:** HL-4
**Acceptance criteria:**
- GIVEN entries across multiple tags, WHEN I filter by "Stool/Droppings", THEN only entries with that tag show.

### US-HL-06 — Filter health history by severity
**As a** user, **I want** to filter by severity, **so that** I can find serious events.
**Priority:** Should · **Estimate:** 1 · **Traces:** HL-4
**Acceptance criteria:**
- GIVEN entries of varying severity, WHEN I filter by "Severe", THEN only Severe entries show.

### US-HL-07 — Filter health history by date range
**As a** user, **I want** to restrict to a date range, **so that** I can scope a review.
**Priority:** Should · **Estimate:** 1 · **Traces:** HL-4
**Acceptance criteria:**
- GIVEN entries spanning months, WHEN I set a range, THEN only entries within it show.

### US-HL-08 — Edit or delete a health entry
**As a** user, **I want** to correct or remove a health entry, **so that** data stays accurate.
**Priority:** Must · **Estimate:** 2 · **Traces:** HL-5
**Acceptance criteria:**
- GIVEN a duplicate entry, WHEN I delete it, THEN it is removed and the change is shared with members.

### US-HL-09 — Household-scoped tag catalog
**As a** product owner, **I want** each household to own its tags, **so that** sharing never corrupts data.
**Priority:** Must · **Estimate:** 3 · **Traces:** HL-6
**Acceptance criteria:**
- GIVEN two households, WHEN each opens health logging, THEN each has its own copy of the canonical tags and no tag is referenced by both.
- GIVEN a household's catalog is populated, WHEN the screen is reopened, THEN no duplicate tags are created.

### US-HL-10 — Health entry detail view
**As a** user, **I want** the severity, tags, note, and photos in one detail, **so that** I can review an event.
**Priority:** Should · **Estimate:** 3 · **Traces:** HL-7
**Acceptance criteria:**
- GIVEN a health entry in the timeline, WHEN I open it, THEN I see severity, all tags, and the full untruncated note.
- GIVEN the detail is open, WHEN I choose Edit or Delete, THEN the timeline reflects the change.

### US-HL-11 — Archived health detail is read-only
**As a** user, **I want** an archived rabbit's health detail without actions, **so that** history is preserved.
**Priority:** Should · **Estimate:** 1 · **Traces:** HL-7, RAB-8
**Acceptance criteria:**
- GIVEN the rabbit is archived, WHEN I open a health entry's detail, THEN it shows the information with no Edit or Delete.

---

## EP-7 — Medication & Dosing

> **Goal:** know what each rabbit is on, when doses are due, and whether they were given —
> shared so the household never double-doses.

### US-MED-01 — Add a medication
**As a** member, **I want** to register a medication with amount and schedule, **so that** the household knows the course.
**Priority:** Must · **Estimate:** 5 · **Traces:** MED-1
**Acceptance criteria:**
- GIVEN a rabbit exists, WHEN I create "Baytril", reason "respiratory infection", start today, 0.2 ml, twice daily 08:00/20:00, THEN it saves and is active.

### US-MED-02 — Multiple concurrent medications
**As a** member, **I want** a rabbit on several medications at once, **so that** combination treatment is supported.
**Priority:** Should · **Estimate:** 1 · **Traces:** MED-1
**Acceptance criteria:**
- GIVEN one active medication, WHEN I add another without an end date, THEN both are active simultaneously.

### US-MED-03 — Create a medication with required fields only
**As a** member, **I want** to skip optional fields, **so that** setup is fast.
**Priority:** Should · **Estimate:** 1 · **Traces:** MED-1
**Acceptance criteria:**
- GIVEN I enter only name, start date, and amount, THEN the medication saves with no reason or note required.

### US-MED-04 — Active/finished lifecycle
**As a** member, **I want** medications to auto-finish after their end date, **so that** the list reflects reality.
**Priority:** Must · **Estimate:** 3 · **Traces:** MED-2
**Acceptance criteria:**
- GIVEN no end date, THEN the medication is active.
- GIVEN an end date has passed, THEN it appears finished with history intact and schedules no reminders.

### US-MED-05 — End a course early
**As a** member, **I want** ending a course to take effect immediately, **so that** reminders stop.
**Priority:** Should · **Estimate:** 2 · **Traces:** MED-2
**Acceptance criteria:**
- GIVEN an active medication, WHEN I set its end date to today, THEN it becomes finished and pending reminders are cancelled.

### US-MED-06 — Configure a times-per-day schedule
**As a** member, **I want** to set N doses per day with times, **so that** frequent dosing is tracked.
**Priority:** Must · **Estimate:** 3 · **Traces:** MED-3
**Acceptance criteria:**
- GIVEN I choose "2 times per day", THEN two editable time slots appear pre-filled with defaults.

### US-MED-07 — Configure an every-X-days schedule
**As a** member, **I want** an interval schedule counted from the start date, **so that** intermittent dosing is tracked.
**Priority:** Should · **Estimate:** 2 · **Traces:** MED-3
**Acceptance criteria:**
- GIVEN a start date of Monday, WHEN I choose "every 3 days" at 09:00, THEN doses are due Monday, Thursday, Sunday, … counted from the start.

### US-MED-08 — Configure a weekly schedule
**As a** member, **I want** a weekday schedule, **so that** long-term weekly medications are tracked.
**Priority:** Should · **Estimate:** 2 · **Traces:** MED-3
**Acceptance criteria:**
- GIVEN I choose a weekly schedule for Mondays at 10:00, THEN the medication is due every Monday at 10:00.

### US-MED-09 — Configure a PRN medication
**As a** member, **I want** an as-needed medication with an optional max-per-day hint, **so that** I can track medications given on demand.
**Priority:** Should · **Estimate:** 2 · **Traces:** MED-3, MED-11
**Acceptance criteria:**
- GIVEN I choose "as needed", THEN no time slots are required and I can set an optional max-per-day hint.

### US-MED-10 — Schedule summary renders for every type
**As a** member, **I want** a readable schedule summary, **so that** I can verify what I configured.
**Priority:** Should · **Estimate:** 2 · **Traces:** MED-4
**Acceptance criteria:**
- GIVEN an every-3-days medication, THEN the summary shows the interval without failing.
- GIVEN a PRN medication with max 3/day, THEN the summary shows the hint.
- GIVEN any schedule with no recorded doses, THEN the summary still renders.

### US-MED-11 — Give a dose from the detail
**As a** member, **I want** a prominent "give dose" action, **so that** logging is one step.
**Priority:** Must · **Estimate:** 2 · **Traces:** MED-8
**Acceptance criteria:**
- GIVEN the medication detail is open, WHEN I choose "Give dose" and confirm the pre-filled amount, THEN a dose is recorded and the list updates.

### US-MED-12 — Log a dose with a default amount
**As a** member, **I want** the amount pre-filled, **so that** I usually just confirm.
**Priority:** Must · **Estimate:** 3 · **Traces:** MED-5
**Acceptance criteria:**
- GIVEN a serving of 0.2 ml, WHEN I confirm giving the dose, THEN a dose is recorded with 0.2 ml, the current time, and me as giver.

### US-MED-13 — Adjust amount, note, or back-date a dose
**As a** member, **I want** to edit a dose's details, **so that** half-doses and forgotten doses are accurate.
**Priority:** Should · **Estimate:** 2 · **Traces:** MED-5
**Acceptance criteria:**
- GIVEN a 0.4 ml serving, WHEN I edit the amount to 0.2 ml, THEN the dose records 0.2 ml.
- GIVEN I forgot a morning dose, WHEN I log it in the evening with a morning timestamp, THEN the edited timestamp is saved.

### US-MED-14 — Review dose history
**As a** member, **I want** a chronological dose list, **so that** I can see what was actually given.
**Priority:** Must · **Estimate:** 2 · **Traces:** MED-6
**Acceptance criteria:**
- GIVEN several doses, WHEN I open the medication detail, THEN each dose's timestamp, amount, and giver are listed chronologically.

### US-MED-15 — Edit or delete a dose
**As a** member, **I want** to correct or remove a dose, **so that** the log stays accurate.
**Priority:** Should · **Estimate:** 2 · **Traces:** MED-6
**Acceptance criteria:**
- GIVEN a mistaken dose, WHEN I delete it and confirm, THEN it is removed and shared with members.

### US-MED-16 — Near-simultaneous dose guard
**As a** member, **I want** a warning if a dose was just logged, **so that** we don't double-dose.
**Priority:** Must · **Estimate:** 3 · **Traces:** MED-7
**Acceptance criteria:**
- GIVEN another member logged a dose for that medication within ~15 minutes, WHEN I confirm my dose, THEN I see a warning and may continue or cancel.

### US-MED-17 — Dose reminders with actions
**As a** member, **I want** a reminder at each slot with record/snooze actions, **so that** I can act directly.
**Priority:** Should · **Estimate:** 5 · **Traces:** MED-9
**Acceptance criteria:**
- GIVEN an active 2x/day medication, WHEN 08:00 arrives, THEN my device raises a reminder offering "gave it" and "snooze".

### US-MED-18 — Medication reminders stay on the configuring device
**As a** member, **I want** my reminders private, **so that** I don't disturb the household.
**Priority:** Should · **Estimate:** 2 · **Traces:** MED-9
**Acceptance criteria:**
- GIVEN members A and B share a medication, WHEN A configures a reminder, THEN only A is reminded.

### US-MED-19 — Record a dose from the reminder
**As a** member, **I want** to confirm a dose straight from the reminder, **so that** logging is frictionless.
**Priority:** Should · **Estimate:** 3 · **Traces:** MED-9
**Acceptance criteria:**
- GIVEN a dose reminder is showing, WHEN I choose "Gave it", THEN a pre-filled confirmation opens and confirming records the dose and satisfies the slot.

### US-MED-20 — Missed-dose follow-up
**As a** member, **I want** a single follow-up if I miss a dose, **so that** I don't forget.
**Priority:** Should · **Estimate:** 3 · **Traces:** MED-10
**Acceptance criteria:**
- GIVEN follow-up is enabled (default) and no dose was recorded for a passed slot, WHEN a short time passes, THEN a single follow-up reminder is raised.
- GIVEN follow-up is disabled, THEN no follow-up is raised.

### US-MED-21 — PRN medications have no reminders
**As a** member, **I want** no scheduled reminders for as-needed medications, **so that** I'm not prompted unnecessarily.
**Priority:** Should · **Estimate:** 1 · **Traces:** MED-11
**Acceptance criteria:**
- GIVEN a PRN medication, THEN no reminder schedule is created and doses are recorded on demand.

### US-MED-22 — Finished / archived medication records are read-only
**As a** member, **I want** historical medication records frozen, **so that** the clinical record is preserved.
**Priority:** Should · **Estimate:** 2 · **Traces:** MED-12
**Acceptance criteria:**
- GIVEN a finished medication or an archived rabbit, WHEN I open its medication section, THEN it is viewable without create/edit/delete actions.

### US-MED-23 — ⚠ Offline dose logging
**As a** member, **I want** to record doses while offline, **so that** I can log care without connectivity.
**Priority:** Must (if offline-first is chosen) · **Estimate:** 8 · **Traces:** MED-11, *conflicts with SYNC-2*
**Acceptance criteria (pending decision):**
- GIVEN no connectivity, WHEN I record a dose, THEN it is kept locally immediately and shared when connectivity returns.
- **Blocked by open decision** — the current product requires connectivity for all saves (SYNC-2). Resolve §13.1 before implementing.

---

## EP-8 — Photos

### US-PHOTO-01 — Attach photos to a weight entry
**As a** user, **I want** to attach photos to a weight entry, **so that** I keep visual context.
**Priority:** Should · **Estimate:** 2 · **Traces:** PHOTO-1
**Acceptance criteria:**
- GIVEN I am logging a weight entry, WHEN I attach a photo from my photos, THEN it is saved as part of that entry.

### US-PHOTO-02 — Attach photos to a health entry
**As a** user, **I want** to attach photos to a health entry, **so that** I can track visible symptoms.
**Priority:** Should · **Estimate:** 2 · **Traces:** PHOTO-1
**Acceptance criteria:**
- GIVEN I am logging a health entry, WHEN I attach a photo, THEN it is saved as part of that entry.

### US-PHOTO-03 — Set a rabbit profile photo
**As a** user, **I want** a profile photo, **so that** I recognize the rabbit at a glance.
**Priority:** Should · **Estimate:** 2 · **Traces:** PHOTO-2
**Acceptance criteria:**
- GIVEN a profile exists, WHEN I select a photo, THEN it shows as the rabbit's thumbnail across the product.

### US-PHOTO-04 — Per-rabbit photo gallery
**As a** user, **I want** a gallery of a rabbit's photos, **so that** I can browse its visual history.
**Priority:** Could · **Estimate:** 3 · **Traces:** PHOTO-3
**Acceptance criteria:**
- GIVEN photos across several entries, WHEN I open the gallery, THEN all are shown ordered by entry date.

### US-PHOTO-05 — Photos are shared with household members
**As a** member, **I want** to see photos another member added, **so that** we share the full record.
**Priority:** Should · **Estimate:** 3 · **Traces:** PHOTO-4
**Acceptance criteria:**
- GIVEN member A attaches a photo, WHEN member B opens the entry after it is shared, THEN B sees the same photo.

### US-PHOTO-06 — View entry photos as a swipeable pager
**As a** user, **I want** to swipe through an entry's photos in its detail, **so that** I can review them full-width.
**Priority:** Should · **Estimate:** 3 · **Traces:** PHOTO-5
**Acceptance criteria:**
- GIVEN an entry has photos, WHEN I open its row or a thumbnail, THEN its detail opens with a swipeable pager (a thumbnail opens the detail, not a separate viewer).
- GIVEN an entry has no photos, THEN no pager is shown.

---

## EP-9 — Reminders

### US-REM-01 — Configure a recurring weigh reminder
**As a** user, **I want** a recurring reminder to weigh, **so that** I stay consistent.
**Priority:** Should · **Estimate:** 3 · **Traces:** REM-1
**Acceptance criteria:**
- GIVEN I configure a reminder for Sundays at 10:00, THEN a recurring reminder is raised on schedule.

### US-REM-02 — Household-wide reminder
**As a** user, **I want** one reminder covering all rabbits, **so that** I can weigh everyone in one sitting.
**Priority:** Could · **Estimate:** 2 · **Traces:** REM-1
**Acceptance criteria:**
- GIVEN I configure a household-wide reminder, THEN it applies to the household rather than a single rabbit.

### US-REM-03 — Reminders are per-device
**As a** member, **I want** my reminders to stay on my device, **so that** I don't disturb others.
**Priority:** Should · **Estimate:** 1 · **Traces:** REM-2
**Acceptance criteria:**
- GIVEN member A sets a reminder, WHEN it fires, THEN only A is reminded and B is unaffected.

### US-REM-04 — Missed-reminder follow-up
**As a** user, **I want** a follow-up if I miss a weigh reminder, **so that** I don't skip a week.
**Priority:** Could · **Estimate:** 2 · **Traces:** REM-3
**Acceptance criteria:**
- GIVEN follow-up is enabled and no weight entry was logged after a reminder fired, WHEN the next day begins, THEN a follow-up reminder is raised.

---

## EP-10 — Export

### US-EXP-01 — Export history as a vet-readable PDF
**As a** user, **I want** a formatted PDF for a date range, **so that** I can hand it to a vet.
**Priority:** Should · **Estimate:** 5 · **Traces:** EXP-1
**Acceptance criteria:**
- GIVEN a rabbit with weight, health, and medication history, WHEN I pick a range and export PDF, THEN a formatted PDF contains the weight and health entries in range plus a medication section.

### US-EXP-02 — PDF includes finished courses and dose history
**As a** user, **I want** finished medications and their doses in the PDF, **so that** the treatment record is complete.
**Priority:** Should · **Estimate:** 3 · **Traces:** EXP-1
**Acceptance criteria:**
- GIVEN active and finished medications with doses, WHEN I export, THEN finished medications and their dose history appear alongside active ones.

### US-EXP-03 — Export raw data as CSV
**As a** user, **I want** raw CSV, **so that** I can analyze or archive the data myself.
**Priority:** Should · **Estimate:** 2 · **Traces:** EXP-2
**Acceptance criteria:**
- GIVEN weight entries, WHEN I export CSV for a range, THEN CSV files contain the raw entries in range.

### US-EXP-04 — Dedicated medication and dose CSV files
**As a** user, **I want** separate medication and dose CSVs, **so that** the data is unambiguous.
**Priority:** Should · **Estimate:** 2 · **Traces:** EXP-2
**Acceptance criteria:**
- GIVEN medications with doses, WHEN I export CSV, THEN a medications CSV lists each medication's fields and a doses CSV lists each dose's timestamp, amount, and giver.

### US-EXP-05 — Share exported files
**As a** user, **I want** standard sharing options, **so that** I can send the export anywhere.
**Priority:** Should · **Estimate:** 1 · **Traces:** EXP-3
**Acceptance criteria:**
- GIVEN an export is generated, THEN the device's standard sharing options are presented with the file attached.

### US-EXP-06 — Export never fails on any schedule type
**As a** user, **I want** export to work for every medication schedule, **so that** unusual courses don't break it.
**Priority:** Should · **Estimate:** 3 · **Traces:** EXP-4, MED-4
**Acceptance criteria:**
- GIVEN an every-X-days medication with no doses, WHEN I export PDF or CSV, THEN it succeeds and shows the schedule summary.
- GIVEN a PRN medication with a max-per-day hint and no doses, THEN export likewise succeeds.

---

## EP-11 — Settings & Profile

### US-SET-01 — Open Settings from the dashboard
**As a** user, **I want** a Settings entry point, **so that** I can configure the product.
**Priority:** Must · **Estimate:** 1 · **Traces:** SET-1
**Acceptance criteria:**
- GIVEN the dashboard is shown, WHEN I use the gear control, THEN Settings & Profile opens as its own screen.

### US-SET-02 — Show my account name read-only
**As a** user, **I want** to see my name, **so that** I know which account I'm using.
**Priority:** Should · **Estimate:** 2 · **Traces:** SET-2
**Acceptance criteria:**
- GIVEN I am signed in and connected, WHEN I open Settings, THEN my account display name is shown and not editable.

### US-SET-03 — Name fallback when unresolved
**As a** user, **I want** Settings to still work offline, **so that** an unresolved identity doesn't break it.
**Priority:** Could · **Estimate:** 2 · **Traces:** SET-2
**Acceptance criteria:**
- GIVEN my identity can't be resolved, THEN Settings falls back to the locally known name, then a localized "You" placeholder, without disrupting the screen.

### US-SET-04 — Override the product language
**As a** user, **I want** to choose Deutsch or English regardless of my device language, **so that** the product is readable to me.
**Priority:** Should · **Estimate:** 3 · **Traces:** SET-3, NFR-5
**Acceptance criteria:**
- GIVEN the override is "System", WHEN I select Deutsch and relaunch, THEN all text is German.
- GIVEN I select System and relaunch, THEN it follows the device language.

### US-SET-05 — Language change applies next launch
**As a** user, **I want** to know when a language change takes effect, **so that** I'm not confused.
**Priority:** Could · **Estimate:** 1 · **Traces:** SET-3
**Acceptance criteria:**
- GIVEN I select a language but don't relaunch, THEN the session keeps the previous language and informs me the change applies next launch.

### US-SET-06 — Choose the display weight unit
**As a** user, **I want** to pick g or kg, **so that** weights read naturally.
**Priority:** Must · **Estimate:** 1 · **Traces:** SET-4, WT-3
**Acceptance criteria:**
- GIVEN Settings is open, WHEN I select kilograms, THEN all values display in kg, stored gram values are unchanged, and the choice persists on the device.

---

## EP-12 — Cross-cutting Quality (enablers)

### US-QUAL-01 — Respect the user's preferred text size
**As a** user with larger text settings, **I want** text to scale, **so that** I can read the product.
**Priority:** Should · **Estimate:** 3 · **Traces:** NFR-4
**Acceptance criteria:**
- GIVEN the preferred text size is increased, WHEN I browse the product, THEN text scales without breaking layout.

### US-QUAL-02 — Screen-reader chart summary
**As a** screen-reader user, **I want** a summary of the weight chart, **so that** I can understand the trend.
**Priority:** Should · **Estimate:** 3 · **Traces:** NFR-4
**Acceptance criteria:**
- GIVEN a screen reader is active, WHEN I focus the weight chart, THEN an accessible summary (latest value + trend direction) is read.

### US-QUAL-03 — German + English localization, metric default
**As a** German-speaking user, **I want** the whole product in German, **so that** I can use it in my language.
**Priority:** Must · **Estimate:** 5 · **Traces:** NFR-5
**Acceptance criteria:**
- GIVEN the device language is German, WHEN I open the product, THEN all text is German and units default to metric.

### US-QUAL-04 — Honest service-status indicator
**As a** user, **I want** the status indicator to be truthful, **so that** I trust it.
**Priority:** Should · **Estimate:** 3 · **Traces:** NFR-6
**Acceptance criteria:**
- GIVEN working connectivity, WHEN a single update fails, THEN the indicator does NOT show offline.
- GIVEN repeated failures or a direct check that the service/account is unavailable, THEN it shows offline.
- GIVEN the product relaunches, THEN it does not claim a previous session's status before a fresh outcome.
- GIVEN rapid consecutive updates, THEN a single stable "syncing" state is shown.

### US-QUAL-05 — Last-write-wins conflict resolution
**As a** member, **I want** conflicting edits resolved predictably, **so that** nothing fails or duplicates.
**Priority:** Must · **Estimate:** 3 · **Traces:** NFR-2, SYNC-5
**Acceptance criteria:**
- GIVEN two members edit the same entry offline, WHEN both reconnect, THEN the most recent edit wins and no duplicate appears.

### US-QUAL-06 — Privacy: no analytics, safe diagnostics
**As a** user, **I want** my health data kept private, **so that** it isn't harvested.
**Priority:** Must · **Estimate:** 3 · **Traces:** NFR-3
**Acceptance criteria:**
- GIVEN I log a health entry, WHEN it is saved, THEN it is kept only in the product's own shared service and locally, with no third-party analytics receiving content.
- GIVEN diagnostics are recorded, THEN they contain identifiers/counts/outcomes only — never rabbit names, note text, photo content, or weight values.

### US-QUAL-07 — Bounded dashboard load and load-once reuse
**As a** user, **I want** the dashboard to open quickly, **so that** the product feels responsive.
**Priority:** Should · **Estimate:** 5 · **Traces:** NFR-7
**Acceptance criteria:**
- GIVEN a household with several rabbits, WHEN I open the dashboard, THEN it loads in a bounded time rather than a long serialized chain.
- GIVEN the dashboard already loaded a rabbit's data, WHEN I open that rabbit, THEN it is NOT loaded again.
- GIVEN a remote change arrives, WHEN I navigate next, THEN only affected data reloads and already-loaded data is reused.

### US-QUAL-08 — Visible actions and confirmations
**As a** user, **I want** important actions to be discoverable and safe, **so that** I don't miss or mis-trigger them.
**Priority:** Should · **Estimate:** 2 · **Traces:** NFR-9
**Acceptance criteria:**
- GIVEN a primary/destructive action (delete household, edit profile, delete entry), THEN it is a directly visible control, not a hidden gesture.
- GIVEN I trigger a destructive action, THEN I must confirm, and the confirmation states the consequence.

### US-QUAL-09 — Data integrity invariants
**As a** product owner, **I want** core invariants enforced, **so that** the data model stays consistent.
**Priority:** Must · **Estimate:** 3 · **Traces:** NFR-10, DM-1…DM-6
**Acceptance criteria:**
- GIVEN any weight value, THEN it is stored in the canonical gram unit.
- GIVEN two households, THEN neither references the other's tags.
- GIVEN an archived rabbit, THEN none of its data is mutable.

---

## 12. Proposed delivery phases

Ordering proposal for the reimplementation; revisit once §13 decisions land.

| Phase | Theme | Epics / notable stories | Exit criteria |
|---|---|---|---|
| **P0 — Foundation** | Shared data + identity + household skeleton | EP-2 (SYNC-01,02,04,05,09), US-HS-01…05, EP-11 core (SET-01,02,06) | A household can be created and its data persists and round-trips. |
| **P1 — MVP core loop** | Rabbits, weight, dashboard | EP-4, EP-5, EP-3, EP-1 (US-HS-14) | Create rabbits, log weight in ≤3 taps, see trends and drop alerts. |
| **P2 — Health & evidence** | Health logging, photos, detail views, full history | EP-6, EP-8, remaining EP-3 history views | Symptom + weight correlation visible; photos attach and render. |
| **P3 — Sharing hardening** | Invite/accept, member list, deletion, propagation | US-HS-06…25, SYNC-06,10,11, US-QUAL-04 | Two accounts share one household reliably; invitations accept deterministically. |
| **P4 — Handoff & preferences** | Export, reminders, settings polish | EP-10, EP-9, US-SET-03…05, US-QUAL-01…03 | PDF/CSV for vet visits; reminders; localization. |
| **P5 — Medications** | Registry, schedules, dosing, reminders | EP-7 (with §13.1 resolved) | Household tracks treatment and prevents double-dosing. |
| **P6 — Performance & polish** | Read-path performance, quality enablers | US-QUAL-07, remaining EP-12 | Dashboard cold load bounded; already-loaded data reused; all quality bars met. |

> **Note:** the phase order deliberately front-loads *sharing correctness* (P0/P3) because
> that is where the current product historically struggled (IMP-2), and defers medication
> (P5) only because it is the largest isolated capability.

---

## 13. Open decisions affecting stories

| # | Decision | Blocks | Notes |
|---|---|---|---|
| **13.1** | **Write model** — connectivity-required saves vs true offline-first. | US-MED-23, shapes EP-2 and all save stories | The product today requires connectivity (SYNC-2); the medication capability contradicts this (IMP-1). Must be resolved before EP-2/EP-7 are built. |
| **13.2** | **Sharing model** — keep owner-dependent sharing, or add ownership transfer. | US-HS-25, IMP-8 | Affects HS-13. |
| **13.3** | **Reminder unification** — one mechanism or two. | EP-7 reminders, EP-9 | IMP-5. |
| **13.4** | **Migration** — must existing users' data be importable? | Cross-cutting | IMP-7. |
| **13.5** | **Chart baseline band** — in or out. | EP-5 | IMP-4. |

---

## 14. Coverage matrix (requirement → stories)

| Requirement | Stories |
|---|---|
| HS-1 | US-HS-01, 02 |
| HS-2 | US-HS-01 |
| HS-3 | US-HS-03, 04 |
| HS-4 | US-HS-04, 05 |
| HS-5 | US-HS-06, 07, 08, 09, 10 |
| HS-6 | US-HS-11, 12 |
| HS-7 | US-HS-13 |
| HS-8 | US-HS-14 |
| HS-9 | US-HS-15, 16, 17, 18 |
| HS-10 | US-HS-19, 20, 21, 22, 23 |
| HS-11 | US-HS-06, 24 |
| HS-12 | US-HS-25 |
| HS-13 | *(accepted risk — no story)* |
| SYNC-1 | US-SYNC-01 |
| SYNC-2 | US-SYNC-02, 03 |
| SYNC-3 | US-SYNC-04, 05, 06 |
| SYNC-4 | US-SYNC-07, 08 |
| SYNC-5 | US-SYNC-09 |
| SYNC-6 | US-SYNC-10 |
| SYNC-7 | US-SYNC-11 |
| DASH-1 | US-DASH-01 |
| DASH-2 | US-DASH-02 |
| DASH-3 | US-DASH-03 |
| DASH-4 | US-DASH-04, 05 |
| DASH-5 | US-DASH-06 |
| DASH-6 | US-DASH-07 |
| DASH-7 | US-DASH-07, 08 |
| DASH-8 | US-DASH-09 |
| DASH-9 | US-DASH-10 |
| RAB-1 | US-RAB-01, 02 |
| RAB-2 | US-RAB-03 |
| RAB-3 | US-RAB-04 |
| RAB-4 | US-RAB-05, US-DASH-09 |
| RAB-5 | US-RAB-06 |
| RAB-6 | US-RAB-07 |
| RAB-7 | US-RAB-08 |
| RAB-8 | US-RAB-09 |
| RAB-9 | US-RAB-07 |
| RAB-10 | US-RAB-10 |
| WT-1 | US-WT-01 |
| WT-2 | US-WT-02, 03, US-DASH-10 |
| WT-3 | US-WT-04, 05, US-SET-06 |
| WT-4 | US-WT-06, 07 |
| WT-5 | US-WT-08 |
| WT-6 | US-WT-09, 10 |
| WT-7 | US-WT-11, 12, US-DASH-04, 05 |
| WT-8 | US-WT-13, 14, 15 |
| HL-1 | US-HL-01, 03 |
| HL-2 | US-HL-02 |
| HL-3 | US-HL-04 |
| HL-4 | US-HL-05, 06, 07 |
| HL-5 | US-HL-08 |
| HL-6 | US-HL-09 |
| HL-7 | US-HL-10, 11 |
| MED-1 | US-MED-01, 02, 03 |
| MED-2 | US-MED-04, 05 |
| MED-3 | US-MED-06, 07, 08, 09 |
| MED-4 | US-MED-10, US-EXP-06 |
| MED-5 | US-MED-12, 13 |
| MED-6 | US-MED-14, 15 |
| MED-7 | US-MED-16 |
| MED-8 | US-MED-11 |
| MED-9 | US-MED-17, 18, 19 |
| MED-10 | US-MED-20 |
| MED-11 | US-MED-09, 21, 23 |
| MED-12 | US-MED-22, US-RAB-09 |
| PHOTO-1 | US-PHOTO-01, 02 |
| PHOTO-2 | US-PHOTO-03 |
| PHOTO-3 | US-PHOTO-04 |
| PHOTO-4 | US-PHOTO-05 |
| PHOTO-5 | US-PHOTO-06 |
| REM-1 | US-REM-01, 02 |
| REM-2 | US-REM-03 |
| REM-3 | US-REM-04 |
| EXP-1 | US-EXP-01, 02 |
| EXP-2 | US-EXP-03, 04 |
| EXP-3 | US-EXP-05 |
| EXP-4 | US-EXP-06 |
| SET-1 | US-SET-01 |
| SET-2 | US-SET-02, 03 |
| SET-3 | US-SET-04, 05 |
| SET-4 | US-SET-06, US-WT-04 |
| NFR-1 | US-SYNC-02 |
| NFR-2 | US-SYNC-09, US-QUAL-05 |
| NFR-3 | US-QUAL-06 |
| NFR-4 | US-QUAL-01, 02 |
| NFR-5 | US-QUAL-03, US-SET-04 |
| NFR-6 | US-QUAL-04 |
| NFR-7 | US-QUAL-07 |
| NFR-8 | US-SYNC-03, US-HS-23 |
| NFR-9 | US-QUAL-08 |
| NFR-10 | US-QUAL-09 |

**Totals:** 12 epics · 136 stories · 1 story (US-MED-23) is blocked on an open decision.

---

## 15. Improvement backlog (candidate stories, not committed)

Derived from the *Improvement Candidates* in the requirements document; promote to real
stories once decided.

| Ref | Candidate story | Requirement | Priority |
|---|---|---|---|
| IMP-1 | Resolve the offline-save contradiction and apply one write model uniformly. | US-MED-23 / SYNC-2 | Must |
| IMP-2 | Make sharing correctness a first-class, testable concern (convergence + end-to-end acceptance). | EP-1, EP-2 | Must |
| IMP-3 | Adopt load-once, parallel loading, serve-from-loaded-data-first, and invalidate only affected data. | US-QUAL-07 | Should |
| IMP-4 | Decide on the chart baseline/healthy-range band. | EP-5 | Could |
| IMP-5 | Unify weigh and medication reminders into one scheduling/follow-up mechanism. | EP-7, EP-9 | Should |
| IMP-6 | Codify visible-action/confirmation conventions as a design-review checklist. | US-QUAL-08 | Should |
| IMP-7 | Define a data migration / export-import path for existing users. | Cross-cutting | Should |
| IMP-8 | Evaluate ownership-transfer or backup/restore for the accepted owner-dependency risk. | US-HS-25 | Could |
