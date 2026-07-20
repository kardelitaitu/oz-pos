# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: auth.spec.ts >> Staff Login >> successful login shows workspace picker with greeting
- Location: e2e\auth.spec.ts:49:3

# Error details

```
Error: expect(page).toHaveURL(expected) failed

Expected pattern: /#\/$/
Received string:  "http://localhost:1420/"
Timeout: 5000ms

Call log:
  - Expect "toHaveURL" with timeout 5000ms
    14 × unexpected value "http://localhost:1420/"

```

```yaml
- text: Hello, Owner
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
  1   | import { test, expect, type Page } from '@playwright/test';
  2   | import { loginAs } from './helpers';
  3   | 
  4   | /**
  5   |  * E2E: Staff Login with PIN — Hard Assertions
  6   |  *
  7   |  * Verifies the complete authentication flow with deterministic assertions.
  8   |  * All guards (`if (count > 0)`) removed — tests hard-fail on regressions.
  9   |  *
  10  |  * Dev-mock credentials:
  11  |  *   - "owner" / "1234"  → role: owner, display: "Owner"
  12  |  *   - "admin" / "admin123"  → role: manager, display: "Admin"
  13  |  *   - "kasir" / "1234"  → role: cashier, display: "Cashier"
  14  |  *
  15  |  * CSS contract (StaffLoginScreen.tsx):
  16  |  *   .staff-login-screen     — container
  17  |  *   .staff-login-input      — username text input
  18  |  *   .staff-login-submit-btn — submit / next button
  19  |  *   .staff-login-pad        — PIN keypad (visible after username step)
  20  |  *   .staff-login-pad-key    — individual digit buttons
  21  |  *   .staff-login-pin-dot--filled — filled PIN dot
  22  |  *   .staff-login-error      — error message alert
  23  |  *   .staff-login-lockout    — lockout countdown (rate-limit)
  24  |  *   .workspace-home         — workspace picker (post-login success)
  25  |  *   .ws-header-greeting     — display name in workspace header
  26  |  */
  27  | 
  28  | const VALID_USER = 'owner';
  29  | const VALID_PIN = '1234';
  30  | const WRONG_PIN = '0000';
  31  | const UNKNOWN_USER = 'nonexistent';
  32  | 
  33  | async function enterPin(page: Page, pin: string) {
  34  |   for (const digit of pin) {
  35  |     const key = page.locator('.staff-login-pad-key').filter({ hasText: digit });
  36  |     await key.click();
  37  |     await page.waitForTimeout(80);
  38  |   }
  39  | }
  40  | 
  41  | test.describe('Staff Login', () => {
  42  |   test.beforeEach(async ({ page }) => {
  43  |     await page.goto('/');
  44  |     await page.waitForSelector('.staff-login-screen', { timeout: 15_000 });
  45  |   });
  46  | 
  47  |   // ── E2E-4: Hard-assert login happy path ──────────────────────
  48  | 
  49  |   test('successful login shows workspace picker with greeting', async ({ page }) => {
  50  |     // Enter username.
  51  |     await page.locator('.staff-login-input').fill(VALID_USER);
  52  |     await page.locator('.staff-login-submit-btn').click();
  53  | 
  54  |     // Wait for PIN pad (not waitForTimeout).
  55  |     await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });
  56  | 
  57  |     // Enter PIN.
  58  |     await enterPin(page, VALID_PIN);
  59  | 
  60  |     // Workspace home must appear (hard assertion).
  61  |     await page.waitForSelector('.workspace-home', { timeout: 15_000 });
  62  |     await expect(page.locator('.workspace-home')).toBeVisible();
  63  | 
  64  |     // Greeting must contain exact display name.
  65  |     await expect(page.locator('.ws-header-greeting')).toContainText('Owner');
  66  | 
  67  |     // URL hash must be at root.
> 68  |     await expect(page).toHaveURL(/#\/$/);
      |                        ^ Error: expect(page).toHaveURL(expected) failed
  69  |   });
  70  | 
  71  |   // ── E2E-5: Assert error text for wrong PIN ───────────────────
  72  | 
  73  |   test('shows "Invalid credentials" for wrong PIN', async ({ page }) => {
  74  |     await page.locator('.staff-login-input').fill(VALID_USER);
  75  |     await page.locator('.staff-login-submit-btn').click();
  76  |     await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });
  77  | 
  78  |     // Enter wrong PIN.
  79  |     await enterPin(page, WRONG_PIN);
  80  | 
  81  |     // Error must appear with exact dev-mock error text.
  82  |     const errorAlert = page.locator('.staff-login-error');
  83  |     await expect(errorAlert).toBeVisible({ timeout: 5_000 });
  84  |     await expect(errorAlert).toContainText('Invalid credentials');
  85  | 
  86  |     // Must stay on login screen.
  87  |     await expect(page.locator('.staff-login-screen')).toBeVisible();
  88  |   });
  89  | 
  90  |   // ── E2E-6: Assert error for unknown username ─────────────────
  91  | 
  92  |   test('shows error for unknown username', async ({ page }) => {
  93  |     await page.locator('.staff-login-input').fill(UNKNOWN_USER);
  94  |     await page.locator('.staff-login-submit-btn').click();
  95  | 
  96  |     // The app should show a toast or inline error about user not found.
  97  |     // Dev-mock: staff_check_username returns { found: false }.
  98  |     // The login screen must remain visible.
  99  |     await expect(page.locator('.staff-login-screen')).toBeVisible({ timeout: 5_000 });
  100 | 
  101 |     // Either the error toast or an inline error should mention "not found".
  102 |     const toastError = page.locator('.toast--error, [class*="toast"][class*="error"]');
  103 |     const inlineError = page.locator('.staff-login-error');
  104 | 
  105 |     const toastVisible = await toastError.isVisible().catch(() => false);
  106 |     const inlineVisible = await inlineError.isVisible().catch(() => false);
  107 | 
  108 |     expect(toastVisible || inlineVisible).toBe(true);
  109 | 
  110 |     if (inlineVisible) {
  111 |       await expect(inlineError).toContainText(/not found|unknown|invalid/i);
  112 |     }
  113 |     if (toastVisible) {
  114 |       await expect(toastError).toContainText(/not found|unknown|invalid/i);
  115 |     }
  116 |   });
  117 | 
  118 |   // ── E2E-7: Rate-limit lockout UI ─────────────────────────────
  119 | 
  120 |   test('rate-limit lockout after 5 wrong PIN attempts', async ({ page }) => {
  121 |     await page.locator('.staff-login-input').fill(VALID_USER);
  122 |     await page.locator('.staff-login-submit-btn').click();
  123 |     await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });
  124 | 
  125 |     // Enter wrong PIN 5 times.
  126 |     for (let attempt = 0; attempt < 5; attempt++) {
  127 |       await enterPin(page, WRONG_PIN);
  128 | 
  129 |       // Wait for error to appear, then clear.
  130 |       const errorAlert = page.locator('.staff-login-error');
  131 |       await errorAlert.waitFor({ state: 'visible', timeout: 5_000 }).catch(() => {});
  132 | 
  133 |       // If we're locked out, stop.
  134 |       const lockoutVisible = await page
  135 |         .locator('.staff-login-lockout, [class*="lockout"]')
  136 |         .isVisible()
  137 |         .catch(() => false);
  138 |       if (lockoutVisible) break;
  139 | 
  140 |       // Wait for PIN to clear before next attempt.
  141 |       await page.waitForTimeout(500);
  142 |     }
  143 | 
  144 |     // After 5 attempts, either lockout appears or PIN pad is disabled.
  145 |     const lockoutEl = page.locator('.staff-login-lockout, [class*="lockout"]');
  146 |     const pinPadDisabled = page.locator('.staff-login-pad[aria-disabled="true"]');
  147 | 
  148 |     const lockoutVisible = await lockoutEl.isVisible().catch(() => false);
  149 |     const padDisabled = await pinPadDisabled.isVisible().catch(() => false);
  150 | 
  151 |     // At least one lockout mechanism must be active.
  152 |     expect(lockoutVisible || padDisabled).toBe(true);
  153 |   });
  154 | 
  155 |   // ── E2E-8: Session persistence across reload ─────────────────
  156 | 
  157 |   test('returns to login screen after page reload', async ({ page }) => {
  158 |     // Login successfully first.
  159 |     await page.locator('.staff-login-input').fill(VALID_USER);
  160 |     await page.locator('.staff-login-submit-btn').click();
  161 |     await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });
  162 |     await enterPin(page, VALID_PIN);
  163 |     await page.waitForSelector('.workspace-home', { timeout: 15_000 });
  164 | 
  165 |     // Reload the page.
  166 |     await page.reload();
  167 | 
  168 |     // Session is NOT persisted in localStorage — login screen should appear.
```