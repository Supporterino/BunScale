/*
 * The archived rabbit list at the component layer (task 4.5; RAB-6, RAB-7,
 * RAB-10, NFR-9).
 *
 * What this layer decides: the list is the household's *archived* list, each
 * row offers restore and permanent deletion, restoration returns the rabbit to
 * the active list, deletion never happens before its confirmation (and the
 * confirmation states the consequence), and the empty state replaces the rows
 * inside a `role="status"` region once the last one leaves.
 *
 * IPC is mocked at `invoke`, the seam the rabbit specs use, so the real
 * `useArchivedRabbits`, `useRestoreRabbit`, `useDeleteRabbit`,
 * `requestOperation` and `unwrapCommandResult` run: the assertions are about the
 * commands that left the UI and the rows a confirming re-read paints, not about
 * a prop recorder. The backend's cascade is its own test's (2.7); this spec
 * asserts that the UI only calls it behind the dialog.
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import {
  cleanup,
  fireEvent,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, it } from 'vitest';
import {
  describeAtBothBaselines,
  renderWithQueryClient,
} from '../../../test/setup';
import type { Household } from '../../households/queries';
import { t } from '../../i18n';
import { resetSaveRetry } from '../../sync/save-retry';
import type { Rabbit, RabbitSummary } from '../queries';
import { ArchivedRabbitList } from './ArchivedRabbitList';

const HOUSEHOLD: Household = {
  id: 'household-home',
  name: 'Home household',
  created_at: '2026-10-01T09:00:00Z',
  owner_member_id: 'member-1',
  is_active: true,
  is_owner: true,
};

/** The archived rabbit's full profile, as `restore_rabbit` answers it. */
const WILLOW: Rabbit = {
  id: 'rabbit-willow',
  household_id: HOUSEHOLD.id,
  name: 'Willow',
  photo_ref: null,
  breed: 'Lionhead',
  dob: null,
  dob_unknown: true,
  sex: 'female',
  neuter_status: 'spayed',
  adoption_date: null,
  is_archived: true,
  weight_drop_threshold_percent: 5,
};

/** The archived list's rows: the summary projection, not the profile. */
function summary(rabbit: Rabbit): RabbitSummary {
  return {
    id: rabbit.id,
    name: rabbit.name,
    thumbnail_ref: rabbit.photo_ref,
    is_archived: rabbit.is_archived,
  };
}

type Call = { command: string; args: Record<string, unknown> };

let calls: Call[];
let archivedRows: RabbitSummary[];
let activeRows: RabbitSummary[];

beforeEach(() => {
  calls = [];
  archivedRows = [summary(WILLOW)];
  activeRows = [];
  resetSaveRetry();

  mockIPC((command, args) => {
    const payload = (args ?? {}) as Record<string, unknown>;

    calls.push({ command, args: payload });

    if (command === 'get_active_household') {
      return HOUSEHOLD;
    }

    if (command === 'list_rabbits') {
      return payload.includeArchived === true
        ? [...archivedRows]
        : [...activeRows];
    }

    // RAB-7: the backend returns the rabbit with its history unchanged; the
    // mock's job is only the list membership the invalidated read will see.
    if (command === 'restore_rabbit') {
      const id = String(payload.rabbitId);

      archivedRows = archivedRows.filter((row) => row.id !== id);
      activeRows = [...activeRows, summary(WILLOW)];

      return { ...WILLOW, is_archived: false };
    }

    // RAB-10: the contract's `()` arrives on the wire as `null`.
    if (command === 'delete_rabbit') {
      const id = String(payload.rabbitId);

      archivedRows = archivedRows.filter((row) => row.id !== id);

      return null;
    }

    throw new Error(`unexpected mocked command "${command}"`);
  });
});

afterEach(async () => {
  cleanup();
  await Promise.resolve();
  clearMocks();
  resetSaveRetry();
});

function renderList() {
  return renderWithQueryClient(<ArchivedRabbitList onClose={() => {}} />);
}

function deleteTrigger(): HTMLElement {
  return screen.getByRole('button', { name: t('rabbit.delete') });
}

function confirmDialog(): HTMLElement {
  return screen.getByRole('alertdialog');
}

