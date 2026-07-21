/**
 * Email report API — send test reports from the Settings screen.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Send a test report email using the currently configured SMTP
 * settings and report schedule.
 *
 * @returns A success message describing the result.
 * @throws An error message string if sending fails.
 */
export async function sendTestReport(): Promise<string> {
  return invoke<string>('send_test_report');
}
