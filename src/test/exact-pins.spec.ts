/*
 * TOOL-8: "Versions are exact" (task 0.24).
 *
 * Task 0.13 created both manifests and no test task owned this scenario. The
 * expectation is not a second list written here: it is **D0.8's pin tables**,
 * read from the change's `design.md` at test time. A hard-coded list would be
 * one more hand-reconciled statement to rot, and the requirement is that the
 * committed manifests match the recorded pins.
 *
 * Both halves are asserted against the corresponding table: missing keys,
 * extra keys, and a pin that differs from D0.8 are all failures. The npm half
 * checks every declared dependency section (including optional/peer entries, so
 * an undeclared pin cannot hide there); the Cargo half parses the three
 * dependency tables line by line, because no TOML parser is a pinned dependency
 * and TOOL-8 forbids adding one for a test. `manifest-pins.ts` states exactly
 * which TOML forms that parse can and cannot see.
 *
 * "Paired crates move together" is asserted as well: `tauri-specta` declares an
 * exact dependency on `specta`, so the two pins must stay equal. What neither
 * test can see is "Both platforms are supported" — a manifest does not declare
 * platform support; that is the per-dependency review recorded in D0.8.
 */
import { describe, expect, it } from 'vitest';
import {
  checkCargoPins,
  checkNpmPins,
  collectCargoPins,
  collectNpmPins,
  parseDesignPinTables,
} from './manifest-pins';
import { DESIGN_RECORD_PATH, readRepositoryFile } from './scaffold-files';

describe('TOOL-8: the toolchain is pinned exactly', () => {
  it('Versions are exact — package.json declares exactly D0.8’s npm pins', () => {
    const tables = parseDesignPinTables(readRepositoryFile(DESIGN_RECORD_PATH));
    const issues = checkNpmPins(
      tables.npm,
      collectNpmPins(readRepositoryFile('package.json')),
    );

    // An empty list is the pass; a non-empty one prints every disagreement —
    // an inexact range, a pin that differs from D0.8, a missing package or an
    // extra one.
    expect(issues, 'package.json disagrees with D0.8').toEqual([]);
  });

  it('Versions are exact — Cargo.toml declares exactly D0.8’s crate pins, each with =', () => {
    const tables = parseDesignPinTables(readRepositoryFile(DESIGN_RECORD_PATH));
    const issues = checkCargoPins(
      tables.crates,
      collectCargoPins(readRepositoryFile('src-tauri/Cargo.toml')),
    );

    expect(issues, 'src-tauri/Cargo.toml disagrees with D0.8').toEqual([]);
  });

  it('Paired crates move together', () => {
    const pins = collectCargoPins(readRepositoryFile('src-tauri/Cargo.toml'));
    const specta = pins.find((pin) => pin.name === 'specta');
    const tauriSpecta = pins.find((pin) => pin.name === 'tauri-specta');

    expect(
      specta,
      'Cargo.toml does not declare specta, so the pair cannot be checked',
    ).toBeDefined();
    expect(
      tauriSpecta,
      'Cargo.toml does not declare tauri-specta, so the pair cannot be checked',
    ).toBeDefined();
    expect(specta?.version).toBe(tauriSpecta?.version);
  });
});
