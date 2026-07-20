import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E: Inventory Workflows — Stock Counts, Transfers, Purchase Orders
 *
 * Covers inventory operations with zero prior E2E coverage.
 * All tests use hard assertions — no soft guards.
 *
 * Routes (all accessible from Inventory workspace):
 *   #/stock-counts      → StockCountsScreen (.sc-screen)
 *   #/stock-transfers   → StockTransfersScreen (.stock-transfers)
 *   #/purchase-orders   → PurchaseOrdersScreen (.po-screen)
 *   #/suppliers         → SuppliersScreen (.suppliers-screen)
 *
 * Mock data: all lists return empty arrays — screens render
 * with empty-state placeholders.
 */

async function navigateTo(page: import('@playwright/test').Page, hash: string) {
  await page.evaluate((h) => {
    window.location.hash = h;
  }, hash);
  // Auto-wait: each test's toBeVisible() already waits for DOM + visibility.
}

test.describe('Inventory Workflows', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'admin', '9999');
    await selectWorkspace(page, WORKSPACES.INVENTORY);
  });

  // ── Stock Counts ────────────────────────────────────────

  test('stock counts screen renders container', async ({ page }) => {
    await navigateTo(page, '#/stock-counts');

    await expect(page.locator('.sc-screen')).toBeVisible({ timeout: 8_000 });
  });

  // ── Stock Transfers ─────────────────────────────────────

  test('stock transfers screen renders with title', async ({ page }) => {
    await navigateTo(page, '#/stock-transfers');

    await expect(page.locator('.stock-transfers')).toBeVisible({ timeout: 8_000 });
    await expect(page.locator('.stock-transfers-title')).toContainText('Stock Transfer');
  });

  // ── Purchase Orders ─────────────────────────────────────

  test('purchase orders screen renders container', async ({ page }) => {
    await navigateTo(page, '#/purchase-orders');

    await expect(page.locator('.po-screen')).toBeVisible({ timeout: 8_000 });
  });

  // ── Suppliers ───────────────────────────────────────────

  test('suppliers screen renders with table', async ({ page }) => {
    await navigateTo(page, '#/suppliers');

    await expect(page.locator('.suppliers-screen')).toBeVisible({ timeout: 8_000 });
    await expect(page.locator('.suppliers-title')).toContainText('Supplier');

    // Table must render (even if empty).
    await expect(page.locator('.suppliers-table')).toBeVisible({ timeout: 5_000 });
  });
});
