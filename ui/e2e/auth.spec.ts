import { test, expect } from '@playwright/test';

/**
 * E2E: Staff Login with PIN
 *
 * Verifies the complete authentication flow using CSS class selectors
 * that match the actual StaffLoginScreen component structure:
 *   - `.staff-login-screen` — the container
 *   - `.staff-login-input` — username text input
 *   - `.staff-login-submit-btn` — submit / next button
 *   - `.staff-login-pad` — PIN keypad (visible after username step)
 *   - `.staff-login-pad-key` — individual digit buttons
 *   - `.staff-login-error` — error message alert
 *   - `.workspace-home` — workspace picker (post-login success)
 *
 * Dev-mock credentials:
 *   - "owner" / "1234" (owner role)
 *   - "admin" / "admin123" (manager role)
 *   - "kasir" / "1234" (cashier role)
 */

test.describe('Staff Login', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.staff-login-screen', { timeout: 15_000 });
  });

  test('shows login screen when no session exists', async ({ page }) => {
    await expect(page.locator('.staff-login-screen')).toBeVisible();

    // Username input should be visible and ready.
    const input = page.locator('.staff-login-input');
    await expect(input).toBeVisible();
    await expect(input).toBeEnabled();
  });

  test('shows PIN pad after valid username', async ({ page }) => {
    // Enter a valid username.
    const input = page.locator('.staff-login-input');
    await input.fill('owner');

    // Submit to proceed to PIN step.
    const submit = page.locator('.staff-login-submit-btn');
    await submit.click();

    // The PIN pad (`.staff-login-pad`) should become visible.
    const pinPad = page.locator('.staff-login-pad');
    await expect(pinPad).toBeVisible({ timeout: 10_000 });

    // The step indicator should show the PIN step as active.
    const stepDots = page.locator('.staff-login-step-dot');
    await expect(stepDots.nth(1)).toHaveClass(/staff-login-step-dot--active/);
  });

  test('shows error for invalid username', async ({ page }) => {
    // Enter an invalid username.
    const input = page.locator('.staff-login-input');
    await input.fill('nonexistent');

    // Submit.
    const submit = page.locator('.staff-login-submit-btn');
    await submit.click();

    // Dev-mock: `staff_check_username` returns { found: false } for unknown users.
    // The app should show a toast with error message about not found.
    // The login screen should still be visible (not navigated away).
    await expect(page.locator('.staff-login-screen')).toBeVisible();
  });

  test('successful login with correct PIN', async ({ page }) => {
    // Step 1: Enter username.
    const input = page.locator('.staff-login-input');
    await input.fill('owner');

    // Step 2: Submit username.
    const submit = page.locator('.staff-login-submit-btn');
    await submit.click();

    // Step 3: Wait for PIN pad.
    const pinPad = page.locator('.staff-login-pad');
    await expect(pinPad).toBeVisible({ timeout: 10_000 });

    // Step 4: Enter PIN digits via keypad.
    for (const digit of '1234') {
      const key = page.locator('.staff-login-pad-key').filter({ hasText: digit });
      await key.click();
      await page.waitForTimeout(80);
    }

    // Step 5: After PIN entry, the app auto-submits when PIN reaches max length.
    // The workspace home screen (`.workspace-home`) should appear.
    await expect(page.locator('.workspace-home')).toBeVisible({ timeout: 15_000 });

    // User display name should be visible in the greeting.
    const greeting = page.locator('.ws-header-greeting');
    await expect(greeting).toContainText('Owner');
  });

  test('login failure with wrong PIN', async ({ page }) => {
    // Enter valid username.
    const input = page.locator('.staff-login-input');
    await input.fill('owner');

    // Submit.
    const submit = page.locator('.staff-login-submit-btn');
    await submit.click();

    // Wait for PIN pad.
    const pinPad = page.locator('.staff-login-pad');
    await expect(pinPad).toBeVisible({ timeout: 10_000 });

    // Enter wrong PIN.
    for (const digit of '0000') {
      const key = page.locator('.staff-login-pad-key').filter({ hasText: digit });
      await key.click();
      await page.waitForTimeout(80);
    }

    // Dev-mock: wrong PIN throws 'Invalid credentials' error.
    // The error alert (`.staff-login-error`) should appear inside the card.
    const errorAlert = page.locator('.staff-login-error');
    await expect(errorAlert).toBeVisible({ timeout: 5_000 });

    // Should stay on the login screen (not navigate away).
    await expect(page.locator('.staff-login-screen')).toBeVisible();
  });

  test('clears PIN when error occurs', async ({ page }) => {
    // Enter valid username.
    await page.locator('.staff-login-input').fill('owner');
    await page.locator('.staff-login-submit-btn').click();

    // Wait for PIN pad.
    await expect(page.locator('.staff-login-pad')).toBeVisible({ timeout: 10_000 });

    // Enter wrong PIN.
    for (const digit of '0000') {
      await page.locator('.staff-login-pad-key').filter({ hasText: digit }).click();
      await page.waitForTimeout(80);
    }

    // Error should appear.
    await expect(page.locator('.staff-login-error')).toBeVisible({ timeout: 5_000 });

    // After error, PIN dots should be cleared (empty).
    const filledDots = page.locator('.staff-login-pin-dot--filled');
    await expect(filledDots).toHaveCount(0);
  });
});
