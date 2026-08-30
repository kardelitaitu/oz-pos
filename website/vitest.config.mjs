import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

// Website unit/regression tests. Node environment by default (SSR render
// tests, pure logic); storage/browser-dependent tests opt into jsdom with a
// `// @vitest-environment jsdom` pragma.
//
// These are the regression net for the SSR-flash bug class: tests render the
// auth forms server-side and assert the "not configured" fallback can never
// appear in the first paint HTML (see src/components/__tests__/ssr-flash).
//
// The react() plugin is required because astro/tsconfigs/strict sets
// `jsx: preserve`, which Vite's import analysis refuses to transform on its
// own (same plugin ui/ uses for its .tsx tests).
//
// Thread pool: on the 7950X (32 logical cores) we use up to 24 workers so
// the OS + IDE keep 8 threads headroom. vitest's default is ceil(cpus/2)
// which on 32 cores = 16. Bumping to 24 shaves ~30% off the 623-test run.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'node',
    include: ['src/**/*.test.{ts,tsx}'],
    pool: 'threads',
    maxThreads: parseInt(process.env.VITEST_MAX_THREADS ?? '24', 10),
    minThreads: parseInt(process.env.VITEST_MIN_THREADS ?? '4', 10),
  },
});
