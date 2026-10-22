/*
 * SYNC-2's save-failure scenarios and NFR-8, driven end to end (task 5.16).
 *
 * **Which save surface, and why.** P0 has three state-changing commands and
 * only one user-authored save whose failure copy the catalog carries: the
 * household creation form (`HouseholdOnboardingScreen`, task 4.1), reached here
 * through the entry guard's own decision. The switch
 * (`set_active_household`) is a local selection whose only refusal is
 * `validation`, and Settings renders it with screen-specific copy rather than
 * the cause map; the display-unit change is a per-device preference whose
 * recovery is re-selecting, which task 7.3 asserts at the component layer. So
 * the create form is where "the message identifies that specific cause" is a
 * claim about a real command, and Settings' inline creation renders the same
 * `SaveFailureNotice`, so the cause-to-copy table is proven once.
 *
 * **One test per cause the requirement names.** SYNC-2 names connectivity, no
 * account, an exhausted storage allowance and rate limiting; each has its own
 * test asserting the catalog's sentence verbatim
 * (`src/features/i18n/messages.ts`) and the cause's retry policy **as this spec
 * mirrors it** from the backend's table (`CommandError::retryable` in
 * `src-tauri/src/ipc/failure.rs`, and `src/bindings.ts`). That mirror is
 * hand-copied, so this file cannot detect a policy change on its own:
 * `failure.rs`'s `the_retryable_policy_is_the_documented_one` test is what
 * pins it, and a change there should prompt the same change here. An exhausted
 * owner allowance is asserted as its own sentence and specifically *not* as
 * the generic one, which is the requirement's "never as a generic write
 * failure" clause. `unknown` is asserted once as the cause that offers no
 * retry: nothing can be promised about re-issuing it.
 *
 * **Every failure carries `message_key: 'error.unknown'` on purpose.** The
 * backend sends each cause's own key, but the surface must derive its copy from
 * `cause` (`src/features/sync/failure.ts` says so) — so the arrangement is the
 * one that discriminates: a screen that started echoing the backend's key would
 * show the generic sentence in every test here and fail. The component spec for
 * this surface (task 5.13) uses the same device for the same reason.
 *
 * **Why the seam and not `stubCommandResult`.** The fixture's helper hands over
 * a value; a save that fails is a *rejection* — the contract's `CommandError`
 * thrown from a handler, which `mockIPC` turns into a rejected invocation and
 * the generated `typedError` into the error branch (`src/test/mock-ipc.ts`
 * documents the shape). The retry test also needs the answer to change
 * mid-document, which is what `setHandler` exists for.
 *
 * **How the retry is observed, and why it is a real save.** A wrapped
 * `window.__TAURI_INTERNALS__.invoke` records `create_household`'s arguments
 * (the same technique `household-flow.spec.ts` uses for its call log), so both
 * attempts' request ids are read from what the app actually dispatched rather
 * than trusted from the screen. The failing override also removes itself after
 * its first answer (`failFirstCreateSave`), so the retry reaches the mock's own
 * `create_household` and its household book: the household that then appears on
 * the dashboard is one the mock created, not one this spec asserted into
 * existence. Whether the shared service deduplicates on that id is the Rust
 * save path's claim (tasks 5.1–5.10); what is claimed here is that the retry
 * re-sends the id of the attempt that failed.
 *
 * **"Never shown as saved" is three absences, not one.** The route stays
 * `#/onboarding`, `data-household-entry="household"` is absent, and no heading
 * carries the attempted name; the create control is also back to its own label
 * rather than `Creating…`, so what is on screen is a settled failure and not an
 * attempt still running. An assertion that only checked "the notice is visible"
 * would pass on a screen that had also rendered a created household. For the
 * connectivity scenario the same absences are asserted after a navigation,
 * because the entry guard re-reads the household book on each navigation
 * (`staleTime: 0`). What `#/` proves is that the app's own read of the book is
 * empty; an optimistic write would be observable as a premature navigation or
 * a rendered household, which `expectNotSaved` catches. This file makes no
 * claim about what the query cache holds.
 *
 * **Both baselines.** The form and its notice are one compact-first column at
 * either width, so nothing here is adaptive and both Playwright projects run
 * the file: 390×844 is where it is written (AGENTS.md baseline 1, and the app's
 * primary platform) and 1280×800 costs one more page load. Same reasoning as
 * `offline-launch.spec.ts`; no test is skipped for a viewport.
 *
 * **`timeout` (D0.14) is covered below the page and not repeated here.** Its
 * copy and retry offer are asserted at the component layer (5.13) and its
 * cause mapping in Rust (5.10); the one thing a page-level replay could add —
 * one household rather than two — is a save-path claim that only a household
 * count distinguishes, so it stays with 5.10.
 */
