/*
 * The rabbit profile detail at the component layer (task 4.4; RAB-5, HS-8).
 *
 * What this layer owns: every RAB-1 field presented as a value, the
 * weight-drop threshold among them, the edit control being directly visible
 * without a gesture, the edit form being pre-filled with the whole saved
 * profile (threshold included) and the saved edit sending that whole profile to
 * `update_rabbit` before the detail returns with the confirmed values.
 *
 * It also owns the read-outcome gate (D1.16): `update_rabbit` resolves the
 * rabbit from this device's copy, so no edit control is offered while the
 * profile read has no data. A failed read is presented through
 * `describeReadFailure`, which keeps its three outcomes distinct:
 * D1.16's content-unloaded refusal (`unknown` carrying
 * `error.household_content_unloaded`) renders that surface key and offers a
 * refresh, because a fresh read can fill the content; a classified `validation`
 * (an unknown or blank id) renders `error.validation` and offers no refresh,
 * because a fresh read cannot correct it; and a value that was not a contract
 * failure renders `error.unknown` and offers a refresh. The still-loading read
 * shows the pending label. None of them offers an edit action.
 *
 * IPC is mocked at `invoke`, the seam the other rabbit specs use, so the real
 * `useRabbitProfile`, `useUpdateRabbit`, `requestOperation` and
 * `unwrapCommandResult` run: the assertions are about the command that left the
 * form and the values the confirming read painted, not about a prop recorder.
 * Sharing with every member is the shared service's obligation; this spec
 * asserts only what the UI does with the confirming read.
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import {
  act,
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
import { t } from '../../i18n';
import { getSaveRetryState, resetSaveRetry } from '../../sync/save-retry';
import { rabbitKeys } from '../keys';
import type { Rabbit } from '../queries';
import { RabbitDetail } from './RabbitDetail';

const HOUSEHOLD_ID = 'household-home';

/** The saved profile the read answers: every RAB-1 field is present. */
const SAVED: Rabbit = {
  id: 'rabbit-hazel',
  household_id: HOUSEHOLD_ID,
  name: 'Hazel',
  photo_ref: 'https://photos.example/hazel.jpg',
  breed: 'Holland Lop',
  dob: '2026-03-01T00:00:00.000Z',
  dob_unknown: false,
  sex: 'female',
  neuter_status: 'neutered',
  adoption_date: '2025-05-01T00:00:00.000Z',
  is_archived: false,
  weight_drop_threshold_percent: 8,
};

/** What the confirming read answers after the edit below is saved. */
const EDITED: Rabbit = {
  ...SAVED,
  name: 'Clover',
  breed: 'Mini Lop',
  dob: '2026-04-02T00:00:00.000Z',
  sex: 'male',
  neuter_status: 'spayed',
  adoption_date: '2025-06-03T00:00:00.000Z',
  weight_drop_threshold_percent: 12,
};

type Call = { command: string; args: Record<string, unknown> };

let calls: Call[];
let reads: Rabbit[];
let archived: Rabbit;
let updated: Rabbit;

/**
 * D1.16's content-unloaded refusal: a frozen `unknown` cause carrying the
 * surface key. This is the real "rabbit reported but not loaded here" pair.
 */
function contentUnloadedError() {
  return {
    cause: 'unknown',
    retryable: false,
    message_key: 'error.household_content_unloaded',
    request_id: null,
  };
}

/** `get_rabbit`'s own refusal: an unknown or blank rabbit id (`api.ts`). */
function validationError() {
  return {
    cause: 'validation',
    retryable: false,
    message_key: 'error.validation',
    request_id: null,
  };
}

/** A refusal the shell's retry surface owns, not the dialog. */
function connectivityError() {
  return {
    cause: 'connectivity',
    retryable: true,
    message_key: 'error.connectivity',
    request_id: null,
  };
}

