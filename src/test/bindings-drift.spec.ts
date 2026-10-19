/*
 * TOOL-3: "Bindings are generated reproducibly and drift fails the build"
 * (task 0.24).
 *
 * Three properties are asserted here, and the second and third are the ones
 * that prove the gate's drift check actually bites — task 0.16 could only state
 * them in prose, because neither the orchestrator nor `frontend-logic` holds
 * edit permission on a generated file and `git checkout` was unavailable.
 *
 * Why this lives in the bun step and not in the Rust integration target:
 * `definition-of-done` runs `cargo test` **before** the bindings drift check, so
 * a Rust test that reached the exporter would rewrite the very file that check
 * is about to verify (design.md D0.9). Here the ordering is the other way
 * round: `bun run test` is the **last** command of the gate, after
 * `bun run gen:bindings` and `git diff --exit-code src/bindings.ts`, so the
 * mutation this file performs in the middle of the hand-edit test cannot
 * invalidate that run's own drift check.
 *
 * `gen:bindings` is run through the committed `package.json` script rather than
 * by repeating its `cargo run --example` body: the requirement is about the
 * gate's command, and a second copy of the invocation could drift from it.
 *
 * Whether the exporter creates an application **window** cannot be observed at
 * runtime here — a process that started Tauri's event loop would not return —
 * so the first test asserts it structurally instead: the two source files that
 * run during an export, the cargo example and `src-tauri/src/ipc/mod.rs`, name
 * no app-run or window constructor. That is TOOL-3's "AND no application
 * window is created" half. The check cannot follow a call deeper than those two
 * files; the Rust side keeps `export_bindings` free of an app handle for that
 * reason (D0.9).
 *
 * That reading is taken over the **code**, with `stripRustComments` having
 * blanked the comments first. A doc comment that merely *names* one of the five
 * is prose *about* the rule rather than a breach of it, and the trap is real
 * rather than hypothetical: design.md D0.9's own prose says `tauri::Builder` as
 * it describes the mount, `app_lib::run()` as it records what the binary edge
 * does and `generate_context!()` as it records the context the app builds —
 * three of these five, written into the record that forbids them in code. A scan
 * over the whole file fails the moment a source comment quotes any of them,
 * which is a failure about the wrong thing.
 */
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  describeCommand,
  REPOSITORY_ROOT,
  readRepositoryFile,
  runBunScript,
  runGitDiff,
} from './scaffold-files';

const BINDINGS_PATH = 'src/bindings.ts';
const bindingsFile = resolve(REPOSITORY_ROOT, BINDINGS_PATH);
const HAND_EDIT_MARKER =
  '\n// hand-edit marker appended by src/test/bindings-drift.spec.ts\n';

/** The five names that would run the app or create a window (TOOL-3, D0.9). */
const FORBIDDEN_NAMES = [
  'app_lib::run',
  'tauri::Builder',
  'WebviewWindow',
  'WindowBuilder',
  'generate_context!',
] as const;

/** The `#` count of the raw string at `index`, or -1 when there is none. */
function rawStringHashes(source: string, index: number): number {
  let cursor = source[index] === 'b' ? index + 1 : index;

  if (source[cursor] !== 'r') {
    return -1;
  }

  cursor += 1;

  let hashes = 0;

  while (source[cursor] === '#') {
    hashes += 1;
    cursor += 1;
  }

  return source[cursor] === '"' ? hashes : -1;
}

/**
 * Blanks Rust comments, offsets and newlines preserved, so the structural check
 * in the spec below reads code rather than prose.
 *
 * This is deliberately **not** `stripComments` from `css-invariants.ts`, which
 * is the suite's existing scanner and was considered first. That one is
 * quote-aware for `'` and `"` because in CSS and TypeScript those two characters
 * are the string delimiters; in Rust a `'` opens a character literal **and** a
 * lifetime, told apart only by what follows it. Handed `&'static str`, the CSS
 * scanner opens a "string" at the lifetime's quote and does not close it until
 * the next `'` in the file happens to appear, so every `//` in between is copied
 * through and a comment naming one of the five forbidden names fails the check
 * again — precisely the false failure this stripping exists to remove. Rust's
 * block comments also **nest**, and its raw strings do not end at the first `"`;
 * the CSS scanner models neither. Reusing it would have been reuse in name only,
 * so the Rust-shaped scanner is written here instead.
 *
 * Modeled: `//` line comments (so `///` and `//!`), block comments to any
 * nesting depth (the `/**` and `/*!` doc forms open one too), ordinary and byte
 * strings with `\` escapes, raw and byte raw strings, and character literals —
 * a `'"'` is copied rather than read as a string delimiter. A `'` with no
 * closing quote one character along is a lifetime and stays code, as does
 * everything else: identifiers, macro input, attribute contents.
 *
 * Not modeled, stated rather than implied: `c"…"` C strings, and a raw string
 * whose `#` counts do not balance (which no longer compiles either). Everything
 * that is not a comment is copied verbatim rather than blanked, strings
 * included, so a forbidden name inside a string literal is still a name the file
 * carries — the conservative direction for this assertion.
 */
