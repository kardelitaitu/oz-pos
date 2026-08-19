// ── IPC contract tests for bundles.ts ──────────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  listBundles,
  getBundle,
  createBundle,
  updateBundle,
  deleteBundle,
  lookupBundleBySku,
} from '@/api/bundles';

describe('bundles.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('listBundles → list_bundles (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listBundles();
    expect(mockInvoke).toHaveBeenCalledWith('list_bundles', undefined);
  });

  it('getBundle → get_bundle with id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getBundle('b1');
    expect(mockInvoke).toHaveBeenCalledWith('get_bundle', { id: 'b1' });
  });

  it('createBundle → create_bundle with args', async () => {
    mockInvoke.mockResolvedValue({ id: 'b1', name: 'Starter Pack' });
    await createBundle({ name: 'Starter Pack', sku: 'BUNDLE-001', items: [] });
    expect(mockInvoke).toHaveBeenCalledWith('create_bundle', { args: expect.objectContaining({ name: 'Starter Pack' }) });
  });

  it('updateBundle → update_bundle with bundle', async () => {
    mockInvoke.mockResolvedValue({ id: 'b1', name: 'Updated Pack' });
    await updateBundle({ id: 'b1', name: 'Updated Pack', sku: 'BUNDLE-001', items: [] });
    expect(mockInvoke).toHaveBeenCalledWith('update_bundle', { bundle: expect.objectContaining({ id: 'b1' }) });
  });

  it('deleteBundle → delete_bundle with id', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteBundle('b1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_bundle', { id: 'b1' });
  });

  it('lookupBundleBySku → lookup_bundle_by_sku with sku', async () => {
    mockInvoke.mockResolvedValue(null);
    await lookupBundleBySku('BUNDLE-001');
    expect(mockInvoke).toHaveBeenCalledWith('lookup_bundle_by_sku', { sku: 'BUNDLE-001' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('bundle not found'));
    await expect(getBundle('missing')).rejects.toThrow('bundle not found');
  });
});
