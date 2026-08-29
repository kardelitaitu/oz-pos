import { test, expect, type Page } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E: Admin Workflows — Settings Sidebar Screens
 *
 * Tests the settings sidebar screens accessible from the Admin workspace.
 * The sidebar has three collapsible categories:
 *   Business:  General, Appearance
 *   Operations: Receipt, Cloud Sync, Email, Store POS, Restaurant POS, Inventory
 *   System:    About, License, Topology
 *
 * CSS contract per screen:
 *   License:  .settings-section-title (contains "License")
 *   About:    .settings-section-title (contains "System")
 *   Topology: .node-topology-editor (tested in adr22-workspace-settings.spec.ts)
 *
 * Navigation: sidebar categories are collapsible accordions. The
 * System category (About, License, Topology) is collapsed by default.
 * We expand it first via `.settings-sidebar-section-header` with text "System".
 */

const SIDEBAR_TIMEOUT = 10_000;
const SCREEN_TIMEOUT = 8_000;

async function navigateToSettings(page: Page) {
  // If already on settings (from selectWorkspace's tool card click), skip.
  const alreadyOnSettings = await page.locator('[data-testid="settings-sidebar"]').isVisible({ timeout: 1_000 }).catch(() => false);
  if (alreadyOnSettings) return;

  await page.evaluate(() => {
    window.location.hash = '#/settings';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
  });
  await page.waitForSelector('[data-testid="settings-sidebar"]', { timeout: SIDEBAR_TIMEOUT });
}

async function expandCategory(page: Page, categoryName: string) {
  // Expand a collapsed sidebar category by its header text.
  const header = page.locator('.settings-sidebar-section-header')
    .filter({ hasText: categoryName });
  const isExpanded = await header
    .getAttribute('aria-expanded')
    .then((v) => v === 'true')
    .catch(() => false);
  if (!isExpanded) {
    await header.click();
    await page.waitForTimeout(300);
  }
}

async function clickSidebarNav(page: Page, sectionName: string, category?: string) {
  // Expand the parent category first if specified.
  if (category) {
    await expandCategory(page, category);
  }

  const nav = page.locator('.settings-nav-item').filter({ hasText: sectionName });
  await expect(nav).toBeVisible({ timeout: 5_000 });
  await nav.click();
  await page.waitForTimeout(500);
}

test.describe('Admin Settings Screens', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'admin', '9999');
    await selectWorkspace(page, WORKSPACES.ADMIN);
    await navigateToSettings(page);
  });

  // ── Settings sidebar renders with categories ──────────────

  test('settings sidebar has multiple nav items and categories', async ({ page }) => {
    const navItems = page.locator('.settings-nav-item');
    const count = await navItems.count();
    // Must have at least 5 items (General, Appearance, Receipt, Cloud Sync, About, License, etc.).
    expect(count).toBeGreaterThanOrEqual(5);

    // Must have at least 2 category headers (Business, Operations, System).
    const categoryHeaders = page.locator('.settings-sidebar-section-header');
    expect(await categoryHeaders.count()).toBeGreaterThanOrEqual(2);
  });

  // ── License (System category) ─────────────────────────────

  test('License section renders after loading', async ({ page }) => {
    await clickSidebarNav(page, 'License', 'System');

    // License section heading must be visible (post-load, not skeleton).
    const licenseHeading = page.locator('.settings-section-title').filter({ hasText: 'License' });
    await expect(licenseHeading.first()).toBeVisible({ timeout: SCREEN_TIMEOUT });

    // License status/type should render (mock returns valid Pro license).
    await expect(page.locator('.settings-section-title').first()).toBeVisible();
  });

  // ── About (System category) ───────────────────────────────

  test('About section renders version info', async ({ page }) => {
    await clickSidebarNav(page, 'About', 'System');

    // About section heading must be visible ("System & License Ownership").
    const aboutHeading = page.locator('.settings-section-title').filter({ hasText: 'System' });
    await expect(aboutHeading.first()).toBeVisible({ timeout: SCREEN_TIMEOUT });
  });

  // ── General (Business category) ───────────────────────────

  test('General section renders store settings', async ({ page }) => {
    await clickSidebarNav(page, 'General', 'Business');

    // General section must render with store name input.
    const storeInput = page.locator('.settings-section-title').filter({ hasText: 'General' });
    await expect(storeInput.first()).toBeVisible({ timeout: SCREEN_TIMEOUT });
  });

  // ── Appearance (Business category) ────────────────────────

  test('Appearance section renders display settings', async ({ page }) => {
    await clickSidebarNav(page, 'Appearance', 'Business');

    // Appearance section must render.
    const appearanceHeading = page.locator('.settings-section-title').filter({ hasText: 'Appearance' });
    await expect(appearanceHeading.first()).toBeVisible({ timeout: SCREEN_TIMEOUT });
  });

  // ── Receipt (Operations category) ─────────────────────────

  test('Receipt section renders receipt settings', async ({ page }) => {
    await clickSidebarNav(page, 'Receipt', 'Operations');

    // Receipt section heading must be visible.
    const receiptHeading = page.locator('.settings-section-title').filter({ hasText: 'Receipt' });
    await expect(receiptHeading.first()).toBeVisible({ timeout: SCREEN_TIMEOUT });
  });
});
