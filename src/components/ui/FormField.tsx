import type { ReactNode } from 'react';
import { useId } from 'react';
import { classNames } from './class-names';
import { FieldError, FieldHint, FieldLabel } from './field-parts';

/*
 * `FormField` — the labelling wrapper for any control that cannot name itself:
 * a `SegmentedControl` today, a picker or a text area later.
 *
 * A `TextField` does not need it: that primitive carries its own label, hint and
 * error, because a text input is always labelled that way and splitting the two
 * halves would let a screen render one without the other. `FormField` is for the
 * controls whose label association is not a `<label for>` — a group needs
 * `aria-labelledby`, so the wrapper hands the control the ids instead of
 * guessing how it labels itself.
 *
 * The wiring is passed through a render prop rather than by cloning the child or
 * through context, so it is typed, explicit at the call site, and works for any
 * number of nested elements without a wrapper component of its own:
 *
 *   <FormField label="Weight unit">
 *     {(field) => (
 *       <SegmentedControl
 *         describedBy={field.describedBy}
 *         id={field.controlId}
 *         invalid={field.invalid}
 *         labelId={field.labelId}
 *         onChange={setUnit}
 *         options={units}
 *         required={field.required}
 *         value={unit}
 *       />
 *     )}
 *   </FormField>
 *
 * What the contract fixes, and this primitive therefore does:
 *   - a real `<label>` (never a placeholder, never a `div` that looks like one);
 *   - the hint and the error are linked with `aria-describedby`, in that order —
 *     this primitive computes the value, the control puts it on the element it
 *     labels (a `SegmentedControl` reads `describedBy` for exactly that);
 *   - `aria-invalid` is present only when there is an error, and the error also
 *     carries announced text (see `./field-parts.tsx`);
 *   - `required` is announced as well as painted.
 *
 * The wiring is only worth what the consumer forwards. The five values above are
 * the whole contract, and `SegmentedControl` is the primitive that takes all
 * five; a consumer that passed only `controlId` and `labelId` would render a
 * group whose visible hint nobody hears.
 */

export type FieldWiring = {
  /** `id` for the control, and the `<label for>` target. */
  controlId: string;
  /** `id` of the label, for a control that names itself with `aria-labelledby`. */
  labelId: string;
  /** `aria-describedby` value, or `undefined` when there is nothing to describe. */
  describedBy: string | undefined;
  /** `aria-invalid`, present only when the field is invalid. */
  invalid: true | undefined;
  /** `required`, present only when the field is required. */
  required: true | undefined;
};

export type FormFieldProps = {
  label: string;
  children: (wiring: FieldWiring) => ReactNode;
  hint?: string;
  error?: string;
  required?: boolean;
  className?: string;
};

export function FormField({
  label,
  children,
  hint,
  error,
  required = false,
  className,
}: FormFieldProps) {
  const fieldId = useId();
  const controlId = `${fieldId}-control`;
  const labelId = `${fieldId}-label`;
  const hintId = `${fieldId}-hint`;
  const errorId = `${fieldId}-error`;

  const describedBy = [
    hint === undefined ? null : hintId,
    error === undefined ? null : errorId,
  ]
    .filter((value): value is string => value !== null)
    .join(' ');

  return (
    <div className={classNames('bs-field', className)}>
      <FieldLabel htmlFor={controlId} id={labelId} required={required}>
        {label}
      </FieldLabel>
      {children({
        controlId,
        labelId,
        describedBy: describedBy === '' ? undefined : describedBy,
        invalid: error === undefined ? undefined : true,
        required: required ? true : undefined,
      })}
      {hint === undefined ? null : <FieldHint id={hintId}>{hint}</FieldHint>}
      {error === undefined ? null : (
        <FieldError id={errorId}>{error}</FieldError>
      )}
    </div>
  );
}
