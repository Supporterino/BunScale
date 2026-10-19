import type { ReactNode } from 'react';
import { VisuallyHidden } from './VisuallyHidden';

/*
 * Internal markup shared by the two labelled-control primitives (`FormField`,
 * `TextField`).
 *
 * This file is NOT part of the design-system inventory: no screen composes it
 * directly. It exists so the label, hint and error rows are drawn in exactly one
 * place — the two primitives cannot drift in their class names, and neither can
 * forget the two accessibility details that are easy to lose:
 *
 *   - `required` is announced, not only painted: the asterisk is `aria-hidden`
 *     and a screen-reader-only " (required)" follows it;
 *   - an error is announced as an error, not only coloured: the message keeps
 *     its cause-specific text and gains a screen-reader-only "Error: " prefix.
 *
 * `FieldLabel` always renders a real `<label>` with `htmlFor`, which is what the
 * inventory requires. A control that cannot be labelled by `for` — a
 * `radiogroup`, for example — reuses the same element's `id` as the value of its
 * `aria-labelledby`, so the `for` reference always points at an element that
 * exists even when the association itself is done by `aria-labelledby`.
 */

type FieldLabelProps = {
  children: ReactNode;
  htmlFor: string;
  id: string;
  required?: boolean;
};

export function FieldLabel({
  children,
  htmlFor,
  id,
  required = false,
}: FieldLabelProps) {
  return (
    <label className="bs-field__label" htmlFor={htmlFor} id={id}>
      {children}
      {required ? (
        <>
          <span aria-hidden="true" className="bs-field__required-mark">
            *
          </span>
          <VisuallyHidden>{' (required)'}</VisuallyHidden>
        </>
      ) : null}
    </label>
  );
}

type FieldTextProps = {
  children: ReactNode;
  id: string;
};

export function FieldHint({ children, id }: FieldTextProps) {
  return (
    <p className="bs-field__hint" id={id}>
      {children}
    </p>
  );
}

export function FieldError({ children, id }: FieldTextProps) {
  return (
    <p className="bs-field__error" id={id}>
      <VisuallyHidden>Error: </VisuallyHidden>
      {children}
    </p>
  );
}
