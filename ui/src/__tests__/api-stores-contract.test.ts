import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  listStores,
  getStore,
  getPrimaryStore,
  createStore,
  updateStore,
  setPrimaryStore,
  deleteStore,
} from '@/api/stores';

describe('stores.ts API contract', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('listStores calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listStores();
    expect(mockInvoke).toHaveBeenCalledWith('list_store_profiles');
  });

  it('getStore calls correct command', async () => {
    mockInvoke.mockResolvedValue({ id: 's1', name: 'Main Store' });
    const result = await getStore('s1');
    expect(mockInvoke).toHaveBeenCalledWith('get_store_profile', { id: 's1' });
    expect(result?.id).toBe('s1');
  });

  it('getPrimaryStore calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue(null);
    await getPrimaryStore();
    expect(mockInvoke).toHaveBeenCalledWith('get_primary_store');
  });

  it('createStore calls correct command', async () => {
    const args = { id: 's2', name: 'New Store', address: '123 Main St', tax_id: '123', currency: 'IDR', timezone: 'Asia/Jakarta' };
    mockInvoke.mockResolvedValue({ ...args });
    const result = await createStore(args);
    expect(mockInvoke).toHaveBeenCalledWith('create_store_profile', { args });
    expect(result.id).toBe('s2');
  });

  it('updateStore calls correct command', async () => {
    const args = { id: 's1', name: 'Updated Store', address: '456 New', tax_id: '456', currency: 'IDR', timezone: 'Asia/Jakarta' };
    mockInvoke.mockResolvedValue(args);
    await updateStore(args);
    expect(mockInvoke).toHaveBeenCalledWith('update_store_profile', { args });
  });

  it('setPrimaryStore calls correct command', async () => {
    mockInvoke.mockResolvedValue({ id: 's1' });
    await setPrimaryStore('s1');
    expect(mockInvoke).toHaveBeenCalledWith('set_primary_store', { id: 's1' });
  });

  it('deleteStore calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteStore('s1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_store_profile', { id: 's1' });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('store not found'));
    await expect(getStore('bad')).rejects.toThrow('store not found');
  });
});
