import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import sharedFtl from '@/locales/shared.ftl?raw';
import staffFtl from '@/locales/staff.ftl?raw';
import StatusBar from '@/components/StatusBar';

// ── Mock Tauri IPC (getVersion + updater check) ───────────────────
vi.mock('@tauri-apps/api/app', () => ({
  getVersion: () => Promise.resolve('0.0.34'),
}));

const mockUpdaterCheck = vi.fn();
vi.mock('@tauri-apps/plugin-updater', () => ({
  check: () => mockUpdaterCheck(),
}));

// ── Mock useSyncConnection (default: connected, fast) ─────────────
const mockSync = vi.hoisted(() => ({
  state: 'connected' as 'checking' | 'connected' | 'disconnected',
  latencyMs: 42 as number | null,
}));

vi.mock('@/hooks/useSyncConnection', () => ({
  useSyncConnection: () => ({ state: mockSync.state, latencyMs: mockSync.latencyMs }),
}));

// ── Mock useHealthLatency for auth ────────────────────────────────
const mockAuth = vi.hoisted(() => ({
  state: 'online' as 'checking' | 'online' | 'offline',
  latencyMs: 42 as number | null,
}));

vi.mock('@/hooks/useHealthLatency', () => ({
  useHealthLatency: () => ({ state: mockAuth.state, latencyMs: mockAuth.latencyMs }),
}));

// ── Mock the Toast hook ───────────────────────────────────────────
const mockAddToast = vi.fn();
vi.mock('@/frontend/shared/Toast', async () => {
  const actual: object = await vi.importActual('@/frontend/shared/Toast');
  return {
    ...actual,
    useToast: () => ({ addToast: mockAddToast }),
  };
});

// ── Tests ──────────────────────────────────────────────────────────

function renderBar() {
  return renderWithProvidersSync(<StatusBar />, sharedFtl, staffFtl);
}

describe('StatusBar (activation screen unified status area)', () => {
  beforeEach(() => {
    mockAddToast.mockClear();
    mockUpdaterCheck.mockReset();
    mockUpdaterCheck.mockResolvedValue(null); // no update available
    mockSync.state = 'connected';
    mockSync.latencyMs = 42;
    mockAuth.state = 'online';
    mockAuth.latencyMs = 42;
  });

  it('renders three icon buttons (auth, sync, version)', () => {
    renderBar();
    const buttons = screen.getAllByRole('button');
    expect(buttons).toHaveLength(3);
  });

  it('has correct ARIA labels', () => {
    renderBar();
    expect(screen.getByLabelText('Auth')).toBeInTheDocument();
    expect(screen.getByLabelText('Sync')).toBeInTheDocument();
    expect(screen.getByLabelText('Version')).toBeInTheDocument();
  });

  it('shows native tooltip with latency for auth (green)', () => {
    renderBar();
    expect(screen.getByLabelText('Auth')).toHaveAttribute('title', 'Auth · 42ms');
  });

  it('shows native tooltip with latency for sync (green)', () => {
    renderBar();
    expect(screen.getByLabelText('Sync')).toHaveAttribute('title', 'Sync · 42ms');
  });

  it('shows up-to-date tooltip for version (green)', async () => {
    renderBar();
    await waitFor(() => {
      expect(screen.getByLabelText('Version')).toHaveAttribute('title', 'Version 0.0.34 · up to date');
    });
  });

  it('shows version-update tooltip (yellow) when update is available', async () => {
    mockUpdaterCheck.mockResolvedValue({ version: '0.0.35', downloadAndInstall: vi.fn() });

    renderBar();
    await waitFor(() => {
      expect(screen.getByLabelText('Version')).toHaveAttribute(
        'title',
        'Version 0.0.34 → 0.0.35 available',
      );
    });
  });

  it('clicks auth icon to show toast with latency info', () => {
    renderBar();
    fireEvent.click(screen.getByLabelText('Auth'));
    expect(mockAddToast).toHaveBeenCalledWith({ type: 'info', message: 'Auth · 42ms' });
  });

  it('clicks sync icon to show toast with latency info', () => {
    renderBar();
    fireEvent.click(screen.getByLabelText('Sync'));
    expect(mockAddToast).toHaveBeenCalledWith({ type: 'info', message: 'Sync · 42ms' });
  });

  // ── Color / tone classes ───────────────────────────────────────

  it('applies good tone (green) for latency < 1000 ms', () => {
    renderBar();
    expect(screen.getByLabelText('Auth').className).toContain('statusbar-tone--good');
  });

  it('applies warn tone (yellow) for latency 1000–2999 ms', () => {
    mockAuth.latencyMs = 1500;
    mockSync.latencyMs = 1500;
    renderBar();
    expect(screen.getByLabelText('Auth').className).toContain('statusbar-tone--warn');
  });

  it('applies bad tone (red) for latency >= 3000 ms', () => {
    mockAuth.latencyMs = 3500;
    mockSync.latencyMs = 3500;
    renderBar();
    expect(screen.getByLabelText('Auth').className).toContain('statusbar-tone--bad');
  });

  it('applies checking tone (blinking grey) while checking', () => {
    mockAuth.state = 'checking';
    mockAuth.latencyMs = null;
    mockSync.state = 'checking';
    mockSync.latencyMs = null;
    renderBar();
    expect(screen.getByLabelText('Auth').className).toContain('statusbar-tone--checking');
    expect(screen.getByLabelText('Sync').className).toContain('statusbar-tone--checking');
  });

  it('applies bad tone (red) when offline', () => {
    mockAuth.state = 'offline';
    mockAuth.latencyMs = null;
    mockSync.state = 'disconnected';
    mockSync.latencyMs = null;
    renderBar();
    expect(screen.getByLabelText('Auth').className).toContain('statusbar-tone--bad');
    expect(screen.getByLabelText('Sync').className).toContain('statusbar-tone--bad');
  });
});
