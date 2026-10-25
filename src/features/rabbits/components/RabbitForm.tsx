/*
 * Rabbit creation and editing (tasks 4.3 and 4.4; RAB-1, RAB-2, RAB-4, RAB-5;
 * D1.11, D1.14).
 *
 * The form records the whole identifying field set RAB-1 names: a required
 * name, an optional photo reference, an optional breed, an optional date of
 * birth that can be explicitly marked unknown, a sex, a neuter/spay status and
 * an optional adoption date. The two fixed value sets are D1.14's, not a
 * vocabulary invented here: `male | female | unknown` and
 * `intact | neutered | spayed | unknown`, with `unknown` as the initial value
 * because "not stated" is a recorded value in the contract rather than an
 * absence (design.md, "Shared types").
 *
 * **One form, two modes (task 4.4; RAB-5).** `initial` is present when the
 * detail's edit control opens this form: it carries the rabbit's id and its
 * whole saved profile, and every field is filled from it. Submitting then
 * replaces that profile through `update_rabbit` — a whole-profile replacement,
 * not a patch — instead of creating. The weight-drop threshold is shown in edit
 * mode only: RAB-1's creation flow keeps the contract's 5% default, and RAB-5
 * requires the threshold to be editable once a rabbit exists. The form is not
 * forked for the two modes, so the field set, the fixed value sets and the
 * blank-name refusal cannot drift between them.
 *
 * **The photo field is carried, not collected.** Choosing a photo is PHOTO-2 in
 * P2 and brings its own capability (D1.11), so P1 has no picker. A creation
 * therefore submits `photo_ref: null`, while an edit carries the saved
 * reference through unchanged: `update_rabbit` is a whole-profile replacement,
 * so submitting `null` would erase a photo the person already supplied (RAB-5).
 * A later photo flow fills the same value without changing this shape.
 *
 * **No `Sheet`/`Dialog`.** The design system defers both, and P0's own form
 * pattern is an inline one (Settings' "create another household" form). This
 * form is rendered in place of the surface that opened it — the dashboard list
 * or the profile detail — so it needs no new primitive and the adaptive frame
 * is unchanged.
 *
 * **Blank names never reach the command.** RAB-2 treats an empty name and a
 * whitespace-only name as the same refusal: the form trims first, shows
 * `rabbit.form.blankName` under the field and returns before the write is
 * called. A backend `validation` refusal gets the same field treatment in
 * create mode; every other cause is already recorded by the chosen write hook
 * (task 3.4) for P0's shell surface, so this component deliberately does not
 * call `recordFailedSave` a second time.
 *
 * **An edit's threshold is refused before the write when it cannot be read.**
 * A cleared, negative or non-finite number would otherwise be saved as a value
 * the person did not type (the contract's `f64` has no "unchanged" spelling).
 * The field says `error.validation` and the form returns; `design.md` states no
 * threshold bound, so there is no threshold-specific premise to word copy from.
 *
 * **One operation per intent (D0.14).** The last handle is kept beside the
 * profile it was created with. Submitting the unchanged profile re-runs that
 * handle (the same `request_id`); any changed value is a new intent with a new
 * handle.
 *
 * **A confirmed save closes the form, whichever surface confirmed it.** The
 * form's own submit and the shell's `retryFailedSave()` reach the same mutation
 * instance (the recording retry re-enters the chosen hook), so the mutation's
 * `isSuccess` is the one signal that covers both. Closing from that signal
 * rather than from a per-call `onSuccess` is what keeps a shell-confirmed retry
 * from leaving the form open on a rabbit that already exists; it also clears an
 * earlier failure record, so the shell does not keep presenting a failure this
 * save has superseded. In edit mode the same two hooks are the update write, so
 * a confirmed edit closes the form and the detail re-reads the profile.
 */
import { useEffect, useRef, useState } from 'react';
import { Button } from '../../../components/ui/Button';
import { Card } from '../../../components/ui/Card';
import { FormField } from '../../../components/ui/FormField';
import {
  SegmentedControl,
  type SegmentedControlOption,
} from '../../../components/ui/SegmentedControl';
import { Text } from '../../../components/ui/Text';
import { TextField } from '../../../components/ui/TextField';
import { t } from '../../i18n';
import { describeSaveFailure } from '../../sync/failure';
import { dismissFailedSave } from '../../sync/save-retry';
import { createRabbit, updateRabbit } from '../api';
import { useCreateRabbit, useUpdateRabbit } from '../mutations';

/**
 * `RabbitProfile` is derived from the wrapper that takes it rather than
 * imported from `src/bindings.ts`, because only `api.ts` may import the
 * generated bindings (project hard rule 2).
 */
