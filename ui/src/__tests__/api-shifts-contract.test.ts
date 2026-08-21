// ── IPC contract tests for shifts.ts ──────────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  openShift,
  openShiftScoped,
  closeShift,
  closeShiftScoped,
  getActiveShift,
  getActiveShiftScoped,
  listShifts,
  listShiftsScoped,
  getShift,
  getShiftReport,
} from '@/api/shifts';

describe('shifts.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('openShift → open_shift with { args: { userId, terminalId, openingBalanceMinor } }', async () => {
    mockInvoke.mockResolvedValue({ id: 'shift-1' });
    await openShift('u1', 50000);
    expect(mockInvoke).toHaveBeenCalledWith('open_shift', { args: { userId: 'u1', terminalId: null, openingBalanceMinor: 50000 } });
  });

  it('openShiftScoped → open_shift_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ id: 'shift-1' });
    await openShiftScoped('tok', 100000, 'terminal-1');
    expect(mockInvoke).toHaveBeenCalledWith('open_shift_scoped', { sessionToken: 'tok', args: { terminalId: 'terminal-1', openingBalanceMinor: 100000 } });
  });

  it('openShiftScoped without terminalId → null terminalId', async () => {
    mockInvoke.mockResolvedValue({ id: 'shift-1' });
    await openShiftScoped('tok', 50000);
    expect(mockInvoke).toHaveBeenCalledWith('open_shift_scoped', { sessionToken: 'tok', args: { terminalId: null, openingBalanceMinor: 50000 } });
  });

  it('closeShift → close_shift with { args }', async () => {
    mockInvoke.mockResolvedValue({ id: 'shift-1' });
    await closeShift({ id: 'shift-1', userId: 'u1', closingBalanceMinor: 55000, notes: 'End of day' });
    expect(mockInvoke).toHaveBeenCalledWith('close_shift', { args: { id: 'shift-1', userId: 'u1', closingBalanceMinor: 55000, notes: 'End of day' } });
  });

  it('closeShiftScoped → close_shift_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ id: 'shift-1' });
    await closeShiftScoped('tok', 'shift-1', 60000, 'Closed on time');
    expect(mockInvoke).toHaveBeenCalledWith('close_shift_scoped', { sessionToken: 'tok', args: { id: 'shift-1', closingBalanceMinor: 60000, notes: 'Closed on time' } });
  });

  it('closeShiftScoped without notes → null notes', async () => {
    mockInvoke.mockResolvedValue({ id: 'shift-1' });
    await closeShiftScoped('tok', 'shift-1', 50000);
    expect(mockInvoke).toHaveBeenCalledWith('close_shift_scoped', { sessionToken: 'tok', args: { id: 'shift-1', closingBalanceMinor: 50000, notes: null } });
  });

  it('getActiveShift → get_active_shift with userId', async () => {
    mockInvoke.mockResolvedValue(null);
    await getActiveShift('u1');
    expect(mockInvoke).toHaveBeenCalledWith('get_active_shift', { userId: 'u1' });
  });

  it('getActiveShiftScoped → get_active_shift_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue(null);
    await getActiveShiftScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('get_active_shift_scoped', { sessionToken: 'tok' });
  });

  it('listShifts → list_shifts (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listShifts();
    expect(mockInvoke).toHaveBeenCalledWith('list_shifts', undefined);
  });

  it('listShiftsScoped → list_shifts_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await listShiftsScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_shifts_scoped', { sessionToken: 'tok' });
  });

  it('getShift → get_shift with id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getShift('shift-1');
    expect(mockInvoke).toHaveBeenCalledWith('get_shift', { id: 'shift-1' });
  });

  it('getShiftReport → get_shift_report with shiftId', async () => {
    mockInvoke.mockResolvedValue({});
    await getShiftReport('shift-1');
    expect(mockInvoke).toHaveBeenCalledWith('get_shift_report', { shiftId: 'shift-1' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('shift already closed'));
    await expect(closeShift({ id: 'shift-1', userId: 'u1', closingBalanceMinor: 0 })).rejects.toThrow('shift already closed');
  });
});