import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';

/** The household the person tries to save; it becomes the dashboard's title. */
const ATTEMPTED = 'Maple Cottage';

/** The failure surface's state header (`error.notSaved`) and its action. */
const NOT_SAVED = 'Not saved';
const RETRY = 'Try again';

/** The create form's own control, in both of its labels. */
const CREATE = 'Create household';
const CREATING = 'Creating…';

/*
 * The catalog's copy, one sentence per cause and a generic one for the
 * remainder (`src/features/i18n/messages.ts`). Asserted verbatim because these
 * sentences are what the requirement is about: "the message identifies that
 * specific cause" is judged by what the person is told.
 */
const CONNECTIVITY_COPY =
  'No connection. Check your internet connection and try again.';
const NO_ACCOUNT_COPY =
  'Your account is not available right now. Try again later.';
const ALLOWANCE_COPY = 'This household has reached its storage limit.';
const RATE_LIMITED_COPY =
  'Too many requests right now. Wait a moment and try again.';
const GENERIC_COPY = 'The change could not be completed. Nothing was changed.';

/** A failure as the contract reports it, minus the operation's own identity. */
type CreateFailure = {
  readonly cause: string;
  readonly retryable: boolean;
};

/*
 * The causes SYNC-2 names, each with the retry policy the backend's table
 * assigns it (`src-tauri/src/ipc/failure.rs::policy`: the conditions a later
 * attempt can clear), plus the remainder, which promises nothing and therefore
 * offers no retry. This block is a hand-copied mirror, not a read of the
 * contract; `failure.rs`'s own test is what pins the policy.
 */
const CONNECTIVITY_CAUSE_FAILURE: CreateFailure = {
  cause: 'connectivity',
  retryable: true,
};
const NO_ACCOUNT_CAUSE_FAILURE: CreateFailure = {
  cause: 'no_account',
  retryable: true,
};
const ALLOWANCE_CAUSE_FAILURE: CreateFailure = {
  cause: 'allowance_exceeded',
  retryable: true,
};
const RATE_LIMITED_CAUSE_FAILURE: CreateFailure = {
  cause: 'rate_limited',
  retryable: true,
};
const UNCLASSIFIED_CAUSE_FAILURE: CreateFailure = {
  cause: 'unknown',
  retryable: false,
};

/**
 * A `create_household` payload, as the generated bindings send it: the Rust
 * parameter names, so `requestId` is the operation identity (D0.14).
 */
type CreateHouseholdArgs = { name?: unknown; requestId?: unknown };

/** One attempt this document dispatched, read from its arguments. */
type CreateAttempt = {
  readonly name: string;
  readonly requestId: string;
};

/*
 * The page-side shapes, declared as module-scope *types* so the functions
 * handed to `page.evaluate` can name them while running in WebKit (the same
 * technique as `fixtures.ts`'s `TauriInternals`: types are erased, values are
 * not).
 */
type CreateHandler = (args?: CreateHouseholdArgs) => unknown;

type SeamWindow = {
  __BUNSCALE_E2E_MOCK__?: {
    setHandler: (command: string, handler: CreateHandler) => void;
    reset: (command?: string) => void;
  };
};

type RecorderWindow = {
  __TAURI_INTERNALS__: {
    invoke: (command: string, args?: unknown, options?: unknown) => unknown;
  };
  __E2E_CREATE_ATTEMPTS__?: CreateAttempt[];
};

/** The screen the guard publishes (task 3.3), by its stable name. */
function entryScreen(page: Page, entry: 'household' | 'onboarding') {
  return page.locator(`[data-household-entry="${entry}"]`);
}

/**
 * Make every `create_household` in this document fail with `failure`.
 *
 * The failure is thrown as the contract's `CommandError` value — not an
 * `Error` — because that is what `mockIPC` rejects with and what the generated
 * `typedError` turns into the error branch (`src/test/mock-ipc.ts`). `cause` is
 * the only field the UI reads; `message_key` is deliberately the generic key so
 * a screen that echoed it instead of deriving copy from the cause would fail.
 */
