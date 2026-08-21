import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  createStockTransfer,
  getStockTransfer,
  listStockTransfers,
  getStockTransferLines,
  sendStockTransfer,
  receiveStockTransfer,
  cancelStockTransfer,
} from '@/api/stockTransfers';

describe('stockTransfers.ts API contract', () => {
  const TOKEN = 'tok_transfer';

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('createStockTransfer calls correct command', async () => {
    const lines = [{ id: 'l1', transfer_id: 'st1', sku: 'SKU-1', product_name: 'Widget', qty: 5, received_qty: 0 }];
    mockInvoke.mockResolvedValue({ id: 'st1', status: 'draft' });
    const result = await createStockTransfer(TOKEN, 'loc-src', 'loc-dst', 'term-1', 'term-2', 'test transfer', lines);
    expect(mockInvoke).toHaveBeenCalledWith('create_stock_transfer_scoped', {
      sessionToken: TOKEN,
      sourceLocation: 'loc-src',
      destinationLocation: 'loc-dst',
      sourceTerminalId: 'term-1',
      destinationTerminalId: 'term-2',
      notes: 'test transfer',
      lines,
    });
    expect(result.id).toBe('st1');
  });

  it('getStockTransfer calls correct command', async () => {
    mockInvoke.mockResolvedValue(null);
    await getStockTransfer(TOKEN, 'st1');
    expect(mockInvoke).toHaveBeenCalledWith('get_stock_transfer_scoped', {
      sessionToken: TOKEN,
      id: 'st1',
    });
  });

  it('listStockTransfers calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listStockTransfers(TOKEN);
    expect(mockInvoke).toHaveBeenCalledWith('list_stock_transfers_scoped', {
      sessionToken: TOKEN,
    });
  });

  it('getStockTransferLines calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await getStockTransferLines(TOKEN, 'st1');
    expect(mockInvoke).toHaveBeenCalledWith('get_stock_transfer_lines_scoped', {
      sessionToken: TOKEN,
      transferId: 'st1',
    });
  });

  it('sendStockTransfer calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await sendStockTransfer(TOKEN, 'st1');
    expect(mockInvoke).toHaveBeenCalledWith('send_stock_transfer_scoped', {
      sessionToken: TOKEN,
      id: 'st1',
    });
  });

  it('receiveStockTransfer calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await receiveStockTransfer(TOKEN, 'st1', []);
    expect(mockInvoke).toHaveBeenCalledWith('receive_stock_transfer_scoped', {
      sessionToken: TOKEN,
      id: 'st1',
      receivedLines: [],
    });
  });

  it('cancelStockTransfer calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await cancelStockTransfer(TOKEN, 'st1');
    expect(mockInvoke).toHaveBeenCalledWith('cancel_stock_transfer_scoped', {
      sessionToken: TOKEN,
      id: 'st1',
    });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('not found'));
    await expect(getStockTransfer(TOKEN, 'bad')).rejects.toThrow('not found');
  });
});
