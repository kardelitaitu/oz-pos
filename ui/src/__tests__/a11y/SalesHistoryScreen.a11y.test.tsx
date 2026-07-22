//! A11y regression tests for SalesHistoryScreen.
//!
//! Ensures no axe-core violations are introduced during refactoring.

import { describe, it, vi } from 'vitest';
import { renderWithProviders, checkA11y } from './axe-helper';
import SalesHistoryScreen from '@/features/sales/SalesHistoryScreen';

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

vi.mock('@/api/sales', () => ({
  getSalesHistory: vi.fn(() => Promise.resolve([])),
  voidSale: vi.fn(),
}));

vi.mock('@/api/branding', () => ({
  getBrandSettings: () =>
    Promise.resolve({
      primary_colour: '#10b981',
      logo_path: null,
      store_name: 'OZ-POS',
    }),
}));

describe('SalesHistoryScreen a11y', () => {
  it('has no axe violations on initial render', async () => {
    const { container } = renderWithProviders(<SalesHistoryScreen />);
    // heading-order fixed — EmptyState now supports configurable headingLevel
    await checkA11y(container);
  });
});
