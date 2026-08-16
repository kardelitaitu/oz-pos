import { test, expect, type Page } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES, navigateTo } from './helpers';

/**
 * PERF-10 — Performance smoke suite (desktop + tablet).
 *
 * Measures wall-clock time for the critical interactions named in the
 * audit: startup to interactive POS, route transition, product search,
 * add-to-cart, checkout open, and KDS refresh. Runs in BOTH Playwright
 * projects (desktop 1366×768, tablet 1024×1366) automatically.
 *
 * Budgets are environment-aware — override any with an env var, e.g.:
 *   PERF_BUDGET_STARTUP=20000 npx playwright test e2e/perf-smoke.spec.ts
 *
 * Only aggregate timings are asserted; no payloads or user data are
 * captured. Timing is measured with `performance.now()` on the page.
 *
 * DOM contracts (kept in sync with current components):
 *   Cart line added:  .retail-cart-line-name (first row of .retail-cart-table)
 *   Payment modal:    .payment-modal (PaymentModal root class)
 *   KDS interactive:  .kds-columns (KdsLayoutKanban board)
 */

// Login flow + workspace selection is the slow part; keep the suite's
// per-test timeout generous (tablet WebKit boots slowly).
test.setTimeout(120_000);

const budget = (name: string, fallback: number): number => {
  const raw = process.env[`PERF_BUDGET_${name}`];
  const parsed = raw ? Number(raw) : Number.NaN;
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
};

const BUDGETS = {
  startup: budget('STARTUP', 15_000), // login → workspace → POS interactive
  route: budget('ROUTE', 5_000),      // route transition → screen visible
  search: budget('SEARCH', 3_000),    // product search → grid updated
  addToCart: budget('ADD_TO_CART', 3_000), // product click → cart line item
  checkout: budget('CHECKOUT', 3_000),     // pay click → payment modal visible
  kds: budget('KDS', 8_000),          // KDS workspace → ticket columns visible
};

/** Time (ms) for an async step. */
async function timeStep(page: Page, step: () => Promise<void>): Promise<number> {
  const t0 = await page.evaluate(() => performance.now());
  await step();
  const t1 = await page.evaluate(() => performance.now());
  return Math.round(t1 - t0);
}

/**
 * Workspace-card click wrapper for this suite.
 *
 * Cards carry an infinite `ws-bg-shift` background animation, so
 * Playwright's default actionability check can keep reporting "element
 * is not stable" on slower targets (tablet WebKit). This perf suite
 * measures route-transition time, not card-interaction mechanics, so
 * force is the right choice — see the shared helper's `opts.force`.
 */
async function selectWorkspaceForce(page: Page, typeKey: string): Promise<void> {
  await selectWorkspace(page, typeKey, { force: true });
}

test.describe('Performance smoke (PERF-10)', () => {
  test('startup to interactive POS within budget', async ({ page }) => {
    const t0 = Date.now();
    await loginAs(page, 'staff', '1234');
    await selectWorkspaceForce(page, WORKSPACES.STORE_POS);
    // Interactive = catalog rendered with at least one add-to-cart button.
    await expect(page.locator('.retail-product-btn').first()).toBeVisible({ timeout: 10_000 });
    const ms = Date.now() - t0;
    expect(ms).toBeLessThan(BUDGETS.startup);
    console.log(`[perf] startup-to-interactive: ${ms}ms (budget ${BUDGETS.startup}ms)`);
  });

  test('route transition within budget', async ({ page }) => {
    await loginAs(page, 'admin', '9999');
    await selectWorkspaceForce(page, WORKSPACES.ADMIN);
    const ms = await timeStep(page, async () => {
      await navigateTo(page, 'sales-history');
      await expect(page.locator('.sales-history')).toBeVisible({ timeout: 8_000 });
    });
    expect(ms).toBeLessThan(BUDGETS.route);
    console.log(`[perf] route-transition: ${ms}ms (budget ${BUDGETS.route}ms)`);
  });

  test('product search within budget', async ({ page }) => {
    await loginAs(page, 'staff', '1234');
    await selectWorkspaceForce(page, WORKSPACES.STORE_POS);
    await expect(page.locator('.retail-search-input')).toBeVisible({ timeout: 10_000 });
    const ms = await timeStep(page, async () => {
      await page.locator('.retail-search-input').fill('Ryzen');
      // Wait for a real state change: a product row containing the query
      // appears (no fixed waits inside the measured window).
      await expect(page.locator('.retail-product-row').filter({ hasText: 'Ryzen' }).first()).toBeVisible({ timeout: 5_000 });
    });
    expect(ms).toBeLessThan(BUDGETS.search);
    console.log(`[perf] product-search: ${ms}ms (budget ${BUDGETS.search}ms)`);
  });

  test('add-to-cart within budget', async ({ page }) => {
    await loginAs(page, 'staff', '1234');
    await selectWorkspaceForce(page, WORKSPACES.STORE_POS);
    await expect(page.locator('.retail-product-btn').first()).toBeVisible({ timeout: 10_000 });
    const ms = await timeStep(page, async () => {
      await page.locator('.retail-product-btn').first().click();
      // Real cart-line contract: the first name cell in the cart table.
      await expect(page.locator('.retail-cart-line-name').first()).toBeVisible({ timeout: 5_000 });
    });
    expect(ms).toBeLessThan(BUDGETS.addToCart);
    console.log(`[perf] add-to-cart: ${ms}ms (budget ${BUDGETS.addToCart}ms)`);
  });

  test('checkout open within budget', async ({ page }) => {
    await loginAs(page, 'staff', '1234');
    await selectWorkspaceForce(page, WORKSPACES.STORE_POS);
    await expect(page.locator('.retail-product-btn').first()).toBeVisible({ timeout: 10_000 });
    await page.locator('.retail-product-btn').first().click();
    await expect(page.locator('.retail-cart-line-name').first()).toBeVisible({ timeout: 5_000 });
    const ms = await timeStep(page, async () => {
      await page.locator('.retail-cart-action-btn--pay').click();
      // PaymentModal root class (no data-testid in the component).
      await expect(page.locator('.payment-modal')).toBeVisible({ timeout: 5_000 });
    });
    expect(ms).toBeLessThan(BUDGETS.checkout);
    console.log(`[perf] checkout-open: ${ms}ms (budget ${BUDGETS.checkout}ms)`);
  });

  test('KDS refresh within budget', async ({ page }) => {
    // Admin role — same login as kds.spec.ts (staff does not reach the board).
    await loginAs(page, 'admin', '9999');
    const ms = await timeStep(page, async () => {
      await selectWorkspaceForce(page, WORKSPACES.KDS);
      // Ticket columns visible = KDS board interactive.
      await expect(page.locator('.kds-columns')).toBeVisible({ timeout: 10_000 });
    });
    expect(ms).toBeLessThan(BUDGETS.kds);
    console.log(`[perf] kds-refresh: ${ms}ms (budget ${BUDGETS.kds}ms)`);
  });
});
