/*
 * The save-failure surface (task 4.5; SYNC-2, NFR-8).
 *
 * `SaveFailureNotice` is the one presentational component, driven by the cause
 * map: `describeSaveFailure(error)` picks the copy key, so a failure is never
 * "something went wrong", never optimistic and never success-shaped. The state
 * header is `error.notSaved`; the action is the retry when the backend marks the
 * cause retryable, and dismissal otherwise. A caller that offers neither gets the
 * copy only — it still does not look saved.
 *
 * **Where it is used.** The screens that own a mutation's failure render it
 * inline: 4.1's create form, and 4.3's create form and display-unit change. The
 * switch keeps its own `Notice` because its validation refusal has screen-specific
 * copy and a re-read action rather than the cause map.
 *
 * **What is deliberately not here.** A store-wired wrapper around
 * `src/features/sync/save-retry.ts`. Nothing in production calls
 * `recordFailedSave` yet, so mounting that surface would create a second,
 * invisible mechanism for the same failure. The store and its tested handle
 * discipline stay in place; the task that introduces a recorder owns adding the
 * wrapper and mounting it once.
 */
import { Button } from '../../../components/ui/Button';
import { Notice } from '../../../components/ui/Notice';
import { t } from '../../i18n';
import { describeSaveFailure } from '../failure';

export type SaveFailureNoticeProps = {
  /** The raw failure value a mutation or command produced. */
  error: unknown;
  onRetry?: () => void;
  onDismiss?: () => void;
  className?: string;
};

export function SaveFailureNotice({
  error,
  onRetry,
  onDismiss,
  className,
}: SaveFailureNoticeProps) {
  const failure = describeSaveFailure(error);
  const showRetry = failure.retryable && onRetry !== undefined;

  const action = showRetry ? (
    <Button onClick={onRetry} size="compact" variant="secondary">
      {t('error.retry')}
    </Button>
  ) : onDismiss === undefined ? undefined : (
    <Button onClick={onDismiss} size="compact" variant="secondary">
      {t('error.dismiss')}
    </Button>
  );

  return (
    <Notice
      action={action}
      className={className}
      title={t('error.notSaved')}
      tone="danger"
    >
      {t(failure.messageKey)}
    </Notice>
  );
}
