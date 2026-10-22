/*
 * Task 3.1's evidence for the display-unit boundary (WT-3, DM-1, design I1):
 * the two conversions are the product's only ones, a stored value is always a
 * whole number of grams, and the round trip a stored value takes — formatted
 * for display and typed back in unchanged — is the same value.
 *
 * The refusals are asserted as hard as the conversions. Each one exists because
 * the alternative stores a value the person never gave: a comma guessed as a
 * decimal point would be 1000× wrong, and sub-gram input rounded instead of
 * refused would be a value no one entered. Silent corruption of a health value
 * is the failure mode this file is here to catch.
 */
import { describe, expect, it } from 'vitest';
import { formatDisplayWeight, parseDisplayWeight } from './api';

/** The display unit, derived from the conversion that takes it. */
type WeightUnit = Parameters<typeof parseDisplayWeight>[1];

/** Both units, so nothing below is exercised in only one of them. */
const UNITS: WeightUnit[] = ['grams', 'kilograms'];

/** Stored values that expose a rounding or padding mistake, in whole grams. */
const STORED: number[] = [0, 1, 10, 100, 500, 999, 1005, 1200, 10_000];

describe('the display-unit round trip', () => {
  it.each(STORED)(
    '%i g survives being formatted and parsed back in both units',
    (grams) => {
      for (const unit of UNITS) {
        expect(parseDisplayWeight(formatDisplayWeight(grams, unit), unit)).toBe(
          grams,
        );
      }
    },
  );

  it('survives the same trip as a negative value', () => {
    for (const unit of UNITS) {
      expect(parseDisplayWeight(formatDisplayWeight(-500, unit), unit)).toBe(
        -500,
      );
    }
  });

  it('reads zero as a value, not as an empty field', () => {
    expect(parseDisplayWeight('0', 'grams')).not.toBeNull();
    expect(parseDisplayWeight('0', 'grams')).toBe(0);
    expect(parseDisplayWeight('0.000', 'kilograms')).toBe(0);
  });
});

describe('formatting stored grams', () => {
  it('shows grams as the whole number it is', () => {
    expect(formatDisplayWeight(1234, 'grams')).toBe('1234');
    expect(formatDisplayWeight(0, 'grams')).toBe('0');
  });

  it('shows kilograms exactly, with no trailing zeros and no rounding', () => {
    expect(formatDisplayWeight(1234, 'kilograms')).toBe('1.234');
    expect(formatDisplayWeight(1200, 'kilograms')).toBe('1.2');
    expect(formatDisplayWeight(1005, 'kilograms')).toBe('1.005');
    expect(formatDisplayWeight(500, 'kilograms')).toBe('0.5');
    expect(formatDisplayWeight(7, 'kilograms')).toBe('0.007');
    expect(formatDisplayWeight(1000, 'kilograms')).toBe('1');
    expect(formatDisplayWeight(0, 'kilograms')).toBe('0');
  });

  it('shows a negative value with its sign', () => {
    expect(formatDisplayWeight(-500, 'kilograms')).toBe('-0.5');
    expect(formatDisplayWeight(-5, 'grams')).toBe('-5');
  });
});

describe('parsing a typed value', () => {
  it('interprets the typed unit as whole grams', () => {
    expect(parseDisplayWeight('1.234', 'kilograms')).toBe(1234);
    expect(parseDisplayWeight('1.2', 'kilograms')).toBe(1200);
    expect(parseDisplayWeight('.5', 'kilograms')).toBe(500);
    expect(parseDisplayWeight('1.', 'kilograms')).toBe(1000);
    expect(parseDisplayWeight('1234', 'grams')).toBe(1234);
  });

  it('trims the field before reading it', () => {
    expect(parseDisplayWeight('  2.5  ', 'kilograms')).toBe(2500);
  });

  it('treats trailing zeros as no precision', () => {
    expect(parseDisplayWeight('500.0', 'grams')).toBe(500);
    // The padding bug this guards: keeping `1.0000`'s extra zeros would make it
    // 10 000 g, a 10× value the person never typed.
    expect(parseDisplayWeight('1.0000', 'kilograms')).toBe(1000);
  });

  it('parses a negative value, leaving the non-positive rule to the command', () => {
    // WT-4 is a value rule the form and the boundary share; a parse failure here
    // would make the form re-derive it from "unreadable" instead.
    expect(parseDisplayWeight('-5', 'kilograms')).toBe(-5000);
  });
});

describe('refusing a value that cannot be stored', () => {
  it('refuses sub-gram precision instead of rounding it', () => {
    expect(parseDisplayWeight('1.2345', 'kilograms')).toBeNull();
    expect(parseDisplayWeight('5.5', 'grams')).toBeNull();
  });

  it('refuses a comma rather than guessing the convention', () => {
    expect(parseDisplayWeight('1,234', 'grams')).toBeNull();
    expect(parseDisplayWeight('1,2', 'kilograms')).toBeNull();
  });

  it('refuses anything that is not a plain decimal number', () => {
    expect(parseDisplayWeight('', 'grams')).toBeNull();
    expect(parseDisplayWeight('   ', 'grams')).toBeNull();
    expect(parseDisplayWeight('1e3', 'grams')).toBeNull();
    expect(parseDisplayWeight('1.2.3', 'grams')).toBeNull();
    expect(parseDisplayWeight('500 g', 'grams')).toBeNull();
    expect(parseDisplayWeight('abc', 'grams')).toBeNull();
  });

  it('refuses a value above what the wire type can carry', () => {
    expect(parseDisplayWeight('2147483647', 'grams')).toBe(2_147_483_647);
    expect(parseDisplayWeight('2147483648', 'grams')).toBeNull();
    expect(parseDisplayWeight('99999999', 'kilograms')).toBeNull();
  });
});
