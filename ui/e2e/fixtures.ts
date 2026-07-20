import { test as base, type Page } from '@playwright/test';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
 * E2E fixtures providing pre-authenticated page objects via Playwright
 * `storageState`. The auth fixture performs a full login once per worker
 * and serializes localStorage to disk, so every subsequent test that uses
 * `loggedInPage` starts already authenticated (~3s saving per test).
 *
 * Usage:
 *   import { test } from '../fixtures';
 *   test('my test', async ({ loggedInPage }) => { ... });
 */

const AUTH_FILE = path.join(__dirname, '..', '.e2e-auth.json');

export type LoggedInFixture = {
  /**
   * A page that is already authenticated as the given role.
   * The page starts at the workspace picker (WorkspaceHome).
   */
  loggedInPage: Page;
};

/**
 * Extended test with the `loggedInPage` fixture.
 *
 * The fixture performs login once per worker using storageState.
 * The first test in the worker runs the login flow; all subsequent
 * tests reuse the serialized state.
 */
export const test = base.extend<LoggedInFixture>({
  loggedInPage: [
    async ({ browser }, use) => {
      // Create a new context that tries to load saved auth state.
      const context = await browser.newContext({
        storageState: AUTH_FILE,
      });
      const page = await context.newPage();

      // Check if we're already logged in (workspace home visible).
      await page.goto('/');
      const alreadyLoggedIn = await page
        .locator('.workspace-home')
        .isVisible({ timeout: 3_000 })
        .catch(() => false);

      if (!alreadyLoggedIn) {
        // Perform fresh login.
        await page.goto('/');
        await page.waitForSelector('.staff-login-screen', { timeout: 15_000 });

        // Enter username.
        const usernameInput = page.locator('.staff-login-input').first();
        await usernameInput.fill('kasir');

        // Submit.
        await page.locator('.staff-login-submit-btn').click();

        // Wait for PIN pad.
        await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });

        // Enter PIN.
        for (const digit of '1234') {
          const key = page.locator('.staff-login-pad-key').filter({ hasText: digit });
          await key.click();
          await page.waitForTimeout(80);
        }

        // Wait for workspace home.
        await page.waitForSelector('.workspace-home', { timeout: 15_000 });

        // Save the auth state for subsequent tests.
        await page.context().storageState({ path: AUTH_FILE });
      }

      await use(page);
      await context.close();
    },
    { scope: 'worker' },
  ],
});

export { expect } from '@playwright/test';
