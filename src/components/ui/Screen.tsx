import type { ReactNode } from 'react';
import { classNames } from './class-names';

/*
 * `Screen` — the scroll container of every screen.
 *
 * It reserves the space the fixed bars and the software keyboard take, so no
 * content or control ends up under the tab bar, the home indicator or the
 * keyboard (`semantic.css` states the same contract at token level):
 *
 *   - `padding-block-end` adds `--keyboard-inset`, which the app shell will set
 *     once from `visualViewport`. No `visualViewport` measurement exists in
 *     `src/` yet, so the token resolves to `0px` today and assigning it is
 *     deferred to the first real form screen (task 4.1) — see `semantic.css`.
 *     The reservation is here so that task only has to assign; no component
 *     measures the keyboard itself, so nothing else has to change.
 *   - `scroll-padding-block-end` reserves the tab bar, so a focused control
 *     scrolled into view is not scrolled *under* the bar.
 *   - `overscroll-behavior: contain` stops the scroll chaining that would
 *     otherwise rubber-band the document on iOS.
 *
 * `--layout-viewport-height` is the iOS 15 floor's `100vh` with a `100dvh`
 * upgrade (defined in `semantic.css`), so the height is right before the
 * dynamic-viewport APIs exist.
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
};

export function Screen({
  children,
  variant = 'scroll',
  constrain = true,
  as: Element = 'div',
  className,
  id,
}: ScreenProps) {
  return (
    <Element
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