async function failCreateSave(
  page: Page,
  failure: CreateFailure,
): Promise<void> {
  await installCreateFailure(page, failure, false);
}

/**
 * Fail only the first attempt, then let the mock's own handler answer.
 *
 * The override removes itself **before** it throws, so the retry reaches the
 * default `create_household` and its household book. That is how the retry test
 * shows a dashboard the mock produced rather than one a stubbed read invented.
 */
async function failFirstCreateSave(
  page: Page,
  failure: CreateFailure,
): Promise<void> {
  await installCreateFailure(page, failure, true);
}

/** The one page-side installation the two arrangements share. */
async function installCreateFailure(
  page: Page,
  failure: CreateFailure,
  retryLands: boolean,
): Promise<void> {
  await page.evaluate(
    ({ answer, lands }) => {
      const seam = (window as unknown as SeamWindow).__BUNSCALE_E2E_MOCK__;

      if (seam === undefined) {
        throw new Error(
          'VITE_E2E_MOCK=1 is not in effect: the page exposes no __BUNSCALE_E2E_MOCK__ seam (Q23).',
        );
      }

      seam.setHandler('create_household', (args) => {
        const requestId = args?.requestId;

        if (lands) {
          seam.reset('create_household');
        }

        throw {
          cause: answer.cause,
          message_key: 'error.unknown',
          request_id: typeof requestId === 'string' ? requestId : null,
          retryable: answer.retryable,
        };
      });
    },
    { answer: failure, lands: retryLands },
  );
}

/**
 * Record every `create_household` the app dispatches, with its arguments.
 *
 * The app sends the command through the generated bindings, so wrapping the
 * mocked `invoke` is the only place the payloads can be read without trusting
 * the screen. Installed after the shared fixture's "invoke is a function"
 * assertion, so there is always a dispatch to wrap, and it survives the hash
 * navigation that follows because that is the same document.
 */
async function recordCreateAttempts(page: Page): Promise<void> {
  await page.evaluate(() => {
    const scope = window as unknown as RecorderWindow;
    const dispatch = scope.__TAURI_INTERNALS__.invoke;
    const attempts: CreateAttempt[] = [];

    scope.__E2E_CREATE_ATTEMPTS__ = attempts;

    scope.__TAURI_INTERNALS__.invoke = (command, args, options) => {
      if (command === 'create_household') {
        const payload = (args ?? {}) as CreateHouseholdArgs;
        const name = payload.name;
        const requestId = payload.requestId;

        attempts.push({
          name: typeof name === 'string' ? name : '',
          requestId: typeof requestId === 'string' ? requestId : '',
        });
      }

      return dispatch(command, args, options);
    };
  });
}

/** Every `create_household` this document dispatched, in order. */
async function createAttempts(page: Page): Promise<CreateAttempt[]> {
  return page.evaluate(
    () => (window as unknown as RecorderWindow).__E2E_CREATE_ATTEMPTS__ ?? [],
  );
}

/**
 * Open the entry route and let the guard decide.
 *
 * With an empty book and a fresh reachable account the decision is onboarding
 * (HS-2), so reaching it through `#/` is the real first-run path rather than a
 * deep link to the form.
 */
async function openOnboarding(page: Page): Promise<void> {
  const onboarding = entryScreen(page, 'onboarding');

  await page.goto('/#/');
  await expect(onboarding).toBeVisible();

  expect(new URL(page.url()).hash).toBe('#/onboarding');
}

/** One save attempt, through the form's own field and control. */
async function attemptSave(page: Page, name: string): Promise<void> {
  await page.getByRole('textbox', { name: /Household name/ }).fill(name);
  await page.getByRole('button', { name: CREATE }).click();
}

/**
 * The state that must hold after a save that did not complete: the person is
 * still on the form, no household is presented, and the attempt has settled.
 *
 * `name` is checked as a heading rather than as text because the attempted name
 * is in the field by design — a save that failed leaves the person's input for
 * them to correct or retry. What must not exist is a *presented household*: the
 * dashboard's title, which is the only place the app names a saved household.
 */
