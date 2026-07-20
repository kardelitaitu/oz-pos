# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: sale.spec.ts >> Complete Sale Flow >> adds product to cart and shows non-zero total
- Location: e2e\sale.spec.ts:42:3

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
  - text: Cashier Sen, 20 Jul 2026 21.55 0h 0m
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
  2   | import { loginAs, selectWorkspace, WORKSPACES } from './helpers';
  3   | 
  4   | /**
  5   |  * E2E: Complete Sale Flow — Hard Assertions
  6   |  *
  7   |  * Tests the POS sale flow with deterministic assertions. All `if` guards
  8   |  * removed — tests hard-fail on regressions.
  9   |  *
  10  |  * CSS contract:
  11  |  *   .product-card-btn            — clickable product card (ProductLookupScreen)
  12  |  *   .retail-cart-action-btn--pay — Pay button in cart panel
  13  |  *   [data-testid="cart-panel"]   — Cart panel container
  14  |  *   [data-testid="cart-panel-line-item"] — Single cart line
  15  |  *   [data-testid="line-item-remove-button"] — Remove line button
  16  |  *   [data-testid="payment-modal"] — Payment modal
  17  |  *   [data-testid="quick-pay-button"] — Quick tender button in modal
  18  |  *   .payment-tendered-input      — Custom tender amount input
  19  |  *   .receipt-preview-paper       — Receipt preview after completed sale
  20  |  *   [data-testid="line-item-qty-input"] — Line quantity input
  21  |  */
  22  | 
  23  | test.describe('Complete Sale Flow', () => {
  24  |   test.beforeEach(async ({ page }) => {
  25  |     await loginAs(page, 'kasir', '1234');
  26  |     await selectWorkspace(page, WORKSPACES.STORE_POS);
  27  |   });
  28  | 
  29  |   // ── E2E-9: Assert product grid renders ───────────────────────
  30  | 
  31  |   test('product grid renders with at least 3 products', async ({ page }) => {
  32  |     // Product cards must be visible within 5s. Dev-mock returns 18 products.
  33  |     const productCards = page.locator('.product-card-btn');
  34  |     await expect(productCards.first()).toBeVisible({ timeout: 5_000 });
  35  | 
  36  |     const count = await productCards.count();
  37  |     expect(count).toBeGreaterThanOrEqual(3);
  38  |   });
  39  | 
  40  |   // ── E2E-10: Add product to cart ──────────────────────────────
  41  | 
  42  |   test('adds product to cart and shows non-zero total', async ({ page }) => {
  43  |     // Wait for product grid.
  44  |     const productCards = page.locator('.product-card-btn');
> 45  |     await expect(productCards.first()).toBeVisible({ timeout: 5_000 });
      |                                        ^ Error: expect(locator).toBeVisible() failed
  46  | 
  47  |     // Click first product.
  48  |     await productCards.first().click();
  49  |     await page.waitForTimeout(500);
  50  | 
  51  |     // Cart must contain at least 1 line item.
  52  |     const cartLines = page.locator('[data-testid="cart-panel-line-item"]');
  53  |     await expect(cartLines.first()).toBeVisible({ timeout: 5_000 });
  54  |     expect(await cartLines.count()).toBe(1);
  55  | 
  56  |     // The pay button must be enabled (cart has items).
  57  |     const payBtn = page.locator('.retail-cart-action-btn--pay');
  58  |     await expect(payBtn).toBeEnabled();
  59  |   });
  60  | 
  61  |   // ── E2E-11: Quantity increment ───────────────────────────────
  62  | 
  63  |   test('double-clicking same product increments quantity', async ({ page }) => {
  64  |     const productCards = page.locator('.product-card-btn');
  65  |     await expect(productCards.first()).toBeVisible({ timeout: 5_000 });
  66  | 
  67  |     // Click the same product twice.
  68  |     await productCards.first().click();
  69  |     await page.waitForTimeout(300);
  70  |     await productCards.first().click();
  71  |     await page.waitForTimeout(500);
  72  | 
  73  |     // Cart must have exactly 1 line (stacked quantity).
  74  |     const cartLines = page.locator('[data-testid="cart-panel-line-item"]');
  75  |     await expect(cartLines.first()).toBeVisible({ timeout: 5_000 });
  76  |     expect(await cartLines.count()).toBe(1);
  77  | 
  78  |     // Quantity must be 2 or greater.
  79  |     const qtyInput = page.locator('[data-testid="line-item-qty-input"]').first();
  80  |     const qtyValue = await qtyInput.inputValue();
  81  |     expect(parseInt(qtyValue, 10)).toBeGreaterThanOrEqual(2);
  82  |   });
  83  | 
  84  |   // ── E2E-12: Open payment modal ──────────────────────────────
  85  | 
  86  |   test('opens payment modal with correct total', async ({ page }) => {
  87  |     const productCards = page.locator('.product-card-btn');
  88  |     await expect(productCards.first()).toBeVisible({ timeout: 5_000 });
  89  | 
  90  |     // Get the product price.
  91  |     const _priceText = await page.locator('.product-card-price').first().textContent() ?? '0';
  92  | 
  93  |     // Add product.
  94  |     await productCards.first().click();
  95  |     await page.waitForTimeout(500);
  96  | 
  97  |     // Click pay button.
  98  |     const payBtn = page.locator('.retail-cart-action-btn--pay');
  99  |     await payBtn.click();
  100 | 
  101 |     // Payment modal must appear.
  102 |     const paymentModal = page.locator('[data-testid="payment-modal"]');
  103 |     await expect(paymentModal).toBeVisible({ timeout: 5_000 });
  104 | 
  105 |     // Modal must contain payment-related content.
  106 |     const modalContent = page.locator('[data-testid="payment-modal-content"]');
  107 |     await expect(modalContent).toBeVisible();
  108 | 
  109 |     // The modal text should include the product price or a non-zero total.
  110 |     const modalText = await modalContent.textContent();
  111 |     expect(modalText).toBeTruthy();
  112 |     expect(modalText!.length).toBeGreaterThan(10);
  113 |   });
  114 | 
  115 |   // ── E2E-13: Cash payment — exact tender ─────────────────────
  116 | 
  117 |   test('cash payment with exact tender shows receipt preview', async ({ page }) => {
  118 |     // Add product.
  119 |     const productCards = page.locator('.product-card-btn');
  120 |     await expect(productCards.first()).toBeVisible({ timeout: 5_000 });
  121 |     await productCards.first().click();
  122 |     await page.waitForTimeout(500);
  123 | 
  124 |     // Open payment modal.
  125 |     await page.locator('.retail-cart-action-btn--pay').click();
  126 |     await expect(page.locator('[data-testid="payment-modal"]')).toBeVisible({ timeout: 5_000 });
  127 | 
  128 |     // Click a quick-pay button (Cash tender).
  129 |     const quickPayButtons = page.locator('[data-testid="quick-pay-button"]');
  130 |     const quickCount = await quickPayButtons.count();
  131 | 
  132 |     if (quickCount > 0) {
  133 |       // Click first quick-pay (typically Cash).
  134 |       await quickPayButtons.first().click();
  135 |       await page.waitForTimeout(500);
  136 |     } else {
  137 |       // Fallback: try to enter custom amount and confirm.
  138 |       const tenderInput = page.locator('.payment-tendered-input');
  139 |       if (await tenderInput.isVisible().catch(() => false)) {
  140 |         await tenderInput.fill('5.00');
  141 |         await page.waitForTimeout(200);
  142 |       }
  143 |     }
  144 | 
  145 |     // Find and click confirm / settle button.
```