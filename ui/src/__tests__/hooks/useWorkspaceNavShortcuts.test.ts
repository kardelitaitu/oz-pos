import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useEffect } from 'react';

function useWorkspaceNavShortcuts(active: string | null, onBack: () => void) {
  useEffect(() => {
    if (!active) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (e.ctrlKey && e.shiftKey) {
          onBack();
        } else if (!document.querySelector('[aria-modal="true"]')) {
          onBack();
        }
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [active, onBack]);
}

describe('useWorkspaceNavShortcuts', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('calls onBack when Escape is pressed and no modal is open', () => {
    const onBack = vi.fn();
    renderHook(() => useWorkspaceNavShortcuts('store-pos', onBack));

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it('does NOT call onBack when Escape is pressed and a modal is open', () => {
    const onBack = vi.fn();
    const modal = document.createElement('div');
    modal.setAttribute('aria-modal', 'true');
    document.body.appendChild(modal);

    renderHook(() => useWorkspaceNavShortcuts('store-pos', onBack));

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

    expect(onBack).not.toHaveBeenCalled();
  });

  it('calls onBack when Ctrl+Shift+Escape is pressed even with a modal open', () => {
    const onBack = vi.fn();
    const modal = document.createElement('div');
    modal.setAttribute('aria-modal', 'true');
    document.body.appendChild(modal);

    renderHook(() => useWorkspaceNavShortcuts('store-pos', onBack));

    document.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Escape', ctrlKey: true, shiftKey: true, bubbles: true }),
    );

    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it('does NOT call onBack on non-Escape keys', () => {
    const onBack = vi.fn();
    renderHook(() => useWorkspaceNavShortcuts('store-pos', onBack));

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'F11', bubbles: true }));
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', bubbles: true }));

    expect(onBack).not.toHaveBeenCalled();
  });

  it('does not register any listener when active is null', () => {
    const onBack = vi.fn();
    renderHook(() => useWorkspaceNavShortcuts(null, onBack));

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

    expect(onBack).not.toHaveBeenCalled();
  });

  it('removes the keydown listener on unmount', () => {
    const onBack = vi.fn();
    const { unmount } = renderHook(() => useWorkspaceNavShortcuts('store-pos', onBack));

    unmount();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

    expect(onBack).not.toHaveBeenCalled();
  });
});
