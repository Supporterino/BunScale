/*
 * The quick-entry path's rabbit picker (task 4.20; WT-2, NFR-9, Q12).
 *
 * **Feature-local, not a promoted primitive.** The frozen inventory lists
 * `Select`/`Picker` as deferred and says a consumer adds the entry first; the
 * record lives where no implementation agent may write. Q12 therefore recorded
 * this one as a native `<select>` composed from `FormField` and the field class
 * contract — the same composition `RabbitForm` already uses for its
 * `<input type="date">` — instead of inventing a primitive or adding a
 * dependency. Promoting it later is the design-system owner's follow-up.
 *
 * **The native control is the accessibility answer, not a shortcut.** At 390 px
 * it is a directly visible control in the dashboard toolbar — never a long press
 * or a hidden menu (NFR-9). The platform draws the picker, so touch, VoiceOver
 * and a keyboard all get the behaviour they already know; the `<label for>` from
 * `FormField` is its accessible name, and every option is the rabbit's name.
 * Only `controlId` is forwarded from the field wiring: the name needs no
 * `aria-labelledby`, and hint, error and required have nothing to describe on a
 * control that launches a route.
 *
 * **The options are the dashboard read's active rabbits**, in the order the read
 * model returned them (DASH-3), never-weighed included because a rabbit without
 * a weigh-in is still one to weigh. Re-sorting here would be a second ordering
 * rule beside the list's.
 *
 * **Choosing navigates; the value is not retained.** The caller owns the
 * destination, so this control holds no routing and no form state. The empty
 * option is what makes the first rabbit reachable at all (a change event only
 * fires on a change) and is the only prompt the catalog offers: task 4.19 added
 * `weight.picker.label` for this control and no second key, so the unset option
 * repeats the label rather than rendering a nameless blank in the reader's ear.
 */
import { FormField } from '../../../components/ui/FormField';
import { t } from '../../i18n';
import type { DashboardRow } from '../queries';

export type RabbitPickerProps = {
  /** The dashboard's rows, in the read model's order (DASH-3). */
  rows: readonly DashboardRow[];
  /** Called with the chosen rabbit's id; the caller owns the destination. */
  onChoose: (rabbitId: string) => void;
};

export function RabbitPicker({ rows, onChoose }: RabbitPickerProps) {
  return (
    <FormField
      className="bs-dashboard__picker"
      label={t('weight.picker.label')}
    >
      {(field) => (
        <select
          className="bs-text-field"
          id={field.controlId}
          onChange={(event) => {
            const rabbitId = event.currentTarget.value;

            if (rabbitId !== '') {
              onChoose(rabbitId);
            }
          }}
          value=""
        >
          {/* The unset state: no rabbit is bound until the person chooses one.
              It is an ordinary option, so the control opens on the prompt
              rather than on the first rabbit; choosing it changes nothing and
              the handler ignores its empty value. */}
          <option value="">{t('weight.picker.label')}</option>
          {rows.map((row) => (
            <option key={row.rabbit.id} value={row.rabbit.id}>
              {row.rabbit.name}
            </option>
          ))}
        </select>
      )}
    </FormField>
  );
}