function stripRustComments(source: string): string {
  let result = '';
  let index = 0;

  while (index < source.length) {
    const character = source[index];

    // A line comment, `///` and `//!` included: blanked to the newline, which
    // is kept so the member offsets and line structure of the file survive.
    if (character === '/' && source[index + 1] === '/') {
      while (index < source.length && source[index] !== '\n') {
        result += ' ';
        index += 1;
      }

      continue;
    }

    // A block comment, which nests in Rust: `depth` counts the openers, so a
    // `/* */` inside the comment does not end it early and leave its tail to be
    // read as code.
    if (character === '/' && source[index + 1] === '*') {
      let depth = 1;

      result += '  ';
      index += 2;

      while (index < source.length && depth > 0) {
        if (source[index] === '/' && source[index + 1] === '*') {
          depth += 1;
          result += '  ';
          index += 2;
        } else if (source[index] === '*' && source[index + 1] === '/') {
          depth -= 1;
          result += '  ';
          index += 2;
        } else {
          result += source[index] === '\n' ? '\n' : ' ';
          index += 1;
        }
      }

      continue;
    }

    const hashes = rawStringHashes(source, index);

    if (hashes >= 0) {
      // Copied verbatim: inside a raw string a `//` is data and only the
      // matching `"` + `#`-run ends it.
      const opener = source.indexOf('"', index) + 1;
      const terminator = `"${'#'.repeat(hashes)}`;
      const close = source.indexOf(terminator, opener);
      const end = close === -1 ? source.length : close + terminator.length;

      result += source.slice(index, end);
      index = end;
      continue;
    }

    if (character === '"') {
      let cursor = index + 1;

      while (cursor < source.length && source[cursor] !== '"') {
        cursor += source[cursor] === '\\' ? 2 : 1;
      }

      const end = Math.min(cursor + 1, source.length);

      result += source.slice(index, end);
      index = end;
      continue;
    }

    // A `'` is a character literal or a lifetime, and the reference lexer tells
    // them apart by what follows: `'\…'` escapes and `'x'` closes one character
    // later, while a lifetime (`'a`, `'static`) has no closing quote there. The
    // literal is copied — `'"'` included, whose quote would otherwise open a
    // "string" that swallows the rest of the file.
    if (
      character === "'" &&
      (source[index + 1] === '\\' || source[index + 2] === "'")
    ) {
      let cursor = index + 1;

      while (cursor < source.length && source[cursor] !== "'") {
        cursor += source[cursor] === '\\' ? 2 : 1;
      }

      const end = Math.min(cursor + 1, source.length);

      result += source.slice(index, end);
      index = end;
      continue;
    }

    result += character;
    index += 1;
  }

  return result;
}

/**
 * The members of `FORBIDDEN_NAMES` that `source`'s **code** contains.
 *
 * The file is passed in exactly as read and `stripRustComments` runs here rather
 * than at the call site, so the stripping cannot be dropped by a caller and
 * silently turn the assertion back into a whole-file search: the failure that
 * reintroduces is one doc comment away, and design.md D0.9's own prose shows how
 * naturally such a comment gets written.
 */
function forbiddenNamesInCode(source: string): readonly string[] {
  const code = stripRustComments(source);

  return FORBIDDEN_NAMES.filter((name) => code.includes(name));
}

