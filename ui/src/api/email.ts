/**
 * Email report API — send test reports and manage report schedules.
 */

import { loggedInvoke } from '@/utils/logged-invoke';

/** Send a test report email using the currently configured SMTP settings. */
export async function sendTestReport(): Promise<string> {
  return loggedInvoke<string>('send_test_report');
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
  return loggedInvoke<ReportScheduleConfig>('get_report_schedule');
}

/** Save the report schedule configuration. */
export async function saveReportSchedule(config: ReportScheduleConfig): Promise<void> {
  return loggedInvoke<void>('save_report_schedule', { config });
}
