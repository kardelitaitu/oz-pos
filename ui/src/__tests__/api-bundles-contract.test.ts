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
    mockInvoke.mockResolvedValue({ id: 'b1', bundle_sku: 'BUNDLE-001', name: 'Drink Combo', items: args.items });
    const result = await createBundle(args);
    expect(mockInvoke).toHaveBeenCalledWith('create_bundle', { args });
    expect(result.bundle_sku).toBe('BUNDLE-001');
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
    const bundle = { id: 'b1', bundle_sku: 'BUNDLE-001', name: 'Updated', items: [] };
    mockInvoke.mockResolvedValue(bundle);
    await updateBundle(bundle);
    expect(mockInvoke).toHaveBeenCalledWith('update_bundle', { bundle });
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
