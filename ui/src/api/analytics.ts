// Analytics IPC wrappers (analytics:view — owner/admin/manager only).
import { loggedInvoke } from '@/utils/logged-invoke';

/** Per-staff analytics row (analytics:view). */
export interface StaffAnalyticsRow {
  user_id: string;
  display_name: string;
  shift_count: number;
  closed_shift_count: number;
  shift_sales_minor: number;
  sale_count: number;
  sale_total_minor: number;
}

/** Per-day series row for one staff member. */
export interface StaffAnalyticsDailyRow {
  day: string;
  sale_count: number;
  sale_total_minor: number;
  shift_count: number;
  shift_sales_minor: number;
}

/**
 * Per-staff shift + sales summary for the session's store over
 * `[from, to]` (inclusive `YYYY-MM-DD`).
 */
export async function getStaffAnalyticsScoped(
  sessionToken: string,
  from: string,
  to: string,
): Promise<StaffAnalyticsRow[]> {
  return loggedInvoke<StaffAnalyticsRow[]>('get_staff_analytics_scoped', {
    sessionToken,
    from,
    to,
  });
}

/**
 * Per-day shift + sales series for one staff member over `[from, to]`
 * (inclusive `YYYY-MM-DD`).
 */
export async function getStaffAnalyticsDailyScoped(
  sessionToken: string,
  userId: string,
  from: string,
  to: string,
): Promise<StaffAnalyticsDailyRow[]> {
  return loggedInvoke<StaffAnalyticsDailyRow[]>('get_staff_analytics_daily_scoped', {
    sessionToken,
    userId,
    from,
    to,
  });
}
