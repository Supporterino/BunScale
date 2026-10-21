import type { InputHTMLAttributes } from 'react';
import { useId } from 'react';
import { classNames } from './class-names';
import { FieldError, FieldHint, FieldLabel } from './field-parts';

/*
 * `TextField` — the labelled text input: a household name, an account name.
 *
 * The label is always visible and always a real `<label for>`; a placeholder is
 * a hint about the expected value and never the name of the field. The input is
 * at least 44 pt tall (`--input-height`), it keeps the native keyboard and text
 * services (`type`, plus `inputMode` and `enterKeyHint` so the software keyboard
 * offers the right keys for the data), and selection colours come from the
 * tokens so they stay legible in both themes.
 *
 * Keyboard: focusing the field scrolls it into view, and `Screen` reserves
 * `--keyboard-inset` at the bottom of the scroll region so the content the
 * keyboard would cover stays reachable. This primitive does not assign that
 * inset: the first real form screen (task 4.1) owns the one `visualViewport`
 * measurement, and `semantic.css` states the same at the token. Nothing here
 * measures the keyboard either.
 *
 * `label`, `hint`, `error` and `required` are the primitive's own, drawn by the
 * shared field parts; `aria-describedby` links hint and error in that order,
 * `aria-invalid` appears only when there is an error and the error announces
 * itself with text rather than colour alone.
 */

export type TextFieldType = 'text' | 'number' | 'search';

export type TextFieldProps = {
  label: string;
  /** Controlled value. Omit for an uncontrolled field with `defaultValue`. */
  value?: string;
  defaultValue?: string;
  /** Called with the field's value, not the event. */
  onChange?: (value: string) => void;
  type?: TextFieldType;
  inputMode?: InputHTMLAttributes<HTMLInputElement>['inputMode'];
  enterKeyHint?: InputHTMLAttributes<HTMLInputElement>['enterKeyHint'];
  autoComplete?: string;
  placeholder?: string;
  hint?: string;
  error?: string;
  disabled?: boolean;
  required?: boolean;
  name?: string;
  id?: string;
  className?: string;
};

export function TextField({
  label,
  value,
  defaultValue,
  onChange,
  type = 'text',
  inputMode,
  enterKeyHint,
  autoComplete,
  placeholder,
  hint,
  error,
  disabled = false,
  required = false,
  name,
  id,
  className,
}: TextFieldProps) {
  const fieldId = useId();
  const inputId = id ?? `${fieldId}-input`;
  const hintId = `${fieldId}-hint`;
  const errorId = `${fieldId}-error`;

  const describedBy = [
    hint === undefined ? null : hintId,
    error === undefined ? null : errorId,
  ]
    .filter((describedId): describedId is string => describedId !== null)
    .join(' ');

  return (
    <div className={classNames('bs-field', className)}>
      <FieldLabel htmlFor={inputId} id={`${fieldId}-label`} required={required}>
        {label}
      </FieldLabel>
      <input
        aria-describedby={describedBy === '' ? undefined : describedBy}
        aria-invalid={error === undefined ? undefined : true}
        autoComplete={autoComplete}
        className="bs-text-field"
        defaultValue={defaultValue}
        disabled={disabled}
        enterKeyHint={enterKeyHint}
        id={inputId}
        inputMode={inputMode}
        name={name}
        onChange={
          onChange === undefined
            ? undefined
            : (event) => {
                onChange(event.currentTarget.value);
              }
        }
        placeholder={placeholder}
        required={required}
        type={type}
        value={value}
      />
      {hint === undefined ? null : <FieldHint id={hintId}>{hint}</FieldHint>}
      {error === undefined ? null : (
        <FieldError id={errorId}>{error}</FieldError>
      )}
    </div>
  );
}
