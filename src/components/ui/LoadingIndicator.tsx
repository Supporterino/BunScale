import { classNames } from './class-names';

/*
 * `LoadingIndicator` — a spinner that never becomes the only feedback.
 *
 * **The announced copy is the caller's, and the type says so (task 7.6, Q48).**
 * `role="status"` with `aria-live="polite"` is the contract: an async state
 * change is announced once, politely, and it is not assertive because a load in
 * progress must not interrupt what the user is reading. What that region *says*
 * used to have a fallback: with no `label`, the primitive announced a
 * hard-coded English `Loading`, which put a P0 string outside
 * `src/features/i18n` where no translator and no catalog test can reach it.
 * The fallback is gone, and the props are a discriminated union so an
 * unlabelled announcement cannot be written:
 *
 *   - `{ decorative: true }` — the caller announces the state itself (a `Button`
 *     that exposes `aria-busy` with its label still visible, a row already
 *     named by its own text). The wrapper is `aria-hidden` with no role and no
 *     text, so it is never a second announcement. It takes no `label` either: a
 *     visible label that assistive tech cannot see is a trap, and a decorative
 *     spinner has no copy to show.
 *   - `{ label: string }` — the ring announces this text. The same string is
 *     rendered visibly and is the live region's content, so what is shown and
 *     what is spoken cannot drift apart.
 *
 * The label is a plain string, not a `messageKey` resolved inside the
 * primitive. `src/components/ui/` is the shared layer; `src/features/i18n` is a
 * feature, so a `messageKey` prop would make the design system import a feature
 * module and hide a catalog lookup in a primitive. Every call site already
 * imports `t`, so it passes `label={t('…')}` and the key it chose stays visible
 * in the caller's own diff — which is also what makes the copy composable with
 * data later without changing this component.
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

/** What both forms share: where the ring sits, not what it says. */
type LoadingIndicatorAppearance = {
  variant?: LoadingIndicatorVariant;
  className?: string;
};

/**
 * The two forms, and nothing between them: an announcing ring needs copy, and a
 * decorative one cannot carry any.
 */
export type LoadingIndicatorAnnouncement =
  | {
      /** The caller announces the state itself; this ring is decoration. */
      decorative: true;
      /** A decorative ring shows no copy: its caller already owns the words. */
      label?: never;
    }
  | {
      decorative?: false;
      /**
       * The copy announced through the polite status — catalogued text from the
       * caller, e.g. `t('entry.loading')`. Required: see the header.
       */
      label: string;
    };

export type LoadingIndicatorProps = LoadingIndicatorAppearance &
  LoadingIndicatorAnnouncement;

export function LoadingIndicator(props: LoadingIndicatorProps) {
  const { variant = 'inline', className } = props;
  const classes = classNames(
    'bs-loading-indicator',
    `bs-loading-indicator--${variant}`,
    className,
  );
  const ring = (
    <span aria-hidden="true" className="bs-loading-indicator__ring" />
  );

  if (props.decorative) {
    return (
      <span aria-hidden="true" className={classes}>
        {ring}
      </span>
    );
  }

  return (
    <span aria-live="polite" className={classes} role="status">
      {ring}
      <span className="bs-loading-indicator__label">{props.label}</span>
    </span>
  );
}
