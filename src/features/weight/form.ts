/*
 * The weight form's validation state (task 3.6; WT-4, D1.9, I4).
 *
 * WT-4 has two rules and only one of them is a rejection. Zero and negative
 * values are refused and the person is prompted for a valid one; a value above
 * `PLAUSIBILITY_BOUND_GRAMS` is a **confirmation threshold**, because a genuinely
 * heavy rabbit still has to be recordable (D1.9). Keeping the distinction in a
 * module rather than in a screen is what stops the two from collapsing into one
 * "invalid" banner, which is the failure WT-4 names twice.
 *
 * The value is parsed by `./api.ts` and never here: `parseDisplayWeight` is the
 * product's one display-unit conversion and already defines what "unreadable"
 * means (sub-gram precision, a comma, an exponent, an empty field, wider than
 * the wire type). This module only classifies its answer, so a second parser
 * cannot disagree with the command about what a weight is.
 *
 * **Empty and zero are different states.** An absent value is not yet an
 * attempt, so there is nothing to correct; `0` is a submitted value the command
 * refuses (WT-4), so the form must prompt. `parseDisplayWeight` reads `"0"` and
 * `"-0"` as the same zero, which is why the zero check comes before the sign
 * check — a rejection, never an absence.
 *
 * **The confirmation belongs to the value, not to the form.** Confirming 12 kg
 * and then editing it to 15 kg must ask again: the person confirmed the number
 * they saw. `editWeightText` therefore clears the flag, and no other transition
 * introduces a new value, so a stale confirmation cannot reach the boundary.
 * Declining is not an error and does not clear the field: the person keeps
 * their value and stays in the form, which is what WT-4's "not saved until the
 * person confirms it" requires. The flag is not part of the value state because
 * it is a fact about the form's history rather than about the text.
 *
 * **Failures are P0's, with one exception.** A failed save is recorded by the
 * weight mutation (task 3.5), which is the recorder for these writes, so this
 * module never calls `recordFailedSave` and must not: a second record would
 * replace the slot with a second copy of one failure. The one cause that stays
 * here is `validation`, because it is the form field's own refusal rather than
 * a save the shell presents (SYNC-2's rule). `weightFormError` describes that
 * cause through P0's map — never the backend's raw key — and returns `null` for
 * every other cause, whose surface is the one `recordFailedSave` feeds. The
 * `null` is the whole point: a form that rendered it too would show a second
 * live region for one failure.
 *
 * No copy is written here. Each state's `kind` is the prompt the form renders,
 * and task 3.8 maps it to a string key (NFR-9). The display unit is not a form
 * setting either: it arrives with the fields from SET-4, and D1.2 gives the
 * form no unit picker that could bypass the bound.
 */
import { describeSaveFailure, SAVE_CAUSE_MESSAGE_KEYS } from '../sync/failure';
import { PLAUSIBILITY_BOUND_GRAMS, parseDisplayWeight } from './api';
import type { WeightEntryInput } from './mutations';

/**
 * The contract's grams, derived from the parser that produces it rather than
 * from `src/bindings.ts` (project hard rule 2).
 */
export type Grams = NonNullable<ReturnType<typeof parseDisplayWeight>>;

/**
 * What the typed value is, as the form needs it to prompt (WT-4).
 *
 * `negative` and `outlier` carry the parsed grams so the surface can state the
 * value it is refusing or confirming without parsing a second time. `zero` is
 * deliberately its own state: it is a rejection, not the absence `empty` means.
 */
export type WeightValueState =
  | { readonly kind: 'empty' }
  | { readonly kind: 'unreadable' }
  | { readonly kind: 'zero' }
  | { readonly kind: 'negative'; readonly grams: Grams }
  | { readonly kind: 'outlier'; readonly grams: Grams }
  | { readonly kind: 'valid'; readonly grams: Grams };

/** The value states that are refused outright and need a prompt, not a save. */
export type WeightPromptState = Extract<
  WeightValueState,
  { kind: 'empty' | 'unreadable' | 'zero' | 'negative' }
>;

/**
 * Classify what the person typed (WT-4).
 *
 * Pure and total: every `text` and `unit` produce exactly one state. The order
 * is the rule — absent, unreadable, zero, negative, above the bound, valid — so
 * `"0"` can never be read as an empty field and `"-0"` never as a negative one.
 */
export function weightValueState(
  text: string,
  unit: Parameters<typeof parseDisplayWeight>[1],
): WeightValueState {
  if (text.trim() === '') {
    return { kind: 'empty' };
  }

  const grams = parseDisplayWeight(text, unit);

  if (grams === null) {
    return { kind: 'unreadable' };
  }

  if (grams === 0) {
    return { kind: 'zero' };
  }

  if (grams < 0) {
    return { kind: 'negative', grams };
  }

  if (grams > PLAUSIBILITY_BOUND_GRAMS) {
    return { kind: 'outlier', grams };
  }

  return { kind: 'valid', grams };
}

/**
 * The form's editable fields: exactly the draft the entry boundary converts
 * (WT-1, WT-3), still in the person's unit.
 *
 * `text` stays unparsed and the unit travels with it, so the same intent
 * re-sends the same number if it is retried (task 3.5). `timestamp: null` means
 * "now" on create and "keep" on update.
 */
