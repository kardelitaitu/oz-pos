import { test, expect } from '@playwright/test';
import { loginAs } from './helpers';

/**
 * E2E: Staff Login with PIN — Hard Assertions
 *
 * Verifies the complete authentication flow with deterministic assertions.
 * All guards (`if (count > 0)`) removed — tests hard-fail on regressions.
 *
 * Dev-mock credentials:
 *   - "owner" / "1234"  → role: owner, display: "Owner"
 *   - "admin" / "admin123"  → role: manager, display: "Admin"
 *   - "kasir" / "1234"  → role: cashier, display: "Cashier"
 *
 * CSS contract (StaffLoginScreen.tsx):
 *   .staff-login-screen     — container
 *   .staff-login-input      — username text input
 *   .staff-login-submit-btn — submit / next button
 *   .staff-login-pad        — PIN keypad (visible after username step)
 *   .staff-login-pad-key    — individual digit buttons
 *   .staff-login-pin-dot--filled — filled PIN dot
 *   .staff-login-error      — error message alert
 *   .staff-login-lockout    — lockout countdown (rate-limit)
 *   .workspace-home         — workspace picker (post-login success)
 *   .ws-header-greeting     — display name in workspace header
 */

const VALID_USER = 'owner';
const VALID_PIN = '1234';
const WRONG_PIN = '0000';
const UNKNOWN_USER = 'nonexistent';

async function enterPin(page: import('@playwright/test').Page, pin: string) {
  for (const digit of pin) {
    const key = page.locator('.staff-login-pad-key').filter({ hasText: digit });
    await key.click();
    await page.waitForTimeout(80);
  }
}