describeAtBothBaselines('Archived rabbit list (RAB-6, RAB-7, RAB-10)', () => {
  it('lists the archived rabbits with their state and both lifecycle controls', async () => {
    renderList();

    expect(await screen.findByText(WILLOW.name)).toBeVisible();
    expect(screen.getByText(t('rabbit.archived.label'))).toBeVisible();
    expect(
      screen.getByRole('button', { name: t('rabbit.restore') }),
    ).toBeVisible();
    expect(deleteTrigger()).toBeVisible();

    // The archived read is the archived list only (D1.6).
    expect(calls.find((call) => call.command === 'list_rabbits')?.args).toEqual(
      { householdId: HOUSEHOLD.id, includeArchived: true },
    );
  });

  it('restores an archived rabbit to the active list', async () => {
    renderList();

    await screen.findByText(WILLOW.name);

    fireEvent.click(screen.getByRole('button', { name: t('rabbit.restore') }));

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'restore_rabbit'),
      ).toHaveLength(1);
    });

    expect(
      calls.find((call) => call.command === 'restore_rabbit')?.args,
    ).toEqual({
      rabbitId: WILLOW.id,
      requestId: expect.any(String),
    });

    // RAB-7: the invalidated archived read no longer holds it, and the empty
    // state replaces the row — announced, because it replaced content.
    expect(
      await screen.findByText(t('rabbit.archivedList.empty')),
    ).toBeVisible();
    expect(screen.queryByText(WILLOW.name)).toBeNull();
    expect(
      within(screen.getByRole('status')).getByText(
        t('rabbit.archivedList.empty'),
      ),
    ).toBeVisible();
  });

  it('asks for the consequence before deleting anything, and cancelling deletes nothing', async () => {
    renderList();

    await screen.findByText(WILLOW.name);

    fireEvent.click(deleteTrigger());

    const dialog = confirmDialog();

    // RAB-10, NFR-9: the confirmation states the consequence, and it is the
    // dialog's accessible name and description rather than a visual subtitle.
    expect(dialog).toHaveAccessibleName(t('rabbit.deleteConfirm.title'));
    expect(dialog).toHaveAccessibleDescription(
      new RegExp(t('rabbit.deleteConfirm.body')),
    );
    expect(screen.getByText(t('rabbit.deleteConfirm.title'))).toBeVisible();
    expect(screen.getByText(t('rabbit.deleteConfirm.body'))).toBeVisible();

    // Until the confirmation is given, nothing is deleted.
    expect(
      calls.filter((call) => call.command === 'delete_rabbit'),
    ).toHaveLength(0);

    fireEvent.click(
      within(dialog).getByRole('button', {
        name: t('rabbit.deleteConfirm.cancel'),
      }),
    );

    expect(screen.queryByRole('alertdialog')).toBeNull();
    expect(
      calls.filter((call) => call.command === 'delete_rabbit'),
    ).toHaveLength(0);
    expect(screen.getByText(WILLOW.name)).toBeVisible();
  });

  it('permanently deletes the rabbit only from the confirmation, then announces the empty list', async () => {
    renderList();

    await screen.findByText(WILLOW.name);

    fireEvent.click(deleteTrigger());
    fireEvent.click(
      within(confirmDialog()).getByRole('button', {
        name: t('rabbit.deleteConfirm.confirm'),
      }),
    );

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'delete_rabbit'),
      ).toHaveLength(1);
    });

    expect(
      calls.find((call) => call.command === 'delete_rabbit')?.args,
    ).toEqual({
      rabbitId: WILLOW.id,
      requestId: expect.any(String),
    });

    // RAB-10: the row is gone, the dialog closed, and the replacement empty
    // state is inside the polite status region.
    expect(
      await screen.findByText(t('rabbit.archivedList.empty')),
    ).toBeVisible();
    expect(screen.queryByRole('alertdialog')).toBeNull();
    expect(screen.queryByText(WILLOW.name)).toBeNull();
    expect(
      within(screen.getByRole('status')).getByText(
        t('rabbit.archivedList.empty'),
      ),
    ).toBeVisible();
  });

  it('opens on the empty state without announcing it', async () => {
    archivedRows = [];

    renderList();

    expect(
      await screen.findByText(t('rabbit.archivedList.empty')),
    ).toBeVisible();

    // The initial empty state is not a replacement, so it carries no live role
    // (`EmptyState`'s contract).
    expect(screen.queryByRole('status')).toBeNull();
  });
});
