import { tanstackRouter } from '@tanstack/router-plugin/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

/*
 * `tauri ios dev`/`tauri android dev` export TAURI_DEV_HOST when the app has to
 * reach the dev server over the network (an iOS device; a simulator only when
 * the CLI decides it needs the public address). It is unset for macOS runs and
 * for simulator runs that can use localhost, where leaving `host` undefined
 * keeps Vite's default. See `@tauri-apps/cli` CHANGELOG 2.0.0-beta.24 /
 * tauri-apps/tauri#10437.
 */
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [
    // File-based routing: renders src/routeTree.gen.ts from src/routes/**. It
    // has to run before the React plugin so route files are rewritten (code
    // splitting, route HMR) before fast refresh inspects them.
    tanstackRouter({
      target: 'react',
      routesDirectory: './src/routes',
      generatedRouteTree: './src/routeTree.gen.ts',
      autoCodeSplitting: true,
    }),
    react(),
  ],

  server: {
    // MUST equal `build.devUrl` in src-tauri/tauri.conf.json
    // (http://localhost:1420). The two sides are reconciled only by hand.
    port: 1420,
    // The Tauri CLI waits for exactly that URL. If 1420 is busy, Vite's default
    // would silently move to 1421 and the window would load a dead address, so
    // fail loudly instead.
    strictPort: true,
    // Reachable from the iOS simulator and from devices on the LAN.
    host,
    watch: {
      // Rust build output and generator artefacts, not frontend input: watching
      // them would restart the dev server on every cargo write.
      ignored: ['**/src-tauri/**'],
    },
  },

  build: {
    // MUST equal `build.frontendDist` in src-tauri/tauri.conf.json ("../dist",
    // i.e. `dist` relative to this file). The two sides are reconciled only by
    // hand.
    outDir: 'dist',
    // The deployment floor is iOS 15.0 / macOS 12.0 (design.md D0.16), whose
    // WebKit implements ES2021. Vite 8's default target
    // ('baseline-widely-available': Safari 16.4+) would emit syntax that floor
    // cannot parse.
    target: 'es2021',
  },

  // Rust compile errors are the point of the Tauri dev loop; do not let Vite
  // wipe them off the terminal.
  clearScreen: false,
});
