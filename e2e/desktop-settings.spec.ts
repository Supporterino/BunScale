/*
 * SET-1's `[desktop]` scenario, with HS-3 and HS-4 at the expanded baseline
 * (task 5.17).
 *
 * **What this file owns that `household-flow.spec.ts` does not.** Task 5.14
 * drives the compact 390×844 flow — onboarding, a blank name, creating a second
 * household from Settings, the switch — and deliberately skips the expanded
 * project, because SET-1 has a *different* desktop clause: at 1280×800 Settings
 * is still its own screen, but the frame around it is a sidebar rather than a
 * tab bar, and "the dashboard remains reachable without leaving the product" is
 * a claim about that sidebar. So this file skips where 5.14 runs: it
 * re-asserts HS-4, DM-3 and HS-3 at the expanded baseline, and adds what 5.14
 * cannot see — the desktop arrangement, the desktop return path and the
 * dispatch the switch sends.
 *
 * **"Its own screen" means a route change inside the same frame, not a pane.**
 * `SettingsScreen` is rendered by its own route (`src/routes/settings.tsx`) in
 * its own `AppShell`, with `settings.title` as the screen's `h1`. Opening it
 * therefore unmounts the dashboard: `data-household-entry="household"` and the
 * household's `h1` are absent, while the sidebar stays on screen. The tests
 * below assert exactly that, plus the arrangement (`sidebar` laid out, `tabs`
 * not, one exposed `navigation` landmark named "Primary"), which is the part a
 * 1280×800 run can see and a component spec cannot (`AppShell` renders both
 * landmarks into the DOM at every width).
 *
 * **Why the household book is seeded and not created through the UI.** Re-driving
 * 5.14's creation flow would spend two creations and the onboarding assertions
 * that file already owns to reach the state this task is not about: a device
 * that *already* holds two. The mock's page-local book
 * (`src/test/mock-ipc.ts`) is the only answer a switch can agree with. An
 * override registered through `stubCommandResult` lasts for the whole document
 * (`e2e/fixtures.ts`), so a pinned `list_households` would never reflect the
 * switch; and the Settings marker is computed from the active read whenever
 * that read has data (`src/features/settings/households.ts`), which
 * `useSwitchActiveHousehold.onSuccess` writes directly
 * (`src/features/households/queries.ts`). A pinned list would therefore keep
 * contradicting the book the reads derive from. Seeding through
 * `create_household` — the same command the app dispatches, with two operation
 * ids — makes every later read agree with the command this test observes.
 * Every state asserted afterwards was produced by that book and by clicking.
 *
 * **Why the switch is read from a wrapped `invoke`.** HS-3's claim is that the
 * selected household *becomes* the active one in the backend's answer, and the
 * marker moving is the screen's rendering of that answer. Recording
 * `set_active_household`'s own arguments — the technique
 * `household-flow.spec.ts` uses for a call log and `save-failure.spec.ts` for a
 * retry's ids — makes "asked to switch, once, with the right id and an
 * operation identity" a measurement rather than an inference from the badge.
 * A hash navigation is the same document, so the wrapper installed before
 * `page.goto('/#/')` is still in place when the row is clicked.
 */
import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';

/** The household created first; after seeding it is the *non-active* row. */
const FIRST_HOUSEHOLD = 'Maple Cottage';

/** The household created second; the book makes the newest creation active. */
const SECOND_HOUSEHOLD = 'Second Home';

/** Copy read from `src/features/i18n/messages.ts`. */
const GEAR = 'Open Settings & Profile';
const SETTINGS_TITLE = 'Settings & Profile';
const DASHBOARD_DESTINATION = 'Your household';
const BACK = 'Back';
const ACTIVE = 'Active';
const MAKE_ACTIVE = 'Make active';

/** `playwright.config.ts`'s expanded baseline: the macOS window geometry. */
const EXPANDED_WIDTH = 1280;

/** The screen's `h1` — `NavigationBar`'s title on every screen. */
function screenTitle(page: Page, name: string) {
  return page.getByRole('heading', { level: 1, name });
}

