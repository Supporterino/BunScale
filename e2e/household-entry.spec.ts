/*
 * The household entry guard, driven end to end (task 3.3; HS-2, SYNC-3).
 *
 * `/` decides between onboarding, the dashboard and the connectivity slot, and
 * *when* it may decide is the rule: "no household" counts only with a fresh
 * reachable-and-identified status, never on an empty read alone. The two states
 * come from the mock's defaults plus `stubCommandResult` (src/test/mock-ipc.ts),
 * and `data-household-entry` is the stable name of the decision.
 */
import type { Page } from '@playwright/test';
import { expect, stubCommandResult, test } from './fixtures';

const AT = '2026-10-20T09:00:00Z';

const HOUSEHOLD = {
  id: 'household-a',
  name: 'Household A',
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

const UNREACHABLE = {
  state: 'unreachable',
  account_valid: null,
  cause: 'connectivity',
  observed_at: AT,
};

async function openEntry(page: Page): Promise<void> {
  await page.goto('/#/');
}

test('A device holding no household is sent to onboarding', async ({
  page,
}) => {
  await openEntry(page);

  await expect(
    page.locator('[data-household-entry="onboarding"]'),
  ).toBeVisible();

  expect(new URL(page.url()).hash).toBe('#/onboarding');
});

test('A held household skips onboarding while the service is unreachable', async ({
  page,
}) => {
  await stubCommandResult(page, 'list_households', [HOUSEHOLD_SUMMARY]);
  await stubCommandResult(page, 'get_active_household', HOUSEHOLD);
  await stubCommandResult(page, 'get_service_status', UNREACHABLE);

  await openEntry(page);

  await expect(
    page.locator('[data-household-entry="household"]'),
  ).toBeVisible();
  await expect(page.locator('[data-household-entry="onboarding"]')).toHaveCount(
    0,
  );

  expect(new URL(page.url()).hash).toBe('#/');
});
