// ── IPC contract tests for sales.ts ───────────────────────────
//
// Verifies that every exported function calls loggedInvoke with the
// correct IPC command name and argument shape. This prevents silent
// drift when the Rust command names change.

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

import {
  startSale,
  startSaleScoped,
  addLine,
  addLineScoped,
  completeSale,
  completeSaleScoped,
  setCartDiscount,
  setCartDiscountScoped,
  overrideLinePrice,
  overrideLinePriceScoped,
  listSales,
  listSalesScoped,
  getSale,
  getSaleScoped,
  voidSale,
  voidSaleScoped,
  holdCart,
  holdCartScoped,
  listHeldCarts,
  listHeldCartsScoped,
  getHeldCart,
  getHeldCartScoped,
  deleteHeldCart,
  deleteHeldCartScoped,
  processRefund,
  processRefundScoped,
  listRefunds,
  listRefundsScoped,
  finalizeSale,
  voidPendingSale,
  exportDailySummary,
  exportDailySummaryScoped,
  exportEodReport,
  exportEodReportScoped,
  printSalesReceipt,
  lookupSaleByReceiptBarcode,
  lookupSaleByReceiptBarcodeScoped,
} from '@/api/sales';

describe('sales.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  // ── Cart Operations ───────────────────────────────────────

  it('startSale → start_sale', async () => {
    mockInvoke.mockResolvedValue({ cartId: 'c1' });
    await startSale({ currency: 'USD' });
    expect(mockInvoke).toHaveBeenCalledWith('start_sale', { args: { currency: 'USD' } });
  });

  it('startSaleScoped → start_sale_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({ cartId: 'c1' });
    await startSaleScoped('tok', { currency: 'IDR' });
    expect(mockInvoke).toHaveBeenCalledWith('start_sale_scoped', { sessionToken: 'tok', args: { currency: 'IDR' } });
  });

  it('addLine → add_line', async () => {
    mockInvoke.mockResolvedValue({ lineId: 'l1', lineTotal: null });
    await addLine({ cartId: 'c1', sku: 'SKU-1', qty: 2, unitPriceMinor: 500 });
    expect(mockInvoke).toHaveBeenCalledWith('add_line', { args: { cartId: 'c1', sku: 'SKU-1', qty: 2, unitPriceMinor: 500 } });
  });

  it('addLineScoped → add_line_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({ lineId: 'l1', lineTotal: null });
    await addLineScoped('tok', { cartId: 'c1', sku: 'SKU-1', qty: 1, unitPriceMinor: 1000 });
    expect(mockInvoke).toHaveBeenCalledWith('add_line_scoped', { sessionToken: 'tok', args: { cartId: 'c1', sku: 'SKU-1', qty: 1, unitPriceMinor: 1000 } });
  });

  it('completeSale → complete_sale', async () => {
    mockInvoke.mockResolvedValue({ saleId: 's1', total: null, lineCount: 1 });
    await completeSale({ cartId: 'c1', paymentMethod: 'cash', tenderedMinor: 10000 });
    expect(mockInvoke).toHaveBeenCalledWith('complete_sale', { args: { cartId: 'c1', paymentMethod: 'cash', tenderedMinor: 10000 } });
  });

  it('completeSaleScoped → complete_sale_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({ saleId: 's1', total: null, lineCount: 1 });
    await completeSaleScoped('tok', { cartId: 'c1', paymentMethod: 'card', tenderedMinor: 5000 });
    expect(mockInvoke).toHaveBeenCalledWith('complete_sale_scoped', { sessionToken: 'tok', args: { cartId: 'c1', paymentMethod: 'card', tenderedMinor: 5000 } });
  });

  it('setCartDiscount → set_cart_discount', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setCartDiscount({ cartId: 'c1', percent: 10, userId: 'u1' });
    expect(mockInvoke).toHaveBeenCalledWith('set_cart_discount', { args: { cartId: 'c1', percent: 10, userId: 'u1' } });
  });

  it('setCartDiscountScoped → set_cart_discount_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setCartDiscountScoped('tok', { cartId: 'c1', percent: 5 });
    expect(mockInvoke).toHaveBeenCalledWith('set_cart_discount_scoped', { sessionToken: 'tok', args: { cartId: 'c1', percent: 5 } });
  });

  it('overrideLinePrice → override_line_price', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await overrideLinePrice({ cartId: 'c1', lineId: 'l1', newPriceMinor: 999, userId: 'u1' });
    expect(mockInvoke).toHaveBeenCalledWith('override_line_price', { args: { cartId: 'c1', lineId: 'l1', newPriceMinor: 999, userId: 'u1' } });
  });

  it('overrideLinePriceScoped → override_line_price_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await overrideLinePriceScoped('tok', 'c1', 'l1', 1500);
    expect(mockInvoke).toHaveBeenCalledWith('override_line_price_scoped', { sessionToken: 'tok', args: { cartId: 'c1', lineId: 'l1', newPriceMinor: 1500 } });
  });

  // ── Sales History ─────────────────────────────────────────

  it('listSales → list_sales (no args)', async () => {
    mockInvoke.mockResolvedValue({ sales: [], salesHistoryCapped: false });
    await listSales();
    expect(mockInvoke).toHaveBeenCalledWith('list_sales', undefined);
  });

  it('listSalesScoped → list_sales_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({ sales: [], salesHistoryCapped: false });
    await listSalesScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_sales_scoped', { sessionToken: 'tok' });
  });

  it('getSale → get_sale with id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getSale('s1');
    expect(mockInvoke).toHaveBeenCalledWith('get_sale', { id: 's1' });
  });

  it('getSaleScoped → get_sale_scoped with sessionToken + id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getSaleScoped('tok', 's1');
    expect(mockInvoke).toHaveBeenCalledWith('get_sale_scoped', { sessionToken: 'tok', id: 's1' });
  });

  // ── Void ──────────────────────────────────────────────────

  it('voidSale → void_sale with args', async () => {
    mockInvoke.mockResolvedValue({ id: 's1', status: 'voided', total: null, line_count: 0, created_at: '' });
    await voidSale({ saleId: 's1', userId: 'u1', reason: 'mistake' });
    expect(mockInvoke).toHaveBeenCalledWith('void_sale', { args: { saleId: 's1', userId: 'u1', reason: 'mistake' } });
  });

  it('voidSaleScoped → void_sale_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ id: 's1', status: 'voided', total: null, line_count: 0, created_at: '' });
    await voidSaleScoped('tok', 's1', 'customer request');
    expect(mockInvoke).toHaveBeenCalledWith('void_sale_scoped', { sessionToken: 'tok', args: { saleId: 's1', reason: 'customer request' } });
  });

  // ── Hold Carts ────────────────────────────────────────────

  it('holdCart → hold_cart with args', async () => {
    mockInvoke.mockResolvedValue({ id: 'h1' });
    await holdCart({ label: 'Table 5', cart_data: '{}', item_count: 3, total_minor: 15000, currency: 'USD' });
    expect(mockInvoke).toHaveBeenCalledWith('hold_cart', { args: { label: 'Table 5', cart_data: '{}', item_count: 3, total_minor: 15000, currency: 'USD' } });
  });

  it('holdCartScoped → hold_cart_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({ id: 'h1' });
    await holdCartScoped('tok', { label: 'T1', cart_data: '{}', item_count: 1, total_minor: 5000, currency: 'USD' });
    expect(mockInvoke).toHaveBeenCalledWith('hold_cart_scoped', { sessionToken: 'tok', args: { label: 'T1', cart_data: '{}', item_count: 1, total_minor: 5000, currency: 'USD' } });
  });

  it('listHeldCarts → list_held_carts (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listHeldCarts();
    expect(mockInvoke).toHaveBeenCalledWith('list_held_carts', undefined);
  });

  it('listHeldCartsScoped → list_held_carts_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await listHeldCartsScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_held_carts_scoped', { sessionToken: 'tok' });
  });

  it('getHeldCart → get_held_cart with id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getHeldCart('h1');
    expect(mockInvoke).toHaveBeenCalledWith('get_held_cart', { id: 'h1' });
  });

  it('getHeldCartScoped → get_held_cart_scoped with sessionToken + id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getHeldCartScoped('tok', 'h1');
    expect(mockInvoke).toHaveBeenCalledWith('get_held_cart_scoped', { sessionToken: 'tok', id: 'h1' });
  });

  it('deleteHeldCart → delete_held_cart with id', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteHeldCart('h1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_held_cart', { id: 'h1' });
  });

  it('deleteHeldCartScoped → delete_held_cart_scoped with sessionToken + id', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteHeldCartScoped('tok', 'h1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_held_cart_scoped', { sessionToken: 'tok', id: 'h1' });
  });

  // ── Refunds ───────────────────────────────────────────────

  it('processRefund → process_refund with args', async () => {
    mockInvoke.mockResolvedValue({ refundId: 'r1', totalMinor: 5000 });
    await processRefund({ saleId: 's1', reason: 'defective', userId: 'u1', lines: [] });
    expect(mockInvoke).toHaveBeenCalledWith('process_refund', { args: { saleId: 's1', reason: 'defective', userId: 'u1', lines: [] } });
  });

  it('processRefundScoped → process_refund_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({ refundId: 'r1', totalMinor: 3000 });
    await processRefundScoped('tok', { saleId: 's1', reason: 'wrong item', lines: [] });
    expect(mockInvoke).toHaveBeenCalledWith('process_refund_scoped', { sessionToken: 'tok', args: { saleId: 's1', reason: 'wrong item', lines: [] } });
  });

  it('listRefunds → list_refunds with saleId', async () => {
    mockInvoke.mockResolvedValue([]);
    await listRefunds('s1');
    expect(mockInvoke).toHaveBeenCalledWith('list_refunds', { saleId: 's1' });
  });

  it('listRefundsScoped → list_refunds_scoped with sessionToken + saleId', async () => {
    mockInvoke.mockResolvedValue([]);
    await listRefundsScoped('tok', 's1');
    expect(mockInvoke).toHaveBeenCalledWith('list_refunds_scoped', { sessionToken: 'tok', saleId: 's1' });
  });

  // ── Pending Sale ──────────────────────────────────────────

  it('finalizeSale → finalize_sale with sessionToken + saleId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await finalizeSale('tok', 's1');
    expect(mockInvoke).toHaveBeenCalledWith('finalize_sale', { sessionToken: 'tok', saleId: 's1' });
  });

  it('voidPendingSale → void_pending_sale with sessionToken + saleId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await voidPendingSale('tok', 's1');
    expect(mockInvoke).toHaveBeenCalledWith('void_pending_sale', { sessionToken: 'tok', saleId: 's1' });
  });

  // ── Reports ───────────────────────────────────────────────

  it('exportDailySummary → export_daily_summary (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await exportDailySummary();
    expect(mockInvoke).toHaveBeenCalledWith('export_daily_summary', undefined);
  });

  it('exportDailySummaryScoped → export_daily_summary_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await exportDailySummaryScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('export_daily_summary_scoped', { sessionToken: 'tok' });
  });

  it('exportEodReport → export_eod_report (no args)', async () => {
    mockInvoke.mockResolvedValue({});
    await exportEodReport();
    expect(mockInvoke).toHaveBeenCalledWith('export_eod_report', undefined);
  });

  it('exportEodReportScoped → export_eod_report_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({});
    await exportEodReportScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('export_eod_report_scoped', { sessionToken: 'tok' });
  });

  // ── Receipt ───────────────────────────────────────────────

  it('printSalesReceipt → print_sales_receipt with args', async () => {
    mockInvoke.mockResolvedValue({ printed: true });
    await printSalesReceipt({ date: '2026-08-19', receiptNumber: 'R001', items: [], subtotal: { minorUnits: 0, currency: 'USD' }, total: { minorUnits: 0, currency: 'USD' }, payments: [] });
    expect(mockInvoke).toHaveBeenCalledWith('print_sales_receipt', expect.objectContaining({ args: expect.objectContaining({ receiptNumber: 'R001' }) }));
  });

  it('lookupSaleByReceiptBarcode → lookup_sale_by_receipt_barcode with barcode', async () => {
    mockInvoke.mockResolvedValue(null);
    await lookupSaleByReceiptBarcode('BC123');
    expect(mockInvoke).toHaveBeenCalledWith('lookup_sale_by_receipt_barcode', { barcode: 'BC123' });
  });

  it('lookupSaleByReceiptBarcodeScoped → lookup_sale_by_receipt_barcode_scoped', async () => {
    mockInvoke.mockResolvedValue(null);
    await lookupSaleByReceiptBarcodeScoped('tok', 'BC456');
    expect(mockInvoke).toHaveBeenCalledWith('lookup_sale_by_receipt_barcode_scoped', { sessionToken: 'tok', barcode: 'BC456' });
  });

  // ── Error propagation ─────────────────────────────────────

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('permission denied'));
    await expect(startSale({ currency: 'USD' })).rejects.toThrow('permission denied');
  });
});
