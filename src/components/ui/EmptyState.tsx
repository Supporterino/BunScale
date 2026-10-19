import type { ReactNode } from 'react';
import { classNames } from './class-names';
import { Heading } from './Heading';

/*
 * `EmptyState` — "there is nothing here yet", which is not an error.
 *
 * It states the situation in a real heading, explains it in a sentence, and
 * offers at most one clear action; the same shape is used for "no households
 * yet" on the dashboard. The icon is decorative and the caller supplies it.
 *
 * It is announced only when it replaces content after an action (the caller
 * decides that, by keeping it out of a live region when it is the initial state
 * of a screen), which is why no `role` is set here: a status region that mounts
 * with the screen announces nothing useful and would talk over the screen title.
 */

export type EmptyStateProps = {
  title: string;
  children: ReactNode;
  icon?: ReactNode;
  action?: ReactNode;
  className?: string;
};

export function EmptyState({
  title,
  children,
  icon,
  action,
  className,
}: EmptyStateProps) {
  return (
    <section className={classNames('bs-empty-state', className)}>
      {icon === undefined ? null : (
        <span aria-hidden="true" className="bs-empty-state__icon">
          {icon}
        </span>
      )}
      <Heading className="bs-empty-state__title" variant="title-2">
        {title}
      </Heading>
      <p className="bs-empty-state__body">{children}</p>
      {action === undefined ? null : (
        <div className="bs-empty-state__action">{action}</div>
      )}
    </section>
  );
}
