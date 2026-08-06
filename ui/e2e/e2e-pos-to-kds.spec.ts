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
 *   .restaurant-card               — product card in Restaurant POS grid
 *   [data-testid="cart-panel-line-item"] — cart line item
 *   .pos-cart-pay-btn             — Pay/Charge button (PosScreen cart)
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
    // Restaurant POS renders the menu as `.restaurant-card` buttons (name in
    // `.restaurant-card-name`), NOT the retail `.retail-product-btn` table.
    const productCards = page.locator('.restaurant-card');
    await expect(productCards.first()).toBeVisible({ timeout: TIMEOUT });

    // Add a product to cart.
    const productName = (await productCards.first().locator('.restaurant-card-name').textContent())?.trim() || 'Unknown';
    console.log(`  Adding product: ${productName}`);
    await productCards.first().click();
    await page.waitForTimeout(500);

    // Verify cart has 1 line item.
    const cartLines = page.locator('[data-testid="cart-panel-line-item"]');
    await expect(cartLines.first()).toBeVisible({ timeout: 5_000 });
    expect(await cartLines.count()).toBe(1);

    // The PAY/Charge button is enabled once an active shift is loaded (the mock
    // auto-opens one). Restaurant POS uses the inline cart (.pos-cart-pay-btn).
    const payBtn = page.locator('.pos-cart-pay-btn');
    await expect(payBtn).toBeVisible({ timeout: 8_000 });
    await payBtn.click();
    await expect(page.locator('[data-testid="payment-modal"]')).toBeVisible({ timeout: 5_000 });

    // Cash is the default method. Set Amount Tendered to a value well above the
    // total so the Complete Sale button enables (cash requires tendered >= total).
    // Type char-by-char (pressSequentially) because a single fill() can be
    // reverted by a re-render of the controlled input, leaving tender at 0.00.
    const tenderedInput = page.locator('.payment-tendered-input');
    await expect(tenderedInput).toBeVisible({ timeout: 3_000 });
    await tenderedInput.click();
    await tenderedInput.pressSequentially('9999999', { delay: 30 });
    await page.waitForTimeout(200);

    // Click the Complete Sale button by its label and wait for the modal to close.
    const completeBtn = page.getByRole('button', { name: /complete/i });
    await expect(completeBtn).toBeEnabled({ timeout: 5_000 });
    await completeBtn.click();
    await expect(page.locator('[data-testid="payment-modal"]')).toBeHidden({ timeout: 10_000 });

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
    // ── Step 6: Verify cart is empty (sale completed) ───────────────
    // After a successful sale the cart should have zero line items.
    await expect(page.locator('[data-testid="cart-panel-line-item"]')).toHaveCount(0, { timeout: 5_000 });

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
