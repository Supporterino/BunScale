import type { ReactNode } from 'react';
import { classNames } from './class-names';

/*
 * `NavigationBar` — the screen's title bar: the screen's `h1`, an optional
 * leading action (Back) and up to two trailing actions.
 *
 * Details that matter:
 *   - the title is a real `h1` inside a `<header>`, so the screen's outline
 *     starts here and VoiceOver's heading rotor can reach the screen title from
 *     anywhere in the scroll region;
 *   - the bar is a minimum height, never a fixed one: at a larger preferred text
 *     size the title wraps and the bar grows instead of clipping it
 *     (`Dynamic Type through --text-scale`, global convention 11);
 *   - it carries the top safe area itself (`--safe-area-inset-top`), because it
 *     is the topmost bar in both arrangements — the sidebar reserves the same
 *     inset on the other side of the split;
 *   - each action slot holds an `IconButton` (44 x 44, required label). The
 *     primitive does not create controls itself, so the caller decides whether
 *     the back control is `router.history.back()` or an explicit parent route,
 *     as the router conventions require.
 */

export type NavigationBarProps = {
  title: string;
  /** Set when another element labels the title (for example a `main`). */
  titleId?: string;
  /** Leading action slot; the Back control belongs here. */
  leadingAction?: ReactNode;
  /** Trailing actions, right-aligned; one or two fit a 390 pt bar. */
  trailingActions?: ReactNode;
  className?: string;
};

export function NavigationBar({
  title,
  titleId,
  leadingAction,
  trailingActions,
  className,
}: NavigationBarProps) {
  return (
    <header className={classNames('bs-nav-bar', className)}>
      <div className="bs-nav-bar__leading">{leadingAction}</div>
      <h1 className="bs-nav-bar__title" id={titleId}>
        {title}
      </h1>
      <div className="bs-nav-bar__trailing">{trailingActions}</div>
    </header>
  );
}
