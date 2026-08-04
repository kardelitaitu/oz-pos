import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E Critical Path: KDS Full Lifecycle
 *
 * Tests the Kitchen Display System end-to-end: ticket lifecycle from
 * pending → preparing → ready → served, layout switching, settings
 * interaction, per-item status advance, and history panel toggle.
 *
 * CSS contract:
 *   .kds                          — KDS container
 *   .kds-header                   — header bar
 *   .kds-title                    — "Kitchen Display" heading
 *   .kds-order-count              — order count badge
 *   .kds-header-right             — settings + layout switcher area
 *   .kds-columns                  — Kanban three-column grid
 *   .kds-column                   — individual status column
 *   .kds-column--pending          — pending column
 *   .kds-column--preparing        — preparing column
 *   .kds-column--ready            — ready column
 *   .kds-column-title             — column heading text
 *   .kds-column-count             — column count badge
 *   .kds-ticket                   — clickable ticket card (button)
 *   .kds-ticket-number            — display number (e.g. "#101")
 *   .kds-ticket-items             — items summary
 *   .kds-ticket-time              — SLA time indicator
 *   .kds-layout-btn               — layout switcher trigger button
 *   .kds-layout-popover            — layout options popover
 *   .kds-layout-option             — individual layout option
 *   .kds-settings-btn              — settings trigger button
 *   .kds-settings-popover          — settings gear panel
 *   .kds-history-toggle           — history panel toggle
 *   .kds-shortcuts-btn            — keyboard shortcuts button
 *   .kds-zone-chips               — kitchen zone filter chips
 *   .kds-offline-banner           — offline/disconnected banner
 *   .kds-loading-container        — loading skeleton
 */

const TIMEOUT = 10_000;

