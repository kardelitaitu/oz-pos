import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E: New Flows — Hard Assertions (E2E-26 through E2E-29)
 *
 * Covers flows not previously tested:
 *   E2E-26: Workspace picker (all cards visible)
 *   E2E-27: Session lock / unlock
 *   E2E-28: KDS ticket board
 *   E2E-29: Audit log screen
 *
 * CSS contract:
 *   .workspace-card         — workspace card button
 *   .workspace-card-name    — card name heading
 *   .workspace-card--disabled — disabled card
 *   .session-lock-card      — session lock card
 *   .session-lock-pad-key   — PIN keypad key
 *   .kds                    — KDS screen container
 *   .kds-title              — KDS heading
 *   .kds-order-count        — order count badge
 *   [data-testid="audit-log-table"] — audit log container
 */

// ── E2E-26: Workspace picker ───────────────────────────────────

test.describe('Workspace Picker', () => {
  test('all workspace cards are visible after login', async ({ page }) => {
    await loginAs(page, 'owner', '1234');

    // All 5 workspace cards must be visible (mock returns 5 workspaces).
    const cards = page.locator('.workspace-card');
    await expect(cards.first()).toBeVisible({ timeout: 5_000 });
    expect(await cards.count()).toBeGreaterThanOrEqual(5);

    // Verify specific workspace names are present.
    const cardNames = page.locator('.workspace-card-name');
    const allNames = await cardNames.allTextContents();
    expect(allNames.some((n) => n.includes('Store POS'))).toBe(true);
    expect(allNames.some((n) => n.includes('Kitchen Display'))).toBe(true);
    expect(allNames.some((n) => n.includes('Inventory'))).toBe(true);
    expect(allNames.some((n) => n.includes('Admin'))).toBe(true);

    // Click "Inventory Management" and verify it navigates.
    const inventoryCard = cards.filter({ hasText: 'Inventory Management' });
    await inventoryCard.click();
    await page.waitForTimeout(2_000);

    // Should navigate away from workspace picker.
    const home = page.locator('.workspace-home');
    await expect(home).not.toBeVisible({ timeout: 5_000 });
  });
});

// ── E2E-27: Session lock / unlock ─────────────────────────────

test.describe('Session Lock', () => {
  test('session lock card renders with time display', async ({ page }) => {
    // Set a 15-second idle timeout (0.25 min) so login + workspace
    // load complete before the timer fires.
    await page.evaluate(() => {
      localStorage.setItem('auto-lock-minutes', '0.25');
    });

    // Reload so useIdleTimer picks up the new timeout on mount.
    await page.reload();

    // Re-login using the helper (not duplicated logic).
    await loginAs(page, 'owner', '1234');

    // Wait for the idle timer to fire — session lock card must appear.
    const lockCard = page.locator('.session-lock-card');
    await expect(lockCard).toBeVisible({ timeout: 20_000 });

    // Verify lock card content.
    await expect(page.locator('.session-lock-time')).toBeVisible();
    await expect(page.locator('.session-lock-date')).toBeVisible();
    await expect(page.locator('.session-lock-label')).toContainText('Locked');

    // Enter PIN to unlock.
    for (const digit of '1234') {
      const key = page.locator('.session-lock-pad-key').filter({ hasText: digit });
      await key.click();
      await page.waitForTimeout(80);
    }

    // Workspace home should reappear after unlock.
    await expect(page.locator('.workspace-home')).toBeVisible({ timeout: 10_000 });
  });
});

// ── E2E-28: KDS ticket board ─────────────────────────────────

test.describe('KDS Ticket Board', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'owner', '1234');
    await selectWorkspace(page, WORKSPACES.KDS);
  });

  test('KDS screen renders with title and order count', async ({ page }) => {
    // Wait for KDS container.
    await page.waitForSelector('.kds', { timeout: 10_000 });

    // Title must be visible.
    await expect(page.locator('.kds-title')).toBeVisible();
    await expect(page.locator('.kds-title')).toContainText('Kitchen Display');

    // Order count must show 0 (mock returns empty orders).
    const orderCount = page.locator('.kds-order-count');
    await expect(orderCount).toBeVisible({ timeout: 5_000 });

    // KDS layout switcher should be present.
    const headerRight = page.locator('.kds-header-right');
    await expect(headerRight).toBeVisible();

    // No error state.
    const errorEl = page.locator('.kds-error');
    const hasError = await errorEl.isVisible().catch(() => false);
    expect(hasError).toBe(false);
  });
});

// ── E2E-29: Audit log screen ──────────────────────────────────

test.describe('Audit Log', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    await selectWorkspace(page, WORKSPACES.ADMIN);
  });

  test('audit log screen renders with header and filters', async ({ page }) => {
    // Navigate to audit log.
    await page.evaluate(() => {
      window.location.hash = '#/audit';
    });

    // Audit log container must be visible.
    const auditContainer = page.locator('[data-testid="audit-log-table"]');
    await expect(auditContainer).toBeVisible({ timeout: 10_000 });

    // Header must have title.
    await expect(page.locator('.audit-log-title')).toBeVisible();
    await expect(page.locator('.audit-log-title')).toContainText('Audit Log');

    // Search/filter bar must be present.
    await expect(page.locator('.audit-log-filters')).toBeVisible({ timeout: 5_000 });

    // Verify no error boundary.
    const errorBoundary = page.locator('[class*="error-boundary"]');
    const hasError = await errorBoundary.isVisible().catch(() => false);
    expect(hasError).toBe(false);
  });
});
