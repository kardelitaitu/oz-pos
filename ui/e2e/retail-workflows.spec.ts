import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E: Retail Workflows — Tables, Gift Cards, Kiosk, Customers, Categories, Loyalty
 *
 * Covers 6 management/retail screens with zero prior E2E coverage.
 * All tests use hard assertions — no soft guards, no dead code.
 *
 * CSS contract per screen:
 *   Tables:       .tables, .tables-title, .tables-floorplan
 *   Gift Cards:   .gift-cards-page, .gift-cards-title, .gift-cards-list
 *   Kiosk:        .kiosk, .kiosk-attract / .kiosk-grid, .kiosk-product-card
 *   Customers:    .customer-mgmt, .customer-mgmt-title, .customer-mgmt-table
 *   Categories:   .cat-mgmt, .cat-mgmt-title, .cat-mgmt-grid
 *   Loyalty:      .loyalty-mgmt, .loyalty-mgmt-title, .loyalty-table
 *
 * Routes (registered in App.tsx):
 *   tables, kiosk, customers, categories, gift-cards, loyalty
 */

const SCREEN_TIMEOUT = 8_000;

/** Navigate to a hash route and wait for the app to re-render. */
async function navigateTo(page: import('@playwright/test').Page, route: string) {
  await page.evaluate((hash) => {
    window.location.hash = hash;
  }, `#/${route}`);
}

test.describe('Retail & Management Screens', () => {
  test.beforeEach(async ({ page }) => {
    // Admin login gives manager role — required for gift-cards, loyalty, categories.
    await loginAs(page, 'admin', '9999');
    await selectWorkspace(page, WORKSPACES.ADMIN);
  });

  // ── Table Management ───────────────────────────────────────

  test('Tables screen renders floor plan', async ({ page }) => {
    await navigateTo(page, 'tables');

    await expect(page.locator('.tables')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.tables-title')).toContainText('Table');

    // Floor plan must be present (mock returns tables with positions).
    await expect(page.locator('.tables-floorplan')).toBeVisible({ timeout: 5_000 });
  });

  // ── Gift Cards ─────────────────────────────────────────────

  test('Gift Cards screen renders with list', async ({ page }) => {
    await navigateTo(page, 'gift-cards');

    await expect(page.locator('.gift-cards-page')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.gift-cards-title')).toContainText('Gift');

    // Search input and status filter must be present.
    await expect(page.locator('.gift-cards-search')).toBeVisible({ timeout: 5_000 });
    await expect(page.locator('.gift-cards-status-filter')).toBeVisible({ timeout: 5_000 });
  });

  // ── Kiosk (Self-Service) ───────────────────────────────────

  test('Kiosk screen renders attract screen or product grid', async ({ page }) => {
    await navigateTo(page, 'kiosk');

    // Fullscreen kiosk view must load.
    await expect(page.locator('.kiosk')).toBeVisible({ timeout: SCREEN_TIMEOUT });

    // Either attract screen or product grid renders (depends on idle state).
    const hasAttract = await page.locator('.kiosk-attract').isVisible({ timeout: 3_000 });
    const hasGrid = await page.locator('.kiosk-grid').isVisible({ timeout: 3_000 });
    // isVisible() returns false for empty locators — no throw, no catch needed.
    expect(hasAttract || hasGrid).toBe(true);
  });

  // ── Customer Management ────────────────────────────────────

  test('Customers screen renders with table', async ({ page }) => {
    await navigateTo(page, 'customers');

    await expect(page.locator('.customer-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.customer-mgmt-title')).toContainText('Customer');

    // Search input must be present.
    await expect(page.locator('.customer-mgmt-search')).toBeVisible({ timeout: 5_000 });

    // Table or empty state must be present (mock returns empty customers).
    await expect(page.locator('.customer-mgmt-table')).toBeVisible({ timeout: 5_000 });
  });

  // ── Category Management ────────────────────────────────────

  test('Categories screen renders with grid', async ({ page }) => {
    await navigateTo(page, 'categories');

    await expect(page.locator('.cat-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.cat-mgmt-title')).toContainText('Categor');

    // Grid of category cards or empty state must render (mock returns empty categories).
    await expect(page.locator('.cat-mgmt-grid')).toBeVisible({ timeout: 5_000 });
  });

  // ── Loyalty Management ─────────────────────────────────────

  test('Loyalty screen renders with table', async ({ page }) => {
    await navigateTo(page, 'loyalty');

    await expect(page.locator('.loyalty-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.loyalty-mgmt-title')).toContainText('Loyalty');

    // Table or empty state must be present.
    await expect(page.locator('.loyalty-table')).toBeVisible({ timeout: 5_000 });
  });
});
