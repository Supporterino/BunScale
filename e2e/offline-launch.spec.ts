/*
 * SYNC-3's two offline-launch scenarios, driven end to end (task 5.15).
 *
 * SYNC-3 asks for two different screens from one condition — the shared service
 * is unreachable at launch — and the difference between them is what the device
 * already holds:
 *
 *   - a device whose household's data was loaded before shows that data, and
 *     says where it came from through the local-copy notice (task 4.4);
 *   - a device that never loaded it gets the explicit connectivity-required
 *     state (task 4.4), and is never shown an empty book as "no household" —
 *     which is what onboarding would claim, and what SYNC-3 forbids.
 *
 * **Why the arrangement is overrides.** "This device loaded the household's
 * data before this launch" is a set of command answers, and `stubCommandResult`
 * is what states them. Each answer is registered after the shared fixture has
 * booted the app and before the navigation to `/`: `openEntry` changes only the
 * fragment (`/#/design-preview` → `/#/`), so the page keeps the document the
 * fixture booted, the overrides stay registered against that document, and they
 * are what the guard reads when the hash navigation reaches `/`. That is also
 * why this spec navigates rather than reloads: a real document load
 * re-evaluates `src/test/mock-ipc.ts`, drops every override and starts again
 * from its defaults — a different arrangement, not this one.
 *
 * **What makes each test discriminate.** The mock's defaults are the fresh,
 * reachable, no-household device — the one state in which onboarding *is* a
 * conclusion (SYNC-3) — so neither test can pass on them. Test 1's book answers
 * a household: `hasHousehold` is "the list is non-empty **or** the active read
 * is non-null", so `get_active_household` alone decides, and the list override
 * is there to keep the two reads describing the same device. Test 2 needs the
 * status to answer `unreachable`, which is what turns the same empty book from
 * onboarding into the connectivity-required slot. Test 2's `Offline` badge is
 * the second reading of that override: the surface's copy branches on the live
 * status, so the badge appears only once the unreachable answer actually
 * arrived and the view is `offline`.
 *
 * Test 1 also stubs `get_service_status` as unreachable, but that answer is
 * arrangement only: nothing on the dashboard reads a service status
 * (`LocalCopyNotice` branches on `load_household`'s `source` and `loaded_at`
 * alone), and the entry guard skips the status read entirely when a household
 * exists. The previously loaded copy is evidenced by the `load_household`
 * answer, which reports `source: local` with a `loaded_at` — the backend's own
 * "served the copy because the refresh could not reach the service" — and is
 * the only thing the notice reads.
 *
 * **Both baselines.** Nothing asserted here is adaptive — the connectivity slot
 * is a standalone single-column `Screen`, and the dashboard's title and notice
 * sit inside its `main` at either width — so the two Playwright projects each
 * run this pair rather than one baseline claiming it alone. 390×844 is the
 * mobile-first baseline this spec is written at; the 1280×800 run is macOS's
 * window geometry and costs a second page load.
 *
 * The requirement's sentences are asserted as the catalog's own strings
 * (`src/features/i18n/messages.ts`): "an explicit state stating that
 * connectivity is required" is judged by what it tells the person, so the
 * literals below are copies of catalog entries and must be updated with them.
 * The entry *state* is asserted through `data-household-entry`, which the three
 * entry screens publish — the dashboard (4.2), onboarding (4.1) and the
 * connectivity slot (4.4).
 */
import type { Page } from '@playwright/test';
import { expect, stubCommandResult, test } from './fixtures';

/** Every mocked observation in this file is a fixed instant (`testing-strategy`). */
const AT = '2026-10-20T09:00:00Z';

/** The copy the connectivity-required state presents (SYNC-3, task 4.4). */
const REQUIRED_TITLE = 'Connect to continue';
const REQUIRED_BODY =
  'Your household data has not been loaded on this device yet. Connect to the internet and try again.';

/** The dashboard's local-copy notice (SYNC-3, task 4.4). */
const LOCAL_COPY_NOTICE =
  'Showing saved data. Recent changes from other members may not be here yet.';

/** The status badge the connectivity screen shows for `state: unreachable`. */
const OFFLINE_BADGE = 'Offline';

/*
 * The household this device held before the launch. It is the contract's shape
 * (`Household`, `src/bindings.ts`), and the summary is its `list_households`
 * (HS-4) row — the guard reads both, so both answer the same device.
 */
const HOUSEHOLD = {
  id: 'household-a',
  name: 'Maple Cottage',
  created_at: AT,
  owner_member_id: 'member-a',
  is_active: true,
  is_owner: true,
};

const HOUSEHOLD_SUMMARY = {
  id: HOUSEHOLD.id,
  name: HOUSEHOLD.name,
  created_at: HOUSEHOLD.created_at,
  is_active: HOUSEHOLD.is_active,
  is_owner: HOUSEHOLD.is_owner,
  member_count: 1,
};

