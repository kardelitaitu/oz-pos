/**
 * Email report API — send test reports and manage report schedules.
 */

import { invoke } from '@tauri-apps/api/core';

/** Send a test report email using the currently configured SMTP settings. */
export async function sendTestReport(): Promise<string> {
  return invoke<string>('send_test_report');
}

/** Report schedule configuration persisted in settings. */
export interface ReportScheduleConfig {
  enabled: boolean;
  cadence: string;
  report_types: string[];
  recipients: string[];
  send_at_time: string;
  timezone: string;
  lookback_days: number;
}

/** Get the current report schedule configuration. */
export async function getReportSchedule(): Promise<ReportScheduleConfig> {
  return invoke<ReportScheduleConfig>('get_report_schedule');
}

/** Save the report schedule configuration. */
export async function saveReportSchedule(config: ReportScheduleConfig): Promise<void> {
  return invoke<void>('save_report_schedule', { config });
}
