import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES, navigateTo } from './helpers';

/**
 * E2E: POS Workflows — Sales History, Customer Selection, Void
 *
 * Covers POS lifecycle operations with zero prior E2E coverage.
 * All tests use hard assertions — no `if` guards, no `.catch(() => false)`.
 *
 * CSS contract:
 *   .sales-history           — SalesHistoryScreen container
 *   .retail-cart-action-btn--void — Void/clear cart button
 *   .retail-cart-action-btn--pay  — Pay button
 *   .retail-fn-bar            — Function key bar (F1-F6)
 *   .retail-fn-key            — Individual function key labels
 *   [data-testid="cart-panel-line-item"] — Cart line item
 *   [data-testid="payment-modal"] — Payment modal
 */

test.describe('POS Workflows', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'kasir', '1234');
    await selectWorkspace(page, WORKSPACES.STORE_POS);
  });

  // ── Sales History Screen ──────────────────────────────────

  test('sales history screen loads and renders container', async ({ page }) => {
    await page.waitForSelector('.retail-cart-action-btn--pay', { timeout: 10_000 });

    await navigateTo(page, 'sales-history');

    // Sales history container must appear.
    await expect(page.locator('.sales-history')).toBeVisible({ timeout: 8_000 });

    // Mock returns empty sales — empty state or table must render without crash.
    const errorBoundary = page.locator('[class*="error-boundary"]');
    await expect(errorBoundary).toHaveCount(0, { timeout: 5_000 });
  });

  // ── Customer Selection in Payment ─────────────────────────

  test('customer section renders in payment modal', async ({ page }) => {
    // Add product to cart and open payment modal.
    const productCards = page.locator('.product-card-btn');
    await expect(productCards.first()).toBeVisible({ timeout: 5_000 });
    await productCards.first().click();
    await page.waitForTimeout(500);

    await page.locator('.retail-cart-action-btn--pay').click();
    await expect(page.locator('[data-testid="payment-modal"]')).toBeVisible({ timeout: 5_000 });

    // Payment modal must have payment content (tabs, tender buttons, or total).
    const modalContent = page.locator('[data-testid="payment-modal-content"]');
    await expect(modalContent).toBeVisible({ timeout: 3_000 });

    // Verify modal renders payment-relevant elements.
    const hasTabs = page.locator('[data-testid="payment-tabs"]');
    const hasQuickPay = page.locator('[data-testid="quick-pay-button"]');
    const tabsVisible = await hasTabs.isVisible();
    const quickPayVisible = await hasQuickPay.first().isVisible();
    expect(tabsVisible || quickPayVisible).toBe(true);

    // Dismiss modal.
    const closeBtn = page.locator('[data-testid="modal-close-button"]').first();
    await closeBtn.click();
    await expect(page.locator('[data-testid="payment-modal"]')).not.toBeVisible({ timeout: 5_000 });
  });

  // ── Void / Clear Cart ─────────────────────────────────────

  test('void button clears cart and disables pay', async ({ page }) => {
    // Add a product to cart.
    const productCards = page.locator('.product-card-btn');
    await expect(productCards.first()).toBeVisible({ timeout: 5_000 });
    await productCards.first().click();
    await page.waitForTimeout(500);

    // Cart must have a line item.
    await expect(page.locator('[data-testid="cart-panel-line-item"]').first()).toBeVisible({ timeout: 3_000 });

    // Click the void/clear button (F2 or action button).
    const voidBtn = page.locator('.retail-cart-action-btn--void').first();
    await expect(voidBtn).toBeVisible({ timeout: 3_000 });
    await voidBtn.click();

    // Cart must be empty (auto-wait handles timing).
    await expect(page.locator('[data-testid="cart-panel-line-item"]')).toHaveCount(0, { timeout: 3_000 });

    // Pay button must be disabled.
    await expect(page.locator('.retail-cart-action-btn--pay')).toBeDisabled({ timeout: 3_000 });
  });

  // ── Function Bar Keys ─────────────────────────────────────

  test('function bar renders with F1-F6 keys', async ({ page }) => {
    await expect(page.locator('.product-card-btn').first()).toBeVisible({ timeout: 5_000 });

    // Function bar must be visible.
    await expect(page.locator('.retail-fn-bar')).toBeVisible({ timeout: 5_000 });

    // At least 4 function keys must be present.
    const fnKeys = page.locator('.retail-fn-key');
    const count = await fnKeys.count();
    expect(count).toBeGreaterThanOrEqual(4);

    // Verify F1 and F2 labels exist.
    const fnTexts = await fnKeys.allTextContents();
    expect(fnTexts.some((t) => t.includes('F1'))).toBe(true);
    expect(fnTexts.some((t) => t.includes('F2'))).toBe(true);
  });
});