describe('TOOL-3: bindings are generated reproducibly and drift fails the build', () => {
  it('Bindings export without launching the app', () => {
    const before = readFileSync(bindingsFile, 'utf8');
    const exportRun = runBunScript('gen:bindings');

    // `spawnSync` returns only when the child exits. An exporter that started
    // the app's event loop would hang to the timeout and raise through
    // `runBunScript` — so "returns at all" is the observable half of "needs no
    // running app". The committed file is the other: an empty command and event
    // set still produces a valid, non-empty module.
    expect(
      exportRun.status,
      `bun run gen:bindings did not exit successfully:\n${describeCommand(exportRun)}`,
    ).toBe(0);
    expect(readFileSync(bindingsFile, 'utf8').length).toBeGreaterThan(0);
    expect(before.length).toBeGreaterThan(0);

    // The other half of the scenario — "AND no application window is created" —
    // cannot be watched at runtime, so it is asserted against the source that
    // runs during an export. The entry point must be the headless exporter, and
    // neither it nor the IPC module that owns `export_bindings` may name an
    // app-run or window constructor **in its code** — the comments are blanked
    // first, so prose about the rule does not count as breaking it. A callee
    // deeper than these two files is out of textual reach; the Rust side keeps
    // the export free of an app handle so that gap stays closed (D0.9).
    const entryPoint = 'src-tauri/examples/export_bindings.rs';

    // Through the stripper as well: the entry point's own doc comment names
    // `app_lib::ipc::export_bindings`, so a raw read here would stay green after
    // the call itself was deleted — the same prose-for-code substitution the
    // scan below strips out.
    expect(
      stripRustComments(readRepositoryFile(entryPoint)),
      `${entryPoint} no longer calls app_lib::ipc::export_bindings, so the window assertion below would guard the wrong code`,
    ).toContain('app_lib::ipc::export_bindings');

    for (const path of [entryPoint, 'src-tauri/src/ipc/mod.rs']) {
      // `forbiddenNamesInCode` strips the comments before it looks, so the
      // assertion is about code: a doc comment naming `tauri::Builder` explains
      // the ban, it does not violate it (design.md D0.9's own prose does exactly
      // that, as does `ipc/mod.rs`'s note about `Builder::invoke_handler`).
      const found = forbiddenNamesInCode(readRepositoryFile(path));

      expect(
        found,
        `${path} names ${found.join(', ')} in its code; a headless export must not run the app or create a window (TOOL-3)`,
      ).toEqual([]);
    }
  }, 180_000);

  it('The empty command set produces a stable artefact', () => {
    const first = readFileSync(bindingsFile, 'utf8');
    const exportRun = runBunScript('gen:bindings');

    expect(
      exportRun.status,
      `bun run gen:bindings failed:\n${describeCommand(exportRun)}`,
    ).toBe(0);

    // Byte equality, not a git comparison: this is the scenario itself
    // ("exporting it a second time produces no diff") without depending on a
    // staged blob.
    expect(readFileSync(bindingsFile, 'utf8')).toBe(first);
  }, 180_000);

  it('A stale bindings file fails the build, and a hand edit is reverted by regeneration', () => {
    const original = readFileSync(bindingsFile, 'utf8');
    let handEdited = false;

    try {
      // Precondition: the committed file currently matches the source. Without
      // this, a non-zero exit after the marker would prove nothing about the
      // marker — the file could have been stale already.
      const clean = runGitDiff([BINDINGS_PATH]);

      expect(
        clean.status,
        `the bindings file is already modified before this test hand-edits it, so the drift assertion below would be ambiguous:\n${describeCommand(clean)}`,
      ).toBe(0);

      // The flag is set **before** the write: if the append itself fails (a
      // partial write, ENOSPC), the `finally` must still regenerate.
      handEdited = true;
      appendFileSync(bindingsFile, HAND_EDIT_MARKER);

      // "A stale bindings file fails the build": the gate's own check, run the
      // way `definition-of-done` runs it.
      const drift = runGitDiff([BINDINGS_PATH]);

      expect(
        drift.status,
        'git diff --exit-code src/bindings.ts exited 0 after a hand edit; the gate would not fail on drift',
      ).not.toBe(0);
      // "AND the failure names the bindings file": the printed diff does.
      expect(`${drift.stdout}${drift.stderr}`).toContain(BINDINGS_PATH);

      // "A hand edit is reverted by regeneration".
      const regenerated = runBunScript('gen:bindings');

      if (regenerated.status === 0) {
        // True generated output now stands in the file, so there is no
        // artificial edit left for the `finally` to undo.
        handEdited = false;
      }

      expect(
        regenerated.status,
        `bun run gen:bindings failed to regenerate after a hand edit:\n${describeCommand(regenerated)}`,
      ).toBe(0);

      expect(readFileSync(bindingsFile, 'utf8')).toBe(original);

      const cleanAgain = runGitDiff([BINDINGS_PATH]);

      expect(
        cleanAgain.status,
        `the bindings drift check still fails after regeneration:\n${describeCommand(cleanAgain)}`,
      ).toBe(0);
    } finally {
      // This test deliberately leaves the tree dirty in the middle, so the
      // restoration must happen on the failure path too: an assertion that
      // throws between the append and the regeneration would otherwise leave a
      // hand-edited generated file behind for a later check to trip over.
      if (handEdited) {
        try {
          if (runBunScript('gen:bindings').status !== 0) {
            // Last resort: the exporter itself is broken. Put back the bytes
            // read at the start — they are the committed content — so the tree
            // is clean even while the exporter failure is being reported.
            writeFileSync(bindingsFile, original);
          }
        } catch {
          writeFileSync(bindingsFile, original);
        }
      }
    }
  }, 180_000);
});
