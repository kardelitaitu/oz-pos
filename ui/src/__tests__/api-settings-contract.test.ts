// ── IPC contract tests for settings.ts ─────────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  getReceiptSettings,
  getReceiptSettingsScoped,
  setReceiptSettings,
  setReceiptSettingsScoped,
  getStoreSettings,
  getStoreSettingsScoped,
  setStoreSettings,
  setStoreSettingsScoped,
  getCreditSettings,
  setCreditSettings,
  setCreditSettingsScoped,
} from '@/api/settings';

describe('settings.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  // ── Receipt Settings ──────────────────────────────────────

  it('getReceiptSettings → get_receipt_settings (no args)', async () => {
    mockInvoke.mockResolvedValue({});
    await getReceiptSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_receipt_settings', undefined);
  });

  it('getReceiptSettingsScoped → get_receipt_settings_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({});
    await getReceiptSettingsScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('get_receipt_settings_scoped', { sessionToken: 'tok' });
  });

  it('setReceiptSettings → set_receipt_settings with args + userId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setReceiptSettings({ storeName: 'My Store', storeAddress: '123 Main St', storePhone: '555-0100', receiptFooter: 'Thank you!', showTaxBreakdown: true, showBarcode: false, paperWidth: 80 }, 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('set_receipt_settings', { args: expect.objectContaining({ storeName: 'My Store' }), userId: 'u1' });
  });

  it('setReceiptSettingsScoped → set_receipt_settings_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setReceiptSettingsScoped('tok', { storeName: 'Shop', storeAddress: '', storePhone: '', receiptFooter: '', showTaxBreakdown: false, showBarcode: false, paperWidth: 58 });
    expect(mockInvoke).toHaveBeenCalledWith('set_receipt_settings_scoped', { sessionToken: 'tok', args: expect.objectContaining({ storeName: 'Shop' }) });
  });

  // ── Store Settings ────────────────────────────────────────

  it('getStoreSettings → get_store_settings (no args)', async () => {
    mockInvoke.mockResolvedValue({});
    await getStoreSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_store_settings', undefined);
  });

  it('getStoreSettingsScoped → get_store_settings_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({});
    await getStoreSettingsScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('get_store_settings_scoped', { sessionToken: 'tok' });
  });

  it('setStoreSettings → set_store_settings with args + userId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setStoreSettings({ storeName: 'My Store', defaultCurrency: 'USD', timezone: 'UTC', locale: 'en-US', fiscalYearStartMonth: 1, lowStockThreshold: 10, defaultTaxRateId: null }, 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('set_store_settings', { args: expect.objectContaining({ storeName: 'My Store' }), userId: 'u1' });
  });

  it('setStoreSettingsScoped → set_store_settings_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setStoreSettingsScoped('tok', { storeName: 'Shop', defaultCurrency: 'IDR', timezone: 'Asia/Jakarta', locale: 'id', fiscalYearStartMonth: 1, lowStockThreshold: 5, defaultTaxRateId: null });
    expect(mockInvoke).toHaveBeenCalledWith('set_store_settings_scoped', { sessionToken: 'tok', args: expect.objectContaining({ defaultCurrency: 'IDR' }) });
  });

  // ── Credit Settings ───────────────────────────────────────

  it('getCreditSettings → get_credit_settings (no args)', async () => {
    mockInvoke.mockResolvedValue({});
    await getCreditSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_credit_settings', undefined);
  });

  it('setCreditSettings → set_credit_settings with args + userId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setCreditSettings({ enabled: true, maxCreditLimitMinor: 500000, paymentTermsDays: 30 }, 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('set_credit_settings', { args: expect.objectContaining({ enabled: true }), userId: 'u1' });
  });

  it('setCreditSettingsScoped → set_credit_settings_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setCreditSettingsScoped('tok', { enabled: false, maxCreditLimitMinor: 0, paymentTermsDays: 0 });
    expect(mockInvoke).toHaveBeenCalledWith('set_credit_settings_scoped', { sessionToken: 'tok', args: expect.objectContaining({ enabled: false }) });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('invalid settings'));
    await expect(getReceiptSettings()).rejects.toThrow('invalid settings');
  });
});
