import type { ReactNode } from 'react';
import { classNames } from './class-names';

/*
 * `List` — the grouped surface of iOS settings-style screens: a rounded container
 * of `ListRow`s.
 *
 * It is a real `<ul>`, and its children are real `<li>`s, because the rows are a
 * list of things whether or not the browser draws bullets (`src/styles/base.css`
 * resets the markers, not the semantics). Grouping and naming come from
 * `ListSection`, which owns the heading.
 *
 * Nothing about a row is swipe-only and nothing is discovered by hover: a
 * navigational row is an `Link` and an actionable one is a `<button>` (see
 * `./ListRow.tsx`), so every row is operable by tap, by keyboard and by
 * assistive technology. Separators between rows are drawn by the stylesheet as
 * pseudo-elements, which keeps them decorative and out of the accessibility
 * tree.
 */

export type ListProps = {
  children: ReactNode;
  className?: string;
};

export function List({ children, className }: ListProps) {
  return <ul className={classNames('bs-list', className)}>{children}</ul>;
}
