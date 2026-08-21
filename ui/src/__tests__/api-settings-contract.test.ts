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
    mockInvoke.mockResolvedValue({ showCurrency: true, decimalSeparator: '.', showTax: true, footer: 'Thanks', paperWidth: '80mm', showTableNumber: true, marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0 });
    await getReceiptSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_receipt_settings');
  });

  it('setReceiptSettings calls correct command', async () => {
    const args = { showCurrency: true, decimalSeparator: '.', showTax: true, footer: 'Thank you', paperWidth: '80mm', showTableNumber: true, marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0 };
    mockInvoke.mockResolvedValue(undefined);
    await setReceiptSettings(args, 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('set_receipt_settings', { args, userId: 'u1' });
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
    await setCreditSettings(args, 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('set_credit_settings', { args, userId: 'u1' });
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