/** The dashboard's `main`, published by the entry guard (task 3.3). */
function dashboard(page: Page) {
  return page.locator('[data-household-entry="household"]');
}

/** The arrangement the expanded stylesheet lays out (above 768 px). */
function sidebar(page: Page) {
  return page.locator('[data-navigation-arrangement="sidebar"]');
}

/** The compact arrangement, present in the DOM and hidden at this width. */
function tabBar(page: Page) {
  return page.locator('[data-navigation-arrangement="tabs"]');
}

/** The one row HS-4 says is indicated as active. */
function activeRow(page: Page) {
  return page.locator('li[aria-current="true"]');
}

/** The visible "Active" badge HS-4 renders on that row. */
function activeBadge(page: Page) {
  return page.getByText(ACTIVE, { exact: true });
}

/** A household row's own switch control, named by the household it lists. */
function switchTo(page: Page, name: string) {
  return page.getByRole('button', { name: new RegExp(name) });
}

/*
 * The page-side shapes, declared as module-scope *types* so the functions
 * handed to `page.evaluate` can name them while running in WebKit (the same
 * technique as `fixtures.ts`'s `TauriInternals`: types are erased, values are
 * not). `mockIPC` installs an `async` `invoke`, so the seed awaits its answers.
 */
type SeedWindow = {
  __TAURI_INTERNALS__: {
    invoke: (command: string, args?: unknown) => Promise<unknown>;
  };
};

type RecorderWindow = {
  __TAURI_INTERNALS__: {
    invoke: (command: string, args?: unknown, options?: unknown) => unknown;
  };
  __E2E_SWITCH_ATTEMPTS__?: SwitchAttempt[];
};

/** One recorded switch, as the generated bindings dispatched it. */
type SwitchAttempt = {
  readonly householdId: string;
  readonly requestId: string;
};

/** The ids the book assigned, so the switch can be checked against them. */
type SeededHouseholds = {
  readonly firstId: string;
  readonly secondId: string;
};

/**
 * Fill the mocked book with two households, newest active (HS-1, HS-4).
 *
 * Two `create_household` calls with distinct operation ids: the mock refuses a
 * blank name and treats a repeated request id as a replay (D0.14), so distinct
 * ids are what makes this two households rather than one answered twice. The
 * ids of the answers are returned because they are the only stable names for
 * the rows — asserting a hard-coded `household-1` would couple this spec to the
 * mock's id scheme rather than to what the switch was asked to select.
 */
async function seedHouseholds(page: Page): Promise<SeededHouseholds> {
  return page.evaluate(
    async ({ first, second }) => {
      const internals = (window as unknown as SeedWindow).__TAURI_INTERNALS__;

      const idOf = (created: unknown): string => {
        const id = (created as { id?: unknown }).id;

        if (typeof id !== 'string' || id === '') {
          throw new Error('create_household answered no household id');
        }

        return id;
      };

      const createdFirst = await internals.invoke('create_household', {
        name: first,
        requestId: 'seed-first-household',
      });
      const createdSecond = await internals.invoke('create_household', {
        name: second,
        requestId: 'seed-second-household',
      });

      return { firstId: idOf(createdFirst), secondId: idOf(createdSecond) };
    },
    { first: FIRST_HOUSEHOLD, second: SECOND_HOUSEHOLD },
  );
}

/*
 * Install the switch recorder on this document's mocked `invoke`.
 *
 * It records the two arguments the operation carries and nothing else: names
 * are copy and ids are opaque, so this is a call log of the switch, not a
 * payload log. Installed after the shared fixture has asserted that `invoke` is
 * a function, and it survives `page.goto('/#/')` because a hash navigation is
 * the same document.
 */
