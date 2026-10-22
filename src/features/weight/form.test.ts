/*
 * Task 3.6's evidence for WT-4: a zero or negative value is rejected with a
 * prompt, a value above the plausibility bound is held for a confirmation and
 * saved only once confirmed, and the confirmation flag reaches the draft.
 *
 * The value states are asserted one by one because the failure they prevent is
 * a collapse into a single "invalid" case: a confirmed 30 kg rabbit must be
 * saveable, and a zero must not read as an untouched field. The submit path is
 * asserted separately from the value state, so "held" is shown to mean "no
 * mutation is reachable" rather than merely "an error is displayed".
 *
 * The failure tests pin the split task 3.5 made: `validation` stays in the form
 * field, every other cause is left to the recorded surface, and the copy comes
 * from the cause rather than the backend's key.
 */
import { describe, expect, it } from 'vitest';
import type { WeightFormFields, WeightFormState } from './form';
import {
  beginWeightForm,
  confirmWeightOutlier,
  declineWeightOutlier,
  editWeightNote,
  editWeightText,
  editWeightTimestamp,
  submitWeightForm,
  weightFormError,
  weightValueState,
} from './form';

/** Fields whose value is overridden per spec. */
const FIELDS: WeightFormFields = {
  text: '1.2',
  unit: 'kilograms',
  timestamp: null,
  note: null,
};

/** A form over `text`, in kilograms unless the spec says otherwise. */
function form(text: string, fields: Partial<WeightFormFields> = {}) {
  return beginWeightForm({ ...FIELDS, text, ...fields });
}

/** A confirmed form over `text`. */
function confirmed(text: string): WeightFormState {
  return confirmWeightOutlier(form(text));
}

describe('the value state machine', () => {
  it('treats an absent value as empty, not as zero', () => {
    expect(weightValueState('', 'grams')).toEqual({ kind: 'empty' });
    expect(weightValueState('   ', 'kilograms')).toEqual({ kind: 'empty' });
  });

  it('delegates unreadable input to the one parser', () => {
    // A comma, sub-gram precision, an exponent and a non-number: the four
    // refusals `parseDisplayWeight` already owns (WT-3).
    expect(weightValueState('1,2', 'kilograms')).toEqual({
      kind: 'unreadable',
    });
    expect(weightValueState('1.2345', 'kilograms')).toEqual({
      kind: 'unreadable',
    });
    expect(weightValueState('1e3', 'grams')).toEqual({ kind: 'unreadable' });
    expect(weightValueState('heavy', 'grams')).toEqual({ kind: 'unreadable' });
  });

  it('rejects zero as a value, including a signed zero', () => {
    expect(weightValueState('0', 'grams')).toEqual({ kind: 'zero' });
    expect(weightValueState('0.000', 'kilograms')).toEqual({ kind: 'zero' });
    expect(weightValueState('-0', 'grams')).toEqual({ kind: 'zero' });
  });

  it('rejects a negative value and reports the grams it parsed', () => {
    expect(weightValueState('-5', 'grams')).toEqual({
      kind: 'negative',
      grams: -5,
    });
    expect(weightValueState('-0.5', 'kilograms')).toEqual({
      kind: 'negative',
      grams: -500,
    });
  });

  it('holds a value above the bound as an outlier, never as a rejection', () => {
    expect(weightValueState('10001', 'grams')).toEqual({
      kind: 'outlier',
      grams: 10_001,
    });
    expect(weightValueState('10.001', 'kilograms')).toEqual({
      kind: 'outlier',
      grams: 10_001,
    });
  });

  it('accepts the bound itself, because the rule is "above"', () => {
    expect(weightValueState('10000', 'grams')).toEqual({
      kind: 'valid',
      grams: 10_000,
    });
    expect(weightValueState('10', 'kilograms')).toEqual({
      kind: 'valid',
      grams: 10_000,
    });
  });

  it('accepts an ordinary value, in either unit', () => {
    expect(weightValueState('1200', 'grams')).toEqual({
      kind: 'valid',
      grams: 1200,
    });
    expect(weightValueState('1.2', 'kilograms')).toEqual({
      kind: 'valid',
      grams: 1200,
    });
  });
});

