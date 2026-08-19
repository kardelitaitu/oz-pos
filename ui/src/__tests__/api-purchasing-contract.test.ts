// ── IPC contract tests for purchasing.ts ───────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  listSuppliers,
  getSupplier,
  createSupplier,
  updateSupplier,
  listPurchaseOrders,
  getPurchaseOrder,
  createPurchaseOrder,
  updatePoStatus,
  receivePurchaseOrder,
} from '@/api/purchasing';

describe('purchasing.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('listSuppliers → list_suppliers (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listSuppliers();
    expect(mockInvoke).toHaveBeenCalledWith('list_suppliers', undefined);
  });

  it('getSupplier → get_supplier with id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getSupplier('s1');
    expect(mockInvoke).toHaveBeenCalledWith('get_supplier', { id: 's1' });
  });

  it('createSupplier → create_supplier with args', async () => {
    mockInvoke.mockResolvedValue({ id: 's1', name: 'Acme' });
    await createSupplier({ name: 'Acme', contactName: 'Bob', email: 'bob@acme.com', phone: '555-0100', address: '123 St', notes: null });
    expect(mockInvoke).toHaveBeenCalledWith('create_supplier', { args: expect.objectContaining({ name: 'Acme' }) });
  });

  it('updateSupplier → update_supplier with args', async () => {
    mockInvoke.mockResolvedValue({ id: 's1', name: 'Acme Updated' });
    await updateSupplier({ id: 's1', name: 'Acme Updated', contactName: 'Bob', email: 'bob@acme.com', phone: '555-0100', address: '123 St', notes: null });
    expect(mockInvoke).toHaveBeenCalledWith('update_supplier', { args: expect.objectContaining({ id: 's1' }) });
  });

  it('listPurchaseOrders → list_purchase_orders (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listPurchaseOrders();
    expect(mockInvoke).toHaveBeenCalledWith('list_purchase_orders', undefined);
  });

  it('getPurchaseOrder → get_purchase_order with id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getPurchaseOrder('po1');
    expect(mockInvoke).toHaveBeenCalledWith('get_purchase_order', { id: 'po1' });
  });

  it('createPurchaseOrder → create_purchase_order with args', async () => {
    mockInvoke.mockResolvedValue({ id: 'po1' });
    await createPurchaseOrder({ supplierId: 's1', notes: 'Urgent', lines: [] });
    expect(mockInvoke).toHaveBeenCalledWith('create_purchase_order', { args: expect.objectContaining({ supplierId: 's1' }) });
  });

  it('updatePoStatus → update_po_status with args', async () => {
    mockInvoke.mockResolvedValue({ id: 'po1' });
    await updatePoStatus({ id: 'po1', status: 'received' });
    expect(mockInvoke).toHaveBeenCalledWith('update_po_status', { args: expect.objectContaining({ id: 'po1' }) });
  });

  it('receivePurchaseOrder → receive_purchase_order with id', async () => {
    mockInvoke.mockResolvedValue({ id: 'po1' });
    await receivePurchaseOrder('po1');
    expect(mockInvoke).toHaveBeenCalledWith('receive_purchase_order', { id: 'po1' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('supplier not found'));
    await expect(getSupplier('missing')).rejects.toThrow('supplier not found');
  });
});
