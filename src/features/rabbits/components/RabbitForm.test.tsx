/*
 * The rabbit-creation form at the component layer (task 4.3; RAB-1, RAB-2).
 *
 * What this layer owns: the field set and the two fixed value sets (D1.14), the
 * "date of birth unknown" state, RAB-2's empty and whitespace-only refusal
 * before any command, and the shape of the profile `create_rabbit` receives.
 * It also owns the save's two edges: exactly one command while a submit is in
 * flight, and a form that closes when the shell's recorded retry is confirmed.
 * IPC is mocked at `invoke`, the seam the other rabbit specs use, so the real
 * `useCreateRabbit`, `requestOperation` and `unwrapCommandResult` run and the
 * assertions are about what left the form rather than what a prop recorded.
 *
 * Reachability (RAB-4) is the dashboard list's contract and is asserted in
 * `../../dashboard/components/rabbit-list.test.tsx`; this file stops at the
 * form itself.
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
import {
  getSaveRetryState,
  resetSaveRetry,
  retryFailedSave,
} from '../../sync/save-retry';
import type { Rabbit } from '../queries';
import { RabbitForm } from './RabbitForm';

/** The active household the form belongs to (RAB-3). */
const HOUSEHOLD_ID = 'household-home';

/** The command's confirmed answer: the created rabbit. */
const CREATED: Rabbit = {
  id: 'rabbit-hazel',
  household_id: HOUSEHOLD_ID,
  name: 'Hazel',
  photo_ref: null,
  breed: 'Holland Lop',
  dob: null,
  dob_unknown: true,
  sex: 'female',
  neuter_status: 'neutered',
  adoption_date: '2025-05-01T00:00:00.000Z',
  is_archived: false,
  weight_drop_threshold_percent: 5,
};

/** RAB-2's field prompt, from the catalog rather than a literal here. */
const BLANK_NAME_PROMPT = t('rabbit.form.blankName');

type Call = { command: string; args: Record<string, unknown> };

let answers: Array<() => unknown>;
let calls: Call[];

function connectivityError() {
  return {
    cause: 'connectivity',
    retryable: true,
    message_key: 'error.connectivity',
    request_id: null,
  };
}

function validationError() {
  return {
    cause: 'validation',
    retryable: false,
    message_key: 'error.validation',
    request_id: null,
  };
}

beforeEach(() => {
  answers = [];
  calls = [];
  resetSaveRetry();

  mockIPC((command, args) => {
    calls.push({
      command,
      args: (args ?? {}) as Record<string, unknown>,
    });

    if (command !== 'create_rabbit') {
      throw new Error(`unexpected mocked command "${command}"`);
    }

    const answer = answers.shift();

    if (answer === undefined) {
      throw new Error('create_rabbit answered without a queued value');
    }

    return answer();
  });
});

afterEach(async () => {
  cleanup();
  await Promise.resolve();
  clearMocks();
  resetSaveRetry();
});

function renderForm(onCreated = vi.fn()) {
  const onCancel = vi.fn();

  renderWithQueryClient(
    <RabbitForm
      householdId={HOUSEHOLD_ID}
      onCancel={onCancel}
      onCreated={onCreated}
    />,
  );

  return { onCancel, onCreated };
}

function nameField(): HTMLElement {
  return screen.getByRole('textbox', { name: /^Name/ });
}

function submitButton(): HTMLElement {
  return screen.getByRole('button', { name: t('rabbit.form.save') });
}

