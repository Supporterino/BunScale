/*
 * The weight entry form (task 4.9; WT-1, WT-3, WT-4, D1.2, D1.9, D1.11).
 *
 * **The form renders `../form.ts`, it does not restate it.** Every rule about
 * what a typed value is — empty, unreadable, zero, negative, above the
 * plausibility bound, valid — and every transition that decides whether a submit
 * prompts, confirms or saves lives in that module. This file owns only what a
 * screen adds: the three fields, the four prompts, the confirmation surface and
 * the focus that follows them, so a zero cannot be "just another invalid input"
 * here while remaining a rejection there.
 *
 * **The display unit is presented, never chosen (WT-3, D1.2, I1).** The unit is
 * `useDisplayWeightUnit()`'s value — SET-4's resolution of the person's choice,
 * the device's region, then canonical grams — and this form neither reads a
 * locale nor picks a default. It shows the unit as a suffix on the value field
 * and as part of the field's accessible name; there is deliberately no
 * `SegmentedControl`, `select` or picker that could reinterpret the typed
 * number. The unit travels with the intent in `WeightFormFields`, so a
 * preference that changes while this form is open cannot reinterpret text that
 * was typed under the previous one.
 *
 * **An above-bound value is a confirmation, not an error (WT-4, D1.9, I4).**
 * The `Notice` is the design system's persistent inline message; the two
 * controls beside it are the catalog's `weight.confirmOutlier.*`. Nothing is
 * sent until "Save it anyway" is chosen, and declining keeps the typed value so
 * the person can correct or resubmit it. The mutation receives
 * `outlierConfirmed: true` only from the confirmed branch.
 *
 * **A timestamp defaults to now and is still editable (WT-1).** The visible
 * `datetime-local` control starts at the current instant, but the untouched
 * form carries `timestamp: null`, which is the contract's "the moment the entry
 * is created" — the exact instant is the storage boundary's, not the form's.
 * Choosing a value replaces it with that explicit instant, including one in the
 * past. The note is optional and empty means absent.
 *
 * **Only `validation` is this form's to show (SYNC-2).** The weight mutation is
 * the recorder for these writes, so every other cause is presented by the
 * shell's `SaveFailureSurface`; `weightFormError` returns `null` for them and
 * this form renders nothing for that cause. A `validation` refusal appears on
 * the value field, which is the input the person can correct. Re-implementing
 * either half here would show one failure twice or record a second copy of it.
 *
 * **The unit read gates the form.** While `useDisplayWeightUnit()` is pending
 * the fields are not drawn, because a value typed before the unit is known
 * could be stored at 1000x its magnitude; a failed read is its own surface with
 * a refresh rather than a silent guess (D1.16).
 *
 * The form is not a dialog: the design system defers `Sheet`/`Dialog`, and the
 * established pattern is the inline form that replaces the surface that opened
 * it (`RabbitForm`). Its mount point is 4.7/4.8's control; this component is
 * what those tasks and 4.10's edit mode render.
 *
 * **Update mode is the same form on an existing entry (task 4.21; WT-5, WT-8).**
 * `entry` turns the form into a correction: the three fields start on the
 * entry's current value, timestamp and note, submit calls `update_weight_entry`
 * with the same rules (the draft is a whole-entry replacement), and the heading
 * says so. Two differences are deliberate. The value is pre-filled in grams
 * through `formatDisplayWeight`, the product's one conversion, so what the
 * person sees re-enters as exactly the stored value; and the timestamp field
 * starts on the entry's own instant rather than "now", because a correction
 * never restamps a back-dated entry (WT-5) — leaving the field untouched
 * re-sends that instant, and clearing it would mean "keep", which the form does
 * not offer as a separate action. Cancel returns to the surface that opened the
 * form on both paths.
 */
import { useEffect, useRef, useState } from 'react';
import { Button } from '../../../components/ui/Button';
import { Card } from '../../../components/ui/Card';
import { ErrorState } from '../../../components/ui/ErrorState';
import { FormField } from '../../../components/ui/FormField';
import { LoadingIndicator } from '../../../components/ui/LoadingIndicator';
import { Notice } from '../../../components/ui/Notice';
import { TextField } from '../../../components/ui/TextField';
import type { MessageKey } from '../../i18n';
import { t } from '../../i18n';
import {
  useDisplayWeightUnit,
  type WeightUnit,
} from '../../settings/display-unit';
import { describeReadFailure } from '../../sync/read-failure';
import { dismissFailedSave } from '../../sync/save-retry';
import { formatDisplayWeight } from '../api';
import {
  beginWeightForm,
  confirmWeightOutlier,
  declineWeightOutlier,
  editWeightNote,
  editWeightText,
  editWeightTimestamp,
  submitWeightForm,
  type WeightFormState,
  type WeightPromptState,
  weightFormError,
} from '../form';
import {
  useCreateWeightEntry,
  useUpdateWeightEntry,
  type WeightEntryInput,
} from '../mutations';
import type { WeightEntry } from '../queries';

