// ── Shift Management API ────────────────────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

// ── DTOs ────────────────────────────────────────────────────────────

/** A cashier shift record with opening/closing balances and sales totals. */
export interface ShiftDto {
  id: string;
  userId: string;
  terminalId: string | null;
  openedAt: string;
  closedAt: string | null;
  openingBalanceMinor: number;
  closingBalanceMinor: number | null;
  expectedCashMinor: number | null;
  cashDifferenceMinor: number | null;
  totalSalesMinor: number;
  totalCashMinor: number;
  totalCardMinor: number;
  totalOtherMinor: number;
  totalVoidsMinor: number;
  totalRefundsMinor: number;
  totalPayoutsMinor: number;
  notes: string;
  status: string;
  createdAt: string;
  updatedAt: string;
}

// ── Commands ────────────────────────────────────────────────────────

/** Open a shift (scoped — ADR #7). */
export const openShiftScoped = (sessionToken: string, openingBalanceMinor: number, terminalId?: string | null): Promise<ShiftDto> =>
  loggedInvoke<ShiftDto>('open_shift_scoped', {
    sessionToken,
    args: { terminalId: terminalId ?? null, openingBalanceMinor },
  });

/** Close a shift (scoped — ADR #7). */
export const closeShiftScoped = (sessionToken: string, id: string, closingBalanceMinor: number, notes?: string | null): Promise<ShiftDto> =>
  loggedInvoke<ShiftDto>('close_shift_scoped', {
    sessionToken,
    args: { id, closingBalanceMinor, notes: notes ?? null },
  });

/** Get the active shift for the session user (scoped — ADR #7). */
export const getActiveShiftScoped = (sessionToken: string): Promise<ShiftDto | null> =>
  loggedInvoke<ShiftDto | null>('get_active_shift_scoped', { sessionToken });

/** List all shifts for the store resolved from a session token. ADR #7. */
export const listShiftsScoped = (sessionToken: string): Promise<ShiftDto[]> =>
  loggedInvoke<ShiftDto[]>('list_shifts_scoped', { sessionToken });

/** Get a single shift in the store resolved from a session token. ADR #7. */
export const getShiftScoped = (sessionToken: string, id: string): Promise<ShiftDto | null> =>
  loggedInvoke<ShiftDto | null>('get_shift_scoped', { sessionToken, id });

// ── Cash Payouts ──────────────────────────────────────────────────────

/** A cash payout (safe drop) recorded against a shift. */
export interface CashPayoutDto {
  id: string;
  shiftId: string;
  amountMinor: number;
  reason: string;
  createdAt: string;
}

/** Record a cash payout (safe drop) against the session's open shift. ADR #7. */
export const createCashPayoutScoped = (
  sessionToken: string,
  shiftId: string,
  amountMinor: number,
  reason: string,
): Promise<CashPayoutDto> =>
  loggedInvoke<CashPayoutDto>('create_cash_payout_scoped', {
    sessionToken,
    args: { shiftId, amountMinor, reason },
  });

// ── Shift Report ─────────────────────────────────────────────────────

/** Comprehensive report for a single shift. */
export interface ShiftReportDto {
  shift: ShiftDto;
  paymentBreakdown: ShiftPaymentBreakdownDto[];
  hourlyBreakdown: ShiftSalesByHourDto[];
  cashPayouts: CashPayoutDto[];
  saleCount: number;
  voidCount: number;
  refundCount: number;
  /** Cost of goods sold in minor units (HPP × qty over completed lines). */
  cogsMinor: number;
  /** Gross profit in minor units: completed-sale revenue − COGS. */
  grossProfitMinor: number;
  /** Gross margin as a percentage of revenue. */
  grossMarginPercent: number;
}

/** Payment method totals within a shift. */
export interface ShiftPaymentBreakdownDto {
  method: string;
  count: number;
  totalMinor: number;
}

/** Hourly sales totals within a shift. */
export interface ShiftSalesByHourDto {
  hour: number;
  totalMinor: number;
  saleCount: number;
}

/** Get a comprehensive report for the session's shift. ADR #7. */
export const getShiftReportScoped = (sessionToken: string, shiftId: string): Promise<ShiftReportDto> =>
  loggedInvoke<ShiftReportDto>('get_shift_report_scoped', { sessionToken, shiftId });
