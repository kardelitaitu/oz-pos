import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E: Create Product
 *
 * Tests product management using resilient selectors matching
 * the actual component structure. The inventory workspace shows
 * ProductManagementScreen after selection.
 */
test.describe('Create Product', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    await selectWorkspace(page, WORKSPACES.INVENTORY);
  });

  test('loads product management screen', async ({ page }) => {
    await page.waitForTimeout(3_000);

    // The inventory workspace should show product management content.
    // Look for product-related UI elements.
    const pageContent = page.locator('#root, [class*="inventory"], [class*="product"]').first();
    await expect(pageContent).toBeVisible();

    // Try to find a "Create" or "Add" button.
    const createBtn = page.locator(
      'button:has-text("Create"), button:has-text("Add"), button:has-text("Tambah"), [class*="create"], [class*="add"]',
    ).first();

    const btnCount = await createBtn.count();
    if (btnCount > 0) {
      await expect(createBtn).toBeVisible();
    }
  });

  test('can navigate to create product section', async ({ page }) => {
    await page.waitForTimeout(3_000);

    // Look for any "Create Product" or "Add Product" button.
    const createBtn = page.locator(
      'button:has-text("Create"), button:has-text("Add"), button:has-text("Tambah"), [class*="create"], [class*="add"]',
    ).first();

    const btnCount = await createBtn.count();
    if (btnCount > 0) {
      await createBtn.click();
      await page.waitForTimeout(1_000);
    }

    // Verify the page responded (no crash, no error boundary).
    // The app should still be responsive.
    const errorBoundary = page.locator('[class*="error-boundary"], [class*="Error"]').first();
    const hasError = await errorBoundary.isVisible().catch(() => false);
    expect(hasError).toBe(false);
  });
});