/**
 * One prompt per refusal state `../form.ts` classifies (WT-4). The mapping is
 * exhaustive, so a state added to the machine without copy is a compile error
 * rather than a silent generic message.
 */
const PROMPT_MESSAGE_KEYS = {
  empty: 'weight.prompt.empty',
  unreadable: 'weight.prompt.unreadable',
  zero: 'weight.prompt.zero',
  negative: 'weight.prompt.negative',
} as const satisfies Record<WeightPromptState['kind'], MessageKey>;

/** The catalog's names for the two units, used in the field's accessible name. */
const UNIT_LABEL_KEYS = {
  grams: 'settings.displayUnit.grams',
  kilograms: 'settings.displayUnit.kilograms',
} as const satisfies Record<WeightUnit, MessageKey>;

/*
 * The unit's short symbol ("g", "kg") for the visible suffix. It comes from the
 * platform's unit formatting rather than a hard-coded string, so the same
 * localized output `WeightSeries` shows beside a stored value appears here
 * (NFR-9). This formats a unit, not a weight: the value's digits stay exactly
 * what the person typed.
 */
const UNIT_SYMBOL_FORMATS: Record<WeightUnit, Intl.NumberFormat> = {
  grams: new Intl.NumberFormat(undefined, {
    style: 'unit',
    unit: 'gram',
    unitDisplay: 'short',
  }),
  kilograms: new Intl.NumberFormat(undefined, {
    style: 'unit',
    unit: 'kilogram',
    unitDisplay: 'short',
  }),
};

function unitSymbol(unit: WeightUnit): string {
  return (
    UNIT_SYMBOL_FORMATS[unit]
      .formatToParts(1)
      .find((part) => part.type === 'unit')?.value ?? ''
  );
}

function pad2(value: number): string {
  return String(value).padStart(2, '0');
}

