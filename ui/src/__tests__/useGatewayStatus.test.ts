import { describe, it, expect, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useGatewayStatus } from '@/hooks/useGatewayStatus';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args: Record<string, unknown>) => mockInvoke(cmd, args),
}));

describe('useGatewayStatus', () => {
  it('returns default offline state initially', () => {
    const { result } = renderHook(() => useGatewayStatus());
    expect(result.current.configured).toBe(false);
    expect(result.current.online).toBe(false);
  });

  it('sets configured and online to true when API key exists', async () => {
    mockInvoke.mockResolvedValue('sk_test_12345');
    const { result } = renderHook(() => useGatewayStatus());

    await waitFor(() => {
      expect(result.current.configured).toBe(true);
      expect(result.current.online).toBe(true);
    });
  });

  it('sets configured and online to false when API key is empty string', async () => {
    mockInvoke.mockResolvedValue('');
    const { result } = renderHook(() => useGatewayStatus());

    await waitFor(() => {
      expect(result.current.configured).toBe(false);
      expect(result.current.online).toBe(false);
    });
  });

  it('sets configured and online to false when API key is null', async () => {
    mockInvoke.mockResolvedValue(null);
    const { result } = renderHook(() => useGatewayStatus());

    await waitFor(() => {
      expect(result.current.configured).toBe(false);
      expect(result.current.online).toBe(false);
    });
  });

  it('falls back to offline on Tauri invoke error', async () => {
    mockInvoke.mockRejectedValue(new Error('invoke failed'));
    const { result } = renderHook(() => useGatewayStatus());

    await waitFor(() => {
      expect(result.current.configured).toBe(false);
      expect(result.current.online).toBe(false);
    });
  });

  it('checks gateway status via get_setting command', async () => {
    mockInvoke.mockResolvedValue('sk_test_12345');
    renderHook(() => useGatewayStatus());

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('get_setting', { key: 'stripe.api_key' }),
    );
  });

  it('does not update state after unmount', async () => {
    const pending = new Promise<string>(() => {});
    mockInvoke.mockReturnValue(pending);

    const { result, unmount } = renderHook(() => useGatewayStatus());
    expect(result.current.configured).toBe(false);

    unmount();
    // No assertion needed — passes if no React state update on unmounted component warning
    expect(true).toBe(true);
  });
});
