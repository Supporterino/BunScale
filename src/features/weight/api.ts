/*
 * Weight commands (WT-1 … WT-8) and the display-unit boundary (WT-3, DM-1):
 * the only module that imports their generated bindings. Project hard rule 2
 * keeps `src/bindings.ts` out of routes, hooks and components, so every weight
 * call in the app goes through these functions.
 *
 * **Grams are canonical, and the display unit exists only here** (D1.2, I1).
 * Every weight command accepts and returns grams; nothing on the wire carries a
 * unit and no command resolves the device's preference. The two conversions the
 * product has live in this module and nowhere else: `parseDisplayWeight` turns
 * what the person typed into whole grams before the call, and
 * `formatDisplayWeight` turns stored grams into the preferred unit after it. A
 * displayed value is therefore never stored, and changing the display unit
 * cannot rewrite history (DM-1) — the rule holds by construction rather than by
 * care, which is what P0's SET-4 preference is designed around.
 *
 * **The unit is P0's, and this module keeps no copy of it.** The unit is a
 * parameter of the two conversions, typed by the contract (`WeightUnit`, from
 * the same generated bindings as every other type here), and its value is what
 * `useDisplayWeightUnit()` surfaces from `get_display_weight_unit` (SET-4),
 * which resolves the person's choice, then the device's region, then the
 * canonical unit. Nothing here reads a locale, picks a default or stores a
 * second unit: a caller with no loaded unit has no unit to pass, and that is
 * the honest state, because a default invented here could disagree with the one
 * the backend already resolved (task 3.5 passes the hook's value in).
 *
 * **A stored value is always an integer count of grams** (I7). The wire type is
 * an `i32` under a transparent newtype, so `Grams` is an exact JavaScript
 * `number`. `parseDisplayWeight` builds the integer from the typed digits rather
 * than multiplying a float, refuses sub-gram precision instead of rounding it,
 * and refuses input the wire type cannot carry — so the value that is stored is
 * exactly the value the person gave, and `parseDisplayWeight(
 * formatDisplayWeight(g, unit), unit) === g` for every stored `g`. A value
 * formatted for display can be re-entered unchanged without moving.
 *
 * The three writes take a `RequestId` and are returned as a `RequestOperation`
 * (D0.14), like every other write in the app: `run()` is the first attempt and
 * the retry, and a corrected value is a new handle. `WeightEntryDraft.timestamp`
 * is `null` to mean "now" on create and "keep the entry's timestamp" on update —
 * an edit never restamps a back-dated entry (WT-1, WT-5).
 *
 * Every write is refused on an archived rabbit and reports `cause: validation`
 * (D1.5); so is a non-positive value and an unconfirmed outlier above
 * `PLAUSIBILITY_BOUND_GRAMS` (WT-4, I4). The form asks *when* to prompt using
 * the constant below and the command decides what is stored — the bound is
 * never a value this module writes.
 *
 * The `weight:changed` subscription is not here yet: no P1 task has asked for
 * it, and it belongs beside these commands when the task that needs it lands
 * (the invalidation policy of task 3.7), the way `household:changed` sits in the
 * household module.
 */
import type {
  CommandError,
  Grams,
  RabbitId,
  WeightEntry,
  WeightEntryDraft,
  WeightEntryId,
  WeightRange,
  WeightTrend,
  WeightUnit,
} from '../../bindings';
import { commands } from '../../bindings';
import type { CommandResult, RequestOperation } from '../../lib/request-id';
import { requestOperation } from '../../lib/request-id';

/** One read's outcome: the contract's `Result`, typed to this domain. */
type Outcome<T> = Promise<CommandResult<T, CommandError>>;

/** A state-changing command's handle; `run()` again re-uses the request id. */
type Operation<T> = RequestOperation<T, CommandError>;

/**
 * The confirmation threshold for an implausible weight, in grams (WT-4, D1.9).
 *
 * The boundary holds the rule — a value above this is refused with
 * `cause: validation` until the person confirms it, and is saved unchanged once
 * they do — so this number only decides *when* a form asks (I4). A drift
 * between the two could change when the prompt appears and never what is
 * stored, which is why mirroring it is safe and why it must not be re-declared
 * at a call site.
 */
