# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: retail-workflows.spec.ts >> Retail & Management Screens >> Tables screen renders floor plan
- Location: e2e\retail-workflows.spec.ts:33:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('.tables')
Expected: visible
Timeout: 8000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 8000ms
  - waiting for locator('.tables')

```

```yaml
- banner:
  - text: Pengaturan
  - textbox "settings-sidebar-search-aria":
    - /placeholder: Search
  - text: Mon, Jul 20, 2026 09:55 PM
  - button "Simpan pengaturan": Simpan
- complementary "Navigasi pengaturan":
  - button "Tutup semua kategori"
  - button "Tutup bilah sisi pengaturan"
  - navigation:
    - button "Bisnis 2" [expanded]
    - button "Umum"
    - button "Tampilan"
    - button "Operasional 2"
    - button "Sistem 4"
    - button "Manajemen 9"
- main:
  - button "Bisnis": Bisnis ›
  - heading "Umum" [level=1]
  - heading "Toko" [level=2]
  - text: Nama toko
  - textbox "Nama toko":
    - /placeholder: OZ-POS Store
    - text: TOKO TEST
  - text: Alamat
  - textbox "Alamat":
    - /placeholder: 123 Main Street
    - text: Jl. Contoh No. 123
  - text: NPWP
  - textbox "NPWP":
    - /placeholder: 12-3456789
    - text: TAX-001
  - text: Bahasa
  - combobox "Bahasa":
    - option "English"
    - option "Bahasa Indonesia" [selected]
    - option "ไทย"
  - button "language-selector-select-aria": Bahasa Indonesia
  - heading "Mata Uang" [level=2]
  - text: Mata uang default
  - combobox "Mata uang default":
    - option "IDR — Indonesian Rupiah (Rp)" [selected]
    - option "USD — US Dollar ($)"
    - option "JPY — Japanese Yen (¥)"
  - button "Mata uang default": IDR — Indonesian Rupiah (Rp)
