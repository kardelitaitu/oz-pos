# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: new-flows.spec.ts >> Audit Log >> audit log screen renders with header and filters
- Location: e2e\new-flows.spec.ts:151:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('[data-testid="audit-log-table"]')
Expected: visible
Timeout: 10000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 10000ms
  - waiting for locator('[data-testid="audit-log-table"]')

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
  59  |     await storeCard.click();
  60  |     await page.waitForTimeout(2_000);
  61  | 
  62  |     // Navigate back to workspace picker (Escape key).
  63  |     await page.keyboard.press('Escape');
  64  |     await page.waitForSelector('.workspace-home', { timeout: 10_000 });
  65  | 
  66  |     // The last-used workspace card should have an active dot.
  67  |     const activeCard = page.locator('.workspace-card--active');
  68  |     await expect(activeCard).toBeVisible({ timeout: 5_000 });
  69  | 
  70  |     const activeDot = activeCard.locator('.workspace-card-active-dot');
  71  |     await expect(activeDot).toBeVisible();
  72  |   });
  73  | });
  74  | 
  75  | // ── E2E-27: Session lock / unlock ─────────────────────────────
  76  | 
  77  | test.describe('Session Lock', () => {
  78  |   test('session lock card renders with time display', async ({ page }) => {
  79  |     // Set a 15-second idle timeout (0.25 min) so login + workspace
  80  |     // load complete before the timer fires.
  81  |     await page.evaluate(() => {
  82  |       localStorage.setItem('auto-lock-minutes', '0.25');
  83  |     });
  84  | 
  85  |     // Reload so useIdleTimer picks up the new timeout on mount.
  86  |     await page.reload();
  87  | 
  88  |     // Re-login using the helper (not duplicated logic).
  89  |     await loginAs(page, 'owner', '1234');
  90  | 
  91  |     // Wait for the idle timer to fire — session lock card must appear.
  92  |     const lockCard = page.locator('.session-lock-card');
  93  |     await expect(lockCard).toBeVisible({ timeout: 20_000 });
  94  | 
  95  |     // Verify lock card content.
  96  |     await expect(page.locator('.session-lock-time')).toBeVisible();
  97  |     await expect(page.locator('.session-lock-date')).toBeVisible();
  98  |     await expect(page.locator('.session-lock-label')).toContainText('Locked');
  99  | 
  100 |     // Enter PIN to unlock.
  101 |     for (const digit of '1234') {
  102 |       const key = page.locator('.session-lock-pad-key').filter({ hasText: digit });
  103 |       await key.click();
  104 |       await page.waitForTimeout(80);
  105 |     }
  106 | 
  107 |     // Workspace home should reappear after unlock.
  108 |     await expect(page.locator('.workspace-home')).toBeVisible({ timeout: 10_000 });
  109 |   });
  110 | });
  111 | 
  112 | // ── E2E-28: KDS ticket board ─────────────────────────────────
  113 | 
  114 | test.describe('KDS Ticket Board', () => {
  115 |   test.beforeEach(async ({ page }) => {
  116 |     await loginAs(page, 'owner', '1234');
  117 |     await selectWorkspace(page, WORKSPACES.KDS);
  118 |   });
  119 | 
  120 |   test('KDS screen renders with title and order count', async ({ page }) => {
  121 |     // Wait for KDS container.
  122 |     await page.waitForSelector('.kds', { timeout: 10_000 });
  123 | 
  124 |     // Title must be visible.
  125 |     await expect(page.locator('.kds-title')).toBeVisible();
  126 |     await expect(page.locator('.kds-title')).toContainText('Kitchen Display');
  127 | 
  128 |     // Order count must show 0 (mock returns empty orders).
  129 |     const orderCount = page.locator('.kds-order-count');
  130 |     await expect(orderCount).toBeVisible({ timeout: 5_000 });
  131 | 
  132 |     // KDS layout switcher should be present.
  133 |     const headerRight = page.locator('.kds-header-right');
  134 |     await expect(headerRight).toBeVisible();
  135 | 
  136 |     // No error state.
  137 |     const errorEl = page.locator('.kds-error');
  138 |     const hasError = await errorEl.isVisible().catch(() => false);
  139 |     expect(hasError).toBe(false);
  140 |   });
  141 | });
  142 | 
  143 | // ── E2E-29: Audit log screen ──────────────────────────────────
  144 | 
  145 | test.describe('Audit Log', () => {
  146 |   test.beforeEach(async ({ page }) => {
  147 |     await loginAs(page, 'admin', '9999');
  148 |     await selectWorkspace(page, WORKSPACES.ADMIN);
  149 |   });
  150 | 
  151 |   test('audit log screen renders with header and filters', async ({ page }) => {
  152 |     // Navigate to audit log.
  153 |     await page.evaluate(() => {
  154 |       window.location.hash = '#/audit';
  155 |     });
  156 | 
  157 |     // Audit log container must be visible.
  158 |     const auditContainer = page.locator('[data-testid="audit-log-table"]');
> 159 |     await expect(auditContainer).toBeVisible({ timeout: 10_000 });
      |                                  ^ Error: expect(locator).toBeVisible() failed
  160 | 
  161 |     // Header must have title.
  162 |     await expect(page.locator('.audit-log-title')).toBeVisible();
  163 |     await expect(page.locator('.audit-log-title')).toContainText('Audit Log');
  164 | 
  165 |     // Search/filter bar must be present.
  166 |     await expect(page.locator('.audit-log-filters')).toBeVisible({ timeout: 5_000 });
  167 | 
  168 |     // Verify no error boundary.
  169 |     const errorBoundary = page.locator('[class*="error-boundary"]');
  170 |     const hasError = await errorBoundary.isVisible().catch(() => false);
  171 |     expect(hasError).toBe(false);
  172 |   });
  173 | });
  174 | 
```