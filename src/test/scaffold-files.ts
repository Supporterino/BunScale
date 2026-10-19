/*
 * Shared plumbing for the scaffold-artefact specs (task 0.24).
 *
 * These specs assert properties of files the gate also checks — the two
 * generated artefacts, the two manifests and the token sheets — rather than of
 * a rendered component, so they need two things `vitest.config.ts` does not
 * supply: the repository root (a spec's cwd is not a contract) and a way to run
 * the gate's own commands as child processes.
 *
 * Why the commands are run the way they are:
 *
 * - `gen:bindings` is run through **`bun run gen:bindings`**, not by repeating
 *   its `cargo run --example` body here. The requirement is about the gate's
 *   behaviour, and the script is the gate's command; a second copy of the cargo
 *   invocation could drift from `package.json` and the test would still pass.
 * - `build` is run through `bun run build` for the same reason: the route tree
 *   is rendered by the router plugin inside the app's Vite config, which only
 *   the script reaches.
 * - `git diff --exit-code` is run **exactly as `definition-of-done` runs it**
 *   (no `--` separator) so the exit status and the printed diff are the gate's
 *   own. `git` is the one external binary these specs depend on; CI checks the
 *   repository out with it.
 *
 * `REPOSITORY_ROOT` is derived from this module's URL rather than from
 * `process.cwd()`, so a spec passes or fails on the repository it lives in, not
 * on where the runner happened to start.
 */
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** The repository root, from `src/test/` — never from the working directory. */
export const REPOSITORY_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '..',
  '..',
);

/** The change record whose D0.8 tables are the manifests' source of truth. */
export const DESIGN_RECORD_PATH =
  'openspec/changes/add-shared-household-foundation/design.md';

export type CommandResult = {
  /** `null` when the process was killed by a signal (for example a timeout). */
  readonly status: number | null;
  readonly stdout: string;
  readonly stderr: string;
};

function run(
  command: string,
  args: readonly string[],
  timeoutMs: number,
): CommandResult {
  const result = spawnSync(command, args, {
    cwd: REPOSITORY_ROOT,
    encoding: 'utf8',
    timeout: timeoutMs,
    // `GIT_PAGER` keeps a user's pager config out of a piped run; `NO_COLOR`
    // keeps ANSI escapes out of the messages a failing assertion prints.
    env: { ...process.env, GIT_PAGER: 'cat', NO_COLOR: '1' },
  });

  if (result.error) {
    throw new Error(
      `could not run \`${command} ${args.join(' ')}\`: ${result.error.message}`,
    );
  }

  return {
    status: result.status,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

/** Runs one of `package.json`'s scripts, from the repository root. */
export function runBunScript(
  script: string,
  timeoutMs = 180_000,
): CommandResult {
  return run('bun', ['run', script], timeoutMs);
}

/** Runs the gate's no-diff check over the given paths. */
export function runGitDiff(
  paths: readonly string[],
  timeoutMs = 30_000,
): CommandResult {
  return run('git', ['diff', '--exit-code', ...paths], timeoutMs);
}

/** Reads a repository-relative file as UTF-8. */
export function readRepositoryFile(relativePath: string): string {
  return readFileSync(resolve(REPOSITORY_ROOT, relativePath), 'utf8');
}

/** Reads an absolute path as UTF-8; for files discovered by directory listing. */
export function readAbsoluteFile(absolutePath: string): string {
  return readFileSync(absolutePath, 'utf8');
}

/** A command's streams as one string, for an assertion's failure message. */
export function describeCommand(result: CommandResult): string {
  return [
    `exit status ${String(result.status)}`,
    result.stdout.trim() === '' ? '' : `stdout:\n${result.stdout}`,
    result.stderr.trim() === '' ? '' : `stderr:\n${result.stderr}`,
  ]
    .filter((part) => part !== '')
    .join('\n');
}
