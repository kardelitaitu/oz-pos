# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: pos-workflows.spec.ts >> POS Workflows >> sales history screen loads and renders container
- Location: e2e\pos-workflows.spec.ts:28:3

# Error details

```
TimeoutError: page.waitForSelector: Timeout 10000ms exceeded.
Call log:
  - waiting for locator('.retail-cart-action-btn--pay') to be visible

```

# Page snapshot

```yaml
- generic [ref=e2]:
  - generic [ref=e4]:
    - banner [ref=e5]:
      - generic [ref=e7]:
        - text: TOKO TEST
        - generic [ref=e8]: · Cabang A
        - text: Jl. Contoh No. 123
      - generic [ref=e9]:
        - generic [ref=e10]: Shift · IDR 0,00
        - button "Kembali ke ruang kerja" [ref=e11] [cursor=pointer]:
          - img [ref=e12]
        - generic [ref=e14]:
          - img [ref=e15]
          - generic [ref=e17]: Cashier
        - generic [ref=e18]:
          - generic [ref=e19]: Sen, 20 Jul 2026
          - generic [ref=e20]: "21.54"
          - generic [ref=e21]: 0h 0m
    - generic [ref=e22]:
      - generic [ref=e23]:
        - generic [ref=e24]:
          - button "Semua Kategori" [ref=e25] [cursor=pointer]
          - button "Cold Drinks" [pressed] [ref=e26] [cursor=pointer]
          - button "Hot Drinks" [ref=e27] [cursor=pointer]
          - button "Food" [ref=e28] [cursor=pointer]
          - button "Snacks" [ref=e29] [cursor=pointer]
        - generic [ref=e30]:
          - img [ref=e31]
          - textbox "Cari produk…" [ref=e34]
        - generic [ref=e35]: Tidak ada produk
        - generic [ref=e36]:
          - generic [ref=e37]: SKU
          - textbox "Scan atau ketik barcode / SKU" [ref=e38]
          - button "CARI" [ref=e39] [cursor=pointer]
      - separator "retail-resize-handle-aria" [ref=e40]
      - generic [ref=e41]:
        - generic [ref=e42]:
          - generic [ref=e43]: Keranjang
          - generic [ref=e44]: 0 item
        - generic [ref=e45]:
          - img [ref=e46]
          - generic [ref=e49]: Keranjang kosong
    - toolbar "Bilah fungsi" [ref=e50]:
      - button "F1 Bayar" [disabled] [ref=e51]:
        - generic [ref=e52]: F1
        - text: Bayar
      - button "F2 Batal" [disabled] [ref=e53]:
        - generic [ref=e54]: F2
        - text: Batal
      - button "F3 Diskon" [disabled] [ref=e55]:
        - generic [ref=e56]: F3
        - text: Diskon
      - button "F4 Tahan" [disabled] [ref=e57]:
        - generic [ref=e58]: F4
        - text: Tahan
      - button "F5 Cari" [ref=e59] [cursor=pointer]:
        - generic [ref=e60]: F5
        - text: Cari
      - button "F6 Riwayat" [ref=e61] [cursor=pointer]:
        - generic [ref=e62]: F6
        - text: Riwayat
      - button "F7 Pelanggan" [ref=e63] [cursor=pointer]:
        - generic [ref=e64]: F7
        - text: Pelanggan
      - button "F8 Stok" [ref=e65] [cursor=pointer]:
        - generic [ref=e66]: F8
        - text: Stok
      - button "F9 Tutup Shift" [ref=e67] [cursor=pointer]:
        - generic [ref=e68]: F9
        - text: Tutup Shift
      - button "F10 Opsi" [disabled] [ref=e69]:
        - generic [ref=e70]: F10
        - text: Opsi
      - button "F12 Tampilan Dapur" [ref=e71] [cursor=pointer]:
        - generic [ref=e72]: F12
        - text: Tampilan Dapur
  - toolbar "Developer tools" [ref=e73]:
    - generic [ref=e75]: DevTools
    - generic [ref=e76]:
      - paragraph [ref=e77]: Theme
      - radiogroup "Theme selector" [ref=e78]:
        - radio "Glass theme" [checked] [ref=e79] [cursor=pointer]:
          - img [ref=e80]
          - generic [ref=e84]: Glass
        - radio "Light theme" [ref=e85] [cursor=pointer]:
          - img [ref=e86]
          - generic [ref=e92]: Light
        - radio "Dark theme" [ref=e93] [cursor=pointer]:
          - img [ref=e94]
          - generic [ref=e96]: Dark
      - generic [ref=e98]: Glass
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
> 29  |     await page.waitForSelector('.retail-cart-action-btn--pay', { timeout: 10_000 });
      |                ^ TimeoutError: page.waitForSelector: Timeout 10000ms exceeded.
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
  75  |     await expect(productCards.first()).toBeVisible({ timeout: 5_000 });
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