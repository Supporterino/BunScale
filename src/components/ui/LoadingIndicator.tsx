import { classNames } from './class-names';
import { VisuallyHidden } from './VisuallyHidden';

/*
 * `LoadingIndicator` — a spinner that never becomes the only feedback.
 *
 * `role="status"` with `aria-live="polite"` is the contract: an async state
 * change is announced once, politely, and it is not assertive because a load in
 * progress must not interrupt what the user is reading. When the caller already
 * announces the same state (a `Button` with `aria-busy` and its label still
 * visible), `decorative` drops the live region rather than double-announcing.
 *
 * Motion: the ring rotates through the duration tokens — one revolution is
 * `calc(var(--duration-slow) * 3)` (about 1 s), so reduced motion, which sets
 * every duration to 0 ms, cannot leave a zero-length infinite animation. The
 * rotation is additionally gated behind `prefers-reduced-motion: no-preference`
 * in `src/styles/ui.css`, which leaves a static, still-legible ring plus the
 * label: the token override is the safety net, not the only guard.
 *
 * The ring is `aria-hidden`: it is decoration, and the announcement is the
 * status text.
 */

export type LoadingIndicatorVariant = 'inline' | 'screen';

export type LoadingIndicatorProps = {
  /** Visible label. Without one, `Loading` is announced instead. */
  label?: string;
  variant?: LoadingIndicatorVariant;
  /** True when the caller announces the state itself. */
  decorative?: boolean;
  className?: string;
};

export function LoadingIndicator({
  label,
  variant = 'inline',
  decorative = false,
  className,
}: LoadingIndicatorProps) {
  const visibleLabel =
    label === undefined ? null : (
      <span className="bs-loading-indicator__label">{label}</span>
    );
  const hiddenLabel =
    decorative || label !== undefined ? null : (
      <VisuallyHidden>Loading</VisuallyHidden>
    );
  const ring = (
    <span aria-hidden="true" className="bs-loading-indicator__ring" />
  );
  const classes = classNames(
    'bs-loading-indicator',
    `bs-loading-indicator--${variant}`,
    className,
  );

  if (decorative) {
    return (
      <span aria-hidden="true" className={classes}>
        {ring}
        {visibleLabel}
      </span>
    );
  }

  return (
    <span aria-live="polite" className={classes} role="status">
      {ring}
      {visibleLabel}
      {hiddenLabel}
    </span>
  );
}
