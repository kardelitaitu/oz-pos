import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  getReceiptSettings,
  setReceiptSettings,
  getStoreSettings,
  setStoreSettings,
  getCreditSettings,
  setCreditSettings,
  getStoreSettingsScoped,
} from '@/api/settings';

describe('settings.ts API contract', () => {
  const TOKEN = 'tok_settings';

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('getReceiptSettings calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue({ header: 'Store', footer: 'Thanks' });
    await getReceiptSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_receipt_settings');
  });

  it('setReceiptSettings calls correct command', async () => {
    const args = { header: 'Updated', footer: 'Thank you' };
    mockInvoke.mockResolvedValue(undefined);
    await setReceiptSettings(args);
    expect(mockInvoke).toHaveBeenCalledWith('set_receipt_settings', { args });
  });

  it('getStoreSettings calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue({ name: 'My Store', address: '123 Main St', taxId: '123', currency: 'IDR', branch: 'Main' });
    await getStoreSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_store_settings');
  });

  it('setStoreSettings calls correct command', async () => {
    const args = { name: 'My Store', address: '456 New', taxId: '456', currency: 'IDR', branch: 'New' };
    mockInvoke.mockResolvedValue(undefined);
    await setStoreSettings(args, 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('set_store_settings', { args, userId: 'u1' });
  });

  it('getCreditSettings calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue({ enabled: true, reminderIntervalHours: 24, maxLimitMinor: 500000 });
    await getCreditSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_credit_settings');
  });

  it('setCreditSettings calls correct command', async () => {
    const args = { enabled: false, reminderIntervalHours: 48, maxLimitMinor: 1000000 };
    mockInvoke.mockResolvedValue(undefined);
    await setCreditSettings(args);
    expect(mockInvoke).toHaveBeenCalledWith('set_credit_settings', { args });
  });

  it('getStoreSettingsScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ name: 'Scoped Store', address: '789', taxId: '000', currency: 'IDR', branch: 'Scoped' });
    await getStoreSettingsScoped(TOKEN);
    expect(mockInvoke).toHaveBeenCalledWith('get_store_settings_scoped', { sessionToken: TOKEN });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('settings error'));
    await expect(getReceiptSettings()).rejects.toThrow('settings error');
  });
});
