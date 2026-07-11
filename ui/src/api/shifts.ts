// ── Shift Management API ────────────────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

// ── DTOs ────────────────────────────────────────────────────────────

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

/** Open a new shift for a user. */
export const openShift = (userId: string, openingBalanceMinor: number): Promise<ShiftDto> =>
  invoke<ShiftDto>('open_shift', {
    args: { userId, terminalId: null as string | null, openingBalanceMinor },
  });

/** Open a shift (scoped — ADR #7). */
export const openShiftScoped = (sessionToken: string, openingBalanceMinor: number, terminalId?: string | null): Promise<ShiftDto> =>
  invoke<ShiftDto>('open_shift_scoped', {
    sessionToken,
    args: { terminalId: terminalId ?? null, openingBalanceMinor },
  });

/** Arguments for closing a shift. */
export interface CloseShiftArgs {
  userId: string;
  id: string;
  closingBalanceMinor: number;
  notes?: string | null;
}

/** Close an active shift with a counted closing balance. */
export const closeShift = (args: CloseShiftArgs): Promise<ShiftDto> =>
  invoke<ShiftDto>('close_shift', { args });

/** Close a shift (scoped — ADR #7). */
export const closeShiftScoped = (sessionToken: string, id: string, closingBalanceMinor: number, notes?: string | null): Promise<ShiftDto> =>
  invoke<ShiftDto>('close_shift_scoped', {
    sessionToken,
    args: { id, closingBalanceMinor, notes: notes ?? null },
  });

/** Get the currently open shift for a user, if any. */
export const getActiveShift = (userId: string): Promise<ShiftDto | null> =>
  invoke<ShiftDto | null>('get_active_shift', { userId });

/** Get the active shift for the session user (scoped — ADR #7). */
export const getActiveShiftScoped = (sessionToken: string): Promise<ShiftDto | null> =>
  invoke<ShiftDto | null>('get_active_shift_scoped', { sessionToken });

/** List all shifts (most recent first). */
export const listShifts = (): Promise<ShiftDto[]> =>
  invoke<ShiftDto[]>('list_shifts');

/** Get a single shift by id. */
export const getShift = (id: string): Promise<ShiftDto | null> =>
  invoke<ShiftDto | null>('get_shift', { id });

// ── Cash Payouts ──────────────────────────────────────────────────────

export interface CashPayoutDto {
  id: string;
  shiftId: string;
  amountMinor: number;
  reason: string;
  createdAt: string;
}

/** Record a cash payout (safe drop) against an open shift. */
export const createCashPayout = (
  shiftId: string,
  amountMinor: number,
  reason: string,
): Promise<CashPayoutDto> =>
  invoke<CashPayoutDto>('create_cash_payout', {
    args: { shiftId, amountMinor, reason },
  });

// ── Shift Report ─────────────────────────────────────────────────────

export interface ShiftReportDto {
  shift: ShiftDto;
  paymentBreakdown: ShiftPaymentBreakdownDto[];
  hourlyBreakdown: ShiftSalesByHourDto[];
  cashPayouts: CashPayoutDto[];
  saleCount: number;
  voidCount: number;
  refundCount: number;
}

export interface ShiftPaymentBreakdownDto {
  method: string;
  count: number;
  totalMinor: number;
}

export interface ShiftSalesByHourDto {
  hour: number;
  totalMinor: number;
  saleCount: number;
}

/** Get a comprehensive report for a single shift. */
export const getShiftReport = (shiftId: string): Promise<ShiftReportDto> =>
  invoke<ShiftReportDto>('get_shift_report', { shiftId });
