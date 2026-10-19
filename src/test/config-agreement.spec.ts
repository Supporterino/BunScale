/*
 * Task 0.14's rule: "the dev-server port and the build output directory MUST
 * match the `--dev-url` and `--frontend-dist` values task 0.1 passed to
 * `tauri init` (change whichever side is wrong, never let the two drift)"
 * (task 0.24, TOOL-1/TOOL-2).
 *
 * Two files state the same two facts and nothing reconciles them but a human,
 * which is exactly what rots. The port is taken out of
 * `src-tauri/tauri.conf.json`'s `build.devUrl` and the directory out of
 * `build.frontendDist`; `vite.config.ts` must agree, and `server.host` must
 * still be wired to `process.env.TAURI_DEV_HOST` (without it the iOS simulator
 * and LAN devices cannot reach the dev server).
 *
 * `vite.config.ts` is read as text, not imported: importing it would execute
 * the router plugin in the test process and would resolve `server.host` through
 * this process's environment, turning an assertion about the wiring into an
 * assertion about the environment. `config-agreement.ts` states what the
 * textual read can and cannot see.
 *
 * The third statement of this address — `playwright.config.ts`'s `baseURL` and
 * `webServer.url` — is the E2E side of the same value and belongs to task 0.25
 * (design.md D0.24).
 */
import { describe, expect, it } from 'vitest';
import { checkConfigAgreement } from './config-agreement';
import { readRepositoryFile } from './scaffold-files';

describe('task 0.14: the two configs never drift', () => {
  it('The dev-server address and build output directory agree', () => {
    const issues = checkConfigAgreement(
      readRepositoryFile('src-tauri/tauri.conf.json'),
      readRepositoryFile('vite.config.ts'),
    );

    // An empty list is the pass; a non-empty one names the side that disagrees
    // and the value each side holds.
    expect(issues, 'tauri.conf.json and vite.config.ts disagree').toEqual([]);
  });
});
