/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath, URL } from 'node:url';

// Tauri expects a fixed port; fail if it isn't available.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],

  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
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
