import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, render } from '@testing-library/react';
import { HardwareAccelProvider, useHardwareAccel } from '@/contexts/HardwareAccelContext';
import type { ReactNode } from 'react';

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute('data-hw-accel');
});

afterEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute('data-hw-accel');
});

function wrapper({ children }: { children: ReactNode }) {
  return <HardwareAccelProvider>{children}</HardwareAccelProvider>;
}

describe('HardwareAccelContext', () => {
  describe('useHardwareAccel', () => {
    it('throws when used outside HardwareAccelProvider', () => {
      expect(() => renderHook(() => useHardwareAccel())).toThrow(
        'useHardwareAccel must be used within a HardwareAccelProvider',
      );
    });

    it('starts enabled by default', () => {
      const { result } = renderHook(() => useHardwareAccel(), { wrapper });
      expect(result.current.enabled).toBe(true);
    });

    it('setEnabled(false) updates state and localStorage', () => {
      const { result } = renderHook(() => useHardwareAccel(), { wrapper });

      act(() => {
        result.current.setEnabled(false);
      });

      expect(result.current.enabled).toBe(false);
      expect(localStorage.getItem('app-hw-accel')).toBe('false');
    });

    it('setEnabled(true) updates state and localStorage', () => {
      const { result } = renderHook(() => useHardwareAccel(), { wrapper });

      act(() => {
        result.current.setEnabled(false);
      });

      act(() => {
        result.current.setEnabled(true);
      });

      expect(result.current.enabled).toBe(true);
      expect(localStorage.getItem('app-hw-accel')).toBe('true');
    });

    it('disabling sets data-hw-accel="disabled" on html element', () => {
      const { result } = renderHook(() => useHardwareAccel(), { wrapper });

      act(() => {
        result.current.setEnabled(false);
      });

      expect(document.documentElement.getAttribute('data-hw-accel')).toBe('disabled');
    });

    it('enabling removes data-hw-accel attribute', () => {
      const { result } = renderHook(() => useHardwareAccel(), { wrapper });

      act(() => {
        result.current.setEnabled(false);
      });

      act(() => {
        result.current.setEnabled(true);
      });

      expect(document.documentElement.hasAttribute('data-hw-accel')).toBe(false);
    });

    it('restores disabled state from localStorage', () => {
      localStorage.setItem('app-hw-accel', 'false');
      const { result } = renderHook(() => useHardwareAccel(), { wrapper });

      expect(result.current.enabled).toBe(false);
    });

    it('restores enabled state from localStorage', () => {
      localStorage.setItem('app-hw-accel', 'true');
      const { result } = renderHook(() => useHardwareAccel(), { wrapper });

      expect(result.current.enabled).toBe(true);
    });

    it('treats unknown localStorage value as enabled', () => {
      localStorage.setItem('app-hw-accel', 'garbage');
      const { result } = renderHook(() => useHardwareAccel(), { wrapper });

      expect(result.current.enabled).toBe(true);
    });

    it('restores data-hw-accel attribute on mount when disabled', () => {
      localStorage.setItem('app-hw-accel', 'false');
      renderHook(() => useHardwareAccel(), { wrapper });

      expect(document.documentElement.getAttribute('data-hw-accel')).toBe('disabled');
    });

    it('removes data-hw-accel attribute on mount when enabled', () => {
      localStorage.setItem('app-hw-accel', 'true');
      renderHook(() => useHardwareAccel(), { wrapper });

      expect(document.documentElement.hasAttribute('data-hw-accel')).toBe(false);
    });
  });

  describe('HardwareAccelProvider rendering', () => {
    it('renders children', () => {
      const { container } = render(
        <HardwareAccelProvider>
          <div data-testid="child">hello</div>
        </HardwareAccelProvider>,
      );
      expect(container.querySelector('[data-testid="child"]')).toHaveTextContent('hello');
    });
  });
});