beforeEach(() => {
  calls = [];
  reads = [];
  updated = EDITED;
  archived = { ...SAVED, is_archived: true };
  resetSaveRetry();

  mockIPC((command, args) => {
    calls.push({ command, args: (args ?? {}) as Record<string, unknown> });

    if (command === 'get_rabbit') {
      const answer = reads.shift();

      if (answer === undefined) {
        throw new Error('get_rabbit answered without a queued profile');
      }

      return answer;
    }

    if (command === 'update_rabbit') {
      return updated;
    }

    if (command === 'archive_rabbit') {
      return archived;
    }

    // RAB-10: the contract's `()` arrives on the wire as `null`.
    if (command === 'delete_rabbit') {
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

function renderDetail() {
  const onClose = vi.fn();

  const view = renderWithQueryClient(
    <RabbitDetail onClose={onClose} rabbitId={SAVED.id} />,
  );

  return { ...view, onClose };
}

function editControl(): HTMLElement | null {
  return screen.queryByRole('button', { name: t('rabbit.edit') });
}

function saveControl(): HTMLElement {
  return screen.getByRole('button', { name: t('rabbit.form.save') });
}

describeAtBothBaselines('Rabbit profile detail (RAB-5)', () => {
  it('shows every profile field and a directly visible edit control', async () => {
    reads.push(SAVED);

    const { container } = renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    // RAB-5: the control is on the surface, not behind a gesture.
    const edit = editControl();

    expect(edit).toBeVisible();
    expect(edit).toBeEnabled();

    // RAB-1's values.
    expect(screen.getByText(SAVED.breed ?? '')).toBeVisible();
    expect(screen.getByText(t('rabbit.sex.female'))).toBeVisible();
    expect(screen.getByText(t('rabbit.neuterStatus.neutered'))).toBeVisible();
    expect(
      container.querySelector(`time[datetime="${SAVED.dob}"]`),
    ).not.toBeNull();
    expect(
      container.querySelector(`time[datetime="${SAVED.adoption_date}"]`),
    ).not.toBeNull();

    // WT-7's threshold is part of the profile, shown as a value here.
    expect(screen.getByText('8%')).toBeVisible();
    expect(screen.getByText(t('rabbit.form.thresholdLabel'))).toBeVisible();
  });

  it('pre-fills every field, saves them all including the threshold, and returns with the confirmed values', async () => {
    reads.push(SAVED, EDITED);

    renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: t('rabbit.edit') }));

    // Every field starts from the saved profile, including the threshold.
    expect(screen.getByRole('textbox', { name: /^Name/ })).toHaveValue(
      SAVED.name,
    );
    expect(
      screen.getByRole('textbox', { name: t('rabbit.form.breedLabel') }),
    ).toHaveValue(SAVED.breed);
    expect(
      screen.getByLabelText(t('rabbit.form.dobLabel'), { exact: true }),
    ).toHaveValue('2026-03-01');
    expect(
      screen.getByLabelText(t('rabbit.form.adoptionDateLabel')),
    ).toHaveValue('2025-05-01');
    // The field is required in edit mode, so its accessible name also says
    // "(required)"; the label prefix is the stable part.
    expect(
      screen.getByRole('spinbutton', {
        name: new RegExp(t('rabbit.form.thresholdLabel')),
      }),
    ).toHaveValue(8);
    expect(
      within(
        screen.getByRole('radiogroup', { name: t('rabbit.sex.label') }),
      ).getByRole('radio', { name: t('rabbit.sex.female') }),
    ).toBeChecked();
    expect(
      within(
        screen.getByRole('radiogroup', {
          name: t('rabbit.neuterStatus.label'),
        }),
      ).getByRole('radio', { name: t('rabbit.neuterStatus.neutered') }),
    ).toBeChecked();

    // Change every field.
    fireEvent.change(screen.getByRole('textbox', { name: /^Name/ }), {
      target: { value: EDITED.name },
    });
    fireEvent.change(
      screen.getByRole('textbox', { name: t('rabbit.form.breedLabel') }),
      { target: { value: EDITED.breed } },
    );
    fireEvent.change(
      screen.getByLabelText(t('rabbit.form.dobLabel'), { exact: true }),
      { target: { value: '2026-04-02' } },
    );
    fireEvent.change(
      screen.getByLabelText(t('rabbit.form.adoptionDateLabel')),
      { target: { value: '2025-06-03' } },
    );
    fireEvent.change(
      screen.getByRole('spinbutton', {
        name: new RegExp(t('rabbit.form.thresholdLabel')),
      }),
      { target: { value: '12' } },
    );
    fireEvent.click(
      within(
        screen.getByRole('radiogroup', { name: t('rabbit.sex.label') }),
      ).getByRole('radio', { name: t('rabbit.sex.male') }),
    );
    fireEvent.click(
      within(
        screen.getByRole('radiogroup', {
          name: t('rabbit.neuterStatus.label'),
        }),
      ).getByRole('radio', { name: t('rabbit.neuterStatus.spayed') }),
    );

    fireEvent.click(saveControl());

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'update_rabbit'),
      ).toHaveLength(1);
    });

    expect(
      calls.find((call) => call.command === 'update_rabbit')?.args,
    ).toEqual({
      rabbitId: SAVED.id,
      profile: {
        name: EDITED.name,
        photo_ref: EDITED.photo_ref,
        breed: EDITED.breed,
        dob: EDITED.dob,
        dob_unknown: false,
        sex: EDITED.sex,
        neuter_status: EDITED.neuter_status,
        adoption_date: EDITED.adoption_date,
        weight_drop_threshold_percent: EDITED.weight_drop_threshold_percent,
      },
      requestId: expect.any(String),
    });

    // The invalidated profile read answers the confirmed values, and the detail
    // returns to its read view with them (the edit control is back).
    expect(
      await screen.findByRole('heading', { name: EDITED.name }),
    ).toBeVisible();
    expect(screen.getByText('12%')).toBeVisible();
    expect(editControl()).toBeVisible();

    // The edit form is gone; the read view replaced it.
    expect(
      screen.queryByRole('button', { name: t('rabbit.form.save') }),
    ).toBeNull();
  });

  it('closes the edit form and keeps focus in the detail when the profile becomes archived mid-edit', async () => {
    reads.push(SAVED);

    const { client } = renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: t('rabbit.edit') }));

    expect(saveControl()).toBeVisible();

    /*
     * Another member archives the rabbit: the detail already observes this
     * profile key, so the confirming read is the invalidation a write performs.
     */
    reads.push({ ...SAVED, is_archived: true });

    await act(async () => {
      await client.invalidateQueries({
        queryKey: rabbitKeys.detail(SAVED.id),
      });
    });

    // RAB-5, RAB-8, D1.5: no Save is left behind on a profile that can no
    // longer be written, and there is no edit control to return to either.
    await waitFor(() => {
      expect(
        screen.queryByRole('button', { name: t('rabbit.form.save') }),
      ).toBeNull();
    });
    expect(editControl()).toBeNull();
    expect(screen.getByText(t('rabbit.archived.readOnly'))).toBeVisible();

    // Focus falls back to the detail's first control rather than to `<body>`.
    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: t('settings.back') }),
      ).toHaveFocus();
    });
  });

  it('shows the content-unloaded reason and no edit control when the rabbit is not loaded here', async () => {
    mockIPC((command, args) => {
      calls.push({ command, args: (args ?? {}) as Record<string, unknown> });

      if (command !== 'get_rabbit') {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      throw contentUnloadedError();
    });

    renderDetail();

    // D1.16: the surface key the backend named is what is rendered, and a fresh
    // read can fill the content, so the read action is offered.
    expect(
      await screen.findByText(t('error.household_content_unloaded')),
    ).toBeVisible();
    expect(editControl()).toBeNull();
    expect(
      screen.getByRole('button', { name: t('error.retry') }),
    ).toBeVisible();
  });

  it('shows a validation refusal as its own copy and offers no edit or refresh', async () => {
    mockIPC((command, args) => {
      calls.push({ command, args: (args ?? {}) as Record<string, unknown> });

      if (command !== 'get_rabbit') {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      throw validationError();
    });

    renderDetail();

    // `get_rabbit`'s `validation` is an unknown/blank id, not D1.16's unloaded
    // content: it must not be relabelled, and a fresh read cannot correct it.
    expect(await screen.findByText(t('error.validation'))).toBeVisible();
    expect(
      screen.queryByText(t('error.household_content_unloaded')),
    ).toBeNull();
    expect(editControl()).toBeNull();
    expect(screen.queryByRole('button', { name: t('error.retry') })).toBeNull();
  });

  it('offers no edit control while the profile read is still loading', async () => {
    let resolveRead: (rabbit: Rabbit) => void = () => {};
    const pending = new Promise<Rabbit>((resolve) => {
      resolveRead = resolve;
    });

    mockIPC((command, args) => {
      calls.push({ command, args: (args ?? {}) as Record<string, unknown> });

      if (command !== 'get_rabbit') {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return pending;
    });

    renderDetail();

    expect(editControl()).toBeNull();
    expect(screen.getByText(t('entry.loading'))).toBeVisible();

    await act(async () => {
      resolveRead(SAVED);
    });

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();
    expect(editControl()).toBeVisible();
  });

  it('presents a supplied photo and falls back to the placeholder when it cannot load', async () => {
    reads.push({ ...SAVED, photo_ref: 'https://photos.example/hazel.jpg' });

    const { container } = renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    // RAB-1: the photo is presented wherever the rabbit is, not replaced by the
    // placeholder just because P1 has no picker (D1.11).
    const image = container.querySelector<HTMLImageElement>(
      '.bs-rabbit-detail__image',
    );

    expect(image?.src).toBe('https://photos.example/hazel.jpg');
    expect(
      screen.queryByRole('img', { name: t('rabbit.thumbnailPlaceholder') }),
    ).toBeNull();

    // An opaque reference is not a promise that the image resolves; the
    // placeholder is the fallback, as it is on the dashboard row.
    if (image !== null) {
      fireEvent.error(image);
    }

    expect(
      screen.getByRole('img', { name: t('rabbit.thumbnailPlaceholder') }),
    ).toBeVisible();
    expect(container.querySelector('.bs-rabbit-detail__image')).toBeNull();
  });

  it("presents an archived rabbit read-only and states why in the catalog's words", async () => {
    reads.push({ ...SAVED, is_archived: true });

    renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    // RAB-8, DM-4: the archived marker is the one the archived list already
    // uses, so the state reads the same wherever the rabbit appears.
    expect(screen.getByText(t('rabbit.archived.label'))).toBeVisible();
    // DASH-7: the read-only sentence sits on the profile itself, covering the
    // dashboard sections this detail is mounted above.
    expect(screen.getByText(t('rabbit.archived.readOnly'))).toBeVisible();

    expect(editControl()).toBeNull();
    // RAB-6: an archived rabbit is not archived again.
    expect(
      screen.queryByRole('button', { name: t('rabbit.archive') }),
    ).toBeNull();
    // RAB-10: the irreversible control is on the archived read-only surface,
    // and it only opens its confirmation.
    expect(
      screen.getByRole('button', { name: t('rabbit.delete') }),
    ).toBeVisible();
    expect(screen.queryByRole('alertdialog')).toBeNull();
  });

  it('permanently deletes an archived rabbit only from the confirmation', async () => {
    // The second answer covers the profile re-read the deletion invalidates.
    reads.push(
      { ...SAVED, is_archived: true },
      { ...SAVED, is_archived: true },
    );

    const { onClose } = renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: t('rabbit.delete') }));

    const dialog = screen.getByRole('alertdialog');

    // NFR-9: the shared confirmation states the consequence before anything is
    // removed, and cancelling deletes nothing.
    expect(dialog).toHaveAccessibleName(t('rabbit.deleteConfirm.title'));
    expect(dialog).toHaveAccessibleDescription(
      new RegExp(t('rabbit.deleteConfirm.body')),
    );
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

    fireEvent.click(screen.getByRole('button', { name: t('rabbit.delete') }));
    fireEvent.click(
      within(screen.getByRole('alertdialog')).getByRole('button', {
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
      rabbitId: SAVED.id,
      requestId: expect.any(String),
    });

    // A confirmed deletion removes the rabbit, so the detail closes.
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
    });
    expect(screen.queryByRole('alertdialog')).toBeNull();
  });

  it('keeps the deletion confirmation open with the validation copy when the delete is refused as validation', async () => {
    mockIPC((command, args) => {
      calls.push({ command, args: (args ?? {}) as Record<string, unknown> });

      if (command === 'get_rabbit') {
        const answer = reads.shift();

        if (answer === undefined) {
          throw new Error('get_rabbit answered without a queued profile');
        }

        return answer;
      }

      if (command === 'delete_rabbit') {
        throw validationError();
      }

      throw new Error(`unexpected mocked command "${command}"`);
    });

    reads.push({ ...SAVED, is_archived: true });

    renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: t('rabbit.delete') }));

    const dialog = screen.getByRole('alertdialog');

    fireEvent.click(
      within(dialog).getByRole('button', {
        name: t('rabbit.deleteConfirm.confirm'),
      }),
    );

    // D0.14: a `validation` refusal belongs to the dialog that owns the input,
    // so it stays open with that copy and the shell records no retry for it.
    expect(
      await within(dialog).findByText(t('error.validation')),
    ).toBeVisible();
    expect(screen.getByRole('alertdialog')).toBe(dialog);
    expect(
      calls.filter((call) => call.command === 'delete_rabbit'),
    ).toHaveLength(1);
    expect(getSaveRetryState().failure).toBeNull();
  });

  it('closes the deletion confirmation when the delete fails for a cause the shell can retry', async () => {
    mockIPC((command, args) => {
      calls.push({ command, args: (args ?? {}) as Record<string, unknown> });

      if (command === 'get_rabbit') {
        const answer = reads.shift();

        if (answer === undefined) {
          throw new Error('get_rabbit answered without a queued profile');
        }

        return answer;
      }

      if (command === 'delete_rabbit') {
        throw connectivityError();
      }

      throw new Error(`unexpected mocked command "${command}"`);
    });

    reads.push({ ...SAVED, is_archived: true });

    renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: t('rabbit.delete') }));
    fireEvent.click(
      within(screen.getByRole('alertdialog')).getByRole('button', {
        name: t('rabbit.deleteConfirm.confirm'),
      }),
    );

    // D0.14: the write hook records this refusal for the shell's retry surface,
    // so the dialog closes rather than offering a second, differently
    // identified attempt beside it.
    await waitFor(() => {
      expect(getSaveRetryState().failure?.cause).toBe('connectivity');
    });

    await waitFor(() => {
      expect(screen.queryByRole('alertdialog')).toBeNull();
    });

    expect(getSaveRetryState().failure?.retryable).toBe(true);
    // The validation copy is the dialog's, and this refusal is not that.
    expect(screen.queryByText(t('error.validation'))).toBeNull();
  });

  it('states no archived read-only copy on an active rabbit', async () => {
    reads.push(SAVED);

    renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    // Both describe the archive, not the profile: an active rabbit states
    // neither of them.
    expect(screen.queryByText(t('rabbit.archived.label'))).toBeNull();
    expect(screen.queryByText(t('rabbit.archived.readOnly'))).toBeNull();
  });

  it('archives an active rabbit from a directly visible control without a confirmation', async () => {
    // RAB-6, NFR-9, D1.8: archiving is reversible, so no consequence
    // confirmation stands between the tap and the write.
    reads.push(SAVED, { ...SAVED, is_archived: true });

    const { onClose } = renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    const archive = screen.getByRole('button', { name: t('rabbit.archive') });

    expect(archive).toBeVisible();
    expect(archive).toBeEnabled();

    fireEvent.click(archive);

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'archive_rabbit'),
      ).toHaveLength(1);
    });

    expect(
      calls.find((call) => call.command === 'archive_rabbit')?.args,
    ).toEqual({
      rabbitId: SAVED.id,
      requestId: expect.any(String),
    });

    // Nothing asked for the deletion consequence along the way.
    expect(screen.queryByText(t('rabbit.deleteConfirm.title'))).toBeNull();
    expect(screen.queryByRole('alertdialog')).toBeNull();

    // The confirmed archive returns to the list the rabbit has left.
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  it('offers no permanent-delete control on an active rabbit', async () => {
    reads.push(SAVED);

    renderDetail();

    expect(
      await screen.findByRole('heading', { name: SAVED.name }),
    ).toBeVisible();

    // RAB-10, D1.8: the irreversible control is absent from the active profile.
    expect(
      screen.queryByRole('button', { name: t('rabbit.delete') }),
    ).toBeNull();
    expect(
      screen.queryByRole('button', { name: t('rabbit.deleteConfirm.confirm') }),
    ).toBeNull();
    expect(screen.queryByRole('alertdialog')).toBeNull();
  });
});