export const PLAUSIBILITY_BOUND_GRAMS: Grams = 10_000;

/**
 * The largest value `Grams` can carry: the wire type is an `i32` (I7). A larger
 * number would fail to deserialise at the boundary rather than be refused by a
 * command, so it is refused here where it is still a typed value.
 */
const MAX_GRAMS = 2_147_483_647;

/** How many decimal places each display unit can express as whole grams. */
const GRAM_DECIMALS: Record<WeightUnit, number> = { grams: 0, kilograms: 3 };

/** A typed weight: an optional sign, digits, and at most one `.`. */
const TYPED_WEIGHT = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$/;

/**
 * What the person typed, in their preferred unit, as whole grams (WT-3, DM-1).
 *
 * `null` means "not a value this product can store" and nothing else: the form
 * prompts for a valid value and no command is called (WT-4's non-positive
 * rejection is a *value* rule and is deliberately not applied here — `-5` is
 * parsed to `-5000` so the form and the boundary can both refuse it by the rule
 * they share, rather than the form re-deriving it from a parse failure).
 *
 * Four refusals, each for a reason the stored value would otherwise misstate:
 *
 * - **Sub-gram precision.** `1.2345 kg` is 1234.5 g and grams are whole. It is
 *   refused rather than rounded, so the stored value is never a value the
 *   person did not give, and formatting a stored value and re-entering it is
 *   always the same number.
 * - **A comma.** `1,234` is a thousands separator in one convention and a
 *   decimal point in another, and guessing wrong stores a 1000× value. Only `.`
 *   is read; a comma is refused and the form prompts.
 *   shortcut: one locale, '.' only; upgrade when a comma-decimal locale ships.
 * - **Anything that is not a plain decimal number** — an empty field, a unit
 *   suffix, an exponent, a second separator, a word.
 * - **A value the wire type cannot carry**, above `MAX_GRAMS`.
 */
export function parseDisplayWeight(
  text: string,
  unit: WeightUnit,
): Grams | null {
  const typed = text.trim();

  if (!TYPED_WEIGHT.test(typed)) {
    return null;
  }

  const negative = typed.startsWith('-');
  const [whole, fraction = ''] = typed.replace(/^[+-]/, '').split('.');
  const decimals = GRAM_DECIMALS[unit];

  // Trailing zeros are not precision: `500.0 g` is 500 whole grams, and
  // `1.0000 kg` must not become 10 000 g by keeping the extra zeros.
  const significant = fraction.replace(/0+$/, '');

  if (significant.length > decimals) {
    return null;
  }

  // Built from the digits, not from `Number(typed) * 1000`: the float product
  // can land just off the integer, and the whole point is an exact count.
  const digits = `${whole}${significant.padEnd(decimals, '0')}`.replace(
    /^0+(?=\d)/,
    '',
  );
  const grams = Number(digits);

  if (grams > MAX_GRAMS) {
    return null;
  }

  return negative && grams !== 0 ? -grams : grams;
}

/**
 * Stored grams as the person's preferred unit, without a unit label (WT-3,
 * DM-1).
 *
 * The inverse of `parseDisplayWeight` and its only other caller-side half: a
 * value shown and re-entered unchanged is the same stored value. The label
 * ("g", "kg") is the surface's copy, not this function's, so no string here is
 * hard-coded (NFR-9). Trailing zeros are trimmed — `1234 g` reads as `1.234`,
 * `1200 g` as `1.2`, `500 g` as `0.5` — and every value is exact, because a
 * kilogram is a whole number of grams.
 */
export function formatDisplayWeight(grams: Grams, unit: WeightUnit): string {
  if (unit === 'grams') {
    return String(grams);
  }

  const magnitude = Math.abs(grams);
  const kilograms = Math.trunc(magnitude / 1000);
  const remainder = magnitude % 1000;
  const text =
    remainder === 0
      ? String(kilograms)
      : `${kilograms}.${String(remainder).padStart(3, '0').replace(/0+$/, '')}`;

  return grams < 0 && text !== '0' ? `-${text}` : text;
}

