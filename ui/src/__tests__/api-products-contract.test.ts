import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  createProduct,
  createProductScoped,
  updateProduct,
  updateProductScoped,
  deleteProduct,
  deleteProductScoped,
  lookupByBarcode,
  recordProductSearchScoped,
} from '@/api/products';

describe('products.ts API contract', () => {
  const TOKEN = 'tok_prod';

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('createProduct calls correct command', async () => {
    const args = {
      userId: 'u1',
      sku: 'SKU-001',
      name: 'Test Product',
      priceMinor: 10000,
      currency: 'IDR',
      initialStock: 10,
      taxRateIds: ['t1'],
    };
    mockInvoke.mockResolvedValue({ sku: 'SKU-001' });
    const result = await createProduct(args);
    expect(mockInvoke).toHaveBeenCalledWith('create_product', { args });
    expect(result.sku).toBe('SKU-001');
  });

  it('createProductScoped calls correct command', async () => {
    const args = {
      userId: 'u1',
      sku: 'SKU-002',
      name: 'Scoped Product',
      priceMinor: 5000,
      currency: 'IDR',
      initialStock: 5,
      taxRateIds: [],
    };
    mockInvoke.mockResolvedValue({ sku: 'SKU-002' });
    await createProductScoped(TOKEN, args);
    expect(mockInvoke).toHaveBeenCalledWith('create_product_scoped', {
      sessionToken: TOKEN,
      args,
    });
  });

  it('updateProduct calls correct command', async () => {
    const args = { userId: 'u1', sku: 'SKU-001', name: 'Updated', priceMinor: 15000, currency: 'IDR', taxRateIds: ['t1'] };
    mockInvoke.mockResolvedValue({ sku: 'SKU-001' });
    await updateProduct(args);
    expect(mockInvoke).toHaveBeenCalledWith('update_product', { args });
  });

  it('updateProductScoped calls correct command', async () => {
    const args = { sku: 'SKU-001', name: 'Updated', priceMinor: 15000, currency: 'IDR', taxRateIds: [] };
    mockInvoke.mockResolvedValue({ sku: 'SKU-001' });
    await updateProductScoped(TOKEN, args);
    expect(mockInvoke).toHaveBeenCalledWith('update_product_scoped', {
      sessionToken: TOKEN,
      args,
    });
  });

  it('deleteProduct calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteProduct({ userId: 'u1', sku: 'SKU-001' });
    expect(mockInvoke).toHaveBeenCalledWith('delete_product', { args: { userId: 'u1', sku: 'SKU-001' } });
  });

  it('deleteProductScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteProductScoped(TOKEN, 'SKU-001');
    expect(mockInvoke).toHaveBeenCalledWith('delete_product_scoped', {
      sessionToken: TOKEN,
      sku: 'SKU-001',
    });
  });

  it('lookupByBarcode calls correct command', async () => {
    mockInvoke.mockResolvedValue(null);
    await lookupByBarcode('123456');
    expect(mockInvoke).toHaveBeenCalledWith('lookup_by_barcode', { barcode: '123456' });
  });

  it('recordProductSearchScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await recordProductSearchScoped(TOKEN, 'SKU-001');
    expect(mockInvoke).toHaveBeenCalledWith('record_product_search_scoped', {
      sessionToken: TOKEN,
      sku: 'SKU-001',
    });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('sku duplicate'));
    await expect(
      createProduct({
        userId: 'u1',
        sku: 'DUP',
        name: 'Dup',
        priceMinor: 0,
        currency: 'IDR',
        initialStock: 0,
        taxRateIds: [],
      })
    ).rejects.toThrow('sku duplicate');
  });

  it('passes return type through', async () => {
    mockInvoke.mockResolvedValue({ sku: 'SKU-NEW' });
    const result = await createProduct({
      userId: 'u1',
      sku: 'SKU-NEW',
      name: 'Product',
      priceMinor: 10000,
      currency: 'IDR',
      initialStock: 10,
      taxRateIds: [],
    });
    expect(result.sku).toBe('SKU-NEW');
  });
});
