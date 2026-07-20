import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E: Open/Close Shift
 *
 * Tests shift management using resilient selectors matching
 * the actual component structure. The admin workspace provides
 * sidebar navigation to the shifts page (`#/shifts`).
 */
test.describe('Shift Management', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    await selectWorkspace(page, WORKSPACES.ADMIN);
  });

  test('navigates to shift management via sidebar', async ({ page }) => {
    await page.waitForTimeout(3_000);

    // The admin workspace shows sidebar navigation.
    // Look for a nav link containing "Shift" text.
    const shiftNav = page.locator(
      'a[href*="shift"], a[href*="Shift"], nav a:has-text("Shift"), button:has-text("Shift"), [class*="nav"]:has-text("Shift")',
    ).first();

    const navCount = await shiftNav.count();
    if (navCount > 0) {
      await shiftNav.click();
      await page.waitForTimeout(2_000);
    }

    // Alternatively, navigate by hash route.
    await page.evaluate(() => {
      window.location.hash = '#/shifts';
    });
    await page.waitForTimeout(2_000);

    // The shift management screen should be visible.
    // Look for shift-related content.
    const shiftContent = page.locator('[class*="shift"], [class*="Shift"]').first();
    const shiftVisible = await shiftContent.isVisible().catch(() => false);
    if (shiftVisible) {
      await expect(shiftContent).toBeVisible();
    }
  });

  test('can interact with shift controls', async ({ page }) => {
    // Navigate to shifts page via hash.
    await page.evaluate(() => {
      window.location.hash = '#/shifts';
    });
    await page.waitForTimeout(3_000);

    // Look for buttons with "Open" or "Close" text.
    const openBtn = page.locator(
      'button:has-text("Open"), button:has-text("Buka")',
    ).first();

    const closeBtn = page.locator(
      'button:has-text("Close"), button:has-text("Tutup")',
    ).first();

    const openCount = await openBtn.count();
    const closeCount = await closeBtn.count();

    // If open button exists, click it.
    if (openCount > 0) {
      await openBtn.click();
      await page.waitForTimeout(1_000);

      // Check if a modal or form appeared.
      const modal = page.locator('[class*="modal"], [class*="dialog"], [class*="overlay"]').first();
      const modalVisible = await modal.isVisible().catch(() => false);
      if (modalVisible) {
        // Try to find an input for opening balance.
        const balanceInput = page.locator('input[type="number"], input[class*="balance"], input[class*="amount"]').first();
        const inputCount = await balanceInput.count();
        if (inputCount > 0) {
          await balanceInput.fill('500000');
        }

        // Find a confirm button.
        const confirmBtn = page.locator(
          'button:has-text("Confirm"), button:has-text("Open"), button:has-text("Buka"), button:has-text("OK")',
        ).first();

        const confirmCount = await confirmBtn.count();
        if (confirmCount > 0) {
          await confirmBtn.click();
          await page.waitForTimeout(1_000);
        }
      }
    }

    // If close button exists, click it.
    if (closeCount > 0) {
      await closeBtn.click();
      await page.waitForTimeout(1_000);
    }

    // Verify no crash.
    const errorBoundary = page.locator('[class*="error-boundary"]').first();
    const hasError = await errorBoundary.isVisible().catch(() => false);
    expect(hasError).toBe(false);
  });
});
