import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, render } from '@testing-library/react';
import { ZoomProvider, useAppZoom } from '@/contexts/ZoomContext';
import type { ReactNode } from 'react';

beforeEach(() => {
  localStorage.clear();
  document.documentElement.style.fontSize = '';
});

afterEach(() => {
  localStorage.clear();
  document.documentElement.style.fontSize = '';
});

function wrapper({ children }: { children: ReactNode }) {
  return <ZoomProvider>{children}</ZoomProvider>;
}

function simulateKeyEvent(key: string, code?: string) {
  return new KeyboardEvent('keydown', { key, code, ctrlKey: true, bubbles: true });
}

describe('ZoomContext', () => {
  describe('useAppZoom', () => {
    it('throws when used outside ZoomProvider', () => {
      expect(() => renderHook(() => useAppZoom())).toThrow(
        'useAppZoom must be used within a ZoomProvider',
      );
    });

    it('starts with auto zoom', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });
      expect(result.current.zoomLevel).toBe('auto');
    });

    it('setZoomLevel persists to localStorage', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => {
        result.current.setZoomLevel('125');
      });

      expect(result.current.zoomLevel).toBe('125');
      expect(localStorage.getItem('app-zoom-level')).toBe('125');
    });

    it('setZoomLevel("auto") persists auto to localStorage', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => {
        result.current.setZoomLevel('150');
      });

      act(() => {
        result.current.setZoomLevel('auto');
      });

      expect(result.current.zoomLevel).toBe('auto');
      expect(localStorage.getItem('app-zoom-level')).toBe('auto');
    });

    it('setZoomLevel applies font-size to document element', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => {
        result.current.setZoomLevel('150');
      });

      expect(document.documentElement.style.fontSize).toBe('24px');
    });

    it('setZoomLevel("auto") recalculates font-size based on window width', () => {
      window.innerWidth = 1920;
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => {
        result.current.setZoomLevel('150');
      });

      act(() => {
        result.current.setZoomLevel('auto');
      });

      expect(document.documentElement.style.fontSize).toBe('16px');
    });

    it('restores zoom from localStorage on mount', () => {
      localStorage.setItem('app-zoom-level', '150');
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      expect(result.current.zoomLevel).toBe('150');
    });

    it('passes through invalid localStorage value (no runtime validation)', () => {
      localStorage.setItem('app-zoom-level', 'huge');
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      expect(result.current.zoomLevel).toBe('huge');
    });
  });

  describe('keyboard shortcuts', () => {
    it('Ctrl+= increases zoom: auto → 125', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => {
        document.dispatchEvent(simulateKeyEvent('='));
      });

      expect(result.current.zoomLevel).toBe('125');
    });

    it('Ctrl+= with NumpadAdd also increases zoom', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => {
        document.dispatchEvent(simulateKeyEvent('', 'NumpadAdd'));
      });

      expect(result.current.zoomLevel).toBe('125');
    });

    it('Ctrl+= cycles through zoom levels: 100 → 125 → 150 → 200', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => { result.current.setZoomLevel('100'); });
      act(() => { document.dispatchEvent(simulateKeyEvent('=')); });
      expect(result.current.zoomLevel).toBe('125');

      act(() => { document.dispatchEvent(simulateKeyEvent('=')); });
      expect(result.current.zoomLevel).toBe('150');

      act(() => { document.dispatchEvent(simulateKeyEvent('=')); });
      expect(result.current.zoomLevel).toBe('200');
    });

    it('Ctrl+= does not increase beyond 200', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => { result.current.setZoomLevel('200'); });
      act(() => { document.dispatchEvent(simulateKeyEvent('=')); });

      expect(result.current.zoomLevel).toBe('200');
    });

    it('Ctrl+- decreases zoom: auto → 100', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => {
        document.dispatchEvent(simulateKeyEvent('-'));
      });

      expect(result.current.zoomLevel).toBe('100');
    });

    it('Ctrl+- with NumpadSubtract also decreases zoom', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => {
        document.dispatchEvent(simulateKeyEvent('', 'NumpadSubtract'));
      });

      expect(result.current.zoomLevel).toBe('100');
    });

    it('Ctrl+- cycles through zoom levels: 200 → 150 → 125 → 100', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => { result.current.setZoomLevel('200'); });
      act(() => { document.dispatchEvent(simulateKeyEvent('-')); });
      expect(result.current.zoomLevel).toBe('150');

      act(() => { document.dispatchEvent(simulateKeyEvent('-')); });
      expect(result.current.zoomLevel).toBe('125');

      act(() => { document.dispatchEvent(simulateKeyEvent('-')); });
      expect(result.current.zoomLevel).toBe('100');
    });

    it('Ctrl+- does not decrease below 100', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => { result.current.setZoomLevel('100'); });
      act(() => { document.dispatchEvent(simulateKeyEvent('-')); });

      expect(result.current.zoomLevel).toBe('100');
    });

    it('Ctrl+0 resets zoom to auto', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => { result.current.setZoomLevel('150'); });
      act(() => { document.dispatchEvent(simulateKeyEvent('0')); });

      expect(result.current.zoomLevel).toBe('auto');
    });

    it('Ctrl+Numpad0 resets zoom to auto', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });

      act(() => { result.current.setZoomLevel('150'); });
      act(() => { document.dispatchEvent(simulateKeyEvent('', 'Numpad0')); });

      expect(result.current.zoomLevel).toBe('auto');
    });

    it('does not respond to non-Ctrl key events', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });
      const event = new KeyboardEvent('keydown', { key: '=', ctrlKey: false, bubbles: true });

      act(() => {
        document.dispatchEvent(event);
      });

      expect(result.current.zoomLevel).toBe('auto');
    });

    it('does not respond to unrelated keys with Ctrl', () => {
      const { result } = renderHook(() => useAppZoom(), { wrapper });
      const event = new KeyboardEvent('keydown', { key: 'F5', ctrlKey: true, bubbles: true });

      act(() => {
        document.dispatchEvent(event);
      });

      expect(result.current.zoomLevel).toBe('auto');
    });
  });

  describe('ZoomProvider rendering', () => {
    it('renders children', () => {
      const { container } = render(
        <ZoomProvider>
          <div data-testid="child">hello</div>
        </ZoomProvider>,
      );
      expect(container.querySelector('[data-testid="child"]')).toHaveTextContent('hello');
    });
  });
});
