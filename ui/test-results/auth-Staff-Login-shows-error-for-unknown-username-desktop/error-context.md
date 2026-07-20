# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: auth.spec.ts >> Staff Login >> shows error for unknown username
- Location: e2e\auth.spec.ts:92:3

# Error details

```
Error: expect(received).toBe(expected) // Object.is equality

Expected: true
Received: false
```

# Page snapshot

```yaml
- generic [ref=e2]:
  - generic [ref=e3]:
    - generic [ref=e4]:
      - img "OZ-POS Demo" [ref=e6]
      - generic [ref=e9]:
        - textbox "Nama Pengguna" [active] [ref=e10]:
          - /placeholder: Username
          - text: nonexistent
        - button "staff-login-next-aria" [ref=e11] [cursor=pointer]:
          - img [ref=e12]
      - status "staff-login-progress-aria" [ref=e15]
    - generic:
      - generic: OZ-POS Enterprise v0.0.14
      - generic: © 2026 OZ-POS. Seluruh hak cipta dilindungi.
  - toolbar "Developer tools" [ref=e19]:
    - generic [ref=e21]: DevTools
    - generic [ref=e22]:
      - paragraph [ref=e23]: Theme
      - radiogroup "Theme selector" [ref=e24]:
        - radio "Glass theme" [checked] [ref=e25] [cursor=pointer]:
          - img [ref=e26]
          - generic [ref=e30]: Glass
        - radio "Light theme" [ref=e31] [cursor=pointer]:
          - img [ref=e32]
          - generic [ref=e38]: Light
        - radio "Dark theme" [ref=e39] [cursor=pointer]:
          - img [ref=e40]
          - generic [ref=e42]: Dark
      - generic [ref=e44]: Glass
  - generic "toast-notifications-aria":
    - alert [ref=e49]:
      - generic [ref=e50]: Pengguna tidak ditemukan
      - button "toast-dismiss-aria" [ref=e51] [cursor=pointer]:
        - img [ref=e52]
```

# Test source

```ts
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
  68  |     await expect(page).toHaveURL(/#\/$/);
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
> 108 |     expect(toastVisible || inlineVisible).toBe(true);
      |                                           ^ Error: expect(received).toBe(expected) // Object.is equality
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
  169 |     await page.waitForSelector('.staff-login-screen', { timeout: 15_000 });
  170 |     await expect(page.locator('.staff-login-screen')).toBeVisible();
  171 |   });
  172 | 
  173 |   // ── Bonus: Clears PIN dots after error ───────────────────────
  174 | 
  175 |   test('clears PIN dots when error occurs', async ({ page }) => {
  176 |     await page.locator('.staff-login-input').fill(VALID_USER);
  177 |     await page.locator('.staff-login-submit-btn').click();
  178 |     await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10_000 });
  179 | 
  180 |     await enterPin(page, WRONG_PIN);
  181 | 
  182 |     // Error should appear.
  183 |     await expect(page.locator('.staff-login-error')).toBeVisible({ timeout: 5_000 });
  184 | 
  185 |     // PIN dots must be cleared (no filled dots).
  186 |     await expect(page.locator('.staff-login-pin-dot--filled')).toHaveCount(0);
  187 |   });
  188 | 
  189 |   // ── Bonus: PIN step indicator shows active step ──────────────
  190 | 
  191 |   test('shows PIN step as active after username submit', async ({ page }) => {
  192 |     await page.locator('.staff-login-input').fill(VALID_USER);
  193 |     await page.locator('.staff-login-submit-btn').click();
  194 | 
  195 |     // PIN pad must appear.
  196 |     await expect(page.locator('.staff-login-pad')).toBeVisible({ timeout: 10_000 });
  197 | 
  198 |     // Step indicator dot for PIN step (index 1) must be active.
  199 |     const stepDots = page.locator('.staff-login-step-dot');
  200 |     const dotCount = await stepDots.count();
  201 |     if (dotCount > 1) {
  202 |       await expect(stepDots.nth(1)).toHaveClass(/staff-login-step-dot--active/);
  203 |     }
  204 |   });
  205 | 
  206 |   // ── Bonus: Admin login shows correct greeting ───────────────
  207 | 
  208 |   test('admin login shows manager greeting', async ({ page }) => {
```