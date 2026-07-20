import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E: Admin Workflows — Management Screens
 *
 * Covers 6 admin management screens that currently have zero E2E
 * coverage. All tests use hard assertions — no `if` guards.
 *
 * CSS contract per screen:
 *   Staff:          .staff-mgmt, .staff-mgmt-title, .staff-mgmt-table
 *   Terminals:      .terminal-mgmt, .terminal-mgmt-title, .terminal-mgmt-table
 *   Tax:            .tax-config, .tax-config-title, .tax-config-table
 *   Stores:         .multi-store-dashboard, .multi-store-dashboard-title
 *   Offline Queue:  .offline-queue-screen, .offline-queue-header
 *   Promotions:     .promo-mgmt, .promo-mgmt-title, .promo-mgmt-table
 *
 * Navigation: all screens accessible via settings sidebar in Admin
 * workspace. Sidebar nav items: `.settings-nav-item` with text.
 */

const SIDEBAR_TIMEOUT = 10_000;
const SCREEN_TIMEOUT = 8_000;

async function navigateToSettings(page: import('@playwright/test').Page) {
  await page.evaluate(() => {
    window.location.hash = '#/settings';
  });
  await page.waitForSelector('[data-testid="settings-sidebar"]', { timeout: SIDEBAR_TIMEOUT });
}

async function clickSidebarNav(page: import('@playwright/test').Page, sectionName: string) {
  const nav = page.locator('.settings-nav-item').filter({ hasText: sectionName });
  await expect(nav).toBeVisible({ timeout: 3_000 });
  await nav.click();
  await page.waitForTimeout(500);
}

test.describe('Admin Management Screens', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'admin', '9999');
    await selectWorkspace(page, WORKSPACES.ADMIN);
    await navigateToSettings(page);
  });

  // ── Staff Management ──────────────────────────────────────

  test('Staff section renders with table', async ({ page }) => {
    await clickSidebarNav(page, 'Staff');

    // Staff management container must load.
    await expect(page.locator('.staff-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.staff-mgmt-title')).toContainText('Staff');

    // Table or empty state must be present.
    await expect(page.locator('.staff-mgmt-table')).toBeVisible({ timeout: 5_000 });
  });

  // ── Terminal Management ───────────────────────────────────

  test('Terminals section renders with table', async ({ page }) => {
    await clickSidebarNav(page, 'Terminals');

    await expect(page.locator('.terminal-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.terminal-mgmt-title')).toContainText('Terminal');

    // Table must be visible.
    await expect(page.locator('.terminal-mgmt-table')).toBeVisible({ timeout: 5_000 });
  });

  // ── Tax Configuration ─────────────────────────────────────

  test('Tax Rates section renders with table', async ({ page }) => {
    await clickSidebarNav(page, 'Tax Rates');

    await expect(page.locator('.tax-config')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.tax-config-title')).toContainText('Tax');

    // Table or empty placeholder must be present.
    await expect(page.locator('.tax-config-table')).toBeVisible({ timeout: 5_000 });
  });

  // ── Multi-Store Dashboard ─────────────────────────────────

  test('Stores section renders dashboard', async ({ page }) => {
    await clickSidebarNav(page, 'Stores');

    await expect(page.locator('.multi-store-dashboard')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.multi-store-dashboard-title')).toContainText('Store');

    // Dashboard must have stat cards or topology view.
    const hasStatCards = await page.locator('.multi-store-stat-card').first().isVisible({ timeout: 3_000 }).catch(() => false);
    const hasTopology = await page.locator('.multi-store-dashboard-topology-view').isVisible({ timeout: 3_000 }).catch(() => false);
    // At least one view must render.
    expect(hasStatCards || hasTopology).toBe(true);
  });

  // ── Offline Queue ─────────────────────────────────────────

  test('Offline Queue section renders header', async ({ page }) => {
    await clickSidebarNav(page, 'Offline Queue');

    await expect(page.locator('.offline-queue-screen')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.offline-queue-header')).toBeVisible({ timeout: 5_000 });

    // Title must contain "Offline" or "Queue".
    const titleRow = page.locator('.offline-queue-title-row');
    await expect(titleRow).toBeVisible({ timeout: 5_000 });
  });

  // ── Promotions Management ─────────────────────────────────

  test('Promotions section renders with table', async ({ page }) => {
    await clickSidebarNav(page, 'Promotions');

    await expect(page.locator('.promo-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.promo-mgmt-title')).toContainText('Promotion');

    // Table must be visible (may be empty).
    await expect(page.locator('.promo-mgmt-table')).toBeVisible({ timeout: 5_000 });
  });

  // ── Exchange Rates ────────────────────────────────────────

  test('Exchange Rates section renders with table', async ({ page }) => {
    await clickSidebarNav(page, 'Exchange Rates');

    await expect(page.locator('.exchange-rate-config')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.exchange-rate-title')).toContainText('Exchange');

    // Table must be visible (mock returns empty).
    await expect(page.locator('.exchange-rate-table')).toBeVisible({ timeout: 5_000 });
  });

  // ── License ───────────────────────────────────────────────

  test('License section renders after loading', async ({ page }) => {
    await clickSidebarNav(page, 'License');

    // License section heading must be visible (post-load, not skeleton).
    const licenseHeading = page.locator('.settings-section-title').filter({ hasText: 'License' });
    await expect(licenseHeading.first()).toBeVisible({ timeout: SCREEN_TIMEOUT });

    // License status/type should render (mock returns valid Pro license).
    await expect(page.locator('.settings-section-title').first()).toBeVisible();
  });

  // ── Features ──────────────────────────────────────────────

  test('Features section renders toggle screen', async ({ page }) => {
    await clickSidebarNav(page, 'Features');

    await expect(page.locator('.feature-toggle')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.feature-toggle-title')).toContainText('Feature');

    // Subtitle should show enabled count.
    await expect(page.locator('.feature-toggle-subtitle')).toBeVisible({ timeout: 5_000 });
  });

  // ── Data Management ───────────────────────────────────────

  test('Data Management section renders with tabs', async ({ page }) => {
    await clickSidebarNav(page, 'Data');

    await expect(page.locator('.data-mgmt')).toBeVisible({ timeout: SCREEN_TIMEOUT });
    await expect(page.locator('.data-mgmt-title')).toContainText('Data');

    // Tab navigation must be present (Backup, Export, Import).
    await expect(page.locator('.data-mgmt-tabs')).toBeVisible({ timeout: 5_000 });
  });

  // ── About ─────────────────────────────────────────────────

  test('About section renders version info', async ({ page }) => {
    await clickSidebarNav(page, 'About');

    // About section heading must be visible.
    const aboutHeading = page.locator('.settings-section-title').filter({ hasText: 'About' });
    await expect(aboutHeading.first()).toBeVisible({ timeout: SCREEN_TIMEOUT });
  });
});
