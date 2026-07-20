import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E: Reporting Workflows — Dashboard, Sales Report, Inventory Report, Menu Engineering
 *
 * Covers 4 reporting screens with zero prior E2E coverage.
 * All tests use hard assertions — no soft guards, no dead code.
 *
 * CSS contract per screen:
 *   Dashboard:         .dashboard, .dashboard-title, .dashboard-kpi-row
 *   Sales Report:      .sales-report, .sales-report-title, .sales-report-chart-card
 *   Inventory Report:  .inventory-report, .inventory-report-title, .inventory-report-table
 *   Menu Engineering:  .menu-eng, .menu-eng-title, .menu-eng-kpis
 *
 * Routes (App.tsx):
 *   dashboard, reports (manager), inventory-report (manager), menu-engineering (manager+restaurant)
 */

const SCREEN_TIMEOUT = 8_000;

async function navigateTo(page: import('@playwright/test').Page, route: string) {
  await page.evaluate((hash) => {
    window.location.hash = hash;
  }, `#/${route}`);
}

test.describe('Reporting Screens', () => {
  test.beforeEach(async ({ page }) => {
    // Admin has manager role — required for reports, inventory-report, menu-engineering.
    await loginAs(page, 'admin', '9999');
    await selectWorkspace(page, WORKSPACES.ADMIN);
  });

  // ── Dashboard ────────────────────────────────────────────

  test('Dashboard renders KPI cards and weekly chart', async ({ page }) => {
    await navigateTo(page, 'dashboard');

    await expect(page.locator('.dashboard')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.dashboard-title')).toContainText('Dashboard');

    // KPI cards (revenue, orders, top product) must render.
    await expect(page.locator('.dashboard-kpi-row')).toBeVisible({ timeout: 5_000 });

    // Weekly chart section must be present.
    await expect(page.locator('.dashboard-weekly-chart')).toBeVisible({ timeout: 5_000 });
  });

  // ── Sales Report ─────────────────────────────────────────

  test('Sales Report renders chart cards and controls', async ({ page }) => {
    await navigateTo(page, 'reports');

    await expect(page.locator('.sales-report')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.sales-report-title')).toContainText('Sales Report');

    // Date controls and view toggle must be present.
    await expect(page.locator('.sales-report-controls')).toBeVisible({ timeout: 5_000 });

    // At least one chart card must render (mock returns empty data — skeleton or chart).
    await expect(page.locator('.sales-report-chart-card').first()).toBeVisible({ timeout: 5_000 });
  });

  // ── Inventory Report ─────────────────────────────────────

  test('Inventory Report renders table with threshold control', async ({ page }) => {
    await navigateTo(page, 'inventory-report');

    await expect(page.locator('.inventory-report')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.inventory-report-title')).toContainText('Inventory Report');

    // Threshold input and export buttons must be present.
    await expect(page.locator('.inventory-report-controls')).toBeVisible({ timeout: 5_000 });

    // Table or empty state must render (mock returns empty low-stock alerts).
    await expect(page.locator('.inventory-report-table')).toBeVisible({ timeout: 5_000 });
  });

  // ── Menu Engineering ─────────────────────────────────────

  test('Menu Engineering renders KPI cards and quadrant cards', async ({ page }) => {
    await navigateTo(page, 'menu-engineering');

    await expect(page.locator('.menu-eng')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.menu-eng-title')).toContainText('Menu Engineering');

    // KPI summary cards (products, revenue, margin, rate) must render.
    await expect(page.locator('.menu-eng-kpis')).toBeVisible({ timeout: 5_000 });

    // Quadrant summary cards or table must render (mock returns empty — loading state).
    const hasQuadrantCards = await page.locator('.menu-eng-quadrant-cards').isVisible({ timeout: 3_000 });
    const hasTable = await page.locator('.menu-eng-table').isVisible({ timeout: 3_000 });
    expect(hasQuadrantCards || hasTable).toBe(true);
  });
});
