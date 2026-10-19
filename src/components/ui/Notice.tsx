import type { ReactNode } from 'react';
import { classNames } from './class-names';
import { Heading } from './Heading';

/*
 * `Notice` — an inline, persistent message: offline, a save failure, a success.
 *
 * The tone decides the announcement role, which is the part that is easy to get
 * wrong:
 *
 *   - `danger` is a failed save, so it is `role="alert"` — assertive, because
 *     the user must know before doing anything else;
 *   - the other three are `role="status"` — polite, so an offline notice does
 *     not interrupt a sentence being read out.
 *
 * It never auto-dismisses and there is no close button: a toast that disappears
 * before it is read is not an accessible way to report a failure, and the
 * state it reports persists until the cause is resolved. The copy is the
 * caller's, and it must be cause-specific — "something went wrong" tells nobody
 * anything.
 *
 * The icon is decorative (`aria-hidden`); the title is a real heading, so the
 * notice is reachable in the reading order, and the action is whatever control
 * the caller passes (a `Button` to retry, a `Link` to the settings screen).
 */

export type NoticeTone = 'info' | 'success' | 'warning' | 'danger';

export type NoticeProps = {
  children: ReactNode;
  tone?: NoticeTone;
  /** Optional title line; rendered as a real heading. */
  title?: string;
  /** Optional action: a retry button, a link. */
  action?: ReactNode;
  className?: string;
};

export function Notice({
  children,
  tone = 'info',
  title,
  action,
  className,
}: NoticeProps) {
  const role = tone === 'danger' ? 'alert' : 'status';

  return (
    <div
      className={classNames('bs-notice', `bs-notice--${tone}`, className)}
      role={role}
    >
      <span aria-hidden="true" className="bs-notice__icon">
        <NoticeIcon tone={tone} />
      </span>
      <div className="bs-notice__body">
        {title === undefined ? null : (
          <Heading className="bs-notice__title" variant="title-3" as="h2">
            {title}
          </Heading>
        )}
        <p className="bs-notice__text">{children}</p>
        {action === undefined ? null : (
          <div className="bs-notice__action">{action}</div>
        )}
      </div>
    </div>
  );
}

function NoticeIcon({ tone }: { tone: NoticeTone }) {
  if (tone === 'success') {
    return (
      <svg
        aria-hidden="true"
        fill="none"
        height="20"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
        viewBox="0 0 20 20"
        width="20"
      >
        <circle cx="10" cy="10" r="8" />
        <polyline points="6.5,10.5 9,13 13.5,7.5" />
      </svg>
    );
  }

  if (tone === 'danger') {
    return (
      <svg
        aria-hidden="true"
        fill="none"
        height="20"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
        viewBox="0 0 20 20"
        width="20"
      >
        <circle cx="10" cy="10" r="8" />
        <line x1="10" x2="10" y1="6" y2="11" />
        <line x1="10" x2="10" y1="14" y2="14" />
      </svg>
    );
  }

  if (tone === 'warning') {
    return (
      <svg
        aria-hidden="true"
        fill="none"
        height="20"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
        viewBox="0 0 20 20"
        width="20"
      >
        <path d="M10 2.5 18 17H2z" />
        <line x1="10" x2="10" y1="8" y2="12" />
        <line x1="10" x2="10" y1="14.5" y2="14.5" />
      </svg>
    );
  }

  return (
    <svg
      aria-hidden="true"
      fill="none"
      height="20"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="2"
      viewBox="0 0 20 20"
      width="20"
    >
      <circle cx="10" cy="10" r="8" />
      <line x1="10" x2="10" y1="9" y2="14" />
      <line x1="10" x2="10" y1="6" y2="6" />
    </svg>
  );
}
