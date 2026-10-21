/*
 * The first-run household flow, driven end to end (task 5.14; HS-1, HS-2, HS-3,
 * HS-4, DM-3, SET-1 `[mobile]`; stories US-HS-01 and US-HS-04).
 *
 * What this spec adds to `household-entry.spec.ts` (task 3.3's entry guard) is
 * the part only a real page can show: the person *does* something. A launch
 * with no household lands on onboarding, a blank name is refused where the
 * field is and never reaches the backend, a valid name creates a household and
 * the guard sends the person to its dashboard, the gear opens Settings as its
 * own screen, and switching the active household moves both the marker and the
 * dashboard's title — all at 390×844, the compact baseline SET-1's `[mobile]`
 * scenario names.
 *
 * **Driven by interaction, not by seeding.** The household book lives in the
 * page's IPC mock (`src/test/mock-ipc.ts`), whose `create_household` and
 * `set_active_household` handlers mutate it and whose reads derive their answers
 * from it. Every state this spec asserts was therefore produced by typing and
 * clicking: the first household comes from onboarding and the second from
 * Settings' own inline form (HS-4's "create an additional household"), which is
 * what also gives the switch two rows to choose between. Nothing here writes a
 * query cache or a router history entry directly.
 *
 * **Why this file reads `window.__TAURI_INTERNALS__.invoke`.** HS-1 says a blank
 * name creates **no** household. The app refuses it before the command
 * (`HouseholdOnboardingScreen`), and the assertion that no `create_household`
 * was dispatched is what makes that a fact rather than an inference from the
 * screen staying put. The recorder below wraps the mock's `invoke` for this
 * document, so the same dispatch path the app uses is what gets observed. The
 * blank test also requires the guard's own `list_households` read to have been
 * recorded first, so an empty log cannot pass as "no calls were made".
 *
 * **Scope.** This is the compact project's spec. Task 5.17 owns the 1280×800
 * counterpart (SET-1's `[desktop]` scenario and HS-3 at that baseline), so this
 * spec skips there rather than asserting a second baseline's arrangement through
 * a test named for the mobile one.
 */
import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';

/** The household onboarding creates first; its name is the dashboard's title. */
const FIRST_HOUSEHOLD = 'Maple Cottage';

/** The household created from Settings; it becomes active on creation (HS-4). */
const SECOND_HOUSEHOLD = 'Second Home';

const BLANK_NAME_PROMPT = 'Enter a name for your household.';

/*
 * The page-side shape of the invoke recorder. A module-scope *type* is erased,
 * so the function passed to `page.evaluate` below can name it while running in
 * WebKit (the same technique as `fixtures.ts`'s `TauriInternals`).
 */
type RecorderWindow = {
  __TAURI_INTERNALS__: {
    invoke: (command: string, args?: unknown, options?: unknown) => unknown;
  };
  __E2E_COMMANDS__?: string[];
};

/** The screen's `h1` — `NavigationBar`'s title on every screen. */
function screenTitle(page: Page, name: string) {
  return page.getByRole('heading', { level: 1, name });
}

/** The first-run form's one field, in either creation entry point. */
function householdNameField(page: Page) {
  return page.getByRole('textbox', { name: /Household name/ });
}

/** Opens the entry route; the guard decides from the mocked book. */
async function openEntry(page: Page): Promise<void> {
  await page.goto('/#/');
}

/*
 * Installs the command recorder on this document's mocked `invoke`.
 *
 * It records the command name only — arguments carry household names and
 * request ids, and this is a call log, not a payload log. It runs after the
 * shared fixture's `invoke is a function` assertion, and it survives the
 * `page.goto` that follows because a hash navigation is the same document.
 */
async function recordCommands(page: Page): Promise<void> {
  await page.evaluate(() => {
    const scope = window as unknown as RecorderWindow;
    const internals = scope.__TAURI_INTERNALS__;
    const dispatch = internals.invoke;
    const commands: string[] = [];

    scope.__E2E_COMMANDS__ = commands;
    internals.invoke = (command, args, options) => {
      commands.push(command);

      return dispatch(command, args, options);
    };
  });
}

