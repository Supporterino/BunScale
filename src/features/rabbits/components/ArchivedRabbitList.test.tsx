/*
 * The archived rabbit list at the component layer (task 4.5; RAB-6, RAB-7,
 * RAB-10, NFR-9).
 *
 * What this layer decides: the list is the household's *archived* list, each
 * row offers restore and permanent deletion and a way into the rabbit's
 * dashboard (RAB-6), restoration returns the rabbit to the active list,
 * deletion never happens before its confirmation (and the confirmation states
 * the consequence), and the empty state's copy is written into a polite region
 * that already existed when the last row leaves.
 *
 * The active list is rendered beside the archived one as a probe
 * (`data-testid="active-list"`): RAB-7 must show the restored rabbit *in the
 * active rabbit list*, not only that the archived list lost it. The probe reads
 * the same invalidated dashboard read the product's dashboard does.
 *
 * IPC is mocked at `invoke`, the seam the rabbit specs use, so the real
 * `useArchivedRabbits`, `useRestoreRabbit`, `useDeleteRabbit`,
 * `requestOperation` and `unwrapCommandResult` run: the assertions are about the
 * commands that left the UI and the rows a confirming re-read paints, not about
 * a prop recorder. The backend's cascade is its own test's (2.7); this spec
 * asserts that the UI only calls it behind the dialog.
 *
 * The dialog itself is exercised here rather than in a second spec: Escape,
 * the Tab trap, focus return, the pending refusal to close and both failure
 * causes are behaviours of this screen's confirmation, reached the same way a
 * person reaches them.
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import {
  cleanup,
  fireEvent,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import {
  describeAtBothBaselines,
  renderWithQueryClient,
} from '../../../test/setup';
import { RabbitList } from '../../dashboard/components/RabbitList';
import type { DashboardRow } from '../../dashboard/queries';
import type { Household } from '../../households/queries';
import { t } from '../../i18n';
import { getSaveRetryState, resetSaveRetry } from '../../sync/save-retry';
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

/** A second archived rabbit, so a removal has a row to hand focus to. */
const HOLLY: Rabbit = {
  ...WILLOW,
  id: 'rabbit-holly',
  name: 'Holly',
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

/** One active rabbit as the dashboard read paints it. */
function dashboardRow(rabbit: Rabbit): DashboardRow {
  return {
    rabbit: { ...summary(rabbit), is_archived: false },
    last_weighed_at: null,
    never_weighed: true,
    has_weight_drop_alert: false,
  };
}

type Call = { command: string; args: Record<string, unknown> };

type DeleteMode = 'ok' | 'pending' | 'validation' | 'connectivity';

/** A failed write as the contract sends it (thrown as a value, not an Error). */
function failure(cause: 'validation' | 'connectivity', requestId: unknown) {
  return {
    cause,
    retryable: cause !== 'validation',
    message_key:
      cause === 'validation' ? 'error.validation' : 'error.connectivity',
    request_id: typeof requestId === 'string' ? requestId : null,
  };
}

let calls: Call[];
let archivedRows: RabbitSummary[];
let activeRows: RabbitSummary[];
let deleteMode: DeleteMode;

beforeEach(() => {
  calls = [];
  archivedRows = [summary(WILLOW)];
  activeRows = [];
  deleteMode = 'ok';
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

    if (command === 'get_dashboard') {
      return activeRows.map((row) =>
        dashboardRow({ ...WILLOW, ...row, is_archived: false }),
      );
    }

    // RAB-7: the backend returns the rabbit with its history unchanged; the
    // mock's job is only the list membership the invalidated reads will see.
    if (command === 'restore_rabbit') {
      const id = String(payload.rabbitId);

      archivedRows = archivedRows.filter((row) => row.id !== id);
      activeRows = [
        ...activeRows,
        { ...summary(WILLOW), id, is_archived: false },
      ];

      return { ...WILLOW, id, is_archived: false };
    }

    // RAB-10: the contract's `()` arrives on the wire as `null`.
    if (command === 'delete_rabbit') {
      if (deleteMode === 'pending') {
        return new Promise<null>(() => {});
      }

      if (deleteMode === 'validation') {
        throw failure('validation', payload.requestId);
      }

      if (deleteMode === 'connectivity') {
        throw failure('connectivity', payload.requestId);
      }

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

/*
 * The archived list is rendered directly, with a spy for the row's open
 * destination (4.17; RAB-6). The route itself is the screen's, so the component
 * contract under test is that the row offers that destination and reports the
 * rabbit it belongs to.
 */
function renderList() {
  const onOpen = vi.fn();

  const view = renderWithQueryClient(
    <>
      <ArchivedRabbitList onClose={() => {}} onOpen={onOpen} />
      {/*
       * RAB-7's active half: the dashboard read the restore invalidates,
       * painted by the product's own `RabbitList`.
       */}
      <div data-testid="active-list">
        <RabbitList />
      </div>
    </>,
  );

  return { ...view, onOpen };
}

function deleteTrigger(): HTMLElement {
  return screen.getByRole('button', { name: t('rabbit.delete') });
}

function confirmDialog(): HTMLElement {
  return screen.getByRole('alertdialog');
}

function confirmControl(): HTMLElement {
  return within(confirmDialog()).getByRole('button', {
    name: t('rabbit.deleteConfirm.confirm'),
  });
}

function cancelControl(): HTMLElement {
  return within(confirmDialog()).getByRole('button', {
    name: t('rabbit.deleteConfirm.cancel'),
  });
}

/** The always-present polite region, with its announced text. */
function announcement(): string {
  return (
    document.querySelector('[data-announcement="archived"]')?.textContent ?? ''
  );
}

/** Opens the dialog from a trigger that actually has focus, as a click does. */
function openDialog(trigger: HTMLElement = deleteTrigger()) {
  trigger.focus();
  fireEvent.click(trigger);

  return confirmDialog();
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

    // The live region is visually hidden on itself, so an empty one is not a
    // flex item and adds no gap above Back (finding 1).
    expect(
      document.querySelector('[data-announcement="archived"]'),
    ).toHaveClass('bs-visually-hidden');

    // The archived read is the archived list only (D1.6).
    expect(calls.find((call) => call.command === 'list_rabbits')?.args).toEqual(
      { householdId: HOUSEHOLD.id, includeArchived: true },
    );
  });

  it('opens the archived rabbit dashboard from its row', async () => {
    const { onOpen } = renderList();

    await screen.findByText(WILLOW.name);

    // RAB-6: the archived rabbit's history is retrievable from the list, not
    // only behind a deep link. The row reports the rabbit it belongs to, and
    // the dashboard screen owns the route it opens (4.16, 4.17).
    const open = screen.getByRole('button', {
      name: t('rabbit.archived.open'),
    });

    expect(open).toBeVisible();

    fireEvent.click(open);

    expect(onOpen).toHaveBeenCalledWith(WILLOW.id);
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

    // RAB-7: it is gone from the archived list…
    await waitFor(() => {
      expect(
        screen.getByRole('heading', { name: t('rabbit.archivedList.title') }),
      ).toBeVisible();
    });
    expect(screen.queryByText(t('rabbit.restore'))).toBeNull();

    // …and the invalidated active-list read paints it again.
    expect(
      await within(screen.getByTestId('active-list')).findByText(WILLOW.name),
    ).toBeVisible();

    // RAB-7, announcement: the empty state replaced rows, and its copy went
    // into the region that already existed.
    await waitFor(() => {
      expect(announcement()).toContain(t('rabbit.archivedList.empty'));
    });

    // Finding 3: the restore control left with its row; focus lands on Back.
    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: t('settings.back') }),
      ).toHaveFocus();
    });
  });

  it('asks for the consequence before deleting anything, and cancelling deletes nothing', async () => {
    renderList();

    await screen.findByText(WILLOW.name);

    openDialog();

    // RAB-10, NFR-9: the confirmation states the consequence, and it is the
    // dialog's accessible name and description rather than a visual subtitle.
    expect(confirmDialog()).toHaveAccessibleName(
      t('rabbit.deleteConfirm.title'),
    );
    expect(confirmDialog()).toHaveAccessibleDescription(
      new RegExp(t('rabbit.deleteConfirm.body')),
    );
    expect(screen.getByText(t('rabbit.deleteConfirm.title'))).toBeVisible();
    expect(screen.getByText(t('rabbit.deleteConfirm.body'))).toBeVisible();

    // Until the confirmation is given, nothing is deleted.
    expect(
      calls.filter((call) => call.command === 'delete_rabbit'),
    ).toHaveLength(0);

    fireEvent.click(cancelControl());

    expect(screen.queryByRole('alertdialog')).toBeNull();
    expect(
      calls.filter((call) => call.command === 'delete_rabbit'),
    ).toHaveLength(0);
    expect(screen.getByText(WILLOW.name)).toBeVisible();
  });

  it('permanently deletes the rabbit only from the confirmation, then announces the empty list', async () => {
    renderList();

    await screen.findByText(WILLOW.name);

    openDialog();
    fireEvent.click(confirmControl());

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

    // RAB-10: the row is gone, the dialog closed, and the empty state's copy
    // was written into the pre-existing polite region.
    await waitFor(() => {
      expect(screen.queryByRole('heading', { name: WILLOW.name })).toBeNull();
      expect(announcement()).toContain(t('rabbit.archivedList.empty'));
    });
    expect(screen.queryByRole('alertdialog')).toBeNull();
    expect(screen.queryByText(WILLOW.name)).toBeNull();

    // Finding 3: with no row left, focus lands on Back rather than `<body>`.
    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: t('settings.back') }),
      ).toHaveFocus();
    });
  });

  it('moves focus to the next row after a confirmed deletion', async () => {
    archivedRows = [summary(WILLOW), summary(HOLLY)];

    renderList();

    await screen.findByText(WILLOW.name);

    const triggers = screen.getAllByRole('button', {
      name: t('rabbit.delete'),
    });

    openDialog(triggers[0]);
    fireEvent.click(confirmControl());

    // The first row leaves; the row that took its place takes focus.
    expect(await screen.findByText(HOLLY.name)).toBeVisible();
    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: t('rabbit.restore') }),
      ).toHaveFocus();
    });
  });

  it('closes on Escape and returns focus to the trigger', async () => {
    renderList();

    await screen.findByText(WILLOW.name);

    const trigger = deleteTrigger();

    openDialog(trigger);

    fireEvent.keyDown(document, { key: 'Escape' });

    expect(screen.queryByRole('alertdialog')).toBeNull();
    expect(trigger).toHaveFocus();
    expect(
      calls.filter((call) => call.command === 'delete_rabbit'),
    ).toHaveLength(0);
  });

  it('traps Tab inside the dialog and recovers focus that has escaped it', async () => {
    renderList();

    await screen.findByText(WILLOW.name);

    openDialog();

    expect(cancelControl()).toHaveFocus();

    // Shift+Tab on the first control wraps to the last.
    fireEvent.keyDown(document.activeElement ?? document, {
      key: 'Tab',
      shiftKey: true,
    });
    expect(confirmControl()).toHaveFocus();

    // Tab on the last wraps back to the first.
    fireEvent.keyDown(document.activeElement ?? document, { key: 'Tab' });
    expect(cancelControl()).toHaveFocus();

    /*
     * A click on the scrim can leave focus on `<body>` in WebKit; the
     * document-level handler pulls it back into the dialog instead of letting
     * Tab walk the page behind it (finding 7).
     */
    document.body.focus();
    fireEvent.keyDown(document, { key: 'Tab' });

    expect(cancelControl()).toHaveFocus();
  });

  it('stays open on Escape while the delete is in flight', async () => {
    deleteMode = 'pending';

    renderList();

    await screen.findByText(WILLOW.name);

    openDialog();
    fireEvent.click(confirmControl());

    // The confirm control turns busy; both controls are unavailable.
    expect(
      await screen.findByRole('button', { name: t('rabbit.deleting') }),
    ).toBeVisible();

    fireEvent.keyDown(document, { key: 'Escape' });

    expect(screen.getByRole('alertdialog')).toBeVisible();
  });

  it('keeps a validation refusal in the dialog', async () => {
    deleteMode = 'validation';

    renderList();

    await screen.findByText(WILLOW.name);

    openDialog();
    fireEvent.click(confirmControl());

    expect(
      await within(confirmDialog()).findByText(t('error.validation')),
    ).toBeVisible();

    // The dialog owns this refusal; it does not close and nothing leaves.
    const panel = confirmDialog();

    expect(panel).toBeVisible();
    expect(screen.getByRole('heading', { name: WILLOW.name })).toBeVisible();
    expect(getSaveRetryState().failure).toBeNull();

    /*
     * The panel itself can hold focus (a busy write moves it there). It is an
     * endpoint: Shift+Tab goes to the last control, Tab to the first, rather
     * than stepping behind the scrim (finding 2).
     */
    panel.focus();
    expect(panel).toHaveFocus();

    fireEvent.keyDown(panel, { key: 'Tab', shiftKey: true });
    expect(confirmControl()).toHaveFocus();

    panel.focus();
    fireEvent.keyDown(panel, { key: 'Tab' });
    expect(cancelControl()).toHaveFocus();
  });

  it('closes the dialog for a non-validation failure and hands it to the shell', async () => {
    deleteMode = 'connectivity';

    renderList();

    await screen.findByText(WILLOW.name);

    openDialog();
    fireEvent.click(confirmControl());

    // The write hook records the failed operation for `SaveFailureSurface`;
    // the dialog does not offer a second, differently identified attempt.
    await waitFor(() => {
      expect(screen.queryByRole('alertdialog')).toBeNull();
    });
    expect(getSaveRetryState().failure?.cause).toBe('connectivity');
    expect(screen.getByRole('heading', { name: WILLOW.name })).toBeVisible();
  });

  it('opens on the empty state without announcing it', async () => {
    archivedRows = [];

    renderList();

    expect(
      await screen.findByText(t('rabbit.archivedList.empty')),
    ).toBeVisible();

    // The initial empty state is not a replacement: the region mounted with the
    // screen stays silent.
    expect(announcement()).toBe('');
  });
});