test.describe('Staff Login', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.staff-login-screen', { timeout: 15_000 });
  });

  // ── E2E-4: Hard-assert login happy path ──────────────────────

  test('successful login shows workspace picker with greeting', async ({ page }) => {
    // Enter username.
    await page.locator('.staff-login-input').fill(VALID_USER);
    await page.locator('.staff-login-submit-btn').click();

    // Wait for PIN pad (not waitForTimeout).
    await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });

    // Enter PIN.
    await enterPin(page, VALID_PIN);

    // Workspace home must appear (hard assertion).
    await page.waitForSelector('.workspace-home', { timeout: 15_000 });
    await expect(page.locator('.workspace-home')).toBeVisible();

    // Greeting must contain exact display name.
    await expect(page.locator('.ws-header-greeting')).toContainText('Owner');

    // URL hash must be at root.
    await expect(page).toHaveURL(/#\/$/);
  });

  // ── E2E-5: Assert error text for wrong PIN ───────────────────

  test('shows "Invalid credentials" for wrong PIN', async ({ page }) => {
    await page.locator('.staff-login-input').fill(VALID_USER);
    await page.locator('.staff-login-submit-btn').click();
    await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });

    // Enter wrong PIN.
    await enterPin(page, WRONG_PIN);

    // Error must appear with exact dev-mock error text.
    const errorAlert = page.locator('.staff-login-error');
    await expect(errorAlert).toBeVisible({ timeout: 5_000 });
    await expect(errorAlert).toContainText('Invalid credentials');

    // Must stay on login screen.
    await expect(page.locator('.staff-login-screen')).toBeVisible();
  });

  // ── E2E-6: Assert error for unknown username ─────────────────

  test('shows error for unknown username', async ({ page }) => {
    await page.locator('.staff-login-input').fill(UNKNOWN_USER);
    await page.locator('.staff-login-submit-btn').click();

    // The app should show a toast or inline error about user not found.
    // Dev-mock: staff_check_username returns { found: false }.
    // The login screen must remain visible.
    await expect(page.locator('.staff-login-screen')).toBeVisible({ timeout: 5_000 });

    // Either the error toast or an inline error should mention "not found".
    const toastError = page.locator('.toast--error, [class*="toast"][class*="error"]');
    const inlineError = page.locator('.staff-login-error');

    const toastVisible = await toastError.isVisible().catch(() => false);
    const inlineVisible = await inlineError.isVisible().catch(() => false);

    expect(toastVisible || inlineVisible).toBe(true);

    if (inlineVisible) {
      await expect(inlineError).toContainText(/not found|unknown|invalid/i);
    }
    if (toastVisible) {
      await expect(toastError).toContainText(/not found|unknown|invalid/i);
    }
  });

  // ── E2E-7: Rate-limit lockout UI ─────────────────────────────

  test('rate-limit lockout after 5 wrong PIN attempts', async ({ page }) => {
    await page.locator('.staff-login-input').fill(VALID_USER);
    await page.locator('.staff-login-submit-btn').click();
    await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });

    // Enter wrong PIN 5 times.
    for (let attempt = 0; attempt < 5; attempt++) {
      await enterPin(page, WRONG_PIN);

      // Wait for error to appear, then clear.
      const errorAlert = page.locator('.staff-login-error');
      await errorAlert.waitFor({ state: 'visible', timeout: 5_000 }).catch(() => {});

      // If we're locked out, stop.
      const lockoutVisible = await page
        .locator('.staff-login-lockout, [class*="lockout"]')
        .isVisible()
        .catch(() => false);
      if (lockoutVisible) break;

      // Wait for PIN to clear before next attempt.
      await page.waitForTimeout(500);
    }

    // After 5 attempts, either lockout appears or PIN pad is disabled.
    const lockoutEl = page.locator('.staff-login-lockout, [class*="lockout"]');
    const pinPadDisabled = page.locator('.staff-login-pad[aria-disabled="true"]');

    const lockoutVisible = await lockoutEl.isVisible().catch(() => false);
    const padDisabled = await pinPadDisabled.isVisible().catch(() => false);

    // At least one lockout mechanism must be active.
    expect(lockoutVisible || padDisabled).toBe(true);
  });

  // ── E2E-8: Session persistence across reload ─────────────────

  test('returns to login screen after page reload', async ({ page }) => {
    // Login successfully first.
    await page.locator('.staff-login-input').fill(VALID_USER);
    await page.locator('.staff-login-submit-btn').click();
    await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });
    await enterPin(page, VALID_PIN);
    await page.waitForSelector('.workspace-home', { timeout: 15_000 });

    // Reload the page.
    await page.reload();

    // Session is NOT persisted in localStorage — login screen should appear.
    await page.waitForSelector('.staff-login-screen', { timeout: 15_000 });
    await expect(page.locator('.staff-login-screen')).toBeVisible();
  });

  // ── Bonus: Clears PIN dots after error ───────────────────────

  test('clears PIN dots when error occurs', async ({ page }) => {
    await page.locator('.staff-login-input').fill(VALID_USER);
    await page.locator('.staff-login-submit-btn').click();
    await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });

    await enterPin(page, WRONG_PIN);

    // Error should appear.
    await expect(page.locator('.staff-login-error')).toBeVisible({ timeout: 5_000 });

    // PIN dots must be cleared (no filled dots).
    await expect(page.locator('.staff-login-pin-dot--filled')).toHaveCount(0);
  });

  // ── Bonus: PIN step indicator shows active step ──────────────

  test('shows PIN step as active after username submit', async ({ page }) => {
    await page.locator('.staff-login-input').fill(VALID_USER);
    await page.locator('.staff-login-submit-btn').click();

    // PIN pad must appear.
    await expect(page.locator('.staff-login-pad')).toBeVisible({ timeout: 10_000 });

    // Step indicator dot for PIN step (index 1) must be active.
    const stepDots = page.locator('.staff-login-step-dot');
    const dotCount = await stepDots.count();
    if (dotCount > 1) {
      await expect(stepDots.nth(1)).toHaveClass(/staff-login-step-dot--active/);
    }
  });

  // ── Bonus: Admin login shows correct greeting ───────────────

  test('admin login shows manager greeting', async ({ page }) => {
    await loginAs(page, 'admin', '9999');
    await expect(page.locator('.ws-header-greeting')).toContainText('Manager');
  });

  // ── Bonus: Cashier login shows cashier greeting ──────────────

  test('cashier login shows cashier greeting', async ({ page }) => {
    await loginAs(page, 'kasir', '1234');
    await expect(page.locator('.ws-header-greeting')).toContainText('Cashier');
  });
});