/** Every command name dispatched through the mock in this document so far. */
async function invokedCommands(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const scope = window as unknown as { __E2E_COMMANDS__?: string[] };

    return scope.__E2E_COMMANDS__ ?? [];
  });
}

/** Launches with no household, creates one, and lands on its dashboard. */
async function createFirstHousehold(page: Page, name: string): Promise<void> {
  const onboarding = page.locator('[data-household-entry="onboarding"]');
  const dashboard = page.locator('[data-household-entry="household"]');
  const heading = screenTitle(page, name);

  await openEntry(page);
  await expect(onboarding).toBeVisible();

  await householdNameField(page).fill(name);
  await page.getByRole('button', { name: 'Create household' }).click();

  await expect(dashboard).toBeVisible();
  await expect(heading).toBeVisible();
}

/** SET-1's gear, from the dashboard it sits on. */
async function openSettingsFromDashboard(page: Page): Promise<void> {
  const gear = page.getByRole('button', { name: 'Open Settings & Profile' });

  await gear.click();

  await expect(screenTitle(page, 'Settings & Profile')).toBeVisible();
}

/** HS-4's inline creation: a second household without first-run onboarding. */
async function createHouseholdFromSettings(
  page: Page,
  name: string,
): Promise<void> {
  const createAnother = page.getByRole('button', {
    name: 'Create another household',
  });
  const create = page.getByRole('button', { name: 'Create household' });
  const created = page.getByText(name, { exact: true });

  await createAnother.click();

  await householdNameField(page).fill(name);
  await create.click();

  await expect(created).toBeVisible();
}