export type WeightFormFields = Pick<
  WeightEntryInput,
  'text' | 'unit' | 'timestamp' | 'note'
>;

/**
 * The form's whole state: the fields plus WT-4's confirmation.
 *
 * `outlierConfirmed` is only ever `true` for the value the person was asked
 * about; every transition that changes the text clears it.
 */
export type WeightFormState = {
  readonly fields: WeightFormFields;
  readonly outlierConfirmed: WeightEntryInput['outlierConfirmed'];
};

/** Start a form on fields, with nothing confirmed yet. */
export function beginWeightForm(fields: WeightFormFields): WeightFormState {
  return { fields, outlierConfirmed: false };
}

/**
 * Replace the typed value and clear the confirmation that belonged to the old
 * one.
 *
 * This is the only way to change `text`, which is what makes a stale
 * confirmation unreachable: a person who confirmed 12 kg and edits to 15 kg is
 * asked about 15 kg.
 */
export function editWeightText(
  state: WeightFormState,
  text: string,
): WeightFormState {
  return {
    fields: { ...state.fields, text },
    outlierConfirmed: false,
  };
}

/*
 * The other two editable fields (WT-1, WT-5).
 *
 * They exist so a surface never has to spread `WeightFormState` by hand to
 * change a note or a timestamp: a hand-built state is the one route that could
 * replace `text` while keeping the confirmation that belonged to the old value.
 * Neither of these can touch `text`, so `editWeightText` above stays the only
 * way in, and neither clears the flag — the value the person confirmed is
 * unchanged.
 */

/** Replace the optional note; the value and its confirmation stand (WT-1). */
export function editWeightNote(
  state: WeightFormState,
  note: WeightFormFields['note'],
): WeightFormState {
  return { ...state, fields: { ...state.fields, note } };
}

/** Replace the timestamp; the value and its confirmation stand (WT-1, WT-5). */
export function editWeightTimestamp(
  state: WeightFormState,
  timestamp: WeightFormFields['timestamp'],
): WeightFormState {
  return { ...state, fields: { ...state.fields, timestamp } };
}

/** The person confirmed the outlier they were shown (WT-4). */
export function confirmWeightOutlier(state: WeightFormState): WeightFormState {
  return { ...state, outlierConfirmed: true };
}

/**
 * The person declined the outlier.
 *
 * Not a reset: the value stays in the form so they can correct it or submit it
 * again, and the next submit asks once more (WT-4's "not saved until the person
 * confirms it").
 */
export function declineWeightOutlier(state: WeightFormState): WeightFormState {
  return { ...state, outlierConfirmed: false };
}

/**
 * What the submit path does next (WT-4).
 *
 * `prompt` means the value is refused and the form shows the carried state;
 * `confirm` means the value is held until the person confirms it; `save` means
 * the entry boundary may be called with `input`. Nothing is saved in the first
 * two cases — no mutation is reachable from them.
 */
export type WeightSubmission =
  | { readonly kind: 'prompt'; readonly value: WeightPromptState }
  | { readonly kind: 'confirm'; readonly grams: Grams }
  | { readonly kind: 'save'; readonly input: WeightEntryInput };

/**
 * Decide a submit, without saving anything (WT-4).
 *
 * `save`'s `input` is the shape task 3.5's `draftOf` turns into the wire draft;
 * `outlierConfirmed` is copied to `WeightEntryDraft.outlier_confirmed` there,
 * and it is `true` only for a value the person confirmed.
 */
export function submitWeightForm(state: WeightFormState): WeightSubmission {
  const value = weightValueState(state.fields.text, state.fields.unit);

  switch (value.kind) {
    case 'empty':
    case 'unreadable':
    case 'zero':
    case 'negative':
      return { kind: 'prompt', value };
    case 'outlier':
      return state.outlierConfirmed
        ? save(state)
        : { kind: 'confirm', grams: value.grams };
    case 'valid':
      return save(state);
  }
}

/** The `save` outcome: the fields as they stand, with the form's flag. */
function save(state: WeightFormState): WeightSubmission {
  return {
    kind: 'save',
    input: {
      ...state.fields,
      outlierConfirmed: state.outlierConfirmed,
    },
  };
}

/**
 * A `validation` refusal, as the form field shows it (WT-4, SYNC-2).
 *
 * The key comes from P0's cause map rather than from the failure's
 * `message_key`, so a backend key the product has no copy for cannot surface
 * here (task 3.8 resolves the string).
 */
export type WeightFormError = {
  readonly cause: 'validation';
  readonly messageKey: typeof SAVE_CAUSE_MESSAGE_KEYS.validation;
};

/**
 * Where a failed save is shown, from the form's side.
 *
 * A `validation` refusal is the field's own and is returned so the form keeps
 * it; every other cause is already recorded by the weight mutation and is
 * presented by P0's surface, so `null` says "not this form's to render". This
 * function records nothing and calls no command (task 3.5 owns the recording;
 * a second record would replace one failure with a copy of itself).
 */
export function weightFormError(error: unknown): WeightFormError | null {
  if (describeSaveFailure(error).cause !== 'validation') {
    return null;
  }

  return {
    cause: 'validation',
    messageKey: SAVE_CAUSE_MESSAGE_KEYS.validation,
  };
}
