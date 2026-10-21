/*
 * First-run household creation (task 4.1; HS-1, HS-2).
 *
 * The screen the entry guard sends a person to when "no household" is a
 * conclusion: a name field, one submit, and nothing else. It creates the
 * household and makes it active through one command (`create_household` does
 * both), then leaves the route to re-decide — the guard reads the book fresh per
 * navigation, which is what makes HS-2's "never again" hold after this succeeds.
 *
 * **One operation per intent (D0.14).** `createHousehold(name)` mints the request
 * id and `run()` sends it. The last attempt's handle is kept beside the name it
 * was created with, and `submit` is the single path for both the primary button
 * and the retry action: a submit carrying that same trimmed name re-runs the
 * stored handle (the same `request_id`), while a corrected name is a new intent
 * with a new handle. There is no second path that could mint a fresh id for the
 * same attempt.
 *
 * **Validation is inline, never a toast.** A blank or whitespace-only name is
 * refused before the command is called, the field is marked invalid and the
 * prompt appears underneath it in the error row, which is a live region
 * (`role="alert"`) — a screen-reader user hears it without focus moving. The
 * backend's `validation` refusal gets the same field treatment. Every other
 * cause is rendered by the shared `SaveFailureNotice`, with a retry only when
 * the failure says it is retryable. There is no optimistic state: the button
 * stays in its loading state until the command answers, and a failure never
 * looks like a created household.
 *
 * **Layout.** One layout for both baselines: compact-first, the form capped at
 * `--layout-form-max` from the first breakpoint up. It is a standalone `Screen`
 * (the route renders it without `AppShell`), so it owns the top and bottom safe
 * areas as well as the keyboard reservation `Screen` already carries.
 */
import { useNavigate } from '@tanstack/react-router';
import { useRef, useState } from 'react';
import { Button } from '../../../components/ui/Button';
import { Heading } from '../../../components/ui/Heading';
import { Screen } from '../../../components/ui/Screen';
import { Text } from '../../../components/ui/Text';
import { TextField } from '../../../components/ui/TextField';
import { t } from '../../i18n';
import { SaveFailureNotice } from '../../sync/components/SaveFailureNotice';
import { describeSaveFailure } from '../../sync/failure';
import { createHousehold } from '../api';
import { useCreateHousehold } from '../queries';
import { useKeyboardInset } from './use-keyboard-inset';

/** One create intent: the handle and the name it was created for. */
type CreateOperation = ReturnType<typeof createHousehold>;

type Attempt = {
  readonly operation: CreateOperation;
  readonly name: string;
};

export function HouseholdOnboardingScreen() {
  // The keyboard reservation is this screen's to assign (Q21) and this screen's
  // alone: nothing else in the app mounts the hook.
  useKeyboardInset();

  const [name, setName] = useState('');
  const [prompted, setPrompted] = useState(false);
  const attempt = useRef<Attempt | null>(null);
  const navigate = useNavigate();

  const creation = useCreateHousehold();

  const failure =
    creation.error === null ? null : describeSaveFailure(creation.error);

  const validationFailure =
    failure !== null && failure.cause === 'validation' ? failure : null;
  const otherFailure =
    failure !== null && failure.cause !== 'validation' ? failure : null;

  /*
   * HS-1's only create refusal is a blank name, so the field says that one thing
   * under its own label — for the client-side prompt and the backend's
   * `validation` refusal alike. The `error.*` copy is reserved for the notice.
   */
  const fieldError =
    prompted || validationFailure !== null
      ? t('household.blankName')
      : undefined;

  /*
   * One path for both "Create household" and "Try again". A submit whose trimmed
   * name is the name the last attempt was created with is a retry of that
   * operation, not a new one: it re-runs the stored handle so the same
   * `request_id` is re-sent (D0.14). Only a changed input — a corrected name —
   * mints a new operation. Without this, pressing the primary button after a
   * retryable failure would re-send an accepted-but-unanswered write under a new
   * id, which the service could deduplicate as nothing.
   *
   * `useCreateHousehold` owns what happens once the backend confirms: it writes
   * the returned household into the active cache and the held set. Navigation is
   * the screen's own follow-up, so it is passed per call — the route re-runs the
   * guard against a fresh read and lands on the dashboard. This screen never
   * renders a "created" state of its own.
   */
  function run(operation: CreateOperation) {
    creation.mutate(operation, {
      onSuccess: () => {
        void navigate({ to: '/' });
      },
    });
  }

  function submit() {
    const trimmed = name.trim();

    if (trimmed === '') {
      setPrompted(true);
      creation.reset();
      return;
    }

    setPrompted(false);

    const last = attempt.current;

    if (last !== null && last.name === trimmed) {
      run(last.operation);
      return;
    }

    const operation = createHousehold(trimmed);
    attempt.current = { operation, name: trimmed };
    run(operation);
  }

  function changeName(next: string) {
    setName(next);
    setPrompted(false);

    if (validationFailure !== null) {
      creation.reset();
    }
  }

  const pending = creation.isPending;

  return (
    <Screen
      as="main"
      className="bs-onboarding"
      data-household-entry="onboarding"
    >
      <header className="bs-onboarding__header">
        <Heading as="h1" variant="large-title">
          {t('onboarding.title')}
        </Heading>
        <Text tone="secondary">{t('onboarding.body')}</Text>
      </header>

      <form
        className="bs-onboarding__form"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <TextField
          autoComplete="off"
          disabled={pending}
          enterKeyHint="done"
          error={fieldError}
          label={t('household.nameLabel')}
          name="household-name"
          onChange={changeName}
          placeholder={t('household.namePlaceholder')}
          required
          value={name}
        />

        {otherFailure === null ? null : (
          <SaveFailureNotice
            error={creation.error}
            onRetry={otherFailure.retryable ? submit : undefined}
          />
        )}

        <Button fullWidth loading={pending} type="submit">
          {pending ? t('household.creating') : t('household.create')}
        </Button>
      </form>
    </Screen>
  );
}
