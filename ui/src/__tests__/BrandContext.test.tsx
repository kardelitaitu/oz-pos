import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import { BrandProvider, useBrand } from '@/contexts/BrandContext';

const mockGetBrandSettings = vi.fn();

vi.mock('@/api/branding', () => ({
  getBrandSettings: (...args: unknown[]) => mockGetBrandSettings(...args),
}));

// Test consumer component
function TestConsumer() {
  const brand = useBrand();
  return (
    <div>
      <span data-testid="colour">{brand.settings.primary_colour}</span>
      <span data-testid="store">{brand.settings.store_name}</span>
      <span data-testid="logo">{brand.settings.logo_path ?? 'no-logo'}</span>
      <button data-testid="refresh" onClick={brand.refreshBrandSettings}>
        Refresh
      </button>
    </div>
  );
}

function renderProvider() {
  return render(
    <BrandProvider>
      <TestConsumer />
    </BrandProvider>,
  );
}

describe('BrandContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetBrandSettings.mockResolvedValue({
      primary_colour: '#4f46e5',
      logo_path: '/logo.png',
      store_name: 'Test Store',
    });
  });

  it('renders defaults before API resolves', () => {
    mockGetBrandSettings.mockImplementation(() => new Promise(() => {}));
    renderProvider();
    expect(screen.getByTestId('colour').textContent).toBe('#10b981'); // default
    expect(screen.getByTestId('store').textContent).toBe(''); // default
    expect(screen.getByTestId('logo').textContent).toBe('no-logo'); // default null
  });

  it('updates with fetched settings after API resolves', async () => {
    renderProvider();
    await waitFor(() => {
      expect(screen.getByTestId('colour').textContent).toBe('#4f46e5');
    });
    expect(screen.getByTestId('store').textContent).toBe('Test Store');
    expect(screen.getByTestId('logo').textContent).toBe('/logo.png');
  });

  it('keeps current settings on API error', async () => {
    mockGetBrandSettings.mockRejectedValue(new Error('Offline'));
    renderProvider();

    // Wait for the error to be swallowed
    await waitFor(() => {
      // Defaults remain
      expect(screen.getByTestId('colour').textContent).toBe('#10b981');
    });

    // Settings should still be defaults
    expect(screen.getByTestId('store').textContent).toBe('');
  });

  it('refreshBrandSettings re-fetches from the API', async () => {
    renderProvider();
    await waitFor(() => {
      expect(screen.getByTestId('store').textContent).toBe('Test Store');
    });

    mockGetBrandSettings.mockClear();
    mockGetBrandSettings.mockResolvedValue({
      primary_colour: '#ef4444',
      logo_path: null,
      store_name: 'Updated Store',
    });

    act(() => {
      screen.getByTestId('refresh').click();
    });

    await waitFor(() => {
      expect(screen.getByTestId('colour').textContent).toBe('#ef4444');
      expect(screen.getByTestId('store').textContent).toBe('Updated Store');
    });
    expect(mockGetBrandSettings).toHaveBeenCalledTimes(1);
  });

  it('throws when useBrand is used outside BrandProvider', () => {
    // Suppress console.error from React for this expected error
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => {
      render(<TestConsumer />);
    }).toThrow('useBrand must be used within a BrandProvider');
    spy.mockRestore();
  });
});
