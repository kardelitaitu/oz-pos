/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath, URL } from 'node:url';

// Tauri expects a fixed port; fail if it isn't available.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],

  resolve: {
    // Use the explicit regex form for the `@/` alias (instead of the
    // shorthand `'@': path`). The shorthand is honored by Vite's
    // dev/build resolver but NOT by Vitest 1.6.x's pool-worker
    // resolver — imports like `@/components/Badge` fall through to
    // Node's ESM resolver and fail with `Cannot find package`.
    // Trailing slashes on both `./src/` and `replacement` keep the
    // path join clean (`…/foo`, never `…/srccomponents/foo`).
    alias: [
      {
        find: /^@\//,
        replacement: `${fileURLToPath(new URL('./src/', import.meta.url))}/`,
      },
    ],
  },

  // Vite options tailored for Tauri development.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: 'ws', host, port: 1421 }
      : undefined,
    watch: {
      // Tell vite to ignore watching `apps/` so the Rust change
      // watcher doesn't trigger a Vite reload.
      ignored: ['**/apps/**'],
    },
  },

  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test-setup.ts'],
    css: false,
    dangerouslyIgnoreUnhandledErrors: true,
    onConsoleLog(log, _type) {
      if (log.includes('[@fluent/react]') && log.includes('did not match any messages')) {
        return false;
      }
      if (log.includes('was not wrapped in act') || log.includes('flushSync was called from inside')) {
        return false;
      }
      if (log.includes('validateDOMNesting') || log.includes('punycode module is deprecated')) {
        return false;
      }
    },




    // ── Coverage ────────────────────────────────────────────────────────
    //
    // Run with `npm run test:coverage` (or `vitest run --coverage`).
    // Uses the v8 provider (native to Node; faster than istanbul).
    // HTML + JSON reports land in `../coverage/ui/` so they sit beside
    // the Rust coverage report produced by `scripts/coverage.{sh,ps1}`.
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'json', 'json-summary', 'lcov'],
      reportsDirectory: '../coverage/ui',
      // Only count source files — never the test code itself.
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        '**/node_modules/**',
        '**/*.test.{ts,tsx}',
        '**/__tests__/**',
        '**/test-setup.ts',
        '**/locales/test-utils.tsx',
        // Type-only modules (Fluent locale bundles are just strings).
        '**/locales/**',
      ],
    },
  },
});
