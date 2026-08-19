// ── IPC contract tests for inventoryCounts.ts ──────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  createStockCount,
  getStockCount,
  listStockCounts,
  getCountLines,
  addCountLine,
  updateCountLine,
  removeCountLine,
  completeStockCount,
  updateStockCountStatus,
  listStockAdjustments,
} from '@/api/inventoryCounts';

describe('inventoryCounts.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('createStockCount → create_stock_count_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ id: 'c1' });
    await createStockCount('tok', { name: 'Weekly Count', locationId: 'loc1' });
    expect(mockInvoke).toHaveBeenCalledWith('create_stock_count_scoped', { sessionToken: 'tok', args: expect.objectContaining({ name: 'Weekly Count' }) });
  });

  it('getStockCount → get_stock_count_scoped', async () => {
    mockInvoke.mockResolvedValue(null);
    await getStockCount('tok', 'c1');
    expect(mockInvoke).toHaveBeenCalledWith('get_stock_count_scoped', { sessionToken: 'tok', id: 'c1' });
  });

  it('listStockCounts → list_stock_counts_scoped', async () => {
    mockInvoke.mockResolvedValue([]);
    await listStockCounts('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_stock_counts_scoped', { sessionToken: 'tok' });
  });

  it('getCountLines → get_count_lines_scoped', async () => {
    mockInvoke.mockResolvedValue([]);
    await getCountLines('tok', 'c1');
    expect(mockInvoke).toHaveBeenCalledWith('get_count_lines_scoped', { sessionToken: 'tok', countId: 'c1' });
  });

  it('addCountLine → add_count_line_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ id: 'l1' });
    await addCountLine('tok', { countId: 'c1', sku: 'SKU-1', countedQty: 10 });
    expect(mockInvoke).toHaveBeenCalledWith('add_count_line_scoped', { sessionToken: 'tok', args: expect.objectContaining({ countId: 'c1' }) });
  });

  it('updateCountLine → update_count_line_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updateCountLine('tok', { id: 'l1', countedQty: 15 });
    expect(mockInvoke).toHaveBeenCalledWith('update_count_line_scoped', { sessionToken: 'tok', args: { id: 'l1', countedQty: 15 } });
  });

  it('removeCountLine → remove_count_line_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await removeCountLine('tok', { id: 'l1' });
    expect(mockInvoke).toHaveBeenCalledWith('remove_count_line_scoped', { sessionToken: 'tok', args: { id: 'l1' } });
  });

  it('completeStockCount → complete_stock_count_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue([]);
    await completeStockCount('tok', { id: 'c1' });
    expect(mockInvoke).toHaveBeenCalledWith('complete_stock_count_scoped', { sessionToken: 'tok', args: { id: 'c1' } });
  });

  it('updateStockCountStatus → update_stock_count_status_scoped', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updateStockCountStatus('tok', 'c1', 'completed');
    expect(mockInvoke).toHaveBeenCalledWith('update_stock_count_status_scoped', { sessionToken: 'tok', id: 'c1', status: 'completed' });
  });

  it('listStockAdjustments → list_stock_adjustments_scoped', async () => {
    mockInvoke.mockResolvedValue([]);
    await listStockAdjustments('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_stock_adjustments_scoped', { sessionToken: 'tok' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('count not found'));
    await expect(getStockCount('tok', 'missing')).rejects.toThrow('count not found');
  });
});
