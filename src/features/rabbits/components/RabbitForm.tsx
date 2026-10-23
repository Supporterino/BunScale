/*
 * Rabbit creation (task 4.3; RAB-1, RAB-2, RAB-4; D1.11, D1.14).
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
 * **The photo field is carried, not collected.** Choosing a photo is PHOTO-2 in
 * P2 and brings its own capability (D1.11), so P1 has no picker. The form
 * therefore renders RAB-1's placeholder and submits `photo_ref: null`; the
 * field is still part of the profile, and a later photo flow fills the same
 * value without changing this shape.
 *
 * **No `Sheet`/`Dialog`.** The design system defers both, and P0's own form
 * pattern is an inline one (Settings' "create another household" form). This
 * form is rendered in place of the dashboard list, so it needs no new primitive
 * and the adaptive frame is unchanged.
 *
 * **Blank names never reach the command.** RAB-2 treats an empty name and a
 * whitespace-only name as the same refusal: the form trims first, shows
 * `rabbit.form.blankName` under the field and returns before `createRabbit` is
 * called. A backend `validation` refusal gets the same field treatment; every
 * other cause is already recorded by `useCreateRabbit` (task 3.4) for P0's shell
 * surface, so this component deliberately does not call `recordFailedSave` a
 * second time.
 *
 * **One operation per intent (D0.14).** The last handle is kept beside the
 * profile it was created with. Submitting the unchanged profile re-runs that
 * handle (the same `request_id`); any changed value is a new intent with a new
 * handle.
 *
 * **A confirmed save closes the form, whichever surface confirmed it.** The
 * form's own submit and the shell's `retryFailedSave()` reach the same mutation
 * instance (the recording retry re-enters `useCreateRabbit`), so
 * `creation.isSuccess` is the one signal that covers both. Closing from that
 * signal rather than from a per-call `onSuccess` is what keeps a shell-confirmed
 * retry from leaving the form open on a rabbit that already exists; it also
 * clears an earlier failure record, so the shell does not keep presenting a
 * failure this save has superseded.
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
import { createRabbit } from '../api';
import { useCreateRabbit } from '../mutations';

/**
 * `RabbitProfile` is derived from the wrapper that takes it rather than
 * imported from `src/bindings.ts`, because only `api.ts` may import the
 * generated bindings (project hard rule 2).
 */
type RabbitProfile = Parameters<typeof createRabbit>[1];

type Sex = RabbitProfile['sex'];
type NeuterStatus = RabbitProfile['neuter_status'];
type CreateOperation = ReturnType<typeof createRabbit>;

/** The contract's documented default threshold (WT-7; design.md I7). */
const DEFAULT_WEIGHT_DROP_THRESHOLD_PERCENT = 5;

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

/** One create intent: the handle and the profile it was created with. */
type Attempt = {
  readonly operation: CreateOperation;
  readonly profile: RabbitProfile;
};

export type RabbitFormProps = {
  /** The active household this rabbit belongs to (RAB-3). */
  householdId: string;
  /** Returns to the dashboard without creating anything. */
  onCancel: () => void;
  /** The backend confirmed the new rabbit; the dashboard can present it. */
  onCreated: () => void;
};

export function RabbitForm({
  householdId,
  onCancel,
  onCreated,
}: RabbitFormProps) {
  const [name, setName] = useState('');
  const [breed, setBreed] = useState('');
  const [dob, setDob] = useState('');
  const [dobUnknown, setDobUnknown] = useState(false);
  const [sex, setSex] = useState<Sex>('unknown');
  const [neuterStatus, setNeuterStatus] = useState<NeuterStatus>('unknown');
  const [adoptionDate, setAdoptionDate] = useState('');
  const [prompted, setPrompted] = useState(false);
  const attempt = useRef<Attempt | null>(null);
  const closed = useRef(false);
  const formRef = useRef<HTMLFormElement>(null);

  const creation = useCreateRabbit();

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
   * recorded retry. `closed` keeps a re-render with `onCreated` a new function
   * from closing twice. Focus restoration is the dashboard's follow-up.
   */
  useEffect(() => {
    if (!creation.isSuccess || closed.current) {
      return;
    }

    closed.current = true;
    dismissFailedSave();
    onCreated();
  }, [creation.isSuccess, onCreated]);

  const failure =
    creation.error === null ? null : describeSaveFailure(creation.error);
  const validationFailure =
    failure !== null && failure.cause === 'validation' ? failure : null;

  /*
   * The reachable `validation` refusal is I8's contradiction or a blank name,
   * and this form cannot produce the first, so the field says RAB-2's one thing
   * for the prompt and the backend refusal alike.
   */
  const fieldError =
    prompted || validationFailure !== null
      ? t('rabbit.form.blankName')
      : undefined;

  const pending = creation.isPending;

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

  /** The whole editable set, with unsupplied optionals absent (RAB-1). */
  function profileFor(trimmedName: string): RabbitProfile {
    const trimmedBreed = breed.trim();

    return {
      name: trimmedName,
      // PHOTO-2 (P2) supplies the picker; P1 carries the field as absent.
      photo_ref: null,
      breed: trimmedBreed === '' ? null : trimmedBreed,
      dob: dobUnknown ? null : instantFromDate(dob),
      dob_unknown: dobUnknown,
      sex,
      neuter_status: neuterStatus,
      adoption_date: instantFromDate(adoptionDate),
      weight_drop_threshold_percent: DEFAULT_WEIGHT_DROP_THRESHOLD_PERCENT,
    };
  }

  function run(operation: CreateOperation) {
    // The confirmed close is the effect above, so the form's own submit and a
    // shell-confirmed retry cannot diverge.
    creation.mutate(operation);
  }

  function submit() {
    if (pending) {
      return;
    }

    const trimmedName = name.trim();

    if (trimmedName === '') {
      setPrompted(true);
      creation.reset();
      return;
    }

    setPrompted(false);

    const profile = profileFor(trimmedName);
    const last = attempt.current;

    if (
      last !== null &&
      JSON.stringify(last.profile) === JSON.stringify(profile)
    ) {
      run(last.operation);
      return;
    }

    const operation = createRabbit(householdId, profile);
    attempt.current = { operation, profile };
    run(operation);
  }

  function changeName(next: string) {
    setName(next);
    setPrompted(false);

    if (validationFailure !== null) {
      creation.reset();
    }
  }

  return (
    <Card className="bs-rabbit-form" heading={t('rabbit.form.createTitle')}>
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