async function expectNotSaved(page: Page, name: string): Promise<void> {
  const onboarding = entryScreen(page, 'onboarding');
  const dashboard = entryScreen(page, 'household');
  const attemptedHeading = page.getByRole('heading', { level: 1, name });
  const pending = page.getByRole('button', { name: CREATING });
  const create = page.getByRole('button', { name: CREATE });

  await expect(onboarding).toBeVisible();
  await expect(dashboard).toHaveCount(0);
  await expect(attemptedHeading).toHaveCount(0);

  // The control is back to its own label rather than the pending one, so this
  // is a failure that settled, not an attempt still in flight; enabled, so the
  // person can press it again.
  await expect(pending).toHaveCount(0);
  await expect(create).toBeEnabled();

  expect(new URL(page.url()).hash).toBe('#/onboarding');
}

test.describe('SYNC-2: a save that cannot complete names its cause', () => {
  test('Save refused without connectivity', async ({ page }) => {
    const onboarding = entryScreen(page, 'onboarding');
    const dashboard = entryScreen(page, 'household');
    const notice = page.getByRole('alert');
    const notSaved = notice.getByRole('heading', { name: NOT_SAVED });
    const retry = notice.getByRole('button', { name: RETRY });

    /*
     * The service is unreachable, so the save path reports the contract's
     * `connectivity` cause. It is retryable because a later attempt can reach
     * the service (SYNC-2 offers the retry for this cause).
     */
    await failCreateSave(page, CONNECTIVITY_CAUSE_FAILURE);
    await recordCreateAttempts(page);
    await openOnboarding(page);

    await attemptSave(page, ATTEMPTED);

    // SYNC-2: told that connectivity is required, in the catalog's sentence.
    await expect(notice).toBeVisible();
    await expect(notSaved).toBeVisible();
    await expect(notice).toContainText(CONNECTIVITY_COPY);
    await expect(notice).not.toContainText(GENERIC_COPY);
    await expect(retry).toBeVisible();
    await expect(retry).toBeEnabled();

    await expectNotSaved(page, ATTEMPTED);

    // The attempt carried an operation identity, which is what a retry re-sends
    // (D0.14). The id is opaque, so only its presence is read here.
    const attempts = await createAttempts(page);

    expect(attempts).toHaveLength(1);
    expect(attempts[0]?.name).toBe(ATTEMPTED);
    expect(attempts[0]?.requestId).not.toBe('');

    /*
     * SYNC-2: nothing is saved. The guard re-reads the household book on every
     * navigation, so a household the app had written would be observable here
     * as the dashboard (or as this name heading); the book is empty and sends
     * the person back to onboarding instead.
     */
    await page.goto('/#/');

    await expect(onboarding).toBeVisible();
    await expect(dashboard).toHaveCount(0);
    await expect(
      page.getByRole('heading', { level: 1, name: ATTEMPTED }),
    ).toHaveCount(0);
  });

  test('Save failure names its cause and offers a retry', async ({ page }) => {
    const onboarding = entryScreen(page, 'onboarding');
    const dashboard = entryScreen(page, 'household');
    const notice = page.getByRole('alert');
    const notSaved = notice.getByRole('heading', { name: NOT_SAVED });

    /*
     * `no_account`: the service answered that the account is missing or
     * invalid. SYNC-2 names it and the contract marks it retryable, because the
     * condition can clear.
     *
     * The first attempt fails and the mock's own handler then answers, so the
     * retry below creates a household in the mock's book rather than being
     * satisfied by a stubbed read.
     */
    await failFirstCreateSave(page, NO_ACCOUNT_CAUSE_FAILURE);
    await recordCreateAttempts(page);
    await openOnboarding(page);

    await attemptSave(page, ATTEMPTED);

    await expect(notice).toBeVisible();
    await expect(notSaved).toBeVisible();
    await expect(notice).toContainText(NO_ACCOUNT_COPY);
    await expect(notice).not.toContainText(GENERIC_COPY);

    const retry = notice.getByRole('button', { name: RETRY });

    await expect(retry).toBeVisible();

    // Before the retry the entry is not saved, and nothing claims it is.
    await expectNotSaved(page, ATTEMPTED);

    const before = await createAttempts(page);

    expect(before).toHaveLength(1);
    expect(before[0]?.requestId).not.toBe('');

    await retry.click();

    // The retry completed: the guard now finds the household and presents its
    // dashboard, which is the first moment a save is claimed.
    await expect(dashboard).toBeVisible();
    await expect(
      page.getByRole('heading', { level: 1, name: ATTEMPTED }),
    ).toBeVisible();
    await expect(onboarding).toHaveCount(0);

    /*
     * D0.14: the retry re-issued the same operation. Both attempts are read
     * from the dispatched payloads, so this is the id the shared service would
     * deduplicate on — not a value the screen echoed back.
     */
    const attempts = await createAttempts(page);

    expect(attempts).toHaveLength(2);
    expect(attempts[1]?.requestId).toBe(attempts[0]?.requestId);
    expect(attempts[1]?.name).toBe(ATTEMPTED);
  });

  test('An exhausted storage allowance is named as its own cause', async ({
    page,
  }) => {
    const notice = page.getByRole('alert');
    const notSaved = notice.getByRole('heading', { name: NOT_SAVED });
    const retry = notice.getByRole('button', { name: RETRY });

    /*
     * The service reported the household owner's exhausted allowance
     * (`allowance_exceeded`). SYNC-2's clause is that this is named as its own
     * cause and never as a generic write failure, so the assertion is both the
     * presence of its sentence and the absence of the generic one.
     */
    await failCreateSave(page, ALLOWANCE_CAUSE_FAILURE);
    await openOnboarding(page);

    await attemptSave(page, ATTEMPTED);

    await expect(notice).toBeVisible();
    await expect(notSaved).toBeVisible();
    await expect(notice).toContainText(ALLOWANCE_COPY);

    // Its own cause: neither the remainder's sentence nor another cause's may
    // stand in for it.
    await expect(notice).not.toContainText(GENERIC_COPY);
    await expect(notice).not.toContainText(CONNECTIVITY_COPY);
    await expect(notice).not.toContainText(RATE_LIMITED_COPY);

    // Retryable: the owner can free space and the attempt can then succeed.
    await expect(retry).toBeVisible();
    await expect(retry).toBeEnabled();

    await expectNotSaved(page, ATTEMPTED);
  });

  test('Rate limiting is named as its own cause', async ({ page }) => {
    const notice = page.getByRole('alert');
    const notSaved = notice.getByRole('heading', { name: NOT_SAVED });
    const retry = notice.getByRole('button', { name: RETRY });

    /*
     * The gateway answered 429 with `Retry-After`: `rate_limited`, retryable
     * because waiting clears it. Three causes with three different sentences is
     * the requirement — one shared failure line would satisfy none of them, so
     * this test reads its own sentence and rejects the other two.
     */
    await failCreateSave(page, RATE_LIMITED_CAUSE_FAILURE);
    await openOnboarding(page);

    await attemptSave(page, ATTEMPTED);

    await expect(notice).toBeVisible();
    await expect(notSaved).toBeVisible();
    await expect(notice).toContainText(RATE_LIMITED_COPY);
    await expect(notice).not.toContainText(GENERIC_COPY);
    await expect(notice).not.toContainText(ALLOWANCE_COPY);
    await expect(notice).not.toContainText(NO_ACCOUNT_COPY);

    await expect(retry).toBeVisible();
    await expect(retry).toBeEnabled();

    await expectNotSaved(page, ATTEMPTED);
  });

  test('A non-retryable cause offers no retry', async ({ page }) => {
    const notice = page.getByRole('alert');
    const notSaved = notice.getByRole('heading', { name: NOT_SAVED });

    /*
     * `unknown` is the honest remainder for a failure with no classified cause,
     * and the contract marks it non-retryable: nothing can be promised about
     * re-issuing it. Task 7.3 owns the display unit's `unknown` at the
     * component layer; the page-level claim is asserted here once so the E2E
     * layer does not inherit it unstated.
     */
    await failCreateSave(page, UNCLASSIFIED_CAUSE_FAILURE);
    await openOnboarding(page);

    await attemptSave(page, ATTEMPTED);

    await expect(notice).toBeVisible();
    await expect(notSaved).toBeVisible();
    await expect(notice).toContainText(GENERIC_COPY);

    /*
     * No retry, and no dismissal either: the create form passes a retry only
     * for the retryable causes, so this notice is copy alone. The assertion is
     * scoped to the notice on purpose — the form's own "Create household"
     * control stays, because it is the screen's primary action and pressing it
     * again is the person repeating the same intent, not the retry the failure
     * surface offers.
     */
    await expect(notice.getByRole('button')).toHaveCount(0);

    await expectNotSaved(page, ATTEMPTED);
  });
});
