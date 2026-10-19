import type { ReactNode } from 'react';
import { classNames } from './class-names';

/*
 * `Badge` — a small non-interactive status label: the connection state, the
 * weight unit.
 *
 * The label is always text, and the tone is always a second signal: the contract
 * forbids a bare coloured dot, because colour alone is not a message (and a dot
 * is not a label at all for a screen reader). A badge is not a notice and does
 * not replace one — a badge states a condition, a `Notice` explains it and says
 * what to do about it.
 *
 * Live announcements are deliberately not its job: a badge that changes while
 * the user is reading is announced by the `Notice` or the status region that
 * describes the change, not by the badge silently mutating. It stays
 * non-interactive, so it never becomes a focus stop.
 */

export type BadgeTone = 'neutral' | 'success' | 'warning' | 'danger' | 'info';

export type BadgeProps = {
  /** The label. Must be text — never a dot or an icon on its own. */
  children: ReactNode;
  tone?: BadgeTone;
  className?: string;
};

export function Badge({ children, tone = 'neutral', className }: BadgeProps) {
  return (
    <span className={classNames('bs-badge', `bs-badge--${tone}`, className)}>
      {children}
    </span>
  );
}