/**
 * Record a weight entry (WT-1, WT-3, WT-4, DM-1).
 *
 * The draft carries grams: the caller parses the typed value with
 * `parseDisplayWeight` first and passes the preferred unit to nothing on the
 * wire (I1). A `timestamp: null` records the moment of creation at the storage
 * boundary — one instant, not one per retry — and the person's own instant is
 * stored unchanged, including in the past (WT-1).
 *
 * A value of zero or less, an unconfirmed value above `PLAUSIBILITY_BOUND_GRAMS`
 * or a write to an archived rabbit is refused with `cause: validation`
 * (WT-4, D1.5); the write-path causes are all reachable behind it (D1.1).
 */
export function createWeightEntry(
  rabbitId: RabbitId,
  entry: WeightEntryDraft,
): Operation<WeightEntry> {
  return requestOperation<WeightEntry, CommandError>((requestId) =>
    commands.createWeightEntry(rabbitId, entry, requestId),
  );
}

/**
 * One rabbit's weight series for one window, with the threshold it is judged
 * against (WT-6, WT-7).
 *
 * The range is a query window, not a stored value (D1.4): the trend and the
 * weight-only history call this with their own `WeightRange`, each held in the
 * route's typed search parameters, so one view cannot influence the other. The
 * window limits which points come back and **not** how the flags are derived —
 * a flag is computed over the rabbit's full entry set (I3), so an entry can be
 * flagged on the basis of a predecessor outside the window. Points are oldest
 * first; a newest-first view reverses them.
 *
 * The points carry grams like everything else here: a chart formats them with
 * `formatDisplayWeight` and converts no further.
 */
export function getWeightEntries(
  rabbitId: RabbitId,
  range: WeightRange,
): Outcome<WeightTrend> {
  return commands.getWeightEntries(rabbitId, range);
}

/**
 * One entry as the detail view and the edit form present it (WT-8, DM-1).
 *
 * The value is grams and there is no unit field; the derived drop alert travels
 * with it (I2), so the detail does not re-derive a rule the contract states.
 * An unknown or blank id is refused with `cause: validation`; the other
 * failures are the local set.
 */
export function getWeightEntry(entryId: WeightEntryId): Outcome<WeightEntry> {
  return commands.getWeightEntry(entryId);
}

/**
 * Correct a past entry (WT-5, WT-7, WT-8).
 *
 * A whole-entry replacement like `create_weight_entry`, with one difference the
 * draft's `timestamp: null` carries: `null` **keeps the entry's existing
 * timestamp**, so correcting a value never restamps a back-dated entry to now.
 * Moving a timestamp is an explicit instant, and there is no "reset to now".
 *
 * The same refusals as create apply, and a write to an archived rabbit is
 * refused with `cause: validation` (D1.5). A corrected edit re-derives the
 * alerts of the entries around it, because the derivation is over the current
 * entry set rather than a snapshot (D1.13).
 */
export function updateWeightEntry(
  entryId: WeightEntryId,
  entry: WeightEntryDraft,
): Operation<WeightEntry> {
  return requestOperation<WeightEntry, CommandError>((requestId) =>
    commands.updateWeightEntry(entryId, entry, requestId),
  );
}

/**
 * Remove a weight entry (WT-5, WT-8).
 *
 * An unknown entry, or one belonging to an archived rabbit, is refused with
 * `cause: validation` (D1.5). The surrounding entries' alerts are re-derived on
 * the next read rather than repaired here (D1.13).
 *
 * The contract's `()` arrives as `null` on the wire.
 */
export function deleteWeightEntry(entryId: WeightEntryId): Operation<null> {
  return requestOperation<null, CommandError>((requestId) =>
    commands.deleteWeightEntry(entryId, requestId),
  );
}
