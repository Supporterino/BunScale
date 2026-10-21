import type { ReactNode } from 'react';
import { Button } from './Button';
import { classNames } from './class-names';
import { Heading } from './Heading';

/*
 * `ErrorState` — "nothing was ever loaded, and the service is unreachable".
 *
 * It is deliberately a different primitive from `EmptyState`, because the two
 * mean different things and ask for different actions: an empty collection is a
 * normal state with a way forward, a failed load is a failure with a cause. The
 * copy names the cause (offline, the service unreachable, a rejected request),
 * and never "something went wrong".
 *
 * `role="alert"` is the default because this surface appears in response to an
 * action or a failed load, and an assertive announcement is what makes it
 * impossible to miss. It stays a real heading plus body text, the retry is a
 * real `Button` (44 pt, keyboard operable, with a busy state of its own), and
 * the request id is offered as secondary copy — available for support, never the
 * headline, and never the only thing the user is told.
 */

export type ErrorStateProps = {
  title: string;
  children: ReactNode;
  /** Renders the retry control. Omit when the failure is not retryable. */
  onRetry?: () => void;
  retryLabel?: string;
  /** Diagnostic reference for support; shown small, under the action. */
  requestId?: string;
  /**
   * Heading level for the title. `h2` is the default inside a screen whose
   * navigation bar carries the `h1`; a standalone surface (the entry route's
   * error component) passes `h1` so the outline does not start at `h2`.
   */
  headingLevel?: 'h1' | 'h2';
  className?: string;
};

export function ErrorState({
  title,
  children,
  onRetry,
  retryLabel = 'Try again',
  requestId,
  headingLevel = 'h2',
  className,
}: ErrorStateProps) {
  return (
    <section className={classNames('bs-error-state', className)} role="alert">
      <span aria-hidden="true" className="bs-error-state__icon">
        <svg
          aria-hidden="true"
          fill="none"
          height="48"
          stroke="currentColor"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="2"
          viewBox="0 0 48 48"
          width="48"
        >
          <path d="M24 6 44 41H4z" />
          <line x1="24" x2="24" y1="19" y2="29" />
          <line x1="24" x2="24" y1="35" y2="35" />
        </svg>
      </span>
      <Heading
        as={headingLevel}
        className="bs-error-state__title"
        variant="title-2"
      >
        {title}
      </Heading>
      <p className="bs-error-state__body">{children}</p>
      {onRetry === undefined ? null : (
        <div className="bs-error-state__action">
          <Button onClick={onRetry} variant="secondary">
            {retryLabel}
          </Button>
        </div>
      )}
      {requestId === undefined ? null : (
        <p className="bs-error-state__reference">Reference: {requestId}</p>
      )}
    </section>
  );
}
