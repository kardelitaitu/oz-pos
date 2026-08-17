import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

// Website unit/regression tests. Node environment by default (SSR render
// tests, pure logic); storage/browser-dependent tests opt into jsdom with a
// `// @vitest-environment jsdom` pragma.
//
// These are the regression net for the SSR-flash + session-staleness bug
// classes: tests render the auth forms server-side, assert the "not
// configured" fallback can never appear in first-paint HTML, and pin the
// clearSession / initialize-once behavior (see src/components/__tests__).
//
// The react() plugin is required because astro/tsconfigs/strict sets
// `jsx: preserve`, which Vite's import analysis refuses to transform on its
// own (same plugin ui/ uses for its .tsx tests).
export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'node',
    include: ['src/**/*.test.{ts,tsx}'],
  },
});
