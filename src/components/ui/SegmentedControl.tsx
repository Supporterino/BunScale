import { useId } from 'react';
import { classNames } from './class-names';

/*
 * `SegmentedControl` — a small set of mutually exclusive choices: the weight
 * unit (kg / lb), a range, a view mode.
 *
 * The semantics are a `radiogroup` made of real `<input type="radio">`
 * elements, one per segment, each inside its own `<label>`. That is deliberate:
 * the browser then owns arrow-key movement, roving focus and the announcement of
 * "2 of 3", so the primitive implements no keyboard logic of its own and cannot
 * get it wrong. The input is visually hidden but stays focusable, and its focus
 * ring is drawn on the visible segment body through the sibling combinator
 * (`input:focus-visible + .bs-segmented__body`), so keyboard focus is always
 * visible even though the input itself is clipped.
 *
 * The selected segment is not signalled by colour alone: it gains the elevated
 * surface, the shadow and the heavier weight as well.
 *
 * The label comes from `FormField` (`labelId`/`id`), which is why this control
 * takes those two ids: a group cannot be named by `<label for>`, so it names
 * itself with `aria-labelledby`. The same `id` is what `FormField`'s `<label
 * for>` points at, so the reference is never dangling.
 *
 * The rest of `FormField`'s wiring arrives the same way, because a `radiogroup`
 * is not an `<input>` and the browser puts none of it on the group for free:
 * `describedBy` (so the hint and the error reach the group as well as the eye),
 * `invalid` (so a radio group in an error state is announced as invalid) and
 * `required`. A group that took only the two ids would look labelled while its
 * visible hint was announced to nobody — the contract `FormField` documents is
 * only true when the consumer forwards all five values.
 */

export type SegmentedControlOption<TValue extends string> = {
  value: TValue;
  label: string;
};

export type SegmentedControlProps<TValue extends string> = {
  options: Array<SegmentedControlOption<TValue>>;
  value: TValue;
  onChange: (value: TValue) => void;
  /** `id` for the group element, from `FormField`'s `controlId`. */
  id?: string;
  /** `id` of the label element, from `FormField`'s `labelId`. */
  labelId?: string;
  /** `aria-describedby` value, from `FormField`'s `describedBy`. */
  describedBy?: string;
  /** `aria-invalid`, from `FormField`'s `invalid`; absent unless invalid. */
  invalid?: boolean;
  /** `aria-required`, from `FormField`'s `required`. */
  required?: boolean;
  /** Radio group name; generated when omitted. */
  name?: string;
  disabled?: boolean;
  className?: string;
};

export function SegmentedControl<TValue extends string>({
  options,
  value,
  onChange,
  id,
  labelId,
  describedBy,
  invalid = false,
  required = false,
  name,
  disabled = false,
  className,
}: SegmentedControlProps<TValue>) {
  const generatedName = useId();

  return (
    <div
      aria-describedby={describedBy}
      aria-invalid={invalid ? true : undefined}
      aria-labelledby={labelId}
      aria-required={required ? true : undefined}
      className={classNames('bs-segmented', className)}
      id={id}
      role="radiogroup"
    >
      {options.map((option) => (
        <label
          className={classNames(
            'bs-segmented__segment',
            option.value === value ? 'bs-segmented__segment--selected' : null,
          )}
          key={option.value}
        >
          <input
            checked={option.value === value}
            className="bs-segmented__input"
            disabled={disabled}
            name={name ?? generatedName}
            onChange={() => {
              onChange(option.value);
            }}
            type="radio"
            value={option.value}
          />
          <span className="bs-segmented__body">{option.label}</span>
        </label>
      ))}
    </div>
  );
}