test.describe('first-run household flow at 390×844', () => {
  test.skip(
    ({ viewport }) => viewport?.width !== 390,
    'task 5.14 is the compact baseline; task 5.17 owns desktop',
  );

  test('Onboarding shown when no household exists', async ({ page }) => {
    const onboarding = page.locator('[data-household-entry="onboarding"]');
    const heading = screenTitle(page, 'Create your household');

    await openEntry(page);

    await expect(onboarding).toBeVisible();
    await expect(heading).toBeVisible();

    expect(new URL(page.url()).hash).toBe('#/onboarding');
  });

  test('Blank name is rejected without calling create_household', async ({
    page,
  }) => {
    const onboarding = page.locator('[data-household-entry="onboarding"]');
    const field = householdNameField(page);
    const submit = page.getByRole('button', { name: 'Create household' });

    await recordCommands(page);
    await openEntry(page);

    await expect(onboarding).toBeVisible();

    // An empty name, submitted through the form's own control.
    await submit.click();
    await expect(page.getByRole('alert')).toContainText(BLANK_NAME_PROMPT);

    // A name of only whitespace is blank too (HS-1's "treated as blank").
    await field.fill('   ');
    await submit.click();
    await expect(page.getByRole('alert')).toContainText(BLANK_NAME_PROMPT);

    /*
     * The recorder is proven live first: the guard's own reads went through the
     * same wrapped dispatch, so "no create" below is a measurement and not an
     * empty log. Then HS-1's refusal: neither submission reached the backend.
     */
    expect(await invokedCommands(page)).toContain('list_households');
    expect(await invokedCommands(page)).not.toContain('create_household');

    // The refusal is inline on the screen that made it, not a navigation away.
    await expect(onboarding).toBeVisible();

    expect(new URL(page.url()).hash).toBe('#/onboarding');
  });

  test('Create the first household', async ({ page }) => {
    const onboarding = page.locator('[data-household-entry="onboarding"]');

    await createFirstHousehold(page, FIRST_HOUSEHOLD);

    // HS-1: it is the active household, so its dashboard is what is presented.
    // HS-2: the onboarding flow is not presented again.
    await expect(onboarding).toHaveCount(0);

    expect(new URL(page.url()).hash).toBe('#/');
  });

  test('Gear control opens Settings as its own screen [mobile]', async ({
    page,
  }) => {
    const dashboard = page.locator('[data-household-entry="household"]');
    const back = page.getByRole('button', { name: 'Back' });

    await createFirstHousehold(page, FIRST_HOUSEHOLD);

    // SET-1: a gear-shaped control is present on the dashboard.
    const gear = page.getByRole('button', { name: 'Open Settings & Profile' });

    await expect(gear).toBeVisible();
    await gear.click();

    await expect(screenTitle(page, 'Settings & Profile')).toBeVisible();

    expect(new URL(page.url()).hash).toBe('#/settings');

    // SET-1 [mobile]: its own screen, replacing the dashboard…
    await expect(dashboard).toHaveCount(0);

    // …and with a control that returns to it.
    await expect(back).toBeVisible();
    await back.click();

    await expect(dashboard).toBeVisible();
    await expect(screenTitle(page, FIRST_HOUSEHOLD)).toBeVisible();

    expect(new URL(page.url()).hash).toBe('#/');
  });

  test('All households are listed with the active one indicated', async ({
    page,
  }) => {
    const onboarding = page.locator('[data-household-entry="onboarding"]');
    const first = page.getByText(FIRST_HOUSEHOLD, { exact: true });
    const second = page.getByText(SECOND_HOUSEHOLD, { exact: true });
    const activeBadge = page.getByText('Active', { exact: true });
    const activeRow = page.locator('li[aria-current="true"]');
    const switchToFirst = page.getByRole('button', {
      name: new RegExp(FIRST_HOUSEHOLD),
    });

    await createFirstHousehold(page, FIRST_HOUSEHOLD);
    await openSettingsFromDashboard(page);
    await createHouseholdFromSettings(page, SECOND_HOUSEHOLD);

    // HS-4: creating another household here does not re-run first-run
    // onboarding; the person stays on Settings.
    await expect(onboarding).toHaveCount(0);

    expect(new URL(page.url()).hash).toBe('#/settings');

    // HS-4: both households are listed.
    await expect(first).toBeVisible();
    await expect(second).toBeVisible();

    // HS-4/DM-3: exactly one is indicated as active, and it is the one the
    // creation made active — the other row offers the switch instead.
    await expect(activeBadge).toHaveCount(1);
    await expect(activeRow).toHaveCount(1);
    await expect(activeRow).toContainText(SECOND_HOUSEHOLD);
    await expect(switchToFirst).toContainText('Make active');
  });

  test('Switching the active household changes the presented data', async ({
    page,
  }) => {
    const activeBadge = page.getByText('Active', { exact: true });
    const activeRow = page.locator('li[aria-current="true"]');
    const switchToFirst = page.getByRole('button', {
      name: new RegExp(FIRST_HOUSEHOLD),
    });
    const switchToSecond = page.getByRole('button', {
      name: new RegExp(SECOND_HOUSEHOLD),
    });
    const back = page.getByRole('button', { name: 'Back' });

    await createFirstHousehold(page, FIRST_HOUSEHOLD);
    await openSettingsFromDashboard(page);
    await createHouseholdFromSettings(page, SECOND_HOUSEHOLD);

    // Select the household that is not active, through its row's own control.
    await switchToFirst.click();

    // The indicator follows the confirmed switch, and only one row has it
    // (HS-3, DM-3).
    await expect(activeRow).toHaveCount(1);
    await expect(activeRow).toContainText(FIRST_HOUSEHOLD);
    await expect(activeBadge).toHaveCount(1);
    await expect(switchToSecond).toContainText('Make active');

    // The dashboard presents the selected household's data, not the previous
    // household's (HS-3).
    await back.click();

    await expect(screenTitle(page, FIRST_HOUSEHOLD)).toBeVisible();
    await expect(screenTitle(page, SECOND_HOUSEHOLD)).toHaveCount(0);
  });
});
