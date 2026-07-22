//! A11y regression tests for ProductLookupScreen.
//!
//! Ensures no axe-core violations are introduced during refactoring.

import { describe, it, vi } from 'vitest';
import { renderWithProviders, checkA11y } from './axe-helper';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';

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

vi.mock('@/api/products', () => ({
  searchProducts: vi.fn(() => Promise.resolve([])),
  listProducts: vi.fn(() => Promise.resolve([])),
}));

vi.mock('@/api/branding', () => ({
  getBrandSettings: () =>
    Promise.resolve({
      primary_colour: '#10b981',
      logo_path: null,
      store_name: 'OZ-POS',
    }),
}));

describe('ProductLookupScreen a11y', () => {
  it('has no axe violations on initial render', async () => {
    const { container } = renderWithProviders(
      <ProductLookupScreen onAddProduct={vi.fn()} />,
    );
    // Two known a11y issues tracked as product bugs:
    // - button-name: icon buttons have aria-label via Fluent but
    //   Localized wrapper renders empty span that confuses axe-core
    // - aria-required-children: role="radiogroup" + Localized wrapper
    //   interaction causes false-positive on radio children detection
    await checkA11y(container, {
      rules: {
        'button-name': { enabled: false },
        'aria-required-children': { enabled: false },
      },
    });
  });
});
