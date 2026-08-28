// ── KdsScreenFooter tests ─────────────────────────────────────────
// Tests the footer renders basic info segments without crashing.

import { describe, expect, it, vi } from 'vitest';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import { KdsScreenFooter } from '@/features/kds/KdsScreenFooter';
import kdsFtl from '@/locales/kds.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

const mocks = vi.hoisted(() => ({
  syncState: 'connected' as 'connected' | 'disconnected' | 'checking',
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({ session: { display_name: 'testuser' } }),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({ activeInstance: { name: 'KDS-Branch', store_name: '' } }),
}));

vi.mock('@/hooks/useDeviceIp', () => ({
  useDeviceIp: () => ({ ip: '192.168.1.42', source: 'local' }),
}));

vi.mock('@/hooks/useSyncConnection', () => ({
  useSyncConnection: () => ({ state: mocks.syncState, latencyMs: 12 }),
}));

describe('KdsScreenFooter', () => {
  it('renders username, workspace, IP, clock, and sync status', () => {
    mocks.syncState = 'connected';
    const { container } = renderWithFluentSync(<KdsScreenFooter />, sharedFtl, kdsFtl);
    const footer = container.querySelector('.kds-screen-footer');
    expect(footer).not.toBeNull();
    expect(footer?.textContent).toContain('testuser');
    expect(footer?.textContent).toContain('KDS-Branch');
    expect(footer?.textContent).toContain('192.168.1.42');
    expect(footer?.textContent).toContain('Connected');
  });

  it('shows "Disconnected" when sync is down', () => {
    mocks.syncState = 'disconnected';
    const { container } = renderWithFluentSync(<KdsScreenFooter />, sharedFtl, kdsFtl);
    expect(container.querySelector('.kds-screen-footer')?.textContent).toContain('Disconnected');
  });
});