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
    await startSale({ currency: 'IDR' });
    expect(mockInvoke).toHaveBeenCalledWith('start_sale', { args: { currency: 'IDR' } });
  });

  it('startSaleScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ saleId: 's1', cartId: 'c1' });
    await startSaleScoped('tok', { currency: 'IDR' });
    expect(mockInvoke).toHaveBeenCalledWith('start_sale_scoped', { sessionToken: 'tok', args: { currency: 'IDR' } });
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
    await holdCart({ label: 'My Table', cart_data: '{}', item_count: 2, total_minor: 5000, currency: 'IDR' });
    expect(mockInvoke).toHaveBeenCalledWith('hold_cart', { args: { label: 'My Table', cart_data: '{}', item_count: 2, total_minor: 5000, currency: 'IDR' } });
  });

  it('holdCartScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ id: 'held-1' });
    await holdCartScoped('tok', { label: 'Table 2', cart_data: '{}', item_count: 1, total_minor: 3000, currency: 'IDR' });
    expect(mockInvoke).toHaveBeenCalledWith('hold_cart_scoped', { sessionToken: 'tok', args: { label: 'Table 2', cart_data: '{}', item_count: 1, total_minor: 3000, currency: 'IDR' } });
  });

  it('voidSale calls correct command', async () => {
    mockInvoke.mockResolvedValue({ voided: true });
    await voidSale({ saleId: 's1', userId: 'u1', reason: 'Damaged' });
    expect(mockInvoke).toHaveBeenCalledWith('void_sale', { args: { saleId: 's1', userId: 'u1', reason: 'Damaged' } });
  });

  it('voidSaleScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ voided: true });
    await voidSaleScoped('tok', 's1', 'Damaged');
    expect(mockInvoke).toHaveBeenCalledWith('void_sale_scoped', { sessionToken: 'tok', args: { saleId: 's1', reason: 'Damaged' } });
  });

  it('processRefund calls correct command', async () => {
    mockInvoke.mockResolvedValue({ refundId: 'r1' });
    await processRefund({ saleId: 's1', userId: 'u1', reason: 'Wrong item', lines: [] });
    expect(mockInvoke).toHaveBeenCalledWith('process_refund', { args: { saleId: 's1', userId: 'u1', reason: 'Wrong item', lines: [] } });
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
    await expect(startSale({ currency: 'IDR' })).rejects.toThrow('cart not found');
  });
});
