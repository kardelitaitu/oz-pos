import { test, expect } from '@playwright/test';
import { loginAs, selectWorkspace, WORKSPACES } from './helpers';

/**
 * E2E: Open/Close Shift — Hard Assertions (E2E-23 through E2E-25)
 *
 * Tests shift management with deterministic assertions. All `if` guards
 * removed — tests hard-fail on regressions.
 *
 * Dev-mock: `get_active_shift` returns `null` initially, so the screen
 * shows "No active shift" with an "Open Shift" button. After opening,
 * `open_shift` returns a mock active shift with status "open". After
 * closing, `close_shift` returns a mock closed shift.
 *
 * CSS contract (ShiftManagementScreen.tsx):
 *   .shift-mgmt                   — container
 *   .shift-mgmt-title             — page title "Shift Management"
 *   .shift-mgmt-no-active         — no active shift banner
 *   .shift-mgmt-no-active-title   — "No active shift" heading
 *   .shift-mgmt-active-card       — active shift card
 *   .shift-mgmt-active-dot        — green status dot
 *   .shift-mgmt-active-label      — "Active Shift" label
 *   .shift-mgmt-overlay           — modal backdrop
 *   .shift-mgmt-modal             — modal panel
 *   .shift-mgmt-modal-header      — modal header with title
 *   .shift-mgmt-modal-body        — modal body
 *   .shift-mgmt-modal-actions     — modal buttons
 *   .shift-mgmt-status-badge--open / --closed — shift status badges
 *   #open-balance                 — opening balance input
 *   #close-balance                — closing balance input
 */

test.describe('Shift Management', () => {
  test.beforeEach(async ({ page }) => {
    await loginAs(page, 'admin', '9999');
    await selectWorkspace(page, WORKSPACES.ADMIN);
  });

  // ── E2E-23: Assert shift screen loads ─────────────────────

  test('shift screen loads and shows "No active shift"', async ({ page }) => {
    // Navigate to shifts page via hash.
    await page.evaluate(() => {
      window.location.hash = '#/shifts';
    });
    await page.waitForTimeout(2_000);

    // Shift management container must be visible.
    const shiftContainer = page.locator('.shift-mgmt');
    await expect(shiftContainer).toBeVisible({ timeout: 10_000 });

    // Title "Shift Management" must be present.
    await expect(page.locator('.shift-mgmt-title')).toBeVisible();

    // Dev-mock returns null for active shift → "No active shift" banner must be visible.
    const noActiveBanner = page.locator('.shift-mgmt-no-active');
    await expect(noActiveBanner).toBeVisible({ timeout: 5_000 });
    await expect(page.locator('.shift-mgmt-no-active-title')).toBeVisible();

    // "Open Shift" button must be visible.
    const openBtn = page.locator('button:has-text("Open Shift"), button:has-text("Buka")');
    await expect(openBtn.first()).toBeVisible({ timeout: 5_000 });
  });

  // ── E2E-24: Open shift flow ───────────────────────────────

  test('opens a shift with opening balance', async ({ page }) => {
    await page.evaluate(() => {
      window.location.hash = '#/shifts';
    });
    await page.waitForTimeout(2_000);

    await expect(page.locator('.shift-mgmt')).toBeVisible({ timeout: 10_000 });

    // Click "Open Shift" button.
    const openBtn = page.locator('button:has-text("Open Shift"), button:has-text("Buka")').first();
    await openBtn.click();
    await page.waitForTimeout(500);

    // Open Shift modal must appear.
    const modal = page.locator('.shift-mgmt-overlay');
    await expect(modal).toBeVisible({ timeout: 3_000 });

    // Modal header should say "Open Shift".
    const modalHeader = page.locator('.shift-mgmt-modal-header h2');
    await expect(modalHeader).toBeVisible({ timeout: 3_000 });

    // Fill opening balance.
    const balanceInput = page.locator('#open-balance');
    await expect(balanceInput).toBeVisible();
    await balanceInput.fill('50000');
    await page.waitForTimeout(200);

    // Click confirm button in modal.
    const confirmBtn = modal.locator('button:has-text("Open Shift"), button:has-text("Buka")');
    await confirmBtn.click();
    await page.waitForTimeout(1_000);

    // After opening, the "No active shift" banner should be gone
    // because an active shift card must appear.
    const activeCard = page.locator('.shift-mgmt-active-card');
    await expect(activeCard).toBeVisible({ timeout: 5_000 });

    // Verify no error boundary.
    const errorBoundary = page.locator('[class*="error-boundary"]');
    const hasError = await errorBoundary.isVisible().catch(() => false);
    expect(hasError).toBe(false);
  });

  // ── E2E-25: Close shift flow ──────────────────────────────

  test('closes an active shift', async ({ page }) => {
    await page.evaluate(() => {
      window.location.hash = '#/shifts';
    });
    await page.waitForTimeout(2_000);

    await expect(page.locator('.shift-mgmt')).toBeVisible({ timeout: 10_000 });

    // Open shift first (mock always returns null for active shift).
    const openBtn = page.locator('button:has-text("Open Shift"), button:has-text("Buka")').first();
    await openBtn.click();

    const openModal = page.locator('.shift-mgmt-overlay');
    await expect(openModal).toBeVisible({ timeout: 3_000 });

    const balanceInput = page.locator('#open-balance');
    await expect(balanceInput).toBeVisible();
    await balanceInput.fill('50000');

    const confirmOpenBtn = openModal.locator('button:has-text("Open Shift"), button:has-text("Buka")');
    await confirmOpenBtn.click();
    await page.waitForTimeout(1_000);

    // Now close the shift.
    const closeBtn = page.locator('button:has-text("Close Shift"), button:has-text("Tutup")').first();
    await expect(closeBtn).toBeVisible({ timeout: 5_000 });
    await closeBtn.click();
    await page.waitForTimeout(500);

    // Close Shift modal must appear.
    const closeModal = page.locator('.shift-mgmt-overlay');
    await expect(closeModal).toBeVisible({ timeout: 3_000 });

    // Fill closing balance.
    const closingInput = page.locator('#close-balance');
    await expect(closingInput).toBeVisible();
    await closingInput.fill('55000');
    await page.waitForTimeout(200);

    // Click confirm close button.
    const confirmCloseBtn = closeModal.locator(
      '.shift-mgmt-modal-actions button:has-text("Close Shift"), .shift-mgmt-modal-actions button:has-text("Tutup")',
    );
    await confirmCloseBtn.click();
    await page.waitForTimeout(1_000);

    // Close summary modal must appear (mock close_shift returns a closed shift).
    const summaryGrid = page.locator('.shift-mgmt-summary-grid');
    await expect(summaryGrid).toBeVisible({ timeout: 5_000 });

    // Verify no crash.
    const errorBoundary = page.locator('[class*="error-boundary"]');
    const hasError = await errorBoundary.isVisible().catch(() => false);
    expect(hasError).toBe(false);
  });
});
