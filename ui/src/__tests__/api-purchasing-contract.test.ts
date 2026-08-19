import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  listSuppliers,
  createSupplier,
  updateSupplier,
  getSupplier,
  listPurchaseOrders,
  createPurchaseOrder,
  receivePurchaseOrder,
} from '@/api/purchasing';

describe('purchasing.ts API contract', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('listSuppliers calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listSuppliers();
    expect(mockInvoke).toHaveBeenCalledWith('list_suppliers');
  });

  it('createSupplier calls correct command', async () => {
    const args = { code: 'SUP-001', name: 'PT Supplier', contact_person: 'Budi', email: 'budi@pt.com' };
    mockInvoke.mockResolvedValue({ id: 'sup1', ...args, phone: null, address: null, tax_id: null, payment_terms: null, notes: null });
    const result = await createSupplier(args);
    expect(mockInvoke).toHaveBeenCalledWith('create_supplier', { args });
    expect(result.id).toBe('sup1');
  });

  it('updateSupplier calls correct command', async () => {
    const args = { id: 'sup1', code: 'SUP-001', name: 'Updated Supplier' };
    mockInvoke.mockResolvedValue(args);
    await updateSupplier(args);
    expect(mockInvoke).toHaveBeenCalledWith('update_supplier', { args });
  });

  it('getSupplier calls correct command', async () => {
    mockInvoke.mockResolvedValue(null);
    await getSupplier('sup1');
    expect(mockInvoke).toHaveBeenCalledWith('get_supplier', { id: 'sup1' });
  });

  it('listPurchaseOrders calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listPurchaseOrders();
    expect(mockInvoke).toHaveBeenCalledWith('list_purchase_orders');
  });

  it('createPurchaseOrder calls correct command', async () => {
    const args = {
      po_number: 'PO-001',
      supplier_id: 'sup1',
      lines: [{ sku: 'SKU-001', product_name: 'Widget', qty: 10, unit_cost_minor: 5000 }],
    };
    mockInvoke.mockResolvedValue({ id: 'po1', ...args, status: 'draft', created_at: '2026-01-01' });
    const result = await createPurchaseOrder(args);
    expect(mockInvoke).toHaveBeenCalledWith('create_purchase_order', { args });
    expect(result.id).toBe('po1');
  });

  it('receivePurchaseOrder calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await receivePurchaseOrder('po1');
    expect(mockInvoke).toHaveBeenCalledWith('receive_purchase_order', { id: 'po1' });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('supplier not found'));
    await expect(listSuppliers()).rejects.toThrow('supplier not found');
  });
});
