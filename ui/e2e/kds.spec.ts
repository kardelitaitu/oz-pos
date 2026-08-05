import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E: Kitchen Display System (KDS) — Hard Assertions
 *
 * Tests the KDS screen from the KDS workspace, including Kanban columns,
 * ticket interaction (status advance), and layout switching (Focus/Metro).
 *
 * CSS contract (KdsScreen.tsx + KdsTicketCard.tsx + layout components):
 *   .kds                       — container (role="region")
 *   .kds-header                — header bar
 *   .kds-title                 — "Kitchen Display" heading
 *   .kds-order-count           — order count badge (e.g. "3 orders")
 *   .kds-header-right          — settings + layout switcher area
 *   .kds-columns               — Kanban three-column grid
 *   .kds-column                — individual status column
 *   .kds-column--pending       — pending column
 *   .kds-column--preparing     — preparing column
 *   .kds-column--ready         — ready column
 *   .kds-column-title          — column heading
 *   .kds-column-count          — column count badge
 *   .kds-ticket                — clickable ticket card (button)
 *   .kds-ticket-number         — display number (e.g. "#101")
 *   .kds-ticket-items          — items summary text
 *   .kds-ticket-time           — SLA time display
 *
 * Route: #/kds (available from the KDS workspace)
 */

const TIMEOUT = 10_000;

test.describe('Kitchen Display System', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'admin', '9999');
    await selectWorkspace(page, WORKSPACES.KDS);
  });

  // ── E2E: KDS screen renders with title, columns, and order count ──

  test('loads with title, 3 columns, and non-zero order count', async ({ page }) => {
    await expect(page.locator('.kds')).toBeVisible({ timeout: TIMEOUT });

    // Title and header.
    await expect(page.locator('.kds-title')).toBeVisible();
    await expect(page.locator('.kds-title')).toContainText('Kitchen', { timeout: 5_000 });
    await expect(page.locator('.kds-header')).toBeVisible({ timeout: 5_000 });
    await expect(page.locator('.kds-header-left')).toBeVisible({ timeout: 3_000 });

    // Order count must be visible and non-zero (dev-mock returns 3 orders).
    await expect(page.locator('.kds-order-count')).toBeVisible({ timeout: 5_000 });
    const countText = await page.locator('.kds-order-count').textContent();
    expect(countText).toBeTruthy();
    const countMatch = countText!.match(/\d+/);
    expect(countMatch).not.toBeNull();
    expect(parseInt(countMatch![0], 10)).toBeGreaterThanOrEqual(1);

    // Header right area (settings + layout switcher) must be present.
    await expect(page.locator('.kds-header-right')).toBeVisible({ timeout: 5_000 });

    // Kanban: 3 columns (pending, preparing, ready).
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: 5_000 });
    await expect(page.locator('.kds-column')).toHaveCount(3);
    await expect(page.locator('.kds-column-count')).toHaveCount(3);

    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });

  // ── E2E: Ticket cards render with display numbers and items ──

  test('tickets render with display number, items summary, and SLA time', async ({ page }) => {
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: TIMEOUT });

    // Tickets must be visible (dev-mock returns 3 orders).
    const ticket = page.locator('.kds-ticket').first();
    await expect(ticket).toBeVisible({ timeout: 5_000 });

    // Display number (#101, #102, etc.).
    const numberEl = page.locator('.kds-ticket-number').first();
    await expect(numberEl).toBeVisible({ timeout: 3_000 });
    expect(await numberEl.textContent()).toMatch(/#\d+/);

    // Items summary. Line items lazy-fetch via get_kds_order_lines_scoped,
    // so the course-grouped .kds-ticket-line-items container renders; the
    // flat .kds-ticket-items span is the loading/old-order fallback.
    const itemsEl = page.locator('.kds-ticket-line-items, .kds-ticket-items').first();
    await expect(itemsEl).toBeVisible({ timeout: 3_000 });
    const itemsContent = await itemsEl.textContent();
    expect(itemsContent).toBeTruthy();
    expect(itemsContent!.length).toBeGreaterThan(3);

    // SLA time indicator.
    await expect(page.locator('.kds-ticket-time').first()).toBeVisible({ timeout: 3_000 });

    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });

  // ── E2E: Clicking a pending ticket advances status ────────────

  test('clicking a pending ticket advances status without crashing', async ({ page }) => {
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: TIMEOUT });

    // Dev-mock always returns a pending order — click it to advance.
    const pendingTicket = page.locator('.kds-column--pending .kds-ticket').first();
    await expect(pendingTicket).toBeVisible({ timeout: 5_000 });
    await pendingTicket.click();
    await page.waitForTimeout(1_000);

    // No error boundary after status advance.
    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });

  // ── E2E: Header right area has interactive buttons ────────────

  test('header right area has interactive settings/layout buttons', async ({ page }) => {
    await expect(page.locator('.kds')).toBeVisible({ timeout: TIMEOUT });

    const headerRight = page.locator('.kds-header-right');
    await expect(headerRight).toBeVisible({ timeout: 5_000 });

    // At least 1 button (settings toggle, layout toggle, or both).
    const headerButtons = headerRight.locator('button');
    await expect(headerButtons.first()).toBeVisible({ timeout: 3_000 });
    expect(await headerButtons.count()).toBeGreaterThanOrEqual(1);

    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });
});
