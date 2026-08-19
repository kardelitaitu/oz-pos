// ── IPC contract tests for products.ts ─────────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  listProducts,
  listProductsScoped,
  createProduct,
  createProductScoped,
  updateProduct,
  updateProductScoped,
  deleteProduct,
  deleteProductScoped,
  lookupByBarcode,
  lookupByBarcodeScoped,
  recordProductSearchScoped,
} from '@/api/products';

describe('products.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('listProducts → list_products (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listProducts();
    expect(mockInvoke).toHaveBeenCalledWith('list_products', undefined);
  });

  it('listProductsScoped → list_products_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await listProductsScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_products_scoped', { sessionToken: 'tok' });
  });

  it('createProduct → create_product with args', async () => {
    mockInvoke.mockResolvedValue({ sku: 'SKU-1' });
    await createProduct({ name: 'Widget', sku: 'SKU-1', priceMinor: 1000, currency: 'USD', categoryId: null, taxRateId: null, stockQty: 10, unit: 'pc', barcode: null, imageUrl: null, description: null, trackInventory: true, trackSerial: false, isActive: true, tags: null });
    expect(mockInvoke).toHaveBeenCalledWith('create_product', { args: expect.objectContaining({ name: 'Widget', sku: 'SKU-1' }) });
  });

  it('createProductScoped → create_product_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({ sku: 'SKU-2' });
    await createProductScoped('tok', { name: 'Gadget', sku: 'SKU-2', priceMinor: 2000, currency: 'USD', categoryId: null, taxRateId: null, stockQty: 5, unit: 'pc', barcode: null, imageUrl: null, description: null, trackInventory: true, trackSerial: false, isActive: true, tags: null });
    expect(mockInvoke).toHaveBeenCalledWith('create_product_scoped', { sessionToken: 'tok', args: expect.objectContaining({ name: 'Gadget' }) });
  });

  it('updateProduct → update_product with args', async () => {
    mockInvoke.mockResolvedValue({ sku: 'SKU-1' });
    await updateProduct({ sku: 'SKU-1', name: 'Widget Updated', priceMinor: 1500, currency: 'USD', categoryId: null, taxRateId: null, stockQty: 8, unit: 'pc', barcode: null, imageUrl: null, description: null, trackInventory: true, trackSerial: false, isActive: true, tags: null });
    expect(mockInvoke).toHaveBeenCalledWith('update_product', { args: expect.objectContaining({ sku: 'SKU-1' }) });
  });

  it('updateProductScoped → update_product_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({ sku: 'SKU-1' });
    await updateProductScoped('tok', { sku: 'SKU-1', name: 'V2', priceMinor: 1500, currency: 'USD', categoryId: null, taxRateId: null, stockQty: 8, unit: 'pc', barcode: null, imageUrl: null, description: null, trackInventory: true, trackSerial: false, isActive: true, tags: null });
    expect(mockInvoke).toHaveBeenCalledWith('update_product_scoped', { sessionToken: 'tok', args: expect.objectContaining({ sku: 'SKU-1' }) });
  });

  it('deleteProduct → delete_product with args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteProduct({ userId: 'u1', sku: 'SKU-1' });
    expect(mockInvoke).toHaveBeenCalledWith('delete_product', { args: { userId: 'u1', sku: 'SKU-1' } });
  });

  it('deleteProductScoped → delete_product_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteProductScoped('tok', 'SKU-1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_product_scoped', { sessionToken: 'tok', args: { sku: 'SKU-1' } });
  });

  it('lookupByBarcode → lookup_by_barcode with barcode', async () => {
    mockInvoke.mockResolvedValue(null);
    await lookupByBarcode('1234567890');
    expect(mockInvoke).toHaveBeenCalledWith('lookup_by_barcode', { barcode: '1234567890' });
  });

  it('lookupByBarcodeScoped → lookup_by_barcode_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue(null);
    await lookupByBarcodeScoped('tok', '9999999999');
    expect(mockInvoke).toHaveBeenCalledWith('lookup_by_barcode_scoped', { sessionToken: 'tok', barcode: '9999999999' });
  });

  it('recordProductSearchScoped → record_product_search_scoped', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await recordProductSearchScoped('tok', 'SKU-1');
    expect(mockInvoke).toHaveBeenCalledWith('record_product_search_scoped', { sessionToken: 'tok', sku: 'SKU-1' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('sku already exists'));
    await expect(createProduct({ name: 'X', sku: 'DUP', priceMinor: 0, currency: 'USD', categoryId: null, taxRateId: null, stockQty: 0, unit: 'pc', barcode: null, imageUrl: null, description: null, trackInventory: false, trackSerial: false, isActive: true, tags: null })).rejects.toThrow('sku already exists');
  });
});