type RabbitProfile = Parameters<typeof createRabbit>[1];

type Sex = RabbitProfile['sex'];
type NeuterStatus = RabbitProfile['neuter_status'];
type WriteOperation =
  | ReturnType<typeof createRabbit>
  | ReturnType<typeof updateRabbit>;

/** The contract's documented default threshold (WT-7; design.md I7). */
export const DEFAULT_WEIGHT_DROP_THRESHOLD_PERCENT = 5;

/**
 * A calendar date as the contract's `Instant`.
 *
 * I8 pins these dates to UTC: the day entered is the day every member and
 * device sees, so this concatenates the input's `YYYY-MM-DD` instead of routing
 * it through a locale or a local midnight.
 */
function instantFromDate(value: string): string | null {
  return value === '' ? null : `${value}T00:00:00.000Z`;
}

/**
 * The `YYYY-MM-DD` an `Instant` was entered as.
 *
 * The inverse of `instantFromDate` for pre-filling an edit: the instant is the
 * UTC midnight of the day that was chosen (I8), so slicing its date part keeps
 * the day every member sees instead of re-deriving one from a local midnight.
 */
function dateFromInstant(value: string | null): string {
  return value === null ? '' : value.slice(0, 10);
}

/** One write intent: the handle and the profile it was created with. */
type Attempt = {
  readonly operation: WriteOperation;
  readonly profile: RabbitProfile;
};

/** The saved profile an edit starts from, beside the rabbit it belongs to. */
export type RabbitFormInitial = {
  readonly rabbitId: string;
  readonly profile: RabbitProfile;
};

export type RabbitFormProps = {
  /** The active household this rabbit belongs to (RAB-3). */
  householdId: string;
  /**
   * Present when editing an existing rabbit (RAB-5): pre-fills every field,
   * including the weight-drop threshold, and submits `update_rabbit`.
   */
  initial?: RabbitFormInitial;
  /** Returns to the previous surface without writing anything. */
  onCancel: () => void;
  /** The backend confirmed the create or the edit; the caller can present it. */
  onSaved: () => void;
};

