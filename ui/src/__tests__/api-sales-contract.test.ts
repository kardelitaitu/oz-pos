// ── IPC contract tests for sales.ts ───────────────────────────
//
// Verifies that every exported function calls loggedInvoke with the
// correct IPC command name and argument shape. This prevents silent
// drift when the Rust command names change.

import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { CartId } from '@/types/domain';

const mockInvoke = vi.fn();

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  startSale,
  startSaleScoped,
  addLine,
  addLineScoped,
  completeSale,
  completeSaleScoped,
  holdCart,
  holdCartScoped,
  voidSale,
  voidSaleScoped,
  processRefund,
  listSales,
  listSalesScoped,
  getSale,
  getSaleScoped,
  listHeldCarts,
  listHeldCartsScoped,
} from '@/api/sales';

describe('sales.ts API contract', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('startSale calls correct command', async () => {
    mockInvoke.mockResolvedValue({ saleId: 's1', cartId: 'c1' });
    await startSale({ userId: 'u1' });
    expect(mockInvoke).toHaveBeenCalledWith('start_sale', { args: { userId: 'u1' } });
  });

  it('startSaleScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ saleId: 's1', cartId: 'c1' });
    await startSaleScoped('tok', { userId: 'u1' });
    expect(mockInvoke).toHaveBeenCalledWith('start_sale_scoped', { sessionToken: 'tok', args: { userId: 'u1' } });
  });

  it('addLine calls correct command', async () => {
    mockInvoke.mockResolvedValue({ lineId: 'l1' });
    await addLine({ cartId: 'c1' as CartId, sku: 'SKU-1', qty: 2, unitPriceMinor: 500 });
    expect(mockInvoke).toHaveBeenCalledWith('add_line', { args: { cartId: 'c1', sku: 'SKU-1', qty: 2, unitPriceMinor: 500 } });
  });

  it('addLineScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ lineId: 'l1' });
    await addLineScoped('tok', { cartId: 'c1' as CartId, sku: 'SKU-1', qty: 1, unitPriceMinor: 1000 });
    expect(mockInvoke).toHaveBeenCalledWith('add_line_scoped', { sessionToken: 'tok', args: { cartId: 'c1', sku: 'SKU-1', qty: 1, unitPriceMinor: 1000 } });
  });

  it('completeSale calls correct command', async () => {
    mockInvoke.mockResolvedValue({ saleId: 's1' });
    await completeSale({ cartId: 'c1' as CartId, paymentMethod: 'cash', tenderedMinor: 10000 });
    expect(mockInvoke).toHaveBeenCalledWith('complete_sale', { args: { cartId: 'c1', paymentMethod: 'cash', tenderedMinor: 10000 } });
  });

  it('completeSaleScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ saleId: 's1' });
    await completeSaleScoped('tok', { cartId: 'c1' as CartId, paymentMethod: 'card', tenderedMinor: 5000 });
    expect(mockInvoke).toHaveBeenCalledWith('complete_sale_scoped', { sessionToken: 'tok', args: { cartId: 'c1', paymentMethod: 'card', tenderedMinor: 5000 } });
  });

  it('holdCart calls correct command', async () => {
    mockInvoke.mockResolvedValue({ id: 'held-1' });
    await holdCart({ cartId: 'c1' as CartId });
    expect(mockInvoke).toHaveBeenCalledWith('hold_cart', { args: { cartId: 'c1' } });
  });

  it('holdCartScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ id: 'held-1' });
    await holdCartScoped('tok', { cartId: 'c1' as CartId });
    expect(mockInvoke).toHaveBeenCalledWith('hold_cart_scoped', { sessionToken: 'tok', args: { cartId: 'c1' } });
  });

  it('voidSale calls correct command', async () => {
    mockInvoke.mockResolvedValue({ voided: true });
    await voidSale({ saleId: 's1', reason: 'Damaged' });
    expect(mockInvoke).toHaveBeenCalledWith('void_sale', { args: { saleId: 's1', reason: 'Damaged' } });
  });

  it('voidSaleScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ voided: true });
    await voidSaleScoped('tok', 's1', 'Damaged');
    expect(mockInvoke).toHaveBeenCalledWith('void_sale_scoped', { sessionToken: 'tok', args: { saleId: 's1', reason: 'Damaged' } });
  });

  it('processRefund calls correct command', async () => {
    mockInvoke.mockResolvedValue({ refunded: true });
    await processRefund({ saleId: 's1', amountMinor: 1000, reason: 'Customer request' });
    expect(mockInvoke).toHaveBeenCalledWith('process_refund', { args: { saleId: 's1', amountMinor: 1000, reason: 'Customer request' } });
  });

  it('listSales calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue({ sales: [], total: 0 });
    await listSales();
    expect(mockInvoke).toHaveBeenCalledWith('list_sales');
  });

  it('listSalesScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ sales: [], total: 0 });
    await listSalesScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_sales_scoped', { sessionToken: 'tok' });
  });

  it('getSale calls correct command', async () => {
    mockInvoke.mockResolvedValue(null);
    await getSale('s1');
    expect(mockInvoke).toHaveBeenCalledWith('get_sale', { id: 's1' });
  });

  it('getSaleScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue(null);
    await getSaleScoped('tok', 's1');
    expect(mockInvoke).toHaveBeenCalledWith('get_sale_scoped', { sessionToken: 'tok', id: 's1' });
  });

  it('listHeldCarts calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listHeldCarts();
    expect(mockInvoke).toHaveBeenCalledWith('list_held_carts');
  });

  it('listHeldCartsScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listHeldCartsScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_held_carts_scoped', { sessionToken: 'tok' });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('cart not found'));
    await expect(startSale({ userId: 'u1' })).rejects.toThrow('cart not found');
  });
});
