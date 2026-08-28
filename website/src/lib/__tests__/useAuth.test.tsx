// @vitest-environment jsdom
import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createRoot } from 'react-dom/client';
import { act } from 'react';
import { useAuth } from '../useAuth';

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

describe('useAuth hook', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    sessionStorage.clear();
    localStorage.clear();
  });

  async function renderHookHarness(callback: () => ReturnType<typeof useAuth>) {
    let currentResult!: ReturnType<typeof useAuth>;
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);

    function TestComponent() {
      currentResult = callback();
      return null;
    }

    await act(async () => {
      root.render(<TestComponent />);
    });

    return {
      get current() {
        return currentResult;
      },
      unmount: async () => {
        await act(async () => {
          root.unmount();
        });
        container.remove();
      },
    };
  }

  it('initializes with default states', async () => {
    const harness = await renderHookHarness(() => useAuth({ locale: 'en' }));
    expect(harness.current.loading).toBe(false);
    expect(harness.current.error).toBe('');
    expect(harness.current.resendCooldown).toBe(0);
    expect(harness.current.resendSuccess).toBe(false);
    await harness.unmount();
  });

  it('handles successful OTP request and starts cooldown', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({}),
    });

    const harness = await renderHookHarness(() => useAuth({ locale: 'en' }));

    let success = false;
    await act(async () => {
      success = await harness.current.requestOtp('test@example.com');
    });

    expect(success).toBe(true);
    expect(harness.current.error).toBe('');
    expect(harness.current.otpSentAt).not.toBeNull();
    await harness.unmount();
  });

  it('handles OTP verification and invokes onAuthSuccess', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ token: 'mock-token' }),
    });

    const onAuthSuccess = vi.fn();
    const harness = await renderHookHarness(() => useAuth({ locale: 'en', onAuthSuccess }));

    let res: { success: boolean; token?: string } = { success: false };
    await act(async () => {
      res = await harness.current.verifyOtp('test@example.com', '123456', 'global');
    });

    expect(res.success).toBe(true);
    expect(res.token).toBe('mock-token');
    expect(sessionStorage.getItem('oz_session')).toBe('mock-token');
    expect(sessionStorage.getItem('oz_email')).toBe('test@example.com');
    expect(localStorage.getItem('oz_region')).toBe('global');
    expect(onAuthSuccess).toHaveBeenCalledWith('mock-token', 'test@example.com');
    await harness.unmount();
  });

  it('handles login with password failure gracefully', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 401,
      json: async () => ({ error: 'Invalid password' }),
    });

    const harness = await renderHookHarness(() => useAuth({ locale: 'en' }));

    let res: { success: boolean; token?: string } = { success: true };
    await act(async () => {
      res = await harness.current.loginPassword('test@example.com', 'wrongpassword');
    });

    expect(res.success).toBe(false);
    expect(harness.current.error).not.toBe('');
    await harness.unmount();
  });
});
