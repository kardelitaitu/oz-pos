import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useFocusTrap } from '@/hooks/useFocusTrap';

/**
 * Create a DOM structure that mirrors a modal dialog:
 *
 *   <div id="panel">
 *     <button id="first">Close</button>
 *     <input id="middle" />
 *     <button id="last">Save</button>
 *   </div>
 *   <button id="outside">Outside</button>
 */
function createPanel(): {
  panel: HTMLDivElement;
  first: HTMLButtonElement;
  middle: HTMLInputElement;
  last: HTMLButtonElement;
  outside: HTMLButtonElement;
} {
  document.body.innerHTML = `
    <div id="panel">
      <button id="first">Close</button>
      <input id="middle" />
      <button id="last">Save</button>
    </div>
    <button id="outside">Outside</button>
  `;

  const panel = document.getElementById('panel') as HTMLDivElement;
  const first = document.getElementById('first') as HTMLButtonElement;
  const middle = document.getElementById('middle') as HTMLInputElement;
  const last = document.getElementById('last') as HTMLButtonElement;
  const outside = document.getElementById('outside') as HTMLButtonElement;

  return { panel, first, middle, last, outside };
}

describe('useFocusTrap', () => {
  let elements: ReturnType<typeof createPanel>;
  let onEscape: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    elements = createPanel();
    onEscape = vi.fn();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    document.body.innerHTML = '';
    vi.restoreAllMocks();
  });

  // ── Activation / deactivation ─────────────────────────────

  it('does not attach event listeners when active is false', () => {
    const addSpy = vi.spyOn(document, 'addEventListener');
    const removeSpy = vi.spyOn(document, 'removeEventListener');

    const { unmount } = renderHook(() =>
      useFocusTrap({ current: elements.panel }, false, onEscape),
    );

    // Should not have registered any listeners
    expect(addSpy).not.toHaveBeenCalledWith('keydown', expect.any(Function));

    unmount();

    // Should not have tried to remove any listeners
    expect(removeSpy).not.toHaveBeenCalledWith('keydown', expect.any(Function));
  });

  it('attaches keydown listener when active is true', () => {
    const addSpy = vi.spyOn(document, 'addEventListener');

    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    expect(addSpy).toHaveBeenCalledWith('keydown', expect.any(Function));
  });

  it('auto-focuses the first focusable element when activated', () => {
    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    expect(document.activeElement).toBe(elements.first);
  });

  it('auto-focuses an input element when it is the first focusable', () => {
    // Remove the first button so the input becomes first
    elements.first.remove();

    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    expect(document.activeElement).toBe(elements.middle);
  });

  it('does not auto-focus when panel has no focusable elements', () => {
    // Remove all focusable children
    elements.panel.innerHTML = '<span>Just text</span>';

    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    // Focus should not have moved since no focusable element exists inside panel.
    // The hook's auto-focus querySelector returns null, so focusable?.focus() is a no-op.
    expect(document.activeElement).toBe(document.body);
  });

  // ── Escape key ────────────────────────────────────────────

  it('calls onEscape when Escape is pressed while active', () => {
    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }),
      );
    });

    expect(onEscape).toHaveBeenCalledTimes(1);
  });

  it('does not call onEscape when active is false', () => {
    renderHook(() =>
      useFocusTrap({ current: elements.panel }, false, onEscape),
    );

    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }),
      );
    });

    expect(onEscape).not.toHaveBeenCalled();
  });

  it('calls updated onEscape callback when it changes', () => {
    const onEscape2 = vi.fn();

    const { rerender } = renderHook(
      ({ cb }) => useFocusTrap({ current: elements.panel }, true, cb),
      { initialProps: { cb: onEscape } },
    );

    // Update the callback
    rerender({ cb: onEscape2 });

    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }),
      );
    });

    // Only the latest callback should be called
    expect(onEscape).not.toHaveBeenCalled();
    expect(onEscape2).toHaveBeenCalledTimes(1);
  });

  // ── Tab cycling ───────────────────────────────────────────

  it('wraps Tab from last element to first', () => {
    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    // Focus the last element
    elements.last.focus();
    expect(document.activeElement).toBe(elements.last);

    // Press Tab (no Shift)
    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'Tab',
          bubbles: true,
          shiftKey: false,
        }),
      );
    });

    // Focus should have wrapped to the first element
    expect(document.activeElement).toBe(elements.first);
  });

  it('wraps Shift+Tab from first element to last', () => {
    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    // Focus is auto-set to first element on activation
    expect(document.activeElement).toBe(elements.first);

    // Press Shift+Tab
    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'Tab',
          bubbles: true,
          shiftKey: true,
        }),
      );
    });

    // Focus should have wrapped to the last element
    expect(document.activeElement).toBe(elements.last);
  });

  it('does not cycle Tab within the panel when non-Tab keys are pressed', () => {
    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    // Focus the last element
    elements.last.focus();
    expect(document.activeElement).toBe(elements.last);

    // Press Enter (not Tab) — should NOT cycle focus
    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }),
      );
    });

    expect(document.activeElement).toBe(elements.last);
  });

  it('does not cycle when there is only one focusable element', () => {
    // Remove middle and last, leaving only first
    elements.middle.remove();
    elements.last.remove();

    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    expect(document.activeElement).toBe(elements.first);

    // Pressing Tab should not cause any issues
    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'Tab',
          bubbles: true,
          shiftKey: false,
        }),
      );
    });

    expect(document.activeElement).toBe(elements.first);
  });

  it('does not cycle Tab when no focusable elements exist', () => {
    elements.panel.innerHTML = '<span>Just text</span>';

    const tabSpy = vi.spyOn(KeyboardEvent.prototype, 'preventDefault');

    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'Tab',
          bubbles: true,
          shiftKey: false,
        }),
      );
    });

    expect(tabSpy).not.toHaveBeenCalled();
  });

  // ── Scroll lock ───────────────────────────────────────────

  it('locks body scroll when active', () => {
    renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    expect(document.body.style.overflow).toBe('hidden');
  });

  it('restores body overflow on cleanup', () => {
    // Set initial overflow
    document.body.style.overflow = 'auto';

    const { unmount } = renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    expect(document.body.style.overflow).toBe('hidden');

    unmount();

    expect(document.body.style.overflow).toBe('auto');
  });

  it('restores body overflow on deactivation (active becomes false)', () => {
    document.body.style.overflow = 'auto';

    const { rerender } = renderHook(
      ({ active }) =>
        useFocusTrap({ current: elements.panel }, active, onEscape),
      { initialProps: { active: true } },
    );

    expect(document.body.style.overflow).toBe('hidden');

    // Deactivate
    rerender({ active: false });

    expect(document.body.style.overflow).toBe('auto');
  });

  it('restores the original overflow value (not just "visible")', () => {
    document.body.style.overflow = 'scroll';

    const { unmount } = renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    expect(document.body.style.overflow).toBe('hidden');

    unmount();

    expect(document.body.style.overflow).toBe('scroll');
  });

  // ── Cleanup ───────────────────────────────────────────────

  it('removes keydown listener on unmount', () => {
    const removeSpy = vi.spyOn(document, 'removeEventListener');

    const { unmount } = renderHook(() =>
      useFocusTrap({ current: elements.panel }, true, onEscape),
    );

    unmount();

    expect(removeSpy).toHaveBeenCalledWith('keydown', expect.any(Function));
  });

  it('does not crash when panelRef.current is null', () => {
    expect(() => {
      renderHook(() =>
        useFocusTrap({ current: null }, true, onEscape),
      );
    }).not.toThrow();
  });

  it('gracefully handles when panelRef.current becomes null during activation', () => {
    // Create a ref that we can clear
    const ref = { current: elements.panel };

    const { unmount } = renderHook(() =>
      useFocusTrap(ref, true, onEscape),
    );

    // Clear the ref (simulating DOM removal during lifecycle)
    ref.current = null;

    // Should not crash on unmount
    expect(() => unmount()).not.toThrow();
  });
});
