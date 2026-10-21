/*
 * The store-backed save-failure mount (task 7.7; SYNC-2, NFR-8, D0.14).
 *
 * `src/features/sync/save-retry.ts` records a failed write that outlives the
 * form that started it. This is the one place that record is presented: the
 * route shell mounts it above `<Outlet />`, so a failure recorded on a screen
 * that is later replaced is still visible, and the retry it offers re-runs the
 * recorded operation handle (whose `request_id` is the attempt's identity —
 * D0.14).
 *
 * **`SaveFailureNotice` stays the presentation.** The store's record already
 * carries the described failure, and the notice derives its copy from that
 * same value, so passing it through keeps one component and one wording: the
 * `Not saved` header, the cause from the cause map, and a retry only when the
 * backend marks it retryable. Nothing here can make a failed save look applied.
 *
 * **No dismissal, for now.** The create flow this serves has always offered a
 * retry for a retryable cause and nothing else, and the store's dismissal is
 * the explicit "give up" action that flow never had. Keeping the surface
 * actionless for a non-retryable cause preserves that behaviour; the record is
 * cleared by a later successful create, not by this mount. During a retry the
 * retry **stays**, in the `Button`'s `loading` form (task 7.9): the store still
 * ignores a second attempt, so `retrying` is passed down instead of withdrawing
 * `onRetry`, and the control keeps its node, its label and its place while the
 * attempt is in flight. The alternative — `onRetry={undefined}` while busy —
 * unmounted the button the person had just pressed.
 *
 * This file deliberately owns no layout: the shell-level placement is the
 * className hook (`bs-save-failure`), styled at both baselines by
 * `src/styles/screens.css` (task 7.9). That rule anchors the surface to the
 * **bottom** edge — above the tab bar on a shell route, above the safe area on
 * a standalone one — because P0's only producer is this flow's onboarding
 * screen and its header has to stay visible while the failure is reported.
 */
import { retryFailedSave, useSaveRetry } from '../save-retry';
import { SaveFailureNotice } from './SaveFailureNotice';

export function SaveFailureSurface() {
  const { failure, retrying } = useSaveRetry();

  if (failure === null) {
    return null;
  }

  const onRetry = () => {
    void retryFailedSave();
  };

  return (
    <SaveFailureNotice
      className="bs-save-failure"
      error={failure.failure}
      onRetry={failure.retryable ? onRetry : undefined}
      retrying={retrying}
    />
  );
}