/** `Date` as the `datetime-local` value the control expects, in local time. */
function localDateTimeValue(date: Date): string {
  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(
    date.getDate(),
  )}T${pad2(date.getHours())}:${pad2(date.getMinutes())}`;
}

/**
 * A `datetime-local` value as the contract's `Instant`.
 *
 * A blank control means "now" on create, so it is the contract's `null`. The
 * input omits a zone, so the platform reads it as local time — the person
 * entered their own clock, not UTC.
 */
function instantFromLocalDateTime(value: string): string | null {
  if (value === '') {
    return null;
  }

  const instant = new Date(value);

  return Number.isNaN(instant.getTime()) ? null : instant.toISOString();
}

/**
 * The entry an update mode starts from (WT-5, WT-8).
 *
 * Exactly the fields the form edits, taken from the detail read's payload so a
 * caller passes the whole entry it loaded rather than restating its shape.
 */
export type WeightFormEntry = Pick<
  WeightEntry,
  'id' | 'value_grams' | 'timestamp' | 'note'
>;

export type WeightFormProps = {
  /** The active household the rabbit belongs to (NFR-10). */
  householdId: string;
  /** The rabbit this entry is bound to (WT-2). */
  rabbitId: string;
  /**
   * The entry to correct. Absent, the form records a new one; present, it
   * starts on that entry's fields and submits with `update_weight_entry`
   * (WT-5).
   */
  entry?: WeightFormEntry;
  /** Returns to the surface that opened the form without writing anything. */
  onCancel: () => void;
  /** The backend confirmed the entry; the caller can present it. */
  onSaved: () => void;
};

/**
 * The entry form, gated on the one read it cannot invent (WT-3).
 *
 * The separate inner component is what lets `useState` initialise the machine
 * with a unit that is already known: a hook cannot be called conditionally, and
 * an initial state built from `undefined` would have to be corrected after the
 * person had already seen it.
 */
export function WeightForm({
  householdId,
  rabbitId,
  entry,
  onCancel,
  onSaved,
}: WeightFormProps) {
  const unit = useDisplayWeightUnit();

  if (unit.data !== undefined) {
    return (
      <WeightFormFields
        entry={entry}
        householdId={householdId}
        onCancel={onCancel}
        onSaved={onSaved}
        rabbitId={rabbitId}
        unit={unit.data}
      />
    );
  }

  return (
    <Card className="bs-weight-form" heading={t('weight.log')}>
      {unit.error === null ? (
        <LoadingIndicator label={t('settings.displayUnit.loading')} />
      ) : (
        <UnitReadFailure
          error={unit.error}
          onRefresh={() => {
            void unit.refetch();
          }}
        />
      )}
    </Card>
  );
}

/**
 * A failed unit read, with the cause's copy and a refresh (NFR-8, D1.16).
 *
 * A refresh is the only action a read has: it takes no `request_id`, so there is
 * no operation to retry. `describeReadFailure` decides whether a fresh read can
 * change the answer and supplies the key the catalog knows.
 */
function UnitReadFailure({
  error,
  onRefresh,
}: {
  error: unknown;
  onRefresh: () => void;
}) {
  const failure = describeReadFailure(error);

  return (
    <ErrorState
      {...(failure.refreshable ? { onRetry: onRefresh } : {})}
      retryLabel={t('error.retry')}
      title={t('settings.displayUnit.heading')}
    >
      {t(failure.messageKey)}
    </ErrorState>
  );
}

type WeightFormFieldsProps = WeightFormProps & {
  /** The unit SET-4 resolved when the form opened; it travels with the intent. */
  unit: WeightUnit;
};

function WeightFormFields({
  householdId,
  rabbitId,
  entry,
  unit,
  onCancel,
  onSaved,
}: WeightFormFieldsProps) {
  const [form, setForm] = useState<WeightFormState>(() =>
    beginWeightForm({
      // The stored value in the person's unit, through the product's one
      // conversion, so an untouched field re-enters as exactly that value.
      text:
        entry === undefined ? '' : formatDisplayWeight(entry.value_grams, unit),
      unit,
      timestamp: entry?.timestamp ?? null,
      note: entry?.note ?? null,
    }),
  );
  const [prompt, setPrompt] = useState<WeightPromptState | null>(null);
  const [outlierGrams, setOutlierGrams] = useState<number | null>(null);
  const [timestampText, setTimestampText] = useState(() =>
    localDateTimeValue(
      entry === undefined ? new Date() : new Date(entry.timestamp),
    ),
  );
  const create = useCreateWeightEntry();
  const update = useUpdateWeightEntry();
  const saveEntry = entry === undefined ? create : update;
  const valueRef = useRef<HTMLInputElement>(null);
  const confirmationRef = useRef<HTMLDivElement>(null);
  const closed = useRef(false);

  /*
   * The form replaces the control that opened it, so focus has to be handed to
   * the new surface. The value field is the form's entry point, exactly as the
   * rabbit form focuses its first field.
   */
  useEffect(() => {
    valueRef.current?.focus();
  }, []);

  /*
   * A confirmed save closes the form, whichever surface confirmed it: the form's
   * own submit, or the shell's recorded retry re-entering the same mutation
   * instance. `closed` keeps a re-render from closing twice.
   */
  useEffect(() => {
    if (!saveEntry.isSuccess || closed.current) {
      return;
    }

    closed.current = true;
    dismissFailedSave();
    onSaved();
  }, [saveEntry.isSuccess, onSaved]);

  /*
   * The confirmation appears after a submit and contains the two controls that
   * answer it. Focus moves to the confirm control so a keyboard or screen-reader
   * user is not left on the submit button with the question behind them; the
   * second control is "Save it anyway" by DOM order below (decline is first).
   */
  useEffect(() => {
    if (outlierGrams === null) {
      return;
    }

    confirmationRef.current?.querySelectorAll('button')[1]?.focus();
  }, [outlierGrams]);

  const validation =
    saveEntry.error === null ? null : weightFormError(saveEntry.error);
  const pending = saveEntry.isPending;

  /*
   * The field's one error slot: a refusal the machine just classified wins over
   * the mutation's `validation`, because the machine's prompt names the value
   * the person can see and correct. A non-`validation` failure is the shell's
   * and is never returned by `weightFormError` (SYNC-2).
   */
  const fieldError =
    prompt !== null
      ? t(PROMPT_MESSAGE_KEYS[prompt.kind])
      : validation === null
        ? undefined
        : t(validation.messageKey);

  const activeUnit = form.fields.unit;
  const unitLabel = t(UNIT_LABEL_KEYS[activeUnit]);

  function changeValue(text: string) {
    // The machine clears the confirmation that belonged to the old value (WT-4).
    setForm((current) => editWeightText(current, text));
    setPrompt(null);
    setOutlierGrams(null);

    if (validation !== null) {
      saveEntry.reset();
    }
  }

  function changeTimestamp(text: string) {
    setTimestampText(text);
    setForm((current) =>
      editWeightTimestamp(current, instantFromLocalDateTime(text)),
    );
  }

  function changeNote(text: string) {
    setForm((current) => editWeightNote(current, text === '' ? null : text));
  }

  function save(input: WeightEntryInput) {
    if (entry === undefined) {
      create.mutate({ householdId, rabbitId, input });
      return;
    }

    // The same fields, as a whole-entry replacement (WT-5).
    update.mutate({ entryId: entry.id, householdId, rabbitId, input });
  }

  function submit() {
    if (pending) {
      return;
    }

    setPrompt(null);
    setOutlierGrams(null);

    /*
     * The note is optional and trimmed at the boundary, like the rabbit form's
     * optional text fields: spaces are not a note, and an empty note is `null`.
     */
    const note = form.fields.note?.trim() ?? '';
    const prepared = editWeightNote(form, note === '' ? null : note);

    setForm(prepared);

    const submission = submitWeightForm(prepared);

    if (submission.kind === 'prompt') {
      setPrompt(submission.value);
      saveEntry.reset();
      return;
    }

    if (submission.kind === 'confirm') {
      setOutlierGrams(submission.grams);
      return;
    }

    save(submission.input);
  }

  function confirmOutlier() {
    const confirmed = confirmWeightOutlier(form);

    setForm(confirmed);
    setOutlierGrams(null);

    const submission = submitWeightForm(confirmed);

    if (submission.kind === 'save') {
      save(submission.input);
    }
  }

  function declineOutlier() {
    // Declining is not an error: the value stays, the form stays, and the next
    // submit asks once more (WT-4).
    setForm((current) => declineWeightOutlier(current));
    setOutlierGrams(null);
    valueRef.current?.focus();
  }

  return (
    <Card
      className="bs-weight-form"
      heading={
        entry === undefined ? t('weight.log') : t('weight.form.editTitle')
      }
    >
      <form
        aria-busy={pending ? true : undefined}
        className="bs-weight-form__form"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <FormField
          error={fieldError}
          label={t('weight.form.valueLabel')}
          required
        >
          {(field) => (
            <div className="bs-weight-form__value">
              {/*
               * A text input, not `type="number"`: the product's own parser
               * decides what a weight is (commas and sub-gram precision are
               * refused with their own prompt), and a native number control
               * would silently drop characters before that rule saw them.
               * `inputMode` still offers the decimal keypad.
               */}
              <input
                aria-describedby={field.describedBy}
                aria-invalid={field.invalid}
                aria-label={`${t('weight.form.valueLabel')} (${unitLabel})`}
                autoComplete="off"
                className="bs-text-field bs-weight-form__value-input"
                disabled={pending}
                enterKeyHint="next"
                id={field.controlId}
                inputMode="decimal"
                name="weight-value"
                onChange={(event) => {
                  changeValue(event.currentTarget.value);
                }}
                ref={valueRef}
                required
                type="text"
                value={form.fields.text}
              />
              <span aria-hidden="true" className="bs-weight-form__unit">
                {unitSymbol(activeUnit)}
              </span>
            </div>
          )}
        </FormField>

        {outlierGrams === null ? null : (
          <div ref={confirmationRef}>
            <Notice
              action={
                <div className="bs-weight-form__confirm-actions">
                  <Button onClick={declineOutlier} variant="plain">
                    {t('weight.confirmOutlier.decline')}
                  </Button>
                  <Button onClick={confirmOutlier}>
                    {t('weight.confirmOutlier.confirm')}
                  </Button>
                </div>
              }
              title={t('weight.confirmOutlier.title')}
              tone="warning"
            >
              {t('weight.confirmOutlier.body')}
            </Notice>
          </div>
        )}

        <FormField label={t('weight.form.timestampLabel')}>
          {(field) => (
            <input
              aria-describedby={field.describedBy}
              className="bs-text-field"
              disabled={pending}
              id={field.controlId}
              onChange={(event) => {
                changeTimestamp(event.currentTarget.value);
              }}
              type="datetime-local"
              value={timestampText}
            />
          )}
        </FormField>

        <TextField
          autoComplete="off"
          disabled={pending}
          enterKeyHint="done"
          label={t('weight.form.noteLabel')}
          name="weight-note"
          onChange={changeNote}
          placeholder={t('weight.form.notePlaceholder')}
          value={form.fields.note ?? ''}
        />

        <div className="bs-weight-form__actions">
          <Button disabled={pending} onClick={onCancel} variant="plain">
            {t('common.cancel')}
          </Button>
          <Button loading={pending} type="submit">
            {pending ? t('weight.form.saving') : t('weight.form.save')}
          </Button>
        </div>
      </form>
    </Card>
  );
}
