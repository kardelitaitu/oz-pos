import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

/**
 * Workspace type_key values used in the mock fallback workspaces
 * (see WorkspaceContext.tsx FALLBACK_WORKSPACES).
 */
export const WORKSPACES = {
  STORE_POS: 'store-pos',
  RESTAURANT_POS: 'restaurant-pos',
  KDS: 'kds',
  INVENTORY: 'inventory',
  ADMIN: 'admin',
} as const;

/**
 * Log in as a staff member.
 *
 * Uses the dev-mock Tauri IPC which accepts:
 *   - owner / 1234  (role: owner)
 *   - admin / admin123  (role: manager)
 *   - kasir / 1234  (role: cashier)
 */
export async function loginAs(
  page: Page,
  username: string,
  pin: string,
): Promise<void> {
  await page.goto('/');

  // Wait for the login screen using its CSS class.
  await page.waitForSelector('.staff-login-screen', { timeout: 15_000 });

  // Enter username into the input with class `staff-login-input`.
  const usernameInput = page.locator('.staff-login-input').first();
  await expect(usernameInput).toBeVisible({ timeout: 5_000 });
  await usernameInput.fill(username);

  // Click the submit button to proceed to PIN step.
  const submitBtn = page.locator('.staff-login-submit-btn');
  await submitBtn.click();

  // Wait for PIN pad to appear — the `.staff-login-pad` element becomes visible.
  const pinPad = page.locator('.staff-login-pad');
  await pinPad.waitFor({ state: 'visible', timeout: 10_000 });

  // Enter PIN digits using the keypad buttons (`.staff-login-pad-key`).
  for (const digit of pin.split('')) {
    const key = page.locator('.staff-login-pad-key').filter({ hasText: digit });
    await key.click();
    // Small delay so the visual dot updates before the next digit.
    await page.waitForTimeout(100);
  }

  // After PIN entry, login processes automatically when PIN reaches max length.
  // Wait for the workspace home screen (`.workspace-home`) to appear.
  await page.waitForSelector('.workspace-home', { timeout: 15_000 });
}

/**
 * Select a workspace from the workspace picker (WorkspaceHome).
 *
 * WorkspaceHome renders workspace cards (`.workspace-card`) with
 * an `<h2 class="workspace-card-name">` containing the workspace name.
 */
export async function selectWorkspace(
  page: Page,
  typeKey: string,
): Promise<void> {
  // Wait for the workspace picker title.
  await page.waitForSelector('.workspace-home', { timeout: 10_000 });

  // Map type_key to the display name used in FALLBACK_WORKSPACES.
  const workspaceNames: Record<string, string> = {
    'store-pos': 'Store POS',
    'restaurant-pos': 'Restaurant POS',
    kds: 'Kitchen Display',
    inventory: 'Inventory Management',
    admin: 'Admin',
  };

  const name = workspaceNames[typeKey];
  if (!name) throw new Error(`Unknown workspace type_key: ${typeKey}`);

  // Find the workspace card with matching name.
  const card = page.locator('.workspace-card').filter({ hasText: name });
  await expect(card).toBeVisible({ timeout: 5_000 });

  // The card is a <button> — click it to activate the workspace.
  await card.click();

  // Wait for navigation to complete.
  await page.waitForTimeout(2_000);
}

/**
 * Wait for the app to be fully loaded and ready.
 */
export async function waitForApp(page: Page): Promise<void> {
  await page.waitForSelector('#root', { timeout: 15_000 });
  await page.waitForTimeout(1_000);
}
