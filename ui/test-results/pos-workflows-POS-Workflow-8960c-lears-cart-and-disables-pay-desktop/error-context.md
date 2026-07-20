# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: pos-workflows.spec.ts >> POS Workflows >> void button clears cart and disables pay
- Location: e2e\pos-workflows.spec.ts:72:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('.product-card-btn').first()
Expected: visible
Timeout: 5000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 5000ms
  - waiting for locator('.product-card-btn').first()

```

```yaml
- banner:
  - text: TOKO TEST · Cabang AJl. Contoh No. 123 Shift · IDR 0,00
  - button "Kembali ke ruang kerja"
  - text: Cashier Sen, 20 Jul 2026 21.54 0h 0m
- button "Semua Kategori"
- button "Cold Drinks" [pressed]
- button "Hot Drinks"
- button "Food"
- button "Snacks"
- textbox "Cari produk…"
- text: Tidak ada produk SKU
- textbox "Scan atau ketik barcode / SKU"
- button "CARI"
- separator "retail-resize-handle-aria"
- text: Keranjang 0 item Keranjang kosong
- toolbar "Bilah fungsi":
  - button "F1 Bayar" [disabled]
  - button "F2 Batal" [disabled]
  - button "F3 Diskon" [disabled]
  - button "F4 Tahan" [disabled]
  - button "F5 Cari"
  - button "F6 Riwayat"
  - button "F7 Pelanggan"
  - button "F8 Stok"
  - button "F9 Tutup Shift"
  - button "F10 Opsi" [disabled]
  - button "F12 Tampilan Dapur"
- toolbar "Developer tools":
  - text: DevTools
  - paragraph: Theme
  - radiogroup "Theme selector":
    - radio "Glass theme" [checked]: Glass
    - radio "Light theme": Light
    - radio "Dark theme": Dark
  - text: Glass
