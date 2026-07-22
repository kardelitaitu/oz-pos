//! A11y regression tests for SettingsPage.
//!
//! Ensures no axe-core violations are introduced during refactoring.

import { describe, it, vi } from 'vitest';
import { renderWithProviders, checkA11y } from './axe-helper';
import SettingsPage from '@/features/settings/SettingsPage';

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { username: 'test', role: 'owner', displayName: 'Test' },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    isManager: true,
    isOwner: true,
  }),
}));

vi.mock('@/api/branding', () => ({
  getBrandSettings: () =>
    Promise.resolve({
      primary_colour: '#10b981',
      logo_path: null,
      store_name: 'OZ-POS',
    }),
}));

vi.mock('@/api/settings', () => ({
  getReceiptSettings: vi.fn(() => Promise.resolve({ showCurrency: false, decimalSeparator: 'dot', showTax: true, footer: '', paperWidth: 'standard', showTableNumber: false, marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0 })),
  getStoreSettings: vi.fn(() => Promise.resolve({ name: 'Test Store', address: '', taxId: '', currency: 'IDR', branch: '' })),
  getUserPreferences: vi.fn(() => Promise.resolve({})),
  setSetting: vi.fn(),
}));

vi.mock('@/api/license', () => ({
  getLicenseStatus: vi.fn(() => Promise.resolve({ tier: 'pro', valid: true })),
}));

vi.mock('@/api/system', () => ({
  getVersion: vi.fn(() => Promise.resolve({ version: '0.0.19' })),
}));

vi.mock('@/api/currency', () => ({
  listCurrencies: vi.fn(() => Promise.resolve([{ code: 'IDR', name: 'Rupiah', symbol: 'Rp' }])),
}));

vi.mock('@/api/offline', () => ({
  getSyncSettings: vi.fn(() => Promise.resolve({ serverUrl: null, hasApiKey: false, enabled: false })),
  updateSyncSettings: vi.fn(),
  syncRun: vi.fn(),
  syncPull: vi.fn(),
  pendingSyncCount: vi.fn(() => Promise.resolve(0)),
  testSyncConnection: vi.fn(),
  requestSyncToken: vi.fn(),
}));

vi.mock('@/api/topology', () => ({
  saveTopology: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(() => Promise.resolve()),
}));

describe('SettingsPage a11y', () => {
  it('has no axe violations on initial render', async () => {
    const { container } = renderWithProviders(<SettingsPage />);
    await checkA11y(container);
  });
});
