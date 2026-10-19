import type { ReactNode } from 'react';

/*
 * `VisuallyHidden` — screen-reader-only text that is never invisible to
 * assistive technology.
 *
 * It exists for the two cases the accessibility contract names: an
 * `IconButton` whose design has no room for a visible label (the label prop is
 * preferred, and this is the escape hatch), and the text of a live region that
 * is announced but not drawn.
 *
 * It uses the clip-rect technique rather than `display: none` or
 * `visibility: hidden`, both of which remove the text from the accessibility
 * tree, and it is never used to hide focusable content: a focusable element
 * that cannot be seen is a keyboard trap, and the focus ring would be invisible
 * too.
 */
export type VisuallyHiddenProps = {
  children: ReactNode;
};

export function VisuallyHidden({ children }: VisuallyHiddenProps) {
  return <span className="bs-visually-hidden">{children}</span>;
}
