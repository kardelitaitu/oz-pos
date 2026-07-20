import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useOrientation } from '@/hooks/useOrientation';

// ── Mocks ─────────────────────────────────────────────────────────────

/**
 * Create a mock screen.orientation object that duck-types the
 * ScreenOrientation API accessed via bracket notation in the hook.
 */
function createMockOrientation() {
  let _angle = 0;
  let _resolveLock: (() => void) | null = null;

  return {
    get angle() { return _angle; },
    set angle(v: number) { _angle = v; },
    lock: vi.fn().mockImplementation((_type: string) => {
      return new Promise<void>((resolve) => {
        _resolveLock = resolve;
      });
    }),
    unlock: vi.fn().mockImplementation(() => {
      if (_resolveLock) {
        _resolveLock();
        _resolveLock = null;
      }
    }),
    // Helper to complete a pending lock
    _resolvePendingLock() {
      if (_resolveLock) {
        _resolveLock();
        _resolveLock = null;
      }
    },
  };
}

type MockOrientation = ReturnType<typeof createMockOrientation>;
let mockOrientation: MockOrientation;

// Track event listeners added to window
let windowListeners: Map<string, Set<EventListener>>;

beforeEach(() => {
  mockOrientation = createMockOrientation();
  windowListeners = new Map();

  // Mock window.screen.orientation
  Object.defineProperty(window.screen, 'orientation', {
    value: mockOrientation,
    writable: true,
    configurable: true,
  });

  // Mock window.addEventListener to track registrations
  vi.spyOn(window, 'addEventListener').mockImplementation(
    (event: string, handler: EventListenerOrEventListenerObject) => {
      if (!windowListeners.has(event)) windowListeners.set(event, new Set());
      windowListeners.get(event)!.add(handler as EventListener);
    },
  );

  // Mock window.removeEventListener to track removals
  vi.spyOn(window, 'removeEventListener').mockImplementation(
    (event: string, handler: EventListenerOrEventListenerObject) => {
      windowListeners.get(event)?.delete(handler as EventListener);
    },
  );

  // Helper to simulate orientationchange
  window.dispatchEvent = vi.fn().mockImplementation((event: Event) => {
    const handlers = windowListeners.get(event.type);
    if (handlers) handlers.forEach((h) => h(event));
    return true;
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  windowListeners.clear();
});

// ── Tests ─────────────────────────────────────────────────────────────

describe('useOrientation', () => {
  it('returns supported=true when ScreenOrientation API is available', async () => {
    const { result } = renderHook(() => useOrientation());
    await waitFor(() => expect(result.current.supported).toBe(true));
  });

  it('returns supported=false when orientation API is missing', async () => {
    Object.defineProperty(window.screen, 'orientation', {
      value: undefined,
      writable: true,
      configurable: true,
    });
    const { result } = renderHook(() => useOrientation());
    await waitFor(() => expect(result.current.supported).toBe(false));
  });

  it('detects landscape when innerWidth > innerHeight', () => {
    // Simulate landscape: width > height
    Object.defineProperty(window, 'innerWidth', { value: 1024, configurable: true });
    Object.defineProperty(window, 'innerHeight', { value: 768, configurable: true });
    const { result } = renderHook(() => useOrientation());
    expect(result.current.orientation.isLandscape).toBe(true);
  });

  it('detects portrait when innerWidth <= innerHeight', () => {
    // Simulate portrait: width < height
    Object.defineProperty(window, 'innerWidth', { value: 768, configurable: true });
    Object.defineProperty(window, 'innerHeight', { value: 1024, configurable: true });
    const { result } = renderHook(() => useOrientation());
    expect(result.current.orientation.isLandscape).toBe(false);
  });

  it('reports the screen.orientation.angle', () => {
    mockOrientation.angle = 90;
    const { result } = renderHook(() => useOrientation());
    expect(result.current.orientation.angle).toBe(90);
  });

  it('calls lock when initialLock is provided', async () => {
    renderHook(() => useOrientation('landscape-primary'));
    // Complete the pending lock promise
    mockOrientation._resolvePendingLock();
    await vi.waitFor(() => {
      expect(mockOrientation.lock).toHaveBeenCalledWith('landscape-primary');
    });
  });

  it('does not call lock when initialLock is omitted', () => {
    renderHook(() => useOrientation());
    expect(mockOrientation.lock).not.toHaveBeenCalled();
  });

  it('sets locking=true during lock and false after', async () => {
    const { result } = renderHook(() => useOrientation('landscape-primary'));
    // Lock is pending — should be true
    expect(result.current.locking).toBe(true);

    // Resolve the lock
    mockOrientation._resolvePendingLock();
    await waitFor(() => expect(result.current.locking).toBe(false));
  });

  it('calls unlock on unmount when locked', async () => {
    const { unmount } = renderHook(() => useOrientation('landscape-primary'));
    mockOrientation._resolvePendingLock();
    await vi.waitFor(() => expect(mockOrientation.unlock).not.toHaveBeenCalled());
    unmount();
    expect(mockOrientation.unlock).toHaveBeenCalled();
  });

  it('registers orientationchange and resize listeners on mount', () => {
    renderHook(() => useOrientation());
    expect(windowListeners.has('orientationchange')).toBe(true);
    expect(windowListeners.has('resize')).toBe(true);
  });

  it('removes listeners on unmount', () => {
    const { unmount } = renderHook(() => useOrientation());
    unmount();
    expect(windowListeners.get('orientationchange')?.size).toBe(0);
    expect(windowListeners.get('resize')?.size).toBe(0);
  });

  it('updates orientation on orientationchange event', () => {
    Object.defineProperty(window, 'innerWidth', { value: 1024, configurable: true });
    Object.defineProperty(window, 'innerHeight', { value: 768, configurable: true });

    const { result } = renderHook(() => useOrientation());
    expect(result.current.orientation.isLandscape).toBe(true);

    // Simulate rotation to portrait via orientationchange
    Object.defineProperty(window, 'innerWidth', { value: 768, configurable: true });
    Object.defineProperty(window, 'innerHeight', { value: 1024, configurable: true });
    mockOrientation.angle = 90;

    act(() => {
      const event = new Event('orientationchange');
      window.dispatchEvent(event);
    });

    expect(result.current.orientation.isLandscape).toBe(false);
    expect(result.current.orientation.angle).toBe(90);
  });

  it('lock() function calls screen.orientation.lock', async () => {
    const { result } = renderHook(() => useOrientation());

    await act(async () => {
      result.current.lock('portrait-primary');
      mockOrientation._resolvePendingLock();
    });

    expect(mockOrientation.lock).toHaveBeenCalledWith('portrait-primary');
  });

  it('unlock() function calls screen.orientation.unlock', () => {
    const { result } = renderHook(() => useOrientation());

    act(() => {
      result.current.unlock();
    });

    expect(mockOrientation.unlock).toHaveBeenCalled();
  });

  it('does not crash when lock fails', async () => {
    mockOrientation.lock.mockRejectedValue(new Error('permission denied'));

    const { result } = renderHook(() => useOrientation('landscape-primary'));

    await waitFor(() => {
      expect(result.current.locking).toBe(false);
    });
  });
});
