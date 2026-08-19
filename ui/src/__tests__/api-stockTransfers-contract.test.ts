// ── IPC contract tests for stockTransfers.ts ───────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  createStockTransfer,
  getStockTransfer,
  listStockTransfers,
  listInTransitTransfers,
  getStockTransferLines,
  addStockTransferLine,
  removeStockTransferLine,
  sendStockTransfer,
  receiveStockTransfer,
  cancelStockTransfer,
} from '@/api/stockTransfers';

describe('stockTransfers.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('createStockTransfer → create_stock_transfer_scoped with flat params', async () => {
    mockInvoke.mockResolvedValue({ id: 't1' });
    await createStockTransfer('tok', 'loc1', 'loc2', null, null, 'Transfer notes');
    expect(mockInvoke).toHaveBeenCalledWith('create_stock_transfer_scoped', {
      sessionToken: 'tok',
      sourceLocation: 'loc1',
      destinationLocation: 'loc2',
      sourceTerminalId: null,
      destinationTerminalId: null,
      notes: 'Transfer notes',
    });
  });

  it('getStockTransfer → get_stock_transfer_scoped', async () => {
    mockInvoke.mockResolvedValue(null);
    await getStockTransfer('tok', 't1');
    expect(mockInvoke).toHaveBeenCalledWith('get_stock_transfer_scoped', { sessionToken: 'tok', id: 't1' });
  });

  it('listStockTransfers → list_stock_transfers_scoped', async () => {
    mockInvoke.mockResolvedValue([]);
    await listStockTransfers('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_stock_transfers_scoped', { sessionToken: 'tok' });
  });

  it('listInTransitTransfers → list_in_transit_transfers_scoped', async () => {
    mockInvoke.mockResolvedValue([]);
    await listInTransitTransfers('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_in_transit_transfers_scoped', { sessionToken: 'tok' });
  });

  it('getStockTransferLines → get_stock_transfer_lines_scoped', async () => {
    mockInvoke.mockResolvedValue([]);
    await getStockTransferLines('tok', 't1');
    expect(mockInvoke).toHaveBeenCalledWith('get_stock_transfer_lines_scoped', { sessionToken: 'tok', transferId: 't1' });
  });

  it('addStockTransferLine → add_stock_transfer_line_scoped with flat params', async () => {
    mockInvoke.mockResolvedValue({ id: 'l1' });
    await addStockTransferLine('tok', 't1', 'SKU-1', 'Widget', 10);
    expect(mockInvoke).toHaveBeenCalledWith('add_stock_transfer_line_scoped', {
      sessionToken: 'tok',
      transferId: 't1',
      sku: 'SKU-1',
      productName: 'Widget',
      qty: 10,
    });
  });

  it('removeStockTransferLine → remove_stock_transfer_line_scoped', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await removeStockTransferLine('tok', 'l1');
    expect(mockInvoke).toHaveBeenCalledWith('remove_stock_transfer_line_scoped', { sessionToken: 'tok', lineId: 'l1' });
  });

  it('sendStockTransfer → send_stock_transfer_scoped', async () => {
    mockInvoke.mockResolvedValue({ id: 't1' });
    await sendStockTransfer('tok', 't1');
    expect(mockInvoke).toHaveBeenCalledWith('send_stock_transfer_scoped', { sessionToken: 'tok', id: 't1' });
  });

  it('receiveStockTransfer → receive_stock_transfer_scoped with flat params', async () => {
    mockInvoke.mockResolvedValue({ id: 't1' });
    await receiveStockTransfer('tok', 't1', []);
    expect(mockInvoke).toHaveBeenCalledWith('receive_stock_transfer_scoped', {
      sessionToken: 'tok',
      id: 't1',
      receivedLines: [],
    });
  });

  it('cancelStockTransfer → cancel_stock_transfer_scoped', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await cancelStockTransfer('tok', 't1');
    expect(mockInvoke).toHaveBeenCalledWith('cancel_stock_transfer_scoped', { sessionToken: 'tok', id: 't1' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('transfer not found'));
    await expect(getStockTransfer('tok', 'missing')).rejects.toThrow('transfer not found');
  });
});
