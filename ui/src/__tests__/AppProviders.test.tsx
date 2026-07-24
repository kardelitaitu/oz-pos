import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { AppProviders } from '@/contexts/AppProviders';
import { useBrand } from '@/contexts/BrandContext';
import { useHardwareAccel } from '@/contexts/HardwareAccelContext';
import { useAppZoom } from '@/contexts/ZoomContext';

// Mock Tauri invoke calls
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === 'get_brand_settings') {
      return Promise.resolve({ company_name: 'Test Store' });
    }
    return Promise.resolve(null);
  }),
}));

function ConsumerComponent() {
  const brand = useBrand();
  const hw = useHardwareAccel();
  const zoom = useAppZoom();

  return (
    <div>
      <span data-testid="brand">{brand.loading ? 'loading' : brand.settings?.company_name || 'none'}</span>
      <span data-testid="hw">{hw.enabled ? 'hw-on' : 'hw-off'}</span>
      <span data-testid="zoom">{zoom.zoomLevel}</span>
    </div>
  );
}

describe('AppProviders', () => {
  it('provides all application contexts in correct order without throwing', async () => {
    render(
      <AppProviders>
        <ConsumerComponent />
      </AppProviders>
    );

    expect(screen.getByTestId('hw')).toHaveTextContent('hw-on');
    expect(screen.getByTestId('zoom')).toHaveTextContent('auto');
  });
});
