import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, render } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import StatusBar from '@/frontend/shell/StatusBar';
import sharedFtl from '@/locales/shared.ftl?raw';

const mockUseGatewayStatus = vi.fn();
const mockUseSyncConnection = vi.fn();
const mockGoToWorkspacePicker = vi.fn();

const authSession = { value: { user_id: 'user-1', username: 'test', role_name: 'cashier' } };

vi.mock('@/hooks/useGatewayStatus', () => ({
  useGatewayStatus: (...args: unknown[]) => mockUseGatewayStatus(...args),
}));

vi.mock('@/hooks/useSyncConnection', () => ({
  useSyncConnection: (...args: unknown[]) => mockUseSyncConnection(...args),
}));

vi.mock('@/hooks/useWorkspaceNav', () => ({
  useWorkspaceNav: () => ({ goToWorkspacePicker: mockGoToWorkspacePicker }),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({ session: authSession.value }),
}));

vi.mock('@/frontend/shell/ThemeToggle', () => ({
  default: () => <button type="button" aria-label="Toggle theme">🌓</button>,
}));

vi.mock('@/frontend/shell/Tooltip', () => ({
  default: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/components/FastPINOverlay', () => ({
  default: () => null,
}));

beforeEach(() => {
  mockUseGatewayStatus.mockReset();
  mockUseSyncConnection.mockReset();
  mockGoToWorkspacePicker.mockReset();
  mockUseGatewayStatus.mockReturnValue({ online: true, configured: true });
  mockUseSyncConnection.mockReturnValue({ state: 'connected', latencyMs: 12, label: 'Connected (12ms)' });
  authSession.value = { user_id: 'user-1', username: 'test', role_name: 'cashier' };
});

function renderBar() {
  return render(withFluent(<StatusBar />, sharedFtl));
}

describe('StatusBar', () => {
  it('shows version string', () => {
    renderBar();
    // Version string comes from __APP_VERSION__ injected at build time
    // Match any v0.0.x format for forward-compatibility
    expect(screen.getByText(/v0\.0\.\d+/)).toBeTruthy();
  });

  it('shows connected dot when online', () => {
    mockUseGatewayStatus.mockReturnValue({ online: true, configured: true });
    const { container } = renderBar();
    expect(container.querySelector('.statusbar-dot--online')).toBeTruthy();
  });

  it('shows disconnected dot when offline', () => {
    mockUseGatewayStatus.mockReturnValue({ online: false, configured: true });
    const { container } = renderBar();
    expect(container.querySelector('.statusbar-dot--offline')).toBeTruthy();
  });

  it('shows gateway pill when configured', () => {
    mockUseGatewayStatus.mockReturnValue({ online: true, configured: true });
    renderBar();
    expect(screen.getByText('Stripe')).toBeTruthy();
  });

  it('hides gateway pill when not configured', () => {
    mockUseGatewayStatus.mockReturnValue({ online: false, configured: false });
    renderBar();
    expect(screen.queryByText('Stripe')).toBeNull();
  });

  it('shows Switch User button when session exists', () => {
    renderBar();
    expect(screen.getByText('Switch User')).toBeTruthy();
  });

  it('hides Switch User button when no session', () => {
    authSession.value = null!;
    renderBar();
    expect(screen.queryByText('Switch User')).toBeNull();
  });

  it('shows Switch Workspace button', () => {
    renderBar();
    expect(screen.getByText('Switch Workspace')).toBeTruthy();
  });

  it('calls goToWorkspacePicker on workspace click', async () => {
    const user = userEvent.setup();
    renderBar();

    await user.click(screen.getByText('Switch Workspace'));
    expect(mockGoToWorkspacePicker).toHaveBeenCalledTimes(1);
  });

  it('has role="status"', () => {
    renderBar();
    expect(screen.getByRole('status')).toBeTruthy();
  });

  it('shows license text', () => {
    renderBar();
    expect(screen.getByText('Proprietary License')).toBeTruthy();
  });

  // ── Sync connection dot tests ───────────────────────────────

  it('shows sync connected dot when sync is online', () => {
    mockUseSyncConnection.mockReturnValue({ state: 'connected', latencyMs: 12, label: 'Connected (12ms)' });
    const { container } = renderBar();
    expect(container.querySelector('.statusbar-dot--online')).toBeTruthy();
    expect(screen.getByText('Sync')).toBeTruthy();
  });

  it('shows sync disconnected dot when sync is offline', () => {
    mockUseSyncConnection.mockReturnValue({ state: 'disconnected', latencyMs: null, label: 'Disconnected' });
    const { container } = renderBar();
    // Sync dot should have offline class (first online dot is from gateway)
    const dots = container.querySelectorAll('.statusbar-dot--offline');
    expect(dots.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Sync')).toBeTruthy();
  });

  it('shows sync checking dot when sync is initializing', () => {
    mockUseSyncConnection.mockReturnValue({ state: 'checking', latencyMs: null, label: 'Checking…' });
    const { container } = renderBar();
    expect(container.querySelector('.statusbar-dot--checking')).toBeTruthy();
    expect(screen.getByText('Sync')).toBeTruthy();
  });

  it('sync dot always shows even when stripe is not configured', () => {
    mockUseGatewayStatus.mockReturnValue({ online: false, configured: false });
    const { container } = renderBar();
    expect(container.querySelector('.statusbar-dot--online')).toBeTruthy();
    expect(screen.getByText('Sync')).toBeTruthy();
  });
});
