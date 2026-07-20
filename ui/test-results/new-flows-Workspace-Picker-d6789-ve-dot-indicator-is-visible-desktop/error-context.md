# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: new-flows.spec.ts >> Workspace Picker >> workspace card active-dot indicator is visible
- Location: e2e\new-flows.spec.ts:54:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('.workspace-card--active')
Expected: visible
Timeout: 5000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 5000ms
  - waiting for locator('.workspace-card--active')

```

```yaml
- text: Ni hao, Owner
- button "Alihkan layar penuh"
- button "Masuk sebagai Owner": Owner owner
- button "Keluar"
- button "Muat Ulang"
- banner
- group "Workspaces":
  - button "Buka ruang kerja Restaurant POS":
    - text: "1"
    - heading "Restaurant POS" [level=2]
    - paragraph: Cashier terminal for restaurant ordering with menu categories and table management
  - button "Buka ruang kerja Store POS":
    - text: "2"
    - heading "Store POS" [level=2]
    - paragraph: Cashier terminal for retail with product lookup, customer management, and loyalty
  - button "Buka ruang kerja Kitchen Display":
    - text: "3"
    - heading "Kitchen Display" [level=2]
    - paragraph: Order queue display for the kitchen — tap tickets to advance their status
  - button "Buka ruang kerja Inventory Management":
    - text: "4"
    - heading "Inventory Management" [level=2]
    - paragraph: Manage products, stock levels, bundles, categories, and inventory reports
  - button "Buka ruang kerja Admin":
    - text: "5"
    - heading "Admin" [level=2]
    - paragraph: System settings, staff management, reports, audit logs, and configuration
  - heading "Loyalty" [level=2]
  - paragraph: Coming soon
  - text: Tidak tersedia
  - heading "Marketing" [level=2]
  - paragraph: Coming soon
  - text: Tidak tersedia
  - heading "Online Orders" [level=2]
  - paragraph: Coming soon
  - text: Tidak tersedia
  - heading "Analytics" [level=2]
  - paragraph: Coming soon
  - text: Tidak tersedia
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
  5   |  * E2E: New Flows — Hard Assertions (E2E-26 through E2E-29)
  6   |  *
  7   |  * Covers flows not previously tested:
  8   |  *   E2E-26: Workspace picker (all cards visible)
  9   |  *   E2E-27: Session lock / unlock
  10  |  *   E2E-28: KDS ticket board
  11  |  *   E2E-29: Audit log screen
  12  |  *
  13  |  * CSS contract:
  14  |  *   .workspace-card         — workspace card button
  15  |  *   .workspace-card-name    — card name heading
  16  |  *   .workspace-card--disabled — disabled card
  17  |  *   .session-lock-card      — session lock card
  18  |  *   .session-lock-pad-key   — PIN keypad key
  19  |  *   .kds                    — KDS screen container
  20  |  *   .kds-title              — KDS heading
  21  |  *   .kds-order-count        — order count badge
  22  |  *   [data-testid="audit-log-table"] — audit log container
  23  |  */
  24  | 
  25  | // ── E2E-26: Workspace picker ───────────────────────────────────
  26  | 
  27  | test.describe('Workspace Picker', () => {
  28  |   test('all workspace cards are visible after login', async ({ page }) => {
  29  |     await loginAs(page, 'owner', '1234');
  30  | 
  31  |     // All 5 workspace cards must be visible (mock returns 5 workspaces).
  32  |     const cards = page.locator('.workspace-card');
  33  |     await expect(cards.first()).toBeVisible({ timeout: 5_000 });
  34  |     expect(await cards.count()).toBeGreaterThanOrEqual(5);
  35  | 
  36  |     // Verify specific workspace names are present.
  37  |     const cardNames = page.locator('.workspace-card-name');
  38  |     const allNames = await cardNames.allTextContents();
  39  |     expect(allNames.some((n) => n.includes('Store POS'))).toBe(true);
  40  |     expect(allNames.some((n) => n.includes('Kitchen Display'))).toBe(true);
  41  |     expect(allNames.some((n) => n.includes('Inventory'))).toBe(true);
  42  |     expect(allNames.some((n) => n.includes('Admin'))).toBe(true);
  43  | 
  44  |     // Click "Inventory Management" and verify it navigates.
  45  |     const inventoryCard = cards.filter({ hasText: 'Inventory Management' });
  46  |     await inventoryCard.click();
  47  |     await page.waitForTimeout(2_000);
  48  | 
  49  |     // Should navigate away from workspace picker.
  50  |     const home = page.locator('.workspace-home');
  51  |     await expect(home).not.toBeVisible({ timeout: 5_000 });
  52  |   });
  53  | 
  54  |   test('workspace card active-dot indicator is visible', async ({ page }) => {
  55  |     await loginAs(page, 'owner', '1234');
  56  | 
  57  |     // Click Store POS to activate it.
  58  |     const storeCard = page.locator('.workspace-card').filter({ hasText: 'Store POS' });
  59  |     await storeCard.click();
  60  |     await page.waitForTimeout(2_000);
  61  | 
  62  |     // Navigate back to workspace picker (Escape key).
  63  |     await page.keyboard.press('Escape');
  64  |     await page.waitForSelector('.workspace-home', { timeout: 10_000 });
  65  | 
  66  |     // The last-used workspace card should have an active dot.
  67  |     const activeCard = page.locator('.workspace-card--active');
> 68  |     await expect(activeCard).toBeVisible({ timeout: 5_000 });
      |                              ^ Error: expect(locator).toBeVisible() failed
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
  159 |     await expect(auditContainer).toBeVisible({ timeout: 10_000 });
  160 | 
  161 |     // Header must have title.
  162 |     await expect(page.locator('.audit-log-title')).toBeVisible();
  163 |     await expect(page.locator('.audit-log-title')).toContainText('Audit Log');
  164 | 
  165 |     // Search/filter bar must be present.
  166 |     await expect(page.locator('.audit-log-filters')).toBeVisible({ timeout: 5_000 });
  167 | 
  168 |     // Verify no error boundary.
```