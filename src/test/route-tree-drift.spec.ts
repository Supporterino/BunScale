/*
 * TOOL-4: "The route tree is generated reproducibly and drift fails the build"
 * (task 0.24).
 *
 * The scenario asserted here is "Route generation is reproducible": read the
 * committed tree, run the app's build (the only step that runs the router
 * plugin), read it again and require the bytes to be identical.
 *
 * Cost and side effects, stated rather than discovered:
 * - `bun run build` is a full Vite build (≈0.1–1 s on this machine). It is the
 *   only honest way to regenerate the tree, because the generator lives in the
 *   app's Vite config and `vitest.config.ts` deliberately leaves it out.
 * - it writes `dist/` and `.tanstack/tmp/`, both gitignored by task 0.19. That
 *   is accepted: the alternative is not running the generator at all.
 * - if the tree were stale, the build would rewrite it and the assertion would
 *   fail **after** the rewrite. This file therefore never leaves an artificial
 *   edit behind (it makes none); on a red run the file holds the correct
 *   generated output that a developer must review and commit.
 *
 * Why TOOL-4's other scenario — "A stale route tree fails the build" — is not
 * asserted here: the gate checks it as `bun run build` followed by
 * `git diff --exit-code src/routeTree.gen.ts`. Because the build regenerates
 * the file, a hand-edited tree is silently repaired *before* the check runs, so
 * the scenario that actually bites is "a file route changed and the regenerated
 * tree was not committed". Asserting that needs a mutation under `src/routes/`,
 * which is outside this task's edit scope; task 0.15 demonstrated the check once
 * it became meaningful and task 0.26 runs it on the full gate.
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  describeCommand,
  REPOSITORY_ROOT,
  runBunScript,
} from './scaffold-files';

describe('TOOL-4: the route tree is generated reproducibly', () => {
  it('Route generation is reproducible', () => {
    const routeTree = resolve(REPOSITORY_ROOT, 'src/routeTree.gen.ts');
    const before = readFileSync(routeTree, 'utf8');
    const build = runBunScript('build');

    expect(
      build.status,
      `bun run build failed, so the route tree was not regenerated:\n${describeCommand(build)}`,
    ).toBe(0);

    expect(
      readFileSync(routeTree, 'utf8'),
      'the build regenerated src/routeTree.gen.ts to different bytes; the committed tree is stale and the route-tree drift check would fail',
    ).toBe(before);
  }, 180_000);
});