/*
 * The launch condition: the shared service did not answer (`cause:
 * connectivity`), which is a status and not a rejection (SYNC-3). `observed_at`
 * is set because the tracker recorded the three unanswered probes that produce
 * this state (D0.13); null would be "we have not looked" and would make the
 * guard probe again instead of deciding on this answer.
 */
const UNREACHABLE = {
  state: 'unreachable',
  account_valid: null,
  cause: 'connectivity',
  observed_at: AT,
};

/*
 * The previously loaded copy: `source: local` with a `loaded_at` is the
 * contract's "served the local copy" answer (D0.2, SYNC-3). `items` is empty
 * because P0's dashboard presents the household's identity, not rabbit rows
 * (P1 owns those, DASH-1..9) — the provenance pair is what the notice reads,
 * and an invented row would be a value no surface under test would show.
 */
const LOADED_LOCAL_COPY = {
  household_id: HOUSEHOLD.id,
  items: [],
  source: 'local',
  loaded_at: '2026-10-20T08:40:00Z',
};

/** The screen's `h1` — `NavigationBar`'s title, or the surface's heading. */
function screenTitle(page: Page, name: string) {
  return page.getByRole('heading', { level: 1, name });
}

/** One entry state by the stable name its screen publishes. */
function entryScreen(
  page: Page,
  entry: 'household' | 'onboarding' | 'service-required',
) {
  return page.locator(`[data-household-entry="${entry}"]`);
}

/** Opens the entry route; the guard decides from the answered reads. */
async function openEntry(page: Page): Promise<void> {
  await page.goto('/#/');
}

test.describe('SYNC-3: an unreachable service at launch', () => {
  test('Previously loaded data is shown while unreachable', async ({
    page,
  }) => {
    const dashboard = entryScreen(page, 'household');
    const notice = page.getByText(LOCAL_COPY_NOTICE);

    /*
     * The device's held household, in both reads the guard performs, and its
     * collection as the local copy the refresh could not replace. Registered
     * before the navigation below: the hash change keeps this document, so the
     * guard reads these answers when it reaches `/`.
     */
    await stubCommandResult(page, 'get_service_status', UNREACHABLE);
    await stubCommandResult(page, 'list_households', [HOUSEHOLD_SUMMARY]);
    await stubCommandResult(page, 'get_active_household', HOUSEHOLD);
    await stubCommandResult(page, 'load_household', LOADED_LOCAL_COPY);

    await openEntry(page);

    // SYNC-3: the previously loaded household is what is presented — its name
    // is the dashboard's title, and it comes from the device's book, which
    // needs no network.
    await expect(dashboard).toBeVisible();
    await expect(screenTitle(page, HOUSEHOLD.name)).toBeVisible();

    // …and it says it is a copy, so "showing data" is not mistaken for "synced".
    await expect(notice).toBeVisible();

    /*
     * And the connectivity-required state is not shown: the route's screen is
     * one `switch` over the decision, so exactly one entry surface is mounted
     * and it is the dashboard asserted above.
     */
    await expect(page.locator('[data-household-entry]')).toHaveCount(1);
  });

  test('Nothing loaded and unreachable', async ({ page }) => {
    const connectivity = entryScreen(page, 'service-required');

    /*
     * The book's defaults are already "this device holds no household"; the one
     * answer that differs from a fresh reachable account is the status. It is
     * what the guard decides on, so it is the override this test needs — and
     * the only one, which is why the `Offline` badge below is evidence that the
     * answer reached the surface rather than that a second stub did.
     */
    await stubCommandResult(page, 'get_service_status', UNREACHABLE);

    await openEntry(page);

    // SYNC-3: the explicit state, with its heading and its body.
    await expect(connectivity).toBeVisible();

    // The scenario's precondition, read from the attribute the surface
    // publishes; the copy further down branches on the same live status.
    await expect(connectivity).toHaveAttribute(
      'data-service-state',
      'unreachable',
    );

    await expect(screenTitle(page, REQUIRED_TITLE)).toBeVisible();
    await expect(page.getByText(REQUIRED_BODY)).toBeVisible();

    // The copy branched on the live status: `unreachable` is the offline view,
    // not the account half's neutral heading.
    await expect(
      connectivity.getByText(OFFLINE_BADGE, { exact: true }),
    ).toBeVisible();

    /*
     * And no absence is presented as the household's current data: the route
     * stays on `/`, so the empty book was not read as "no household" and no
     * first-run onboarding was started, and exactly one entry surface is
     * mounted — the connectivity slot asserted above, not a dashboard behind
     * it.
     */
    expect(new URL(page.url()).hash).toBe('#/');
    await expect(page.locator('[data-household-entry]')).toHaveCount(1);
  });
});
