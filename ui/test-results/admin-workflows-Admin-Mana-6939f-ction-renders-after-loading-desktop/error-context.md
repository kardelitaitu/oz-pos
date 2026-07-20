# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: admin-workflows.spec.ts >> Admin Management Screens >> License section renders after loading
- Location: e2e\admin-workflows.spec.ts:137:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('.settings-nav-item').filter({ hasText: 'License' })
Expected: visible
Timeout: 3000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 3000ms
  - waiting for locator('.settings-nav-item').filter({ hasText: 'License' })

```

```yaml
- banner:
  - text: Pengaturan
  - textbox "settings-sidebar-search-aria":
    - /placeholder: Search
  - text: Mon, Jul 20, 2026 09:54 PM
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
  1   | import { test, expect, type Page } from '@playwright/test';
  2   | import { loginAs, selectWorkspace, WORKSPACES } from './helpers';
  3   | 
  4   | /**
  5   |  * E2E: Admin Workflows — Management Screens
  6   |  *
  7   |  * Covers 6 admin management screens that currently have zero E2E
  8   |  * coverage. All tests use hard assertions — no `if` guards.
  9   |  *
  10  |  * CSS contract per screen:
  11  |  *   Staff:          .staff-mgmt, .staff-mgmt-title, .staff-mgmt-table
  12  |  *   Terminals:      .terminal-mgmt, .terminal-mgmt-title, .terminal-mgmt-table
  13  |  *   Tax:            .tax-config, .tax-config-title, .tax-config-table
  14  |  *   Stores:         .multi-store-dashboard, .multi-store-dashboard-title
  15  |  *   Offline Queue:  .offline-queue-screen, .offline-queue-header
  16  |  *   Promotions:     .promo-mgmt, .promo-mgmt-title, .promo-mgmt-table
  17  |  *
  18  |  * Navigation: all screens accessible via settings sidebar in Admin
  19  |  * workspace. Sidebar nav items: `.settings-nav-item` with text.
  20  |  */
  21  | 
  22  | const SIDEBAR_TIMEOUT = 10_000;
  23  | const SCREEN_TIMEOUT = 8_000;
  24  | 
  25  | async function navigateToSettings(page: Page) {
  26  |   await page.evaluate(() => {
  27  |     window.location.hash = '#/settings';
  28  |   });
  29  |   await page.waitForSelector('[data-testid="settings-sidebar"]', { timeout: SIDEBAR_TIMEOUT });
  30  | }
  31  | 
  32  | async function clickSidebarNav(page: Page, sectionName: string) {
  33  |   const nav = page.locator('.settings-nav-item').filter({ hasText: sectionName });
> 34  |   await expect(nav).toBeVisible({ timeout: 3_000 });
      |                     ^ Error: expect(locator).toBeVisible() failed
  35  |   await nav.click();
  36  |   await page.waitForTimeout(500);
  37  | }
  38  | 
  39  | test.describe('Admin Management Screens', () => {
  40  |   test.beforeEach(async ({ page }) => {
  41  |     await loginAs(page, 'admin', '9999');
  42  |     await selectWorkspace(page, WORKSPACES.ADMIN);
  43  |     await navigateToSettings(page);
  44  |   });
  45  | 
  46  |   // ── Staff Management ──────────────────────────────────────
  47  | 
  48  |   test('Staff section renders with table', async ({ page }) => {
  49  |     await clickSidebarNav(page, 'Staff');
  50  | 
  51  |     // Staff management container must load.
  52  |     await expect(page.locator('.staff-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  53  |     await expect(page.locator('.staff-mgmt-title')).toContainText('Staff');
  54  | 
  55  |     // Table or empty state must be present.
  56  |     await expect(page.locator('.staff-mgmt-table')).toBeVisible({ timeout: 5_000 });
  57  |   });
  58  | 
  59  |   // ── Terminal Management ───────────────────────────────────
  60  | 
  61  |   test('Terminals section renders with table', async ({ page }) => {
  62  |     await clickSidebarNav(page, 'Terminals');
  63  | 
  64  |     await expect(page.locator('.terminal-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  65  |     await expect(page.locator('.terminal-mgmt-title')).toContainText('Terminal');
  66  | 
  67  |     // Table must be visible.
  68  |     await expect(page.locator('.terminal-mgmt-table')).toBeVisible({ timeout: 5_000 });
  69  |   });
  70  | 
  71  |   // ── Tax Configuration ─────────────────────────────────────
  72  | 
  73  |   test('Tax Rates section renders with table', async ({ page }) => {
  74  |     await clickSidebarNav(page, 'Tax Rates');
  75  | 
  76  |     await expect(page.locator('.tax-config')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  77  |     await expect(page.locator('.tax-config-title')).toContainText('Tax');
  78  | 
  79  |     // Table or empty placeholder must be present.
  80  |     await expect(page.locator('.tax-config-table')).toBeVisible({ timeout: 5_000 });
  81  |   });
  82  | 
  83  |   // ── Multi-Store Dashboard ─────────────────────────────────
  84  | 
  85  |   test('Stores section renders dashboard', async ({ page }) => {
  86  |     await clickSidebarNav(page, 'Stores');
  87  | 
  88  |     await expect(page.locator('.multi-store-dashboard')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  89  |     await expect(page.locator('.multi-store-dashboard-title')).toContainText('Store');
  90  | 
  91  |     // Dashboard must have stat cards or topology view.
  92  |     const hasStatCards = await page.locator('.multi-store-stat-card').first().isVisible({ timeout: 3_000 }).catch(() => false);
  93  |     const hasTopology = await page.locator('.multi-store-dashboard-topology-view').isVisible({ timeout: 3_000 }).catch(() => false);
  94  |     // At least one view must render.
  95  |     expect(hasStatCards || hasTopology).toBe(true);
  96  |   });
  97  | 
  98  |   // ── Offline Queue ─────────────────────────────────────────
  99  | 
  100 |   test('Offline Queue section renders header', async ({ page }) => {
  101 |     await clickSidebarNav(page, 'Offline Queue');
  102 | 
  103 |     await expect(page.locator('.offline-queue-screen')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  104 |     await expect(page.locator('.offline-queue-header')).toBeVisible({ timeout: 5_000 });
  105 | 
  106 |     // Title must contain "Offline" or "Queue".
  107 |     const titleRow = page.locator('.offline-queue-title-row');
  108 |     await expect(titleRow).toBeVisible({ timeout: 5_000 });
  109 |   });
  110 | 
  111 |   // ── Promotions Management ─────────────────────────────────
  112 | 
  113 |   test('Promotions section renders with table', async ({ page }) => {
  114 |     await clickSidebarNav(page, 'Promotions');
  115 | 
  116 |     await expect(page.locator('.promo-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  117 |     await expect(page.locator('.promo-mgmt-title')).toContainText('Promotion');
  118 | 
  119 |     // Table must be visible (may be empty).
  120 |     await expect(page.locator('.promo-mgmt-table')).toBeVisible({ timeout: 5_000 });
  121 |   });
  122 | 
  123 |   // ── Exchange Rates ────────────────────────────────────────
  124 | 
  125 |   test('Exchange Rates section renders with table', async ({ page }) => {
  126 |     await clickSidebarNav(page, 'Exchange Rates');
  127 | 
  128 |     await expect(page.locator('.exchange-rate-config')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  129 |     await expect(page.locator('.exchange-rate-title')).toContainText('Exchange');
  130 | 
  131 |     // Table must be visible (mock returns empty).
  132 |     await expect(page.locator('.exchange-rate-table')).toBeVisible({ timeout: 5_000 });
  133 |   });
  134 | 
```