import { describe, it, expect, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useTerminalProfile } from '@/hooks/useTerminalProfile';
import type { TerminalDto, TerminalProfileDto } from '@/api/terminals';

// ── Mocks ────────────────────────────────────────────────────────

const mockListTerminals = vi.fn<() => Promise<TerminalDto[]>>();
const mockGetTerminalProfile = vi.fn<(arg: string) => Promise<TerminalProfileDto | null>>();

vi.mock('@/api/terminals', () => ({
  listTerminals: () => mockListTerminals(),
  getTerminalProfile: (id: string) => mockGetTerminalProfile(id),
}));

// ── Helpers ──────────────────────────────────────────────────────

function makeTerminal(overrides: Partial<TerminalDto> = {}): TerminalDto {
  return {
    id: 't-1',
    name: 'Terminal 1',
    deviceId: 'dev-1',
    isActive: true,
    lastSeenAt: '2025-01-01T00:00:00Z',
    metadata: null,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
    ...overrides,
  };
}

function makeProfile(overrides: Partial<TerminalProfileDto> = {}): TerminalProfileDto {
  return {
    terminalId: 't-1',
    profileType: 'pos',
    lockedScreen: null,
    updatedAt: '2025-01-01T00:00:00Z',
    ...overrides,
  };
}

// ── Tests ────────────────────────────────────────────────────────

describe('useTerminalProfile', () => {
  it('starts in loading state', () => {
    mockListTerminals.mockReturnValue(new Promise(() => {})); // never resolves
    const { result } = renderHook(() => useTerminalProfile());
    expect(result.current.loading).toBe(true);
    expect(result.current.profile).toBeNull();
    expect(result.current.error).toBeNull();
    expect(result.current.isKdsKiosk).toBe(false);
  });

  it('returns null when no terminals are available', async () => {
    mockListTerminals.mockResolvedValue([]);
    const { result } = renderHook(() => useTerminalProfile());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.profile).toBeNull();
    expect(result.current.error).toBeNull();
    expect(result.current.isKdsKiosk).toBe(false);
  });

  it('loads profile successfully for a regular terminal', async () => {
    const terminal = makeTerminal();
    const profile = makeProfile();
    mockListTerminals.mockResolvedValue([terminal]);
    mockGetTerminalProfile.mockResolvedValue(profile);

    const { result } = renderHook(() => useTerminalProfile());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.profile).toEqual(profile);
    expect(result.current.error).toBeNull();
    expect(result.current.isKdsKiosk).toBe(false);
    expect(mockGetTerminalProfile).toHaveBeenCalledWith('t-1');
  });

  it('detects kds_kiosk profile type', async () => {
    const terminal = makeTerminal();
    const profile = makeProfile({ profileType: 'kds_kiosk' });
    mockListTerminals.mockResolvedValue([terminal]);
    mockGetTerminalProfile.mockResolvedValue(profile);

    const { result } = renderHook(() => useTerminalProfile());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.isKdsKiosk).toBe(true);
    expect(result.current.profile?.profileType).toBe('kds_kiosk');
  });

  it('handles listTerminals error', async () => {
    mockListTerminals.mockRejectedValue(new Error('Network error'));

    const { result } = renderHook(() => useTerminalProfile());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe('Network error');
    expect(result.current.profile).toBeNull();
    expect(result.current.isKdsKiosk).toBe(false);
  });

  it('handles getTerminalProfile error', async () => {
    const terminal = makeTerminal();
    mockListTerminals.mockResolvedValue([terminal]);
    mockGetTerminalProfile.mockRejectedValue(new Error('Profile load failed'));

    const { result } = renderHook(() => useTerminalProfile());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe('Profile load failed');
    expect(result.current.profile).toBeNull();
  });

  it('handles non-Error rejection with fallback message', async () => {
    mockListTerminals.mockRejectedValue('string error');

    const { result } = renderHook(() => useTerminalProfile());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe('Failed to load terminal profile');
  });

  it('falls back to first terminal when none is active', async () => {
    const t1 = makeTerminal({ id: 't-first', isActive: false });
    const t2 = makeTerminal({ id: 't-second', isActive: false });
    const profile = makeProfile({ terminalId: 't-first' });

    mockListTerminals.mockResolvedValue([t1, t2]);
    mockGetTerminalProfile.mockResolvedValue(profile);

    const { result } = renderHook(() => useTerminalProfile());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(mockGetTerminalProfile).toHaveBeenCalledWith('t-first');
    expect(result.current.profile?.terminalId).toBe('t-first');
  });

  it('selects the first active terminal', async () => {
    const inactive = makeTerminal({ id: 't-inactive', isActive: false });
    const active = makeTerminal({ id: 't-active', isActive: true, name: 'Active Terminal' });
    const profile = makeProfile({ terminalId: 't-active' });

    mockListTerminals.mockResolvedValue([inactive, active]);
    mockGetTerminalProfile.mockResolvedValue(profile);

    const { result } = renderHook(() => useTerminalProfile());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(mockGetTerminalProfile).toHaveBeenCalledWith('t-active');
    expect(result.current.profile?.terminalId).toBe('t-active');
  });

  it('handles getTerminalProfile returning null (no profile configured)', async () => {
    const terminal = makeTerminal();
    mockListTerminals.mockResolvedValue([terminal]);
    mockGetTerminalProfile.mockResolvedValue(null);

    const { result } = renderHook(() => useTerminalProfile());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.profile).toBeNull();
    expect(result.current.error).toBeNull();
    expect(result.current.isKdsKiosk).toBe(false);
  });

  it('does not update state after unmount (cancellation)', async () => {
    // Create a deferred promise so we can resolve it after unmount.
    let resolveList: (value: TerminalDto[]) => void;
    const listPromise = new Promise<TerminalDto[]>((resolve) => {
      resolveList = resolve;
    });
    mockListTerminals.mockReturnValue(listPromise);
    mockGetTerminalProfile.mockResolvedValue(makeProfile());

    const { result, unmount } = renderHook(() => useTerminalProfile());

    // Hook starts loading.
    expect(result.current.loading).toBe(true);

    // Unmount before the promise resolves.
    unmount();

    // Now resolve the list promise — the cancelled flag should prevent
    // getTerminalProfile from being called and state from updating.
    resolveList!([makeTerminal()]);

    // getTerminalProfile should not have been called because the
    // component was already unmounted when listTerminals resolved.
    expect(mockGetTerminalProfile).not.toHaveBeenCalled();
  });
});