async function recordSwitches(page: Page): Promise<void> {
  await page.evaluate(() => {
    const scope = window as unknown as RecorderWindow;
    const dispatch = scope.__TAURI_INTERNALS__.invoke;
    const attempts: SwitchAttempt[] = [];

    scope.__E2E_SWITCH_ATTEMPTS__ = attempts;
    scope.__TAURI_INTERNALS__.invoke = (command, args, options) => {
      if (command === 'set_active_household') {
        const payload = (args ?? {}) as {
          householdId?: unknown;
          requestId?: unknown;
        };

        attempts.push({
          householdId:
            typeof payload.householdId === 'string' ? payload.householdId : '',
          requestId:
            typeof payload.requestId === 'string' ? payload.requestId : '',
        });
      }

      return dispatch(command, args, options);
    };
  });
}

/** Every `set_active_household` this document dispatched, in order. */
async function switchAttempts(page: Page): Promise<SwitchAttempt[]> {
  return page.evaluate(
    () => (window as unknown as RecorderWindow).__E2E_SWITCH_ATTEMPTS__ ?? [],
  );
}

/**
 * Seed two households and open the dashboard the guard decides from that book.
 *
 * The second household is the book's active one, so its name is the dashboard's
 * title (task 4.2). Asserting that here keeps every test below from having to
 * re-establish which household the app currently presents.
 */
async function openDashboardWithTwoHouseholds(
  page: Page,
): Promise<SeededHouseholds> {
  const seeded = await seedHouseholds(page);

  await page.goto('/#/');

  await expect(dashboard(page)).toBeVisible();
  await expect(screenTitle(page, SECOND_HOUSEHOLD)).toBeVisible();

  return seeded;
}

/** SET-1's gear, activated from the dashboard it sits on. */
async function openSettingsFromDashboard(page: Page): Promise<void> {
  await page.getByRole('button', { name: GEAR }).click();

  await expect(screenTitle(page, SETTINGS_TITLE)).toBeVisible();
  expect(new URL(page.url()).hash).toBe('#/settings');
}