describeAtBothBaselines('Rabbit creation form (RAB-1, RAB-2)', () => {
  it('offers the full identifying field set and the exact value sets', () => {
    renderForm();

    // RAB-1's fields, each by its visible label.
    expect(nameField()).toBeVisible();
    expect(
      screen.getByRole('textbox', { name: t('rabbit.form.breedLabel') }),
    ).toBeVisible();
    const dobField = screen.getByLabelText(t('rabbit.form.dobLabel'), {
      exact: true,
    });
    const adoptionField = screen.getByLabelText(
      t('rabbit.form.adoptionDateLabel'),
    );

    expect(dobField).toBeVisible();
    // WebKit paints today's date as an empty date input's placeholder; the
    // form's own value stays empty until a person chooses a day.
    expect(dobField).toHaveValue('');
    expect(adoptionField).toBeVisible();
    expect(adoptionField).toHaveValue('');

    // D1.11: the photo field is carried but no picker exists in P1.
    expect(screen.getByText(t('rabbit.thumbnailPlaceholder'))).toBeVisible();

    // D1.14: exactly male / female / unknown.
    const sex = screen.getByRole('radiogroup', {
      name: t('rabbit.sex.label'),
    });
    const sexOptions = within(sex).getAllByRole('radio');

    expect(sexOptions.map((option) => option.getAttribute('value'))).toEqual([
      'male',
      'female',
      'unknown',
    ]);

    // D1.14: exactly intact / neutered / spayed / unknown, with the two
    // fixed states kept distinct.
    const neuter = screen.getByRole('radiogroup', {
      name: t('rabbit.neuterStatus.label'),
    });
    const neuterOptions = within(neuter).getAllByRole('radio');

    expect(neuterOptions.map((option) => option.getAttribute('value'))).toEqual(
      ['intact', 'neutered', 'spayed', 'unknown'],
    );

    expect(
      screen.getByRole('checkbox', { name: t('rabbit.form.dobUnknown') }),
    ).not.toBeChecked();
  });

  it('submits every supplied identifying detail', async () => {
    answers.push(() => CREATED);

    const { onCreated } = renderForm();

    fireEvent.change(nameField(), { target: { value: '  Hazel  ' } });
    fireEvent.change(
      screen.getByRole('textbox', { name: t('rabbit.form.breedLabel') }),
      { target: { value: 'Holland Lop' } },
    );
    fireEvent.change(
      screen.getByLabelText(t('rabbit.form.dobLabel'), { exact: true }),
      { target: { value: '2026-03-01' } },
    );
    fireEvent.change(
      screen.getByLabelText(t('rabbit.form.adoptionDateLabel')),
      { target: { value: '2025-05-01' } },
    );

    fireEvent.click(
      within(
        screen.getByRole('radiogroup', { name: t('rabbit.sex.label') }),
      ).getByRole('radio', { name: t('rabbit.sex.female') }),
    );
    fireEvent.click(
      within(
        screen.getByRole('radiogroup', {
          name: t('rabbit.neuterStatus.label'),
        }),
      ).getByRole('radio', { name: t('rabbit.neuterStatus.neutered') }),
    );

    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(calls).toHaveLength(1);
    });

    expect(calls[0]).toEqual({
      command: 'create_rabbit',
      args: {
        householdId: HOUSEHOLD_ID,
        profile: {
          // A name is trimmed before it is sent (RAB-2).
          name: 'Hazel',
          photo_ref: null,
          breed: 'Holland Lop',
          dob: '2026-03-01T00:00:00.000Z',
          dob_unknown: false,
          sex: 'female',
          neuter_status: 'neutered',
          adoption_date: '2025-05-01T00:00:00.000Z',
          weight_drop_threshold_percent: 5,
        },
        requestId: expect.any(String),
      },
    });

    expect(onCreated).toHaveBeenCalledTimes(1);
  });

  it('saves an unknown date of birth without a date', async () => {
    answers.push(() => ({ ...CREATED, name: 'Clover', dob: null }));

    renderForm();

    fireEvent.change(nameField(), { target: { value: 'Clover' } });
    fireEvent.click(
      screen.getByRole('checkbox', { name: t('rabbit.form.dobUnknown') }),
    );

    // The explicit "unknown" state wins over the date control (I8).
    expect(
      screen.getByLabelText(t('rabbit.form.dobLabel'), { exact: true }),
    ).toBeDisabled();

    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(calls).toHaveLength(1);
    });

    const profile = calls[0]?.args.profile as Record<string, unknown>;

    expect(profile.dob).toBeNull();
    expect(profile.dob_unknown).toBe(true);
    // RAB-1: "unknown" is a recorded value for sex and neuter status, not an
    // absence the form silently leaves out (D1.14).
    expect(profile.sex).toBe('unknown');
    expect(profile.neuter_status).toBe('unknown');
    // Fields the person did not supply stay absent (RAB-1).
    expect(profile.breed).toBeNull();
    expect(profile.adoption_date).toBeNull();
    expect(profile.photo_ref).toBeNull();
  });

  it('rejects an empty name before calling the command', async () => {
    renderForm();

    fireEvent.click(submitButton());

    expect(await screen.findByText(BLANK_NAME_PROMPT)).toBeVisible();
    expect(nameField()).toHaveAttribute('aria-invalid', 'true');
    expect(calls).toHaveLength(0);
  });

  it('treats a whitespace-only name as blank', async () => {
    renderForm();

    fireEvent.change(nameField(), { target: { value: '   ' } });
    fireEvent.click(submitButton());

    expect(await screen.findByText(BLANK_NAME_PROMPT)).toBeVisible();
    expect(calls).toHaveLength(0);
  });

  it('shows a backend validation refusal on the name field, not the shell', async () => {
    answers.push(() => {
      throw validationError();
    });

    renderForm();

    fireEvent.change(nameField(), { target: { value: 'Hazel' } });
    fireEvent.click(submitButton());

    expect(await screen.findByText(BLANK_NAME_PROMPT)).toBeVisible();
    // `useCreateRabbit` deliberately does not record `validation` (task 3.4),
    // so the field owns it and no shell notice is produced.
    expect(getSaveRetryState().failure).toBeNull();
  });

  it('re-sends the same operation for an unchanged retry', async () => {
    answers.push(() => {
      throw connectivityError();
    });
    answers.push(() => CREATED);

    renderForm();

    fireEvent.change(nameField(), { target: { value: 'Hazel' } });
    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(getSaveRetryState().failure?.cause).toBe('connectivity');
    });

    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(calls).toHaveLength(2);
    });

    expect(calls.map((call) => call.args.requestId)).toEqual([
      calls[0]?.args.requestId,
      calls[0]?.args.requestId,
    ]);
    // A save that landed clears the superseded failure record.
    expect(getSaveRetryState().failure).toBeNull();
  });

  it('closes on a shell-confirmed retry of its failed create', async () => {
    answers.push(() => {
      throw connectivityError();
    });
    answers.push(() => CREATED);

    const { onCreated } = renderForm();

    fireEvent.change(nameField(), { target: { value: 'Hazel' } });
    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(getSaveRetryState().failure?.cause).toBe('connectivity');
    });

    // The shell's retry re-enters the same mutation instance, so the form sees
    // the confirmation and closes even though the form did not start it.
    await act(async () => {
      await retryFailedSave();
    });

    await waitFor(() => {
      expect(onCreated).toHaveBeenCalledTimes(1);
    });

    expect(calls.map((call) => call.args.requestId)).toEqual([
      calls[0]?.args.requestId,
      calls[0]?.args.requestId,
    ]);
    expect(getSaveRetryState().failure).toBeNull();
  });

  it('does not submit twice while the first create is in flight', async () => {
    let resolveCreate: (rabbit: Rabbit) => void = () => {};
    const unresolved = new Promise<Rabbit>((resolve) => {
      resolveCreate = resolve;
    });

    answers.push(() => unresolved);

    const { onCreated } = renderForm();

    fireEvent.change(nameField(), { target: { value: 'Hazel' } });
    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(calls).toHaveLength(1);
    });

    // The control is the busy state: disabled and labelled "Saving…".
    const saving = screen.getByRole('button', {
      name: t('rabbit.form.saving'),
    });

    expect(saving).toBeDisabled();
    fireEvent.click(saving);

    expect(calls).toHaveLength(1);

    await act(async () => {
      resolveCreate(CREATED);
    });

    await waitFor(() => {
      expect(onCreated).toHaveBeenCalledTimes(1);
    });
  });
});