export function RabbitForm({
  householdId,
  initial,
  onCancel,
  onSaved,
}: RabbitFormProps) {
  const saved = initial?.profile;

  const [name, setName] = useState(saved?.name ?? '');
  const [breed, setBreed] = useState(saved?.breed ?? '');
  const [dob, setDob] = useState(dateFromInstant(saved?.dob ?? null));
  const [dobUnknown, setDobUnknown] = useState(saved?.dob_unknown ?? false);
  const [sex, setSex] = useState<Sex>(saved?.sex ?? 'unknown');
  const [neuterStatus, setNeuterStatus] = useState<NeuterStatus>(
    saved?.neuter_status ?? 'unknown',
  );
  const [adoptionDate, setAdoptionDate] = useState(
    dateFromInstant(saved?.adoption_date ?? null),
  );
  const [threshold, setThreshold] = useState(
    String(
      saved?.weight_drop_threshold_percent ??
        DEFAULT_WEIGHT_DROP_THRESHOLD_PERCENT,
    ),
  );
  const [prompted, setPrompted] = useState(false);
  const [thresholdPrompted, setThresholdPrompted] = useState(false);
  const attempt = useRef<Attempt | null>(null);
  const closed = useRef(false);
  const formRef = useRef<HTMLFormElement>(null);

  /*
   * Both hooks are mounted, but only the one matching the mode is ever started:
   * a hook cannot be called conditionally, and the create hook's cache policy is
   * wrong for an edit (it would not invalidate the profile or the series).
   */
  const create = useCreateRabbit();
  const update = useUpdateRabbit();
  const write = initial === undefined ? create : update;

  /*
   * The Add control unmounts when this form replaces the list, so focus has to
   * be handed to the new surface rather than left on `<body>`. The first field
   * is the form's own entry point; the dashboard returns focus to the Add
   * control when the form closes.
   */
  useEffect(() => {
    formRef.current?.querySelector('input')?.focus();
  }, []);

  /*
   * The confirmed save, from either surface: the form's submit or the shell's
   * recorded retry. `closed` keeps a re-render with `onSaved` a new function
   * from closing twice. Focus restoration is the caller's follow-up.
   */
  useEffect(() => {
    if (!write.isSuccess || closed.current) {
      return;
    }

    closed.current = true;
    dismissFailedSave();
    onSaved();
  }, [write.isSuccess, onSaved]);

  const failure =
    write.error === null ? null : describeSaveFailure(write.error);
  const validationFailure =
    failure !== null && failure.cause === 'validation' ? failure : null;

  /*
   * The local blank-name prompt is RAB-2's one thing for both modes. A backend
   * refusal is different in an edit: the only reachable one there is "the
   * rabbit is already archived" (D1.5), so the form shows the catalog's
   * validation copy for this cause instead of claiming the name is blank.
   * `describeSaveFailure` deliberately discards the backend's `message_key` and
   * maps the cause to its own entry (`../sync/failure`), so the text is the
   * cause's catalog copy (`error.validation`) rather than a message the command
   * named.
   */
  const fieldError = prompted
    ? t('rabbit.form.blankName')
    : validationFailure === null
      ? undefined
      : initial === undefined
        ? t('rabbit.form.blankName')
        : t(validationFailure.messageKey);

  const pending = write.isPending;

  const sexOptions: Array<SegmentedControlOption<Sex>> = [
    { value: 'male', label: t('rabbit.sex.male') },
    { value: 'female', label: t('rabbit.sex.female') },
    { value: 'unknown', label: t('rabbit.sex.unknown') },
  ];

  const neuterOptions: Array<SegmentedControlOption<NeuterStatus>> = [
    { value: 'intact', label: t('rabbit.neuterStatus.intact') },
    { value: 'neutered', label: t('rabbit.neuterStatus.neutered') },
    { value: 'spayed', label: t('rabbit.neuterStatus.spayed') },
    { value: 'unknown', label: t('rabbit.neuterStatus.unknown') },
  ];

  /**
   * The edit's threshold, or `null` when the field cannot be submitted.
   *
   * A cleared, negative or non-finite value is refused before the write rather
   * than replaced by the saved number: closing on a value the person did not
   * type would report a success that did not happen. `error.validation` is the
   * catalog's cause copy for this, because `design.md` states no threshold bound
   * and therefore no threshold-specific premise exists to word one from.
   */
  function parsedThreshold(): number | null {
    const parsed = Number(threshold);

    return threshold.trim() !== '' && Number.isFinite(parsed) && parsed >= 0
      ? parsed
      : null;
  }

  /** The whole editable set, with unsupplied optionals absent (RAB-1). */
  function profileFor(
    trimmedName: string,
    weightDropThreshold: number,
  ): RabbitProfile {
    const trimmedBreed = breed.trim();

    return {
      name: trimmedName,
      // PHOTO-2 (P2) supplies the picker; an edit must not erase a reference
      // the saved profile already carries (RAB-5, whole-profile replacement).
      photo_ref: saved?.photo_ref ?? null,
      breed: trimmedBreed === '' ? null : trimmedBreed,
      dob: dobUnknown ? null : instantFromDate(dob),
      dob_unknown: dobUnknown,
      sex,
      neuter_status: neuterStatus,
      adoption_date: instantFromDate(adoptionDate),
      weight_drop_threshold_percent: weightDropThreshold,
    };
  }

  function run(operation: WriteOperation) {
    // The confirmed close is the effect above, so the form's own submit and a
    // shell-confirmed retry cannot diverge.
    write.mutate(operation);
  }

  function submit() {
    if (pending) {
      return;
    }

    const trimmedName = name.trim();

    if (trimmedName === '') {
      setPrompted(true);
      write.reset();
      return;
    }

    setPrompted(false);

    /*
     * The edit's threshold is a trust-boundary input: it must not be replaced
     * with a value the person did not type, and it cannot reach the write when
     * it cannot be read. Creation keeps the contract's default (WT-7).
     */
    const thresholdValue =
      initial === undefined
        ? DEFAULT_WEIGHT_DROP_THRESHOLD_PERCENT
        : parsedThreshold();

    if (thresholdValue === null) {
      setThresholdPrompted(true);
      write.reset();
      return;
    }

    setThresholdPrompted(false);

    const profile = profileFor(trimmedName, thresholdValue);
    const last = attempt.current;

    if (
      last !== null &&
      JSON.stringify(last.profile) === JSON.stringify(profile)
    ) {
      run(last.operation);
      return;
    }

    /*
     * A whole-profile replacement on edit (RAB-5): the command's profile is the
     * same field set the detail pre-filled, threshold included.
     */
    const operation =
      initial === undefined
        ? createRabbit(householdId, profile)
        : updateRabbit(initial.rabbitId, profile);

    attempt.current = { operation, profile };
    run(operation);
  }

  function changeName(next: string) {
    setName(next);
    setPrompted(false);

    if (validationFailure !== null) {
      write.reset();
    }
  }

  function changeThreshold(next: string) {
    setThreshold(next);
    setThresholdPrompted(false);
  }

  return (
    <Card
      className="bs-rabbit-form"
      heading={
        initial === undefined
          ? t('rabbit.form.createTitle')
          : t('rabbit.form.editTitle')
      }
    >
      <form
        aria-busy={pending ? true : undefined}
        className="bs-rabbit-form__form"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
        ref={formRef}
      >
        <div className="bs-rabbit-form__photo">
          <span aria-hidden="true" className="bs-rabbit-form__photo-mark">
            <svg
              aria-hidden="true"
              fill="none"
              height="24"
              stroke="currentColor"
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth="1.8"
              viewBox="0 0 24 24"
              width="24"
            >
              <rect height="15" rx="3" width="18" x="3" y="4.5" />
              <circle cx="8.5" cy="9.5" r="1.5" />
              <path d="M4 16.5 8.5 12.5l3.5 3 3-2.5 5 4.5" />
            </svg>
          </span>
          <Text as="span" tone="secondary" variant="footnote">
            {t('rabbit.thumbnailPlaceholder')}
          </Text>
        </div>

        <TextField
          autoComplete="off"
          disabled={pending}
          enterKeyHint="next"
          error={fieldError}
          label={t('rabbit.form.nameLabel')}
          name="rabbit-name"
          onChange={changeName}
          placeholder={t('rabbit.form.namePlaceholder')}
          required
          value={name}
        />

        <TextField
          autoComplete="off"
          disabled={pending}
          enterKeyHint="next"
          label={t('rabbit.form.breedLabel')}
          name="rabbit-breed"
          onChange={setBreed}
          placeholder={t('rabbit.form.breedPlaceholder')}
          value={breed}
        />

        <FormField label={t('rabbit.form.dobLabel')}>
          {(field) => (
            <>
              <input
                aria-describedby={field.describedBy}
                aria-invalid={field.invalid}
                className="bs-text-field"
                disabled={pending || dobUnknown}
                id={field.controlId}
                onChange={(event) => {
                  setDob(event.currentTarget.value);
                }}
                type="date"
                value={dob}
              />
              <label
                className="bs-rabbit-form__unknown"
                htmlFor={`${field.controlId}-unknown`}
              >
                <input
                  checked={dobUnknown}
                  disabled={pending}
                  id={`${field.controlId}-unknown`}
                  onChange={(event) => {
                    const unknown = event.currentTarget.checked;

                    setDobUnknown(unknown);

                    if (unknown) {
                      setDob('');
                    }
                  }}
                  type="checkbox"
                />
                <Text as="span" variant="subheadline">
                  {t('rabbit.form.dobUnknown')}
                </Text>
              </label>
            </>
          )}
        </FormField>

        <FormField label={t('rabbit.sex.label')}>
          {(field) => (
            <SegmentedControl
              describedBy={field.describedBy}
              disabled={pending}
              id={field.controlId}
              invalid={field.invalid}
              labelId={field.labelId}
              name="rabbit-sex"
              onChange={setSex}
              options={sexOptions}
              required={field.required}
              value={sex}
            />
          )}
        </FormField>

        <FormField label={t('rabbit.neuterStatus.label')}>
          {(field) => (
            <SegmentedControl
              describedBy={field.describedBy}
              disabled={pending}
              id={field.controlId}
              invalid={field.invalid}
              labelId={field.labelId}
              name="rabbit-neuter-status"
              onChange={setNeuterStatus}
              options={neuterOptions}
              required={field.required}
              value={neuterStatus}
            />
          )}
        </FormField>

        <FormField label={t('rabbit.form.adoptionDateLabel')}>
          {(field) => (
            <input
              aria-describedby={field.describedBy}
              aria-invalid={field.invalid}
              className="bs-text-field"
              disabled={pending}
              id={field.controlId}
              onChange={(event) => {
                setAdoptionDate(event.currentTarget.value);
              }}
              type="date"
              value={adoptionDate}
            />
          )}
        </FormField>

        {/*
         * RAB-5's threshold is editable once the rabbit exists. Creation keeps
         * the contract default (WT-7), so the field is shown in edit mode only;
         * the labels and help text are the catalog's existing 3.8 copy.
         */}
        {initial === undefined ? null : (
          <TextField
            disabled={pending}
            enterKeyHint="done"
            error={thresholdPrompted ? t('error.validation') : undefined}
            hint={t('rabbit.form.thresholdHelp')}
            inputMode="decimal"
            label={t('rabbit.form.thresholdLabel')}
            name="rabbit-threshold"
            onChange={changeThreshold}
            required
            type="number"
            value={threshold}
          />
        )}

        <div className="bs-rabbit-form__actions">
          <Button disabled={pending} onClick={onCancel} variant="plain">
            {t('common.cancel')}
          </Button>
          <Button loading={pending} type="submit">
            {pending ? t('rabbit.form.saving') : t('rabbit.form.save')}
          </Button>
        </div>
      </form>
    </Card>
  );
}