- contentinfo:
  - button "Alihkan ke mode terang"
  - text: OZ-POS Enterprise v0.0.9 Ctrl + S Simpan Proprietary
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
  5   |  * E2E: Retail Workflows — Tables, Gift Cards, Kiosk, Customers, Categories, Loyalty
  6   |  *
  7   |  * Covers 6 management/retail screens with zero prior E2E coverage.
  8   |  * All tests use hard assertions — no soft guards, no dead code.
  9   |  *
  10  |  * CSS contract per screen:
  11  |  *   Tables:       .tables, .tables-title, .tables-floorplan
  12  |  *   Gift Cards:   .gift-cards-page, .gift-cards-title, .gift-cards-list
  13  |  *   Kiosk:        .kiosk, .kiosk-attract / .kiosk-grid, .kiosk-product-card
  14  |  *   Customers:    .customer-mgmt, .customer-mgmt-title, .customer-mgmt-table
  15  |  *   Categories:   .cat-mgmt, .cat-mgmt-title, .cat-mgmt-grid
  16  |  *   Loyalty:      .loyalty-mgmt, .loyalty-mgmt-title, .loyalty-table
  17  |  *
  18  |  * Routes (registered in App.tsx):
  19  |  *   tables, kiosk, customers, categories, gift-cards, loyalty
  20  |  */
  21  | 
  22  | const SCREEN_TIMEOUT = 8_000;
  23  | 
  24  | test.describe('Retail & Management Screens', () => {
  25  |   test.beforeEach(async ({ page }) => {
  26  |     // Admin login gives manager role — required for gift-cards, loyalty, categories.
  27  |     await loginAs(page, 'admin', '9999');
  28  |     await selectWorkspace(page, WORKSPACES.ADMIN);
  29  |   });
  30  | 
  31  |   // ── Table Management ───────────────────────────────────────
  32  | 
  33  |   test('Tables screen renders floor plan', async ({ page }) => {
  34  |     await navigateTo(page, 'tables');
  35  | 
> 36  |     await expect(page.locator('.tables')).toBeVisible({ timeout: SCREEN_TIMEOUT });
      |                                           ^ Error: expect(locator).toBeVisible() failed
  37  |     await expect(page.locator('.tables-title')).toContainText('Table');
  38  | 
  39  |     // Floor plan must be present (mock returns tables with positions).
  40  |     await expect(page.locator('.tables-floorplan')).toBeVisible({ timeout: 5_000 });
  41  |   });
  42  | 
  43  |   // ── Gift Cards ─────────────────────────────────────────────
  44  | 
  45  |   test('Gift Cards screen renders with list', async ({ page }) => {
  46  |     await navigateTo(page, 'gift-cards');
  47  | 
  48  |     await expect(page.locator('.gift-cards-page')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  49  |     await expect(page.locator('.gift-cards-title')).toContainText('Gift');
  50  | 
  51  |     // Search input and status filter must be present.
  52  |     await expect(page.locator('.gift-cards-search')).toBeVisible({ timeout: 5_000 });
  53  |     await expect(page.locator('.gift-cards-status-filter')).toBeVisible({ timeout: 5_000 });
  54  |   });
  55  | 
  56  |   // ── Kiosk (Self-Service) ───────────────────────────────────
  57  | 
  58  |   test('Kiosk screen renders attract screen or product grid', async ({ page }) => {
  59  |     await navigateTo(page, 'kiosk');
  60  | 
  61  |     // Fullscreen kiosk view must load.
  62  |     await expect(page.locator('.kiosk')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  63  | 
  64  |     // Either attract screen or product grid renders (depends on idle state).
  65  |     const hasAttract = await page.locator('.kiosk-attract').isVisible({ timeout: 3_000 });
  66  |     const hasGrid = await page.locator('.kiosk-grid').isVisible({ timeout: 3_000 });
  67  |     // isVisible() returns false for empty locators — no throw, no catch needed.
  68  |     expect(hasAttract || hasGrid).toBe(true);
  69  |   });
  70  | 
  71  |   // ── Customer Management ────────────────────────────────────
  72  | 
  73  |   test('Customers screen renders with table', async ({ page }) => {
  74  |     await navigateTo(page, 'customers');
  75  | 
  76  |     await expect(page.locator('.customer-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  77  |     await expect(page.locator('.customer-mgmt-title')).toContainText('Customer');
  78  | 
  79  |     // Search input must be present.
  80  |     await expect(page.locator('.customer-mgmt-search')).toBeVisible({ timeout: 5_000 });
  81  | 
  82  |     // Table or empty state must be present (mock returns empty customers).
  83  |     await expect(page.locator('.customer-mgmt-table')).toBeVisible({ timeout: 5_000 });
  84  |   });
  85  | 
  86  |   // ── Category Management ────────────────────────────────────
  87  | 
  88  |   test('Categories screen renders with grid', async ({ page }) => {
  89  |     await navigateTo(page, 'categories');
  90  | 
  91  |     await expect(page.locator('.cat-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  92  |     await expect(page.locator('.cat-mgmt-title')).toContainText('Categor');
  93  | 
  94  |     // Grid of category cards or empty state must render (mock returns empty categories).
  95  |     await expect(page.locator('.cat-mgmt-grid')).toBeVisible({ timeout: 5_000 });
  96  |   });
  97  | 
  98  |   // ── Loyalty Management ─────────────────────────────────────
  99  | 
  100 |   test('Loyalty screen renders with table', async ({ page }) => {
  101 |     await navigateTo(page, 'loyalty');
  102 | 
  103 |     await expect(page.locator('.loyalty-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  104 |     await expect(page.locator('.loyalty-mgmt-title')).toContainText('Loyalty');
  105 | 
  106 |     // Table or empty state must be present.
  107 |     await expect(page.locator('.loyalty-table')).toBeVisible({ timeout: 5_000 });
  108 |   });
  109 | });
  110 | 
```