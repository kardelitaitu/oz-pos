import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  listStoresScoped,
  getStoreProfileScoped,
  getPrimaryStoreScoped,
  createStoreProfileScoped,
  updateStoreProfileScoped,
  setPrimaryStoreScoped,
  deleteStoreProfileScoped,
} from '@/api/stores';

describe('stores.ts API contract', () => {
  const TOKEN = 'tok_store';

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('listStoresScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listStoresScoped(TOKEN);
    expect(mockInvoke).toHaveBeenCalledWith('list_store_profiles_scoped', { sessionToken: TOKEN });
  });

  it('getStoreProfileScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ id: 's1', name: 'Main Store' });
    const result = await getStoreProfileScoped(TOKEN, 's1');
    expect(mockInvoke).toHaveBeenCalledWith('get_store_profile_scoped', { sessionToken: TOKEN, id: 's1' });
    expect(result?.id).toBe('s1');
  });

  it('getPrimaryStoreScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue(null);
    await getPrimaryStoreScoped(TOKEN);
    expect(mockInvoke).toHaveBeenCalledWith('get_primary_store_scoped', { sessionToken: TOKEN });
  });

  it('createStoreProfileScoped calls correct command', async () => {
    const args = { id: 's2', name: 'New Store', address: '123 Main St', tax_id: '123', currency: 'IDR', timezone: 'Asia/Jakarta' };
    mockInvoke.mockResolvedValue({ ...args });
    const result = await createStoreProfileScoped(TOKEN, args);
    expect(mockInvoke).toHaveBeenCalledWith('create_store_profile_scoped', { sessionToken: TOKEN, args });
    expect(result.id).toBe('s2');
  });

  it('updateStoreProfileScoped calls correct command', async () => {
    const args = { id: 's1', name: 'Updated Store', address: '456 New', tax_id: '456', currency: 'IDR', timezone: 'Asia/Jakarta' };
    mockInvoke.mockResolvedValue(args);
    await updateStoreProfileScoped(TOKEN, args);
    expect(mockInvoke).toHaveBeenCalledWith('update_store_profile_scoped', { sessionToken: TOKEN, args });
  });

  it('setPrimaryStoreScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ id: 's1' });
    await setPrimaryStoreScoped(TOKEN, 's1');
    expect(mockInvoke).toHaveBeenCalledWith('set_primary_store_scoped', { sessionToken: TOKEN, id: 's1' });
  });

  it('deleteStoreProfileScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteStoreProfileScoped(TOKEN, 's1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_store_profile_scoped', { sessionToken: TOKEN, id: 's1' });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('store not found'));
    await expect(getStoreProfileScoped(TOKEN, 'bad')).rejects.toThrow('store not found');
  });
});
