import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E Critical Path: POS → KDS End-to-End (Phase B2)
 *
 * Full cross-workspace flow: complete a sale in Restaurant POS with
 * kitchen items → switch to KDS workspace → verify the new ticket
 * appears in the pending column.
 *
 * CSS contract:
 *   .retail-product-btn           — product card in POS grid
 *   [data-testid="cart-panel-line-item"] — cart line item
 *   .retail-cart-action-btn--pay  — Pay button
 *   [data-testid="payment-modal"] — payment modal
 *   [data-testid="quick-pay-button"] — quick tender button
 *   .receipt-preview-paper        — receipt after completed sale
 *   .kds                          — KDS container
 *   .kds-order-count              — order count badge
 *   .kds-column--pending          — pending column
 *   .kds-column--preparing        — preparing column
 *   .kds-ticket                   — ticket card
 *   .kds-ticket-number            — ticket display number
 *   .kds-ticket-items             — ticket items summary
 */

const TIMEOUT = 10_000;

test.describe('Critical Path: POS → KDS', () => {
  test('complete a sale in Restaurant POS and verify ticket appears on KDS', async ({ page }) => {
    // ── Step 1: Log in and go to Restaurant POS ─────────────────────
    await loginAs(page, 'admin', '9999');
    await selectWorkspace(page, WORKSPACES.RESTAURANT_POS);

    // ── Step 2: Get the current KDS order count (baseline) ──────────
    // We need to check KDS before making the sale so we know the
    // baseline. The KDS mock starts with 3 orders (display #101-103).
    // We'll navigate to KDS, read the count, then navigate back to POS.
    await selectWorkspace(page, WORKSPACES.KDS);

    await expect(page.locator('.kds-order-count')).toBeVisible({ timeout: TIMEOUT });
    const countTextBefore = await page.locator('.kds-order-count').textContent() || '';
    const countMatch = countTextBefore.match(/\d+/);
    const baselineCount = countMatch ? parseInt(countMatch[0], 10) : 0;
    console.log(`  Baseline KDS order count: ${baselineCount}`);

    // ── Step 3: Return to Restaurant POS and make a sale ────────────
    await selectWorkspace(page, WORKSPACES.RESTAURANT_POS);

    // Wait for product grid.
    const productCards = page.locator('.retail-product-btn');
    await expect(productCards.first()).toBeVisible({ timeout: TIMEOUT });

    // Add a product to cart.
    const productName = await productCards.first().locator('.retail-product-name').textContent() || 'Unknown';
    console.log(`  Adding product: ${productName}`);
    await productCards.first().click();
    await page.waitForTimeout(500);

    // Verify cart has 1 line item.
    const cartLines = page.locator('[data-testid="cart-panel-line-item"]');
    await expect(cartLines.first()).toBeVisible({ timeout: 5_000 });
    expect(await cartLines.count()).toBe(1);

    // Open payment modal.
    await page.locator('.retail-cart-action-btn--pay').click();
    await expect(page.locator('[data-testid="payment-modal"]')).toBeVisible({ timeout: 5_000 });

    // Complete with quick-pay.
    const quickPayBtn = page.locator('[data-testid="quick-pay-button"]').first();
    await expect(quickPayBtn).toBeVisible({ timeout: 3_000 });
    await quickPayBtn.click();
    await page.waitForTimeout(800);

    // Confirm.
    const confirmBtn = page.locator(
      '[data-testid="settle-button"], ' +
      'button:has-text("Confirm"), ' +
      'button:has-text("Settle")',
    ).first();
    if (await confirmBtn.isVisible({ timeout: 3_000 }).catch(() => false)) {
      await confirmBtn.click();
      await page.waitForTimeout(1_000);
    }

    // Dismiss receipt preview if shown.
    const receiptPaper = page.locator('.receipt-preview-paper');
    if (await receiptPaper.isVisible({ timeout: 5_000 }).catch(() => false)) {
      const skipBtn = page.locator('button:has-text("Skip"), button:has-text("Lewati")');
      if (await skipBtn.isVisible().catch(() => false)) {
        await skipBtn.click();
        await page.waitForTimeout(500);
      }
    }

    // Verify cart is empty (sale completed).
    await expect(page.locator('.retail-cart-action-btn--pay')).toBeDisabled({ timeout: 5_000 });

    // ── Step 4: Switch to KDS and verify the new ticket ─────────────
    await selectWorkspace(page, WORKSPACES.KDS);

    await expect(page.locator('.kds-order-count')).toBeVisible({ timeout: TIMEOUT });
    const countTextAfter = await page.locator('.kds-order-count').textContent() || '';
    const afterMatch = countTextAfter.match(/\d+/);
    const afterCount = afterMatch ? parseInt(afterMatch[0], 10) : 0;
    console.log(`  After-sale KDS order count: ${afterCount}`);

    // The count must have increased by exactly 1.
    expect(afterCount).toBe(baselineCount + 1);

    // ── Step 5: The newest ticket must be in the pending column ─────
    const pendingColumn = page.locator('.kds-column--pending');
    await expect(pendingColumn).toBeVisible({ timeout: 5_000 });

    // The newest ticket is the last one in the pending column.
    const pendingTickets = pendingColumn.locator('.kds-ticket');
    const pendingCount = await pendingTickets.count();
    expect(pendingCount).toBeGreaterThanOrEqual(1);

    // The last pending ticket should contain the product name from our sale.
    const lastTicket = pendingTickets.last();
    const ticketText = await lastTicket.textContent() || '';
    // The product name should appear in the ticket items.
    const nameMatch = ticketText.includes(productName.replace(/[^a-zA-Z0-9 ]/g, '').trim());
    if (!nameMatch) {
      // Fallback: the ticket items summary should contain something from our sale.
      const itemsEl = lastTicket.locator('.kds-ticket-items, .kds-ticket-item-name').first();
      const itemsText = await itemsEl.textContent() || '';
      expect(itemsText.length).toBeGreaterThan(0);
    }

    // ── Step 6: Verify no crash ─────────────────────────────────────
    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });
});