describe('submitting the form', () => {
  it('prompts instead of saving an absent value', () => {
    expect(submitWeightForm(form(''))).toEqual({
      kind: 'prompt',
      value: { kind: 'empty' },
    });
  });

  it('prompts instead of saving an unreadable value', () => {
    expect(submitWeightForm(form('1,2'))).toEqual({
      kind: 'prompt',
      value: { kind: 'unreadable' },
    });
  });

  it('rejects a zero value with a prompt', () => {
    expect(submitWeightForm(form('0'))).toEqual({
      kind: 'prompt',
      value: { kind: 'zero' },
    });
  });

  it('rejects a negative value with a prompt', () => {
    expect(submitWeightForm(form('-0.5'))).toEqual({
      kind: 'prompt',
      value: { kind: 'negative', grams: -500 },
    });
  });

  it('asks for confirmation before saving a value above the bound', () => {
    expect(submitWeightForm(form('30'))).toEqual({
      kind: 'confirm',
      grams: 30_000,
    });
    expect(submitWeightForm(form('12'))).toEqual({
      kind: 'confirm',
      grams: 12_000,
    });
  });

  it('saves a confirmed outlier with the flag the draft needs', () => {
    const submission = submitWeightForm(confirmed('30'));

    /*
     * `outlierConfirmed` is the field task 3.5's `draftOf` copies to
     * `WeightEntryDraft.outlier_confirmed`, so this is the confirmation
     * reaching the boundary; the copy itself is 3.5's to assert.
     */
    expect(submission).toEqual({
      kind: 'save',
      input: {
        text: '30',
        unit: 'kilograms',
        timestamp: null,
        note: null,
        outlierConfirmed: true,
      },
    });
  });

  it('keeps the value in the form when the person declines, and asks again', () => {
    const declined = declineWeightOutlier(confirmed('30'));

    // Declining is not a reset and not a rejection: the person stays with the
    // value they typed and the next submit prompts again.
    expect(declined.fields.text).toBe('30');
    expect(declined.outlierConfirmed).toBe(false);
    expect(submitWeightForm(declined)).toEqual({
      kind: 'confirm',
      grams: 30_000,
    });
  });

  it('asks again after the value is edited, so a stale confirmation cannot save it', () => {
    const edited = editWeightText(confirmed('30'), '35');

    expect(edited.outlierConfirmed).toBe(false);
    expect(submitWeightForm(edited)).toEqual({
      kind: 'confirm',
      grams: 35_000,
    });
  });

  it('edits the note without disturbing the confirmed value', () => {
    const submission = submitWeightForm(
      editWeightNote(confirmed('30'), 'weighed after dinner'),
    );

    expect(submission).toEqual({
      kind: 'save',
      input: {
        text: '30',
        unit: 'kilograms',
        timestamp: null,
        note: 'weighed after dinner',
        outlierConfirmed: true,
      },
    });
    // A note edit is not a confirmation: the unconfirmed value still asks.
    expect(submitWeightForm(editWeightNote(form('30'), 'note'))).toEqual({
      kind: 'confirm',
      grams: 30_000,
    });
  });

  it('edits the timestamp without disturbing the confirmed value', () => {
    const chosen = '2026-10-02T07:15:00.000Z';
    const submission = submitWeightForm(
      editWeightTimestamp(confirmed('30'), chosen),
    );

    expect(submission).toEqual({
      kind: 'save',
      input: {
        text: '30',
        unit: 'kilograms',
        timestamp: chosen,
        note: null,
        outlierConfirmed: true,
      },
    });
  });

  it('saves a valid value with no confirmation flag', () => {
    expect(submitWeightForm(form('1.2'))).toEqual({
      kind: 'save',
      input: { ...FIELDS, outlierConfirmed: false },
    });
  });

  it('carries the timestamp and note through unchanged', () => {
    const fields: WeightFormFields = {
      ...FIELDS,
      timestamp: '2026-10-01T09:30:00.000Z',
      note: 'annual check',
    };

    expect(submitWeightForm(beginWeightForm(fields))).toEqual({
      kind: 'save',
      input: { ...fields, outlierConfirmed: false },
    });
  });
});

describe('a failed save', () => {
  it("keeps a validation refusal in the form, using the cause's copy", () => {
    expect(
      weightFormError({
        cause: 'validation',
        retryable: false,
        message_key: 'error.validation',
        request_id: null,
      }),
    ).toEqual({ cause: 'validation', messageKey: 'error.validation' });
  });

  it('derives the copy from the cause, not from the backend key', () => {
    expect(
      weightFormError({
        cause: 'validation',
        retryable: false,
        message_key: 'error.not_implemented',
        request_id: null,
      }),
    ).toEqual({ cause: 'validation', messageKey: 'error.validation' });
  });

  it('leaves every other cause to the recorded surface', () => {
    expect(
      weightFormError({
        cause: 'connectivity',
        retryable: true,
        message_key: 'error.connectivity',
        request_id: 'request-1',
      }),
    ).toBeNull();
  });

  it("does not claim an IPC-level failure as the form's own", () => {
    expect(weightFormError(new Error('denied'))).toBeNull();
    expect(weightFormError('denied')).toBeNull();
  });
});