test.describe('SET-1 [desktop] and the household switch at 1280×800', () => {
  test.skip(
    ({ viewport }) => viewport?.width !== EXPANDED_WIDTH,
    'task 5.17 is the expanded baseline; task 5.14 owns the compact one',
  );

  test('Gear control opens Settings as its own screen', async ({ page }) => {
    await openDashboardWithTwoHouseholds(page);

    // SET-1: a gear-shaped control is present on the dashboard. A role query
    // names it, and the tag check is what makes "a real control, not a div" a
    // fact rather than an assumption about how the glyph is wrapped.
    const gear = page.getByRole('button', { name: GEAR });

    await expect(gear).toBeVisible();
    expect(await gear.evaluate((node) => node.tagName)).toBe('BUTTON');
    expect(await gear.evaluate((node) => node.getAttribute('type'))).toBe(
      'button',
    );

    await gear.click();

    /*
     * Its own screen: the route changed, Settings' title is the screen's `h1`,
     * and the dashboard's region and its household title are gone rather than
     * sitting beside Settings. The shell is the thing that carries over, which
     * is why the sidebar is still the laid-out arrangement below.
     */
    await expect(screenTitle(page, SETTINGS_TITLE)).toBeVisible();
    expect(new URL(page.url()).hash).toBe('#/settings');

    await expect(dashboard(page)).toHaveCount(0);
    await expect(screenTitle(page, SECOND_HOUSEHOLD)).toHaveCount(0);

    // SET-1 [desktop]: the frame at this baseline is the sidebar, not the tab
    // bar — and `display: none` leaves exactly one navigation landmark exposed.
    const navigation = page.getByRole('navigation', { name: 'Primary' });

    await expect(sidebar(page)).toBeVisible();
    await expect(tabBar(page)).toBeHidden();
    await expect(navigation).toHaveCount(1);
  });

  test('Settings is its own screen on desktop [desktop]', async ({ page }) => {
    await openDashboardWithTwoHouseholds(page);
    await openSettingsFromDashboard(page);

    // SET-1 [desktop]: Settings keeps the Back control (SET-1 [mobile]'s
    // affordance) *and* the sidebar's own dashboard destination, which is the
    // desktop way back that does not leave the product.
    await expect(page.getByRole('button', { name: BACK })).toBeVisible();

    const destination = sidebar(page).getByRole('link', {
      name: DASHBOARD_DESTINATION,
    });

    await expect(destination).toBeVisible();
    await destination.click();

    // Back on the dashboard, presenting the same household: the return was a
    // route change inside the app, not a reload that lost the device's book.
    await expect(dashboard(page)).toBeVisible();
    await expect(screenTitle(page, SECOND_HOUSEHOLD)).toBeVisible();
    await expect(screenTitle(page, SETTINGS_TITLE)).toHaveCount(0);

    expect(new URL(page.url()).hash).toBe('#/');
  });

  test('All households are listed with the active one indicated', async ({
    page,
  }) => {
    await openDashboardWithTwoHouseholds(page);
    await openSettingsFromDashboard(page);

    // HS-4: every household the person holds is listed…
    await expect(
      page.getByText(FIRST_HOUSEHOLD, { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByText(SECOND_HOUSEHOLD, { exact: true }),
    ).toBeVisible();

    // …exactly one is indicated as active, and it is the book's active one.
    await expect(activeBadge(page)).toHaveCount(1);
    await expect(activeRow(page)).toHaveCount(1);
    await expect(activeRow(page)).toContainText(SECOND_HOUSEHOLD);

    // The other row offers the switch instead of a second marker.
    await expect(switchTo(page, FIRST_HOUSEHOLD)).toBeVisible();
    await expect(switchTo(page, FIRST_HOUSEHOLD)).toContainText(MAKE_ACTIVE);
  });

  test('Switching asks the backend and moves the marker', async ({ page }) => {
    await recordSwitches(page);

    const seeded = await openDashboardWithTwoHouseholds(page);

    await openSettingsFromDashboard(page);

    // Before the switch: the second household is the one the app presents and
    // the only row HS-4 marks (DM-3's "exactly one").
    await expect(activeRow(page)).toHaveCount(1);
    await expect(activeRow(page)).toContainText(SECOND_HOUSEHOLD);
    await expect(switchTo(page, FIRST_HOUSEHOLD)).toBeVisible();

    // HS-3: select the household that is not active, through its row's control.
    await switchTo(page, FIRST_HOUSEHOLD).click();

    // The marker follows the confirmed switch and remains a single row.
    await expect(activeRow(page)).toHaveCount(1);
    await expect(activeRow(page)).toContainText(FIRST_HOUSEHOLD);
    await expect(activeBadge(page)).toHaveCount(1);
    await expect(switchTo(page, SECOND_HOUSEHOLD)).toContainText(MAKE_ACTIVE);

    /*
     * The backend was asked to switch, once, for the row that was clicked, and
     * the request carried an operation identity (D0.14). Read from the
     * dispatched arguments, so this is not the screen echoing the badge it drew.
     */
    const attempts = await switchAttempts(page);

    expect(attempts).toHaveLength(1);
    expect(attempts[0]?.householdId).toBe(seeded.firstId);
    expect(attempts[0]?.requestId).not.toBe('');
  });

  test('Switching the active household changes the presented data', async ({
    page,
  }) => {
    await openDashboardWithTwoHouseholds(page);
    await openSettingsFromDashboard(page);
    await switchTo(page, FIRST_HOUSEHOLD).click();
    await expect(activeRow(page)).toContainText(FIRST_HOUSEHOLD);

    // The way back at this baseline is the sidebar destination.
    const destination = sidebar(page).getByRole('link', {
      name: DASHBOARD_DESTINATION,
    });

    await destination.click();

    /*
     * HS-3's second clause at the expanded baseline: what is presented is the
     * newly active household's, and no longer the previously active one's. The
     * title is the dashboard's only household fact in P0 (4.2), so its being
     * the first household's *and* not the second's is the whole claim here.
     */
    await expect(dashboard(page)).toBeVisible();
    await expect(screenTitle(page, FIRST_HOUSEHOLD)).toBeVisible();
    await expect(screenTitle(page, SECOND_HOUSEHOLD)).toHaveCount(0);

    expect(new URL(page.url()).hash).toBe('#/');
  });
});
