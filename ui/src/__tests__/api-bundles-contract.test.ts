import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  createBundle,
  listBundles,
  getBundle,
  lookupBundleBySku,
  updateBundle,
  deleteBundle,
} from '@/api/bundles';

describe('bundles.ts API contract', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('createBundle calls correct command', async () => {
    const args = {
      bundle_sku: 'BUNDLE-001',
      name: 'Drink Combo',
      items: [{ sku: 'COKE', qty: 2 }],
    };
    const bundleWithItems = { bundle: { id: 'b1', ...args, bundle_price_minor: null, currency: 'IDR', description: '', active: true, created_at: '2026-01-01', updated_at: '2026-01-01' }, items: args.items };
    mockInvoke.mockResolvedValue(bundleWithItems);
    const result = await createBundle(args);
    expect(mockInvoke).toHaveBeenCalledWith('create_bundle', { args });
    expect(result.bundle.bundle_sku).toBe('BUNDLE-001');
  });

  it('listBundles calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listBundles();
    expect(mockInvoke).toHaveBeenCalledWith('list_bundles');
  });

  it('getBundle calls correct command', async () => {
    mockInvoke.mockResolvedValue(null);
    await getBundle('b1');
    expect(mockInvoke).toHaveBeenCalledWith('get_bundle', { id: 'b1' });
  });

  it('lookupBundleBySku calls correct command', async () => {
    mockInvoke.mockResolvedValue(null);
    await lookupBundleBySku('BUNDLE-001');
    expect(mockInvoke).toHaveBeenCalledWith('lookup_bundle_by_sku', { sku: 'BUNDLE-001' });
  });

  it('updateBundle calls correct command', async () => {
    const bundleWithItems = { bundle: { id: 'b1', bundle_sku: 'B-001', name: 'Updated', bundle_price_minor: null, currency: 'IDR', description: '', active: true, created_at: '2026-01-01', updated_at: '2026-01-01' }, items: [] };
    mockInvoke.mockResolvedValue(bundleWithItems);
    await updateBundle(bundleWithItems);
    expect(mockInvoke).toHaveBeenCalledWith('update_bundle', { bundle: bundleWithItems });
  });

  it('deleteBundle calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteBundle('b1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_bundle', { id: 'b1' });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('sku exists'));
    await expect(
      createBundle({ bundle_sku: 'DUP', name: 'Dup', items: [] })
    ).rejects.toThrow('sku exists');
  });
});
