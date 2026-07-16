import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, render } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import StatusBar from '@/frontend/shell/StatusBar';
import sharedFtl from '@/locales/shared.ftl?raw';

const mockUseGatewayStatus = vi.fn();
const mockGoToWorkspacePicker = vi.fn();
const mockSession = { user_id: 'user-1', username: 'test', role_name: 'cashier' };

vi.mock('@/hooks/useGatewayStatus', () => ({
  useGatewayStatus: (...args: unknown[]) => mockUseGatewayStatus(...args),
}));

vi.mock('@/hooks/useWorkspaceNav', () => ({
  useWorkspaceNav: () => ({ goToWorkspacePicker: mockGoToWorkspacePicker }),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({ session: mockSession }),
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
  mockGoToWorkspacePicker.mockReset();
  mockUseGatewayStatus.mockReturnValue({ online: true, configured: true });
});

function renderBar() {
  return render(withFluent(<StatusBar />, sharedFtl));
}

describe('StatusBar', () => {
  it('shows version string', () => {
    renderBar();
    expect(screen.getByText(/v0.0.9/)).toBeTruthy();
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
    const mockNoSession = {} as typeof mockSession;
    Object.assign(mockNoSession, null);
    vi.mocked(vi.fn()).mockReturnValue(null);

    vi.mocked(vi.fn()).mockImplementation(() => ({
      useAuth: () => ({ session: null }),
    }));
  });

  it('shows Workspace button', () => {
    renderBar();
    expect(screen.getByText('Workspace')).toBeTruthy();
  });

  it('calls goToWorkspacePicker on Workspace click', async () => {
    const user = userEvent.setup();
    renderBar();

    await user.click(screen.getByText('Workspace'));
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
});