```

# Test source

```ts
  1   | import { test, expect } from '@playwright/test';
  2   | import { loginAs, selectWorkspace, WORKSPACES, navigateTo } from './helpers';
  3   | 
  4   | /**
  5   |  * E2E: POS Workflows — Sales History, Customer Selection, Void
  6   |  *
  7   |  * Covers POS lifecycle operations with zero prior E2E coverage.
  8   |  * All tests use hard assertions — no `if` guards, no `.catch(() => false)`.
  9   |  *
  10  |  * CSS contract:
  11  |  *   .sales-history           — SalesHistoryScreen container
  12  |  *   .retail-cart-action-btn--void — Void/clear cart button
  13  |  *   .retail-cart-action-btn--pay  — Pay button
  14  |  *   .retail-fn-bar            — Function key bar (F1-F6)
  15  |  *   .retail-fn-key            — Individual function key labels
  16  |  *   [data-testid="cart-panel-line-item"] — Cart line item
  17  |  *   [data-testid="payment-modal"] — Payment modal
  18  |  */
  19  | 
  20  | test.describe('POS Workflows', () => {
  21  |   test.beforeEach(async ({ page }) => {
  22  |     await loginAs(page, 'kasir', '1234');
  23  |     await selectWorkspace(page, WORKSPACES.STORE_POS);
  24  |   });
  25  | 
  26  |   // ── Sales History Screen ──────────────────────────────────
  27  | 
  28  |   test('sales history screen loads and renders container', async ({ page }) => {
  29  |     await page.waitForSelector('.retail-cart-action-btn--pay', { timeout: 10_000 });
  30  | 
  31  |     await navigateTo(page, 'sales-history');
  32  | 
  33  |     // Sales history container must appear.
  34  |     await expect(page.locator('.sales-history')).toBeVisible({ timeout: 8_000 });
  35  | 
  36  |     // Mock returns empty sales — empty state or table must render without crash.
  37  |     const errorBoundary = page.locator('[class*="error-boundary"]');
  38  |     await expect(errorBoundary).toHaveCount(0, { timeout: 5_000 });
  39  |   });
  40  | 
  41  |   // ── Customer Selection in Payment ─────────────────────────
  42  | 
  43  |   test('customer section renders in payment modal', async ({ page }) => {
  44  |     // Add product to cart and open payment modal.
  45  |     const productCards = page.locator('.product-card-btn');
  46  |     await expect(productCards.first()).toBeVisible({ timeout: 5_000 });
  47  |     await productCards.first().click();
  48  |     await page.waitForTimeout(500);
  49  | 
  50  |     await page.locator('.retail-cart-action-btn--pay').click();
  51  |     await expect(page.locator('[data-testid="payment-modal"]')).toBeVisible({ timeout: 5_000 });
  52  | 
  53  |     // Payment modal must have payment content (tabs, tender buttons, or total).
  54  |     const modalContent = page.locator('[data-testid="payment-modal-content"]');
  55  |     await expect(modalContent).toBeVisible({ timeout: 3_000 });
  56  | 
  57  |     // Verify modal renders payment-relevant elements.
  58  |     const hasTabs = page.locator('[data-testid="payment-tabs"]');
  59  |     const hasQuickPay = page.locator('[data-testid="quick-pay-button"]');
  60  |     const tabsVisible = await hasTabs.isVisible();
  61  |     const quickPayVisible = await hasQuickPay.first().isVisible();
  62  |     expect(tabsVisible || quickPayVisible).toBe(true);
  63  | 
  64  |     // Dismiss modal.
  65  |     const closeBtn = page.locator('[data-testid="modal-close-button"]').first();
  66  |     await closeBtn.click();
  67  |     await expect(page.locator('[data-testid="payment-modal"]')).not.toBeVisible({ timeout: 5_000 });
  68  |   });
  69  | 
  70  |   // ── Void / Clear Cart ─────────────────────────────────────
  71  | 
  72  |   test('void button clears cart and disables pay', async ({ page }) => {
  73  |     // Add a product to cart.
  74  |     const productCards = page.locator('.product-card-btn');
> 75  |     await expect(productCards.first()).toBeVisible({ timeout: 5_000 });
      |                                        ^ Error: expect(locator).toBeVisible() failed
  76  |     await productCards.first().click();
  77  |     await page.waitForTimeout(500);
  78  | 
  79  |     // Cart must have a line item.
  80  |     await expect(page.locator('[data-testid="cart-panel-line-item"]').first()).toBeVisible({ timeout: 3_000 });
  81  | 
  82  |     // Click the void/clear button (F2 or action button).
  83  |     const voidBtn = page.locator('.retail-cart-action-btn--void').first();
  84  |     await expect(voidBtn).toBeVisible({ timeout: 3_000 });
  85  |     await voidBtn.click();
  86  | 
  87  |     // Cart must be empty (auto-wait handles timing).
  88  |     await expect(page.locator('[data-testid="cart-panel-line-item"]')).toHaveCount(0, { timeout: 3_000 });
  89  | 
  90  |     // Pay button must be disabled.
  91  |     await expect(page.locator('.retail-cart-action-btn--pay')).toBeDisabled({ timeout: 3_000 });
  92  |   });
  93  | 
  94  |   // ── Function Bar Keys ─────────────────────────────────────
  95  | 
  96  |   test('function bar renders with F1-F6 keys', async ({ page }) => {
  97  |     await expect(page.locator('.product-card-btn').first()).toBeVisible({ timeout: 5_000 });
  98  | 
  99  |     // Function bar must be visible.
  100 |     await expect(page.locator('.retail-fn-bar')).toBeVisible({ timeout: 5_000 });
  101 | 
  102 |     // At least 4 function keys must be present.
  103 |     const fnKeys = page.locator('.retail-fn-key');
  104 |     const count = await fnKeys.count();
  105 |     expect(count).toBeGreaterThanOrEqual(4);
  106 | 
  107 |     // Verify F1 and F2 labels exist.
  108 |     const fnTexts = await fnKeys.allTextContents();
  109 |     expect(fnTexts.some((t) => t.includes('F1'))).toBe(true);
  110 |     expect(fnTexts.some((t) => t.includes('F2'))).toBe(true);
  111 |   });
  112 | });
  113 | 
```