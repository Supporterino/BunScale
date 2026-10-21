import type { HTMLAttributes, ReactNode } from 'react';
import { classNames } from './class-names';

/*
 * `Screen` — the scroll container of every screen.
 *
 * It reserves the space the fixed bars and the software keyboard take, so no
 * content or control ends up under the tab bar, the home indicator or the
 * keyboard (`semantic.css` states the same contract at token level):
 *
 *   - `padding-block-end` adds `--keyboard-inset`. The first real form screen
 *     (task 4.1, first-run household creation) assigns it from
 *     `visualViewport` through its own hook; `Screen` itself still measures
 *     nothing. On every other surface the token keeps its defined `0px`, so the
 *     reservation costs nothing until a keyboard is reported — see
 *     `semantic.css`.
 *   - `scroll-padding-block-end` reserves the tab bar, so a focused control
 *     scrolled into view is not scrolled *under* the bar.
 *   - `overscroll-behavior: contain` stops the scroll chaining that would
 *     otherwise rubber-band the document on iOS.
 *
 * `--layout-viewport-height` is `100vh` with a deliberate progressive `100dvh`
 * upgrade (defined in `semantic.css` under `@supports`). `100dvh` is inside the
 * iOS 16.4 / macOS 13.3 floor (design.md D0.16, Q16), so the upgrade is a
 * choice rather than a capability gap: the base value always resolves, and the
 * dynamic viewport wins where it exists.
 *
 * `variant="fixed"` is for a screen that lays itself out (a form pinned above
 * the keyboard); it clips instead of scrolling. The content column is capped at
 * `--layout-content-max` and centred, which is what makes the same screen read
 * as a phone screen at 390 px and as a readable window at 1280 px.
 */

export type ScreenVariant = 'scroll' | 'fixed';

export type ScreenProps = {
  children: ReactNode;
  variant?: ScreenVariant;
  /**
   * Renders the `--layout-content-max` column. Defaults to `true`.
   */
  constrain?: boolean;
  /**
   * Element to render. `main` is for a screen that stands alone — the root
   * not-found surface, which has no `AppShell` around it — so the screen is
   * still a landmark with one `main` and a skip-link target of its own.
   */
  as?: 'div' | 'main';
  className?: string;
  id?: string;
} & Omit<HTMLAttributes<HTMLElement>, 'children' | 'className' | 'id'> & {
    /*
     * `data-*` markers reach the element as well. A screen needs one occasionally
     * — the household entry guard publishes `data-household-entry` on its
     * `main`, which is the stable name of the decision its specs assert — and
     * without this index signature a feature would either fork `Screen` or wrap
     * a second landmark around it. Only `data-*` passes through; the rest of the
     * element's props are the declared ones above.
     */
    [key: `data-${string}`]: string | undefined;
  };

export function Screen({
  children,
  variant = 'scroll',
  constrain = true,
  as: Element = 'div',
  className,
  id,
  ...elementProps
}: ScreenProps) {
  return (
    <Element
      {...elementProps}
      className={classNames('bs-screen', `bs-screen--${variant}`, className)}
      id={id}
    >
      <div
        className={classNames(
          'bs-screen__content',
          constrain ? null : 'bs-screen__content--full',
        )}
      >
        {children}
      </div>
    </Element>
  );
}