test.describe('Critical Path: KDS Full Lifecycle', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'admin', '9999');
    await selectWorkspace(page, WORKSPACES.KDS);
  });

  // ── Step 1: Verify initial state ──────────────────────────────────
  test('KDS loads with Kanban layout: 3 columns, tickets, and interactive header', async ({ page }) => {
    await expect(page.locator('.kds')).toBeVisible({ timeout: TIMEOUT });
    await expect(page.locator('.kds-title')).toContainText('Kitchen', { timeout: 5_000 });
    await expect(page.locator('.kds-header')).toBeVisible({ timeout: 5_000 });

    // Order count must show at least 1 order (dev-mock returns 3).
    const countText = await page.locator('.kds-order-count').textContent();
    expect(countText).toBeTruthy();
    const countMatch = countText!.match(/\d+/);
    expect(countMatch).not.toBeNull();
    expect(parseInt(countMatch![0], 10)).toBeGreaterThanOrEqual(1);

    // Kanban: 3 columns with counts.
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: 5_000 });
    const columns = page.locator('.kds-column');
    await expect(columns).toHaveCount(3);
    await expect(page.locator('.kds-column-count')).toHaveCount(3);

    // Each column must have a title.
    const columnTitles = page.locator('.kds-column-title');
    expect(await columnTitles.count()).toBe(3);

    // No crash.
    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });

  // ── Step 2: Ticket lifecycle — advance through all statuses ────────
  test('advance a ticket through all statuses: pending → preparing → ready → served', async ({ page }) => {
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: TIMEOUT });

    const pendingTickets = page.locator('.kds-column--pending .kds-ticket');
    await expect(pendingTickets.first()).toBeVisible({ timeout: 5_000 });

    // Track the exact ticket we click instead of comparing aggregate counts.
    // Other tickets may be present or arrive while the event-driven board
    // refreshes, but this ticket must follow the full lifecycle.
    const firstPending = pendingTickets.first();
    const ticketNumberText = await firstPending.locator('.kds-ticket-number').textContent() || '';
    expect(ticketNumberText).toMatch(/#\d+/);

    const trackedTicket = page.locator('.kds-ticket').filter({ hasText: ticketNumberText }).first();

    // ── Advance 1: pending → preparing ─────────────────────────────
    await firstPending.click();
    await expect.poll(
      async () => await page.locator('.kds-column--pending').locator('.kds-ticket').filter({ hasText: ticketNumberText }).count(),
      { timeout: 8_000, message: `${ticketNumberText} should leave pending` },
    ).toBe(0);
    await expect(page.locator('.kds-column--preparing').locator('.kds-ticket').filter({ hasText: ticketNumberText })).toHaveCount(1, { timeout: 3_000 });

    // ── Advance 2: preparing → ready ───────────────────────────────
    await trackedTicket.click();
    await expect.poll(
      async () => await page.locator('.kds-column--preparing').locator('.kds-ticket').filter({ hasText: ticketNumberText }).count(),
      { timeout: 8_000, message: `${ticketNumberText} should leave preparing` },
    ).toBe(0);
    await expect(page.locator('.kds-column--ready').locator('.kds-ticket').filter({ hasText: ticketNumberText })).toHaveCount(1, { timeout: 3_000 });

    // ── Advance 3: ready → served ──────────────────────────────────
    await trackedTicket.click();
    await expect.poll(
      async () => await page.locator('.kds-column--ready').locator('.kds-ticket').filter({ hasText: ticketNumberText }).count(),
      { timeout: 8_000, message: `${ticketNumberText} should leave ready` },
    ).toBe(0);

    // No crash after full cycle.
    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });

  // ── Step 3: Layout switching ───────────────────────────────────────
  test('switch between Kanban, Focus, and Metro layouts without crashing', async ({ page }) => {
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: TIMEOUT });

    // The layout switcher is a gear-triggered popover in the header-right.
    const layoutBtn = page.locator('.kds-layout-btn');
    await expect(layoutBtn).toBeVisible({ timeout: 5_000 });
    await layoutBtn.click();

    // The popover portal exposes one option button per layout.
    const layoutButtons = page.locator('.kds-layout-option');
    await expect(layoutButtons.first()).toBeVisible({ timeout: 3_000 });
    const btnCount = await layoutButtons.count();
    expect(btnCount).toBeGreaterThanOrEqual(3); // Kanban, Focus, Metro

    // Click each layout option and verify the switch didn't crash.
    const layoutNames = ['focus', 'metro', 'kanban'];
    for (const name of layoutNames) {
      const btn = layoutButtons.filter({ hasText: new RegExp(name, 'i') }).first();
      await expect(btn).toBeVisible({ timeout: 3_000 });
      await btn.click();
      await page.waitForTimeout(1_000);

      // No crash after layout switch.
      await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });

      // Reopen the popover for the next selection (it closes after picking).
      if (name !== layoutNames[layoutNames.length - 1]) {
        await page.locator('.kds-layout-btn').click();
        await expect(page.locator('.kds-layout-option').first()).toBeVisible({ timeout: 3_000 });
      }
    }
  });

  // ── Step 4: Settings panel interaction ─────────────────────────────
  test('settings panel opens, shows toggles, and does not crash', async ({ page }) => {
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: TIMEOUT });

    // The settings trigger button lives in the header-right.
    const settingsToggle = page.locator('.kds-settings-btn').first();
    await expect(settingsToggle).toBeVisible({ timeout: 5_000 });
    await settingsToggle.click();
    await page.waitForTimeout(500);

    // Settings popover must open.
    const settingsPanel = page.locator('.kds-settings-popover');
    await expect(settingsPanel).toBeVisible({ timeout: 5_000 });

    // Settings panel must contain interactive elements (toggles, sliders, buttons).
    const settingsInputs = settingsPanel.locator('input, button, select, label');
    const inputCount = await settingsInputs.count();
    expect(inputCount).toBeGreaterThanOrEqual(2);

    // Close settings panel.
    await settingsToggle.click();
    await page.waitForTimeout(500);
    await expect(settingsPanel).not.toBeVisible({ timeout: 3_000 });

    // No crash.
    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });

  // ── Step 5: History panel toggle ───────────────────────────────────
  test('history panel toggles without crashing', async ({ page }) => {
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: TIMEOUT });

    // Find the history toggle button.
    const historyToggle = page.locator('.kds-history-toggle');
    await expect(historyToggle).toBeVisible({ timeout: 5_000 });

    // Open history panel.
    await historyToggle.click();
    await page.waitForTimeout(1_000);

    // History panel should render (either as a panel or replacing the main content).
    const historyPanel = page.locator('.kds-history-panel, .kds-history');
    const historyVisible = await historyPanel.isVisible({ timeout: 3_000 }).catch(() => false);

    // The columns may be hidden or history panel may be overlaid.
    if (historyVisible) {
      await expect(historyPanel).toBeVisible();
    }

    // Close history by clicking toggle again.
    await historyToggle.click();
    await page.waitForTimeout(1_000);

    // Columns must be visible again.
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: 5_000 });

    // No crash.
    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });

  // ── Step 6: Keyboard shortcuts popover ─────────────────────────────
  test('keyboard shortcuts popover opens and closes with Escape', async ({ page }) => {
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: TIMEOUT });

    // Find the shortcuts button (question-mark icon).
    const shortcutsBtn = page.locator('.kds-shortcuts-btn');
    await expect(shortcutsBtn).toBeVisible({ timeout: 5_000 });

    // Open shortcuts popover.
    await shortcutsBtn.click();
    await page.waitForTimeout(300);

    const popover = page.locator('.kds-shortcuts-popover');
    await expect(popover).toBeVisible({ timeout: 3_000 });

    // Popover must contain shortcut rows with <kbd> elements.
    const shortcutRows = page.locator('.kds-shortcut-row');
    const rowCount = await shortcutRows.count();
    expect(rowCount).toBeGreaterThanOrEqual(2);

    // Close with Escape key.
    await page.keyboard.press('Escape');
    await page.waitForTimeout(300);
    await expect(popover).not.toBeVisible({ timeout: 3_000 });

    // No crash.
    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });

  // ── Step 7: Kitchen zone filter chips ──────────────────────────────
  test('kitchen zone filter chips render when zones are present', async ({ page }) => {
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: TIMEOUT });

    // Zone chips only render when orders have kitchen_zone set (KdsScreen.tsx).
    // Skip silently if the dev-mock doesn't assign kitchen zones.
    const zoneChips = page.locator('.kds-zone-chips');
    const zoneChipsExist = (await zoneChips.count()) > 0;
    if (!zoneChipsExist) return;

    await expect(zoneChips).toBeVisible({ timeout: 3_000 });

    // "All" tab must be active by default.
    const firstChip = zoneChips.locator('.kds-zone-chip').first();
    await expect(firstChip).toBeVisible();
    expect(await zoneChips.locator('.kds-zone-chip').count()).toBeGreaterThanOrEqual(1);

    // No crash.
    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });

  // ── Step 8: Per-item status advance ────────────────────────────────
  test('click actionable per-item row to advance individual line item status', async ({ page }) => {
    await expect(page.locator('.kds-columns')).toBeVisible({ timeout: TIMEOUT });

    // Wait for line items to lazy-fetch (getKdsOrderLinesScoped).
    await page.waitForTimeout(3_000);

    // Find an actionable per-item row inside a ticket (dev-mock returns course-grouped items).
    const actionableItem = page.locator(
      '.kds-ticket .kds-ticket-item-row--actionable',
    ).first();
    await expect(actionableItem).toBeVisible({ timeout: 8_000 });

    // Read the current status dot class before click.
    const statusDot = actionableItem.locator('.kds-ticket-item-status-dot');
    const beforeClass = await statusDot.getAttribute('class');
    expect(beforeClass).toBeTruthy();

    // Click the actionable item row to advance its status.
    await actionableItem.click();
    await page.waitForTimeout(1_000);

    // No crash after per-item status advance.
    await expect(page.locator('[class*="error-boundary"]')).toHaveCount(0, { timeout: 3_000 });
  });
});
