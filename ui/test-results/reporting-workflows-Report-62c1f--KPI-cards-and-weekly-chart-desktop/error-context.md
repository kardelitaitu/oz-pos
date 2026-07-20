# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: reporting-workflows.spec.ts >> Reporting Screens >> Dashboard renders KPI cards and weekly chart
- Location: e2e\reporting-workflows.spec.ts:31:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('.dashboard')
Expected: visible
Timeout: 8000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 8000ms
  - waiting for locator('.dashboard')

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
  1  | import { test, expect } from '@playwright/test';
  2  | import { loginAs, selectWorkspace, WORKSPACES, navigateTo } from './helpers';
  3  | 
  4  | /**
  5  |  * E2E: Reporting Workflows — Dashboard, Sales Report, Inventory Report, Menu Engineering
  6  |  *
  7  |  * Covers 4 reporting screens with zero prior E2E coverage.
  8  |  * All tests use hard assertions — no soft guards, no dead code.
  9  |  *
  10 |  * CSS contract per screen:
  11 |  *   Dashboard:         .dashboard, .dashboard-title, .dashboard-kpi-row
  12 |  *   Sales Report:      .sales-report, .sales-report-title, .sales-report-chart-card
  13 |  *   Inventory Report:  .inventory-report, .inventory-report-title, .inventory-report-table
  14 |  *   Menu Engineering:  .menu-eng, .menu-eng-title, .menu-eng-kpis
  15 |  *
  16 |  * Routes (App.tsx):
  17 |  *   dashboard, reports (manager), inventory-report (manager), menu-engineering (manager+restaurant)
  18 |  */
  19 | 
  20 | const SCREEN_TIMEOUT = 8_000;
  21 | 
  22 | test.describe('Reporting Screens', () => {
  23 |   test.beforeEach(async ({ page }) => {
  24 |     // Admin has manager role — required for reports, inventory-report, menu-engineering.
  25 |     await loginAs(page, 'admin', '9999');
  26 |     await selectWorkspace(page, WORKSPACES.ADMIN);
  27 |   });
  28 | 
  29 |   // ── Dashboard ────────────────────────────────────────────
  30 | 
  31 |   test('Dashboard renders KPI cards and weekly chart', async ({ page }) => {
  32 |     await navigateTo(page, 'dashboard');
  33 | 
> 34 |     await expect(page.locator('.dashboard')).toBeVisible({ timeout: SCREEN_TIMEOUT });
     |                                              ^ Error: expect(locator).toBeVisible() failed
  35 |     await expect(page.locator('.dashboard-title')).toContainText('Dashboard');
  36 | 
  37 |     // KPI cards (revenue, orders, top product) must render.
  38 |     await expect(page.locator('.dashboard-kpi-row')).toBeVisible({ timeout: 5_000 });
  39 | 
  40 |     // Weekly chart section must be present.
  41 |     await expect(page.locator('.dashboard-weekly-chart')).toBeVisible({ timeout: 5_000 });
  42 |   });
  43 | 
  44 |   // ── Sales Report ─────────────────────────────────────────
  45 | 
  46 |   test('Sales Report renders chart cards and controls', async ({ page }) => {
  47 |     await navigateTo(page, 'reports');
  48 | 
  49 |     await expect(page.locator('.sales-report')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  50 |     await expect(page.locator('.sales-report-title')).toContainText('Sales Report');
  51 | 
  52 |     // Date controls and view toggle must be present.
  53 |     await expect(page.locator('.sales-report-controls')).toBeVisible({ timeout: 5_000 });
  54 | 
  55 |     // At least one chart card must render (mock returns empty data — skeleton or chart).
  56 |     await expect(page.locator('.sales-report-chart-card').first()).toBeVisible({ timeout: 5_000 });
  57 |   });
  58 | 
  59 |   // ── Inventory Report ─────────────────────────────────────
  60 | 
  61 |   test('Inventory Report renders table with threshold control', async ({ page }) => {
  62 |     await navigateTo(page, 'inventory-report');
  63 | 
  64 |     await expect(page.locator('.inventory-report')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  65 |     await expect(page.locator('.inventory-report-title')).toContainText('Inventory Report');
  66 | 
  67 |     // Threshold input and export buttons must be present.
  68 |     await expect(page.locator('.inventory-report-controls')).toBeVisible({ timeout: 5_000 });
  69 | 
  70 |     // Table or empty state must render (mock returns empty low-stock alerts).
  71 |     await expect(page.locator('.inventory-report-table')).toBeVisible({ timeout: 5_000 });
  72 |   });
  73 | 
  74 |   // ── Menu Engineering ─────────────────────────────────────
  75 | 
  76 |   test('Menu Engineering renders KPI cards and quadrant cards', async ({ page }) => {
  77 |     await navigateTo(page, 'menu-engineering');
  78 | 
  79 |     await expect(page.locator('.menu-eng')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  80 |     await expect(page.locator('.menu-eng-title')).toContainText('Menu Engineering');
  81 | 
  82 |     // KPI summary cards (products, revenue, margin, rate) must render.
  83 |     await expect(page.locator('.menu-eng-kpis')).toBeVisible({ timeout: 5_000 });
  84 | 
  85 |     // Quadrant summary cards or table must render (mock returns empty — loading state).
  86 |     const hasQuadrantCards = await page.locator('.menu-eng-quadrant-cards').isVisible({ timeout: 3_000 });
  87 |     const hasTable = await page.locator('.menu-eng-table').isVisible({ timeout: 3_000 });
  88 |     expect(hasQuadrantCards || hasTable).toBe(true);
  89 |   });
  90 | });
  91 | 
```