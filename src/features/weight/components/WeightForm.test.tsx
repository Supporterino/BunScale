/*
 * The weight entry form at the component layer (task 4.9; WT-1, WT-3, WT-4).
 *
 * What this layer owns: the three fields, the unit being presented rather than
 * chosen, the six value states `../form.ts` classifies being rendered as their
 * own prompts or confirmation, and what leaves the form for the command. The
 * validation machine itself is covered by `../form.test.ts`, and the display
 * unit boundary by `../mutations.test.ts`; this file asserts that the screen
 * wires the two together rather than re-deriving either.
 *
 * IPC is mocked at `invoke`, the seam the other feature specs use, so the real
 * `useCreateWeightEntry`, `requestOperation` and parse run and the assertions
 * are about the draft that left the form. `get_display_weight_unit` is the one
 * read the form waits on: it answers what SET-4 already resolved (the person's
 * choice, the device region, then canonical grams), which is why changing that
 * answer is how "the form uses the regional default" is exercised here — the
 * form must never invent its own.
 *
 * **Task 5.12 (WT-1, WT-3, WT-4) extended this file with the cases the scenario
 * set names and 4.9 did not yet pin:** "The note is optional" (saved without a
 * note, and a whitespace-only note treated as none), the confirmation being
 * released when a corrected value falls back under the bound, and "A confirmed
 * outlier is saved" carrying the chosen timestamp and note alongside the value.
 * The WT-1 timestamp cases and the value/unit cases already here are kept as
 * 5.12's evidence too. 5.13 owns the edit-mode prefill and the confirmed Delete
 * of the entry detail; 5.14 owns the dashboard, the trend/history ranges and the
 * chart summary. Neither is asserted in this file.
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import {
  describeAtBothBaselines,
  renderWithQueryClient,
} from '../../../test/setup';
import { t } from '../../i18n';
import type { WeightUnit } from '../../settings/display-unit';
import { getSaveRetryState, resetSaveRetry } from '../../sync/save-retry';
import { WeightForm } from './WeightForm';

/** The rabbit the entry is bound to (WT-2). */
const RABBIT_ID = 'rabbit-hazel';

/** The active household (NFR-10). */
const HOUSEHOLD_ID = 'household-home';

/** The command's confirmed answer: one saved entry. */
const SAVED = {
  id: 'entry-a',
  rabbit_id: RABBIT_ID,
  value_grams: 1200,
  timestamp: '2026-10-01T09:00:00Z',
  created_at: '2026-10-01T09:00:00Z',
  note: null,
  photo_refs: [],
  has_weight_drop_alert: false,
};

type Call = { command: string; args: Record<string, unknown> };

let answers: Array<() => unknown>;
let calls: Call[];
let unit: WeightUnit;

function validationError() {
  return {
    cause: 'validation',
    retryable: false,
    message_key: 'error.validation',
    request_id: null,
  };
}

function connectivityError() {
  return {
    cause: 'connectivity',
    retryable: true,
    message_key: 'error.connectivity',
    request_id: null,
  };
}

beforeEach(() => {
  answers = [];
  calls = [];
  unit = 'kilograms';
  resetSaveRetry();

  mockIPC((command, args) => {
    calls.push({ command, args: (args ?? {}) as Record<string, unknown> });

    if (command === 'get_display_weight_unit') {
      return unit;
    }

    if (command !== 'create_weight_entry') {
      throw new Error(`unexpected mocked command "${command}"`);
    }

    const answer = answers.shift();

    if (answer === undefined) {
      throw new Error('create_weight_entry answered without a queued value');
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

function renderForm(onSaved = vi.fn()) {
  const onCancel = vi.fn();

  renderWithQueryClient(
    <WeightForm
      householdId={HOUSEHOLD_ID}
      onCancel={onCancel}
      onSaved={onSaved}
      rabbitId={RABBIT_ID}
    />,
  );

  return { onCancel, onSaved };
}

/** The value field, once the unit read has resolved and the form exists. */
async function valueField(): Promise<HTMLElement> {
  return screen.findByRole('textbox', {
    name: `${t('weight.form.valueLabel')} (${t('settings.displayUnit.kilograms')})`,
  });
}

function submitButton(): HTMLElement {
  return screen.getByRole('button', { name: t('weight.form.save') });
}

function typeAndSubmit(value: string): void {
  fireEvent.change(valueInput(), { target: { value } });
  fireEvent.click(submitButton());
}

/** The value input by its stable name, for synchronous changes after render. */
function valueInput(): HTMLElement {
  return screen.getByRole('textbox', {
    name: `${t('weight.form.valueLabel')} (${t('settings.displayUnit.kilograms')})`,
  });
}

/** The create intent, as the command received it (WT-3's boundary). */
function entryOf(index = 0): Record<string, unknown> {
  const creates = calls.filter(
    (call) => call.command === 'create_weight_entry',
  );

  return creates[index]?.args.entry as Record<string, unknown>;
}

describeAtBothBaselines('Weight entry form (WT-1, WT-3, WT-4)', () => {
  it('saves a value entered in kilograms as canonical grams (WT-3)', async () => {
    answers.push(() => SAVED);

    const { onSaved } = renderForm();

    await valueField();
    fireEvent.change(valueInput(), { target: { value: '1.2' } });
    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_weight_entry'),
      ).toHaveLength(1);
    });

    expect(calls[calls.length - 1]).toEqual({
      command: 'create_weight_entry',
      args: {
        rabbitId: RABBIT_ID,
        entry: {
          value_grams: 1200,
          timestamp: null,
          note: null,
          outlier_confirmed: false,
        },
        requestId: expect.any(String),
      },
    });

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
  });

  it('presents the unit without offering any control that chooses it (WT-3)', async () => {
    renderForm();

    await valueField();

    // The unit is part of the value field's name and a visible suffix…
    expect(
      screen.getByRole('textbox', {
        name: `${t('weight.form.valueLabel')} (${t('settings.displayUnit.kilograms')})`,
      }),
    ).toBeVisible();

    const suffix = document.querySelector('.bs-weight-form__unit');

    expect(suffix?.textContent?.trim()).not.toBe('');

    // …and there is no group, select or button that could change it (D1.2).
    expect(screen.queryByRole('radiogroup')).toBeNull();
    expect(screen.queryByRole('combobox')).toBeNull();
    expect(document.querySelector('select')).toBeNull();

    const unitButtons = screen
      .queryAllByRole('button')
      .filter((button) =>
        /kilogram|gram|\bkg\b|\bg\b/i.test(button.textContent ?? ''),
      );

    expect(unitButtons).toHaveLength(0);
  });

  it('uses the unit the device resolved, so an unchosen preference follows the region (WT-3)', async () => {
    // What `get_display_weight_unit` answers when no preference was stored and
    // the region resolves to grams. The form must use it, not a local default.
    unit = 'grams';
    answers.push(() => ({ ...SAVED, value_grams: 1200 }));

    renderForm();

    const field = await screen.findByRole('textbox', {
      name: `${t('weight.form.valueLabel')} (${t('settings.displayUnit.grams')})`,
    });

    fireEvent.change(field, { target: { value: '1200' } });
    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_weight_entry'),
      ).toHaveLength(1);
    });

    expect(entryOf()).toMatchObject({ value_grams: 1200 });
  });

  it('rejects zero with its own prompt and sends nothing (WT-4)', async () => {
    renderForm();

    await valueField();
    typeAndSubmit('0');

    expect(await screen.findByText(t('weight.prompt.zero'))).toBeVisible();
    expect(valueInput()).toHaveAttribute('aria-invalid', 'true');
    expect(
      calls.filter((call) => call.command === 'create_weight_entry'),
    ).toHaveLength(0);
  });

  it('rejects a negative value with its own prompt (WT-4)', async () => {
    renderForm();

    await valueField();
    typeAndSubmit('-5');

    expect(await screen.findByText(t('weight.prompt.negative'))).toBeVisible();
    expect(valueInput()).toHaveAttribute('aria-invalid', 'true');
    expect(
      calls.filter((call) => call.command === 'create_weight_entry'),
    ).toHaveLength(0);
  });

  it('prompts for a value when the field is empty (WT-4)', async () => {
    renderForm();

    await valueField();
    fireEvent.click(submitButton());

    expect(await screen.findByText(t('weight.prompt.empty'))).toBeVisible();
    expect(
      calls.filter((call) => call.command === 'create_weight_entry'),
    ).toHaveLength(0);
  });

  it('holds a value above the bound for confirmation and sends nothing (WT-4)', async () => {
    renderForm();

    await valueField();
    typeAndSubmit('12');

    expect(
      await screen.findByText(t('weight.confirmOutlier.title')),
    ).toBeVisible();
    expect(screen.getByText(t('weight.confirmOutlier.body'))).toBeVisible();
    expect(
      screen.getByRole('button', { name: t('weight.confirmOutlier.confirm') }),
    ).toBeVisible();
    expect(
      calls.filter((call) => call.command === 'create_weight_entry'),
    ).toHaveLength(0);
  });

  it('saves a confirmed outlier with `outlier_confirmed` (WT-4)', async () => {
    answers.push(() => ({ ...SAVED, value_grams: 12_000 }));

    const { onSaved } = renderForm();

    await valueField();
    typeAndSubmit('12');

    fireEvent.click(
      await screen.findByRole('button', {
        name: t('weight.confirmOutlier.confirm'),
      }),
    );

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_weight_entry'),
      ).toHaveLength(1);
    });

    expect(entryOf()).toEqual({
      value_grams: 12_000,
      timestamp: null,
      note: null,
      outlier_confirmed: true,
    });

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
  });

  it('saves a confirmed outlier with the chosen timestamp and note (WT-1, WT-4)', async () => {
    answers.push(() => ({
      ...SAVED,
      value_grams: 12_000,
      timestamp: new Date('2026-01-02T09:30').toISOString(),
      note: 'Morning check',
    }));

    const { onSaved } = renderForm();

    await valueField();
    fireEvent.change(screen.getByLabelText(t('weight.form.timestampLabel')), {
      target: { value: '2026-01-02T09:30' },
    });
    fireEvent.change(
      screen.getByRole('textbox', { name: t('weight.form.noteLabel') }),
      { target: { value: 'Morning check' } },
    );
    typeAndSubmit('12');

    fireEvent.click(
      await screen.findByRole('button', {
        name: t('weight.confirmOutlier.confirm'),
      }),
    );

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_weight_entry'),
      ).toHaveLength(1);
    });

    // Confirming the value must not drop what else the person set (WT-1).
    expect(entryOf()).toEqual({
      value_grams: 12_000,
      timestamp: new Date('2026-01-02T09:30').toISOString(),
      note: 'Morning check',
      outlier_confirmed: true,
    });

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
  });

  it('asks for confirmation again after the value is corrected above the bound (WT-4)', async () => {
    answers.push(() => ({ ...SAVED, value_grams: 15_000 }));

    renderForm();

    await valueField();
    typeAndSubmit('12');

    // The confirmation belongs to the value that was shown, not to the form.
    expect(
      await screen.findByText(t('weight.confirmOutlier.title')),
    ).toBeVisible();

    fireEvent.change(valueInput(), { target: { value: '15' } });

    expect(screen.queryByText(t('weight.confirmOutlier.title'))).toBeNull();

    fireEvent.click(submitButton());

    expect(
      await screen.findByText(t('weight.confirmOutlier.title')),
    ).toBeVisible();
    expect(
      calls.filter((call) => call.command === 'create_weight_entry'),
    ).toHaveLength(0);
  });

  it('saves a corrected value below the bound without asking for confirmation (WT-4)', async () => {
    answers.push(() => ({ ...SAVED, value_grams: 5000 }));

    const { onSaved } = renderForm();

    await valueField();
    typeAndSubmit('12');

    expect(
      await screen.findByText(t('weight.confirmOutlier.title')),
    ).toBeVisible();

    fireEvent.change(valueInput(), { target: { value: '5' } });
    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_weight_entry'),
      ).toHaveLength(1);
    });

    expect(entryOf()).toEqual({
      value_grams: 5000,
      timestamp: null,
      note: null,
      outlier_confirmed: false,
    });

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
  });

  it('keeps the value after declining and asks again on the next submit (WT-4)', async () => {
    renderForm();

    await valueField();
    typeAndSubmit('12');

    fireEvent.click(
      await screen.findByRole('button', {
        name: t('weight.confirmOutlier.decline'),
      }),
    );

    expect(screen.queryByText(t('weight.confirmOutlier.title'))).toBeNull();
    expect(valueInput()).toHaveValue('12');
    expect(
      calls.filter((call) => call.command === 'create_weight_entry'),
    ).toHaveLength(0);

    fireEvent.click(submitButton());

    expect(
      await screen.findByText(t('weight.confirmOutlier.title')),
    ).toBeVisible();
  });

  it('defaults the timestamp to now and sends "now" while it is untouched (WT-1)', async () => {
    answers.push(() => SAVED);

    renderForm();

    await valueField();

    const timestamp = screen.getByLabelText(
      t('weight.form.timestampLabel'),
    ) as HTMLInputElement;
    const shown = new Date(timestamp.value);

    // No longer than a minute behind "now": the control is minute-precise.
    expect(Number.isNaN(shown.getTime())).toBe(false);
    expect(Math.abs(Date.now() - shown.getTime())).toBeLessThan(60_000);

    fireEvent.change(valueInput(), { target: { value: '1.2' } });
    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_weight_entry'),
      ).toHaveLength(1);
    });

    // `null` is the contract's "the moment the entry is created" (WT-1).
    expect(entryOf()).toMatchObject({ timestamp: null, note: null });
  });

  it('sends an edited timestamp as the instant the person chose (WT-1)', async () => {
    answers.push(() => SAVED);

    renderForm();

    await valueField();
    fireEvent.change(screen.getByLabelText(t('weight.form.timestampLabel')), {
      target: { value: '2026-01-02T09:30' },
    });
    fireEvent.change(valueInput(), { target: { value: '1.2' } });
    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_weight_entry'),
      ).toHaveLength(1);
    });

    // The control omits a zone, so the platform reads the person's own clock.
    expect(entryOf()).toMatchObject({
      timestamp: new Date('2026-01-02T09:30').toISOString(),
    });
  });

  it('sends an optional note when one is given (WT-1)', async () => {
    answers.push(() => ({ ...SAVED, note: 'Morning check' }));

    renderForm();

    await valueField();
    fireEvent.change(
      screen.getByRole('textbox', { name: t('weight.form.noteLabel') }),
      {
        target: { value: '  Morning check  ' },
      },
    );
    fireEvent.change(valueInput(), { target: { value: '1.2' } });
    fireEvent.click(submitButton());

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_weight_entry'),
      ).toHaveLength(1);
    });

    expect(entryOf()).toMatchObject({ note: 'Morning check' });
  });

  it('saves an entry with no note (WT-1)', async () => {
    answers.push(() => SAVED);

    const { onSaved } = renderForm();

    await valueField();
    typeAndSubmit('1.2');

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_weight_entry'),
      ).toHaveLength(1);
    });

    // The note field was never touched: the entry is saved and carries none.
    expect(entryOf()).toMatchObject({ note: null });

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
  });

  it('treats a whitespace-only note as no note (WT-1)', async () => {
    answers.push(() => SAVED);

    renderForm();

    await valueField();
    fireEvent.change(
      screen.getByRole('textbox', { name: t('weight.form.noteLabel') }),
      { target: { value: '   ' } },
    );
    typeAndSubmit('1.2');

    await waitFor(() => {
      expect(
        calls.filter((call) => call.command === 'create_weight_entry'),
      ).toHaveLength(1);
    });

    expect(entryOf()).toMatchObject({ note: null });
  });

  it('shows a validation refusal on the value field, not as a save (SYNC-2)', async () => {
    answers.push(() => {
      throw validationError();
    });

    const { onSaved } = renderForm();

    await valueField();
    typeAndSubmit('1.2');

    expect(await screen.findByText(t('error.validation'))).toBeVisible();
    expect(valueInput()).toHaveAttribute('aria-invalid', 'true');
    expect(onSaved).not.toHaveBeenCalled();
  });

  it("leaves every other cause to the shell's failure surface (SYNC-2)", async () => {
    answers.push(() => {
      throw connectivityError();
    });

    renderForm();

    await valueField();
    typeAndSubmit('1.2');

    await waitFor(() => {
      expect(getSaveRetryState().failure?.cause).toBe('connectivity');
    });

    // The form renders no second live region for a save the shell presents.
    expect(screen.queryByText(t('error.connectivity'))).toBeNull();
    expect(screen.queryByText(t('error.validation'))).toBeNull();
  });
});
