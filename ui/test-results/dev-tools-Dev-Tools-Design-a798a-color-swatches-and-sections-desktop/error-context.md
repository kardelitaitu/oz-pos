# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: dev-tools.spec.ts >> Dev Tools >> Design System renders color swatches and sections
- Location: e2e\dev-tools.spec.ts:28:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('.ds-page')
Expected: visible
Timeout: 8000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 8000ms
  - waiting for locator('.ds-page')

```

```yaml
- banner:
  - text: Pengaturan
  - textbox "settings-sidebar-search-aria":
    - /placeholder: Search
  - text: Mon, Jul 20, 2026 09:53 PM
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
  5  |  * E2E: Dev Tools — Design System, Tooltip Preview
  6  |  *
  7  |  * Covers the last 2 untested routes to reach 100% route coverage.
  8  |  * All tests use hard assertions — no soft guards, no dead code.
  9  |  *
  10 |  * CSS contract per screen:
  11 |  *   Design System:   .ds-page, .ds-header, .ds-section
  12 |  *   Tooltip Preview: .tp-page, .tp-header, .tp-section
  13 |  *
  14 |  * Routes (App.tsx):
  15 |  *   design (line 172), tooltips (line 175)
  16 |  */
  17 | 
  18 | const SCREEN_TIMEOUT = 8_000;
  19 | 
  20 | test.describe('Dev Tools', () => {
  21 |   test.beforeEach(async ({ page }) => {
  22 |     await loginAs(page, 'admin', '9999');
  23 |     await selectWorkspace(page, WORKSPACES.ADMIN);
  24 |   });
  25 | 
  26 |   // ── Design System ─────────────────────────────────────────
  27 | 
  28 |   test('Design System renders color swatches and sections', async ({ page }) => {
  29 |     await navigateTo(page, 'design');
  30 | 
> 31 |     await expect(page.locator('.ds-page')).toBeVisible({ timeout: SCREEN_TIMEOUT });
     |                                            ^ Error: expect(locator).toBeVisible() failed
  32 |     await expect(page.locator('.ds-header')).toBeVisible({ timeout: 5_000 });
  33 | 
  34 |     // At least one design section must render (Colors, Typography, Spacing, etc.).
  35 |     await expect(page.locator('.ds-section').first()).toBeVisible({ timeout: 5_000 });
  36 |   });
  37 | 
  38 |   // ── Tooltip Preview ───────────────────────────────────────
  39 | 
  40 |   test('Tooltip Preview renders position grid and sections', async ({ page }) => {
  41 |     await navigateTo(page, 'tooltips');
  42 | 
  43 |     await expect(page.locator('.tp-page')).toBeVisible({ timeout: SCREEN_TIMEOUT });
  44 |     await expect(page.locator('.tp-header')).toBeVisible({ timeout: 5_000 });
  45 | 
  46 |     // At least one preview section must render.
  47 |     await expect(page.locator('.tp-section').first()).toBeVisible({ timeout: 5_000 });
  48 |   });
  49 | });
  50 | 
```