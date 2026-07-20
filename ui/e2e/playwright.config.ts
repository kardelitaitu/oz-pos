import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for OZ-POS E2E tests.
 *
 * Tests run against the Vite dev server (port 1420) which serves the
 * React app with mocked Tauri IPC (`dev-mock/tauri-api.ts`).  No Rust
 * backend is needed for browser-based UI tests.
 *
 * API-level integration tests (api.spec.ts) target the cloud-server
 * running via Docker Compose (`docker-compose.e2e.yml`).
 *
 * Usage:
 *   # Start the Vite dev server first:
 *   cd ui && npm run dev
 *
 *   # In another terminal, run tests:
 *   cd ui && npx playwright test --config e2e/playwright.config.ts
 *
 *   # With the Playwright UI mode for debugging:
 *   cd ui && npx playwright test --config e2e/playwright.config.ts --ui
 *
 *   # Headed mode (watch the browser):
 *   cd ui && npx playwright test --config e2e/playwright.config.ts --headed
 */
export default defineConfig({
  // Look for test files in the e2e directory.
  testDir: '.',

  // Fail the build on CI if you leave test.only in the source code.
  forbidOnly: !!process.env['CI'],

  // Retry twice on CI to reduce flaky-test noise.
  retries: process.env['CI'] ? 2 : 0,

  // Run all tests in parallel (up to 4 workers).
  workers: process.env['CI'] ? 2 : 4,

  // Each test gets 30 seconds to finish.
  timeout: 30_000,

  // Reporters: list output in terminal, produce JSON + HTML on CI.
  reporter: process.env['CI']
    ? [['list'], ['json', { outputFile: 'e2e-results/results.json' }], ['html', { outputFolder: 'e2e-results/html' }]]
    : [['list'], ['html', { open: 'never' }]],

  // Shared base URL — override with BASE_URL env var for custom dev ports.
  use: {
    baseURL: process.env['BASE_URL'] ?? 'http://localhost:1420',
    // Collect trace on first failure (screenshots + DOM snapshots).
    trace: 'retain-on-failure',
    // Capture screenshot on failure for debugging.
    screenshot: 'only-on-failure',
  },

  // Auto-start the Vite dev server (no more manual second terminal).
  // In CI, always start fresh; locally, reuse an existing server if running.
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env['CI'],
    timeout: 120_000,
    cwd: '..',
  },

  // Configure projects for desktop and tablet viewports.
  projects: [
    {
      name: 'desktop',
      use: {
        ...devices['Desktop Chrome'],
        // 1366×768 is a common POS terminal resolution.
        viewport: { width: 1366, height: 768 },
      },
    },
    {
      name: 'tablet',
      use: {
        ...devices['iPad Pro 11'],
        // 1024×1366 portrait — typical tablet POS orientation.
        viewport: { width: 1024, height: 1366 },
      },
    },
  ],
});
