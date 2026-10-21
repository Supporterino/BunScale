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
 * **A retry in flight keeps the control (task 7.9).** `retrying` is handed to
 * the same `Button` as its `loading` form rather than toggling `onRetry` off:
 * the label stays, `aria-busy` says the work is running, and the Button's own
 * contract disables it so a second attempt cannot land. Withdrawing the button
 * instead would unmount the node the person just pressed and leave the alert
 * with a different layout under their hand.
 *
 * **One live region, not two.** The `Button`'s busy state adds none:
 * `LoadingIndicator` is rendered `decorative`, which is `aria-hidden` with no
 * `role` and no `aria-live`, and `aria-busy` is exposed state rather than a
 * region. The notice's own `role="alert"` (from `Notice`'s danger tone) is the
 * single announcement, so the in-flight change cannot be spoken twice. Keeping
 * that call site decorative is part of this contract: `LoadingIndicator` has no
 * unlabelled announcing form (task 7.6), so the `Button` stays `decorative` and
 * the alert remains the single announcement.
 *
 * **Where it is used.** Three render sites, all live, and no others. The route
 * shell presents the store's record through `SaveFailureSurface`, with
 * `bs-save-failure` as its placement hook (`src/styles/screens.css`); 4.3's
 * Settings screen renders the notice inline for its create form and for the
 * display-unit change, which is the pattern the shell mount generalises. The
 * create flow (`HouseholdOnboarding`) does not render it at all: it hands
 * every non-`validation` failure to `recordFailedSave` — the store's first
 * production caller — and the shell presents that record. The switch keeps its
 * own `Notice` because its validation refusal has screen-specific copy and a
 * re-read action rather than the cause map.
 *
 * **`validation` is never the shell's.** A `validation` refusal belongs to the
 * field that owns it, so the create flow keeps `validation` out of the record
 * (it records only the other causes) and the store's retry path clears it
 * (`settleRetryFailure`). The shell presenting it would put the same refusal on
 * screen twice, once without a field to fix.
 *
 * **What is deliberately not here.** The store itself
 * (`src/features/sync/save-retry.ts`) and the shell mount; this file stays
 * driven by props so both an inline form and the shell can use it without a
 * second wording.
 */
import { Button } from '../../../components/ui/Button';
import { Notice } from '../../../components/ui/Notice';
import { t } from '../../i18n';
import { describeSaveFailure } from '../failure';

export type SaveFailureNoticeProps = {
  /** The raw failure value a mutation or command produced. */
  error: unknown;
  onRetry?: () => void;
  /**
   * True while the retry `onRetry` started is in flight.
   *
   * Ignored unless a retry is on screen: the non-retryable cause stays copy-only
   * (or dismissal-only) either way. The control is not withdrawn while busy —
   * see the header.
   */
  retrying?: boolean;
  onDismiss?: () => void;
  className?: string;
};

export function SaveFailureNotice({
  error,
  onRetry,
  retrying = false,
  onDismiss,
  className,
}: SaveFailureNoticeProps) {
  const failure = describeSaveFailure(error);
  const showRetry = failure.retryable && onRetry !== undefined;

  const action = showRetry ? (
    <Button
      loading={retrying}
      onClick={onRetry}
      size="compact"
      variant="secondary"
    >
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
