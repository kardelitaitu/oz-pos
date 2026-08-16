import { describe, expect, it, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useActionCooldown, createCooldownWrapper } from '@/features/kds/hooks/useActionCooldown';

describe('useActionCooldown', () => {
  it('initial cooldownActive is false', () => {
    const action = vi.fn();
    const { result } = renderHook(() => useActionCooldown(action));
    expect(result.current.cooldownActive).toBe(false);
  });

  it('fires action on first invocation', () => {
    const action = vi.fn();
    const { result } = renderHook(() => useActionCooldown(action));
    act(() => {
      result.current.debouncedAction();
    });
    expect(action).toHaveBeenCalledTimes(1);
  });

  it('blocks rapid second invocation within cooldown', () => {
    const action = vi.fn();
    const { result } = renderHook(() => useActionCooldown(action, 200));
    act(() => {
      result.current.debouncedAction();
    });
    // Immediately call again — should be blocked.
    act(() => {
      result.current.debouncedAction();
    });
    expect(action).toHaveBeenCalledTimes(1);
  });

  it('allows invocation after cooldown expires', async () => {
    const action = vi.fn();
    const { result } = renderHook(() => useActionCooldown(action, 200));
    act(() => {
      result.current.debouncedAction();
    });
    // Wait past the real cooldown (200ms + buffer).
    await new Promise((r) => setTimeout(r, 250));
    act(() => {
      result.current.debouncedAction();
    });
    expect(action).toHaveBeenCalledTimes(2);
  });

  it('passes arguments through to the action', () => {
    const action = vi.fn();
    const { result } = renderHook(() => useActionCooldown(action));
    act(() => {
      result.current.debouncedAction('arg1', 42);
    });
    expect(action).toHaveBeenCalledWith('arg1', 42);
  });

  it('uses custom cooldown duration', async () => {
    const action = vi.fn();
    const { result } = renderHook(() => useActionCooldown(action, 500));
    act(() => {
      result.current.debouncedAction();
    });
    // At 300ms — still blocked (cooldown is 500ms).
    await new Promise((r) => setTimeout(r, 300));
    act(() => {
      result.current.debouncedAction();
    });
    expect(action).toHaveBeenCalledTimes(1);
    // At 600ms — should fire.
    await new Promise((r) => setTimeout(r, 300));
    act(() => {
      result.current.debouncedAction();
    });
    expect(action).toHaveBeenCalledTimes(2);
  });
});

describe('createCooldownWrapper', () => {
  it('fires action on first call', () => {
    const action = vi.fn();
    const wrapped = createCooldownWrapper(action, 200);
    wrapped();
    expect(action).toHaveBeenCalledTimes(1);
  });

  it('blocks rapid second call', () => {
    const action = vi.fn();
    const wrapped = createCooldownWrapper(action, 200);
    wrapped();
    wrapped();
    expect(action).toHaveBeenCalledTimes(1);
  });

  it('allows call after cooldown', async () => {
    const action = vi.fn();
    const wrapped = createCooldownWrapper(action, 200);
    wrapped();
    await new Promise((r) => setTimeout(r, 250));
    wrapped();
    expect(action).toHaveBeenCalledTimes(2);
  });

  it('does not require React hooks context', async () => {
    const calls: string[] = [];
    const action = (s: string) => calls.push(s);
    const wrapped = createCooldownWrapper(action, 100);
    wrapped('a');
    wrapped('b'); // blocked
    await new Promise((r) => setTimeout(r, 150));
    wrapped('c');
    expect(calls).toEqual(['a', 'c']);
  });
});
