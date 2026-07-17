import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { ReactNode } from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { ZoomProvider, useAppZoom } from '@/contexts/ZoomContext';

// ── Test helper component that exposes zoom context ────────────────

function ZoomConsumer() {
  const { zoomLevel, setZoomLevel } = useAppZoom();
  return (
    <div>
      <span data-testid="zoom-level">{zoomLevel}</span>
      <button data-testid="set-auto" onClick={() => setZoomLevel('auto')}>Auto</button>
      <button data-testid="set-125" onClick={() => setZoomLevel('125')}>125%</button>
      <button data-testid="set-150" onClick={() => setZoomLevel('150')}>150%</button>
      <button data-testid="set-200" onClick={() => setZoomLevel('200')}>200%</button>
    </div>
  );
}

function renderWithZoom(ui: ReactNode) {
  return render(<ZoomProvider>{ui}</ZoomProvider>);
}

describe('ZoomContext — element sizing', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.style.fontSize = '';
    // Set a standard window width for consistent auto-zoom calculations
    Object.defineProperty(window, 'innerWidth', { value: 1920, configurable: true });
  });

  afterEach(() => {
    localStorage.clear();
    document.documentElement.style.fontSize = '';
  });

  // ── Initial state ────────────────────────────────────────────

  it('defaults to auto zoom level when no saved preference', () => {
    renderWithZoom(<ZoomConsumer />);
    expect(screen.getByTestId('zoom-level').textContent).toBe('auto');
  });

  it('restores saved zoom level from localStorage', () => {
    localStorage.setItem('app-zoom-level', '125');
    renderWithZoom(<ZoomConsumer />);
    expect(screen.getByTestId('zoom-level').textContent).toBe('125');
  });

  it('applies auto zoom font-size based on viewport width at 1920px', () => {
    renderWithZoom(<ZoomConsumer />);
    // At 1920px: scale = 1920/1920 = 1, fontSize = 16 * 1 = 16
    const fontSize = document.documentElement.style.fontSize;
    expect(fontSize).toBe('16px');
  });

  it('applies auto zoom font-size at 1366px viewport (minimum clamp)', () => {
    Object.defineProperty(window, 'innerWidth', { value: 1366, configurable: true });
    renderWithZoom(<ZoomConsumer />);
    // At 1366px: scale = 1366/1920 = 0.711, fontSize = max(14, min(16, 16 * 0.711)) = 14
    const fontSize = document.documentElement.style.fontSize;
    expect(fontSize).toBe('14px');
  });

  it('applies fixed 125% zoom font-size correctly', () => {
    renderWithZoom(
      <ZoomProvider>
        <ZoomConsumer />
      </ZoomProvider>,
    );

    // Set to 125%
    fireEvent.click(screen.getByTestId('set-125'));
    // fontSize = 16 * (125/100) = 20
    expect(document.documentElement.style.fontSize).toBe('20px');
  });

  // ── Zoom level changes ───────────────────────────────────────

  it('updates zoomLevel when setZoomLevel is called', () => {
    renderWithZoom(<ZoomConsumer />);
    fireEvent.click(screen.getByTestId('set-150'));
    expect(screen.getByTestId('zoom-level').textContent).toBe('150');
  });

  it('persists zoom level to localStorage on change', () => {
    renderWithZoom(<ZoomConsumer />);
    fireEvent.click(screen.getByTestId('set-200'));

    expect(localStorage.getItem('app-zoom-level')).toBe('200');
  });

  // ── Keyboard shortcuts ───────────────────────────────────────

  it('increments zoom on Ctrl+=', () => {
    renderWithZoom(<ZoomConsumer />);

    // From 'auto' → '125'
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true }));
    });

    expect(screen.getByTestId('zoom-level').textContent).toBe('125');
  });

  it('increments zoom on Ctrl+NumpadAdd', () => {
    renderWithZoom(<ZoomConsumer />);

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { code: 'NumpadAdd', ctrlKey: true }));
    });

    expect(screen.getByTestId('zoom-level').textContent).toBe('125');
  });

  it('decrements zoom on Ctrl+-', () => {
    // Start from a known level
    renderWithZoom(
      <ZoomProvider>
        <ZoomConsumer />
      </ZoomProvider>,
    );

    // First set to '125' then decrement to '100'
    fireEvent.click(screen.getByTestId('set-125'));
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '-', ctrlKey: true }));
    });

    expect(screen.getByTestId('zoom-level').textContent).toBe('100');
  });

  it('resets zoom to auto on Ctrl+0', () => {
    renderWithZoom(<ZoomConsumer />);

    // Set to 125 first
    fireEvent.click(screen.getByTestId('set-125'));
    expect(screen.getByTestId('zoom-level').textContent).toBe('125');

    // Now Ctrl+0 resets to auto
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '0', ctrlKey: true }));
    });

    expect(screen.getByTestId('zoom-level').textContent).toBe('auto');
  });

  it('does not change zoom for non-ctrl keypresses', () => {
    renderWithZoom(<ZoomConsumer />);

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: false }));
    });

    expect(screen.getByTestId('zoom-level').textContent).toBe('auto');
  });

  // ── Error boundary ───────────────────────────────────────────

  it('throws error when useAppZoom is used outside ZoomProvider', () => {
    // Suppress expected console error
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const preventJsdomError = (e: ErrorEvent) => e.preventDefault();
    window.addEventListener('error', preventJsdomError);

    function BadComponent() {
      useAppZoom();
      return null;
    }

    expect(() => render(<BadComponent />)).toThrow(
      'useAppZoom must be used within a ZoomProvider',
    );

    window.removeEventListener('error', preventJsdomError);
    consoleSpy.mockRestore();
  });

  // ── Re-render and resize ─────────────────────────────────────

  it('re-applies zoom on window resize', () => {
    renderWithZoom(<ZoomConsumer />);
    // Initial: 1920px → 16px
    expect(document.documentElement.style.fontSize).toBe('16px');

    // Resize to 1600px: scale = 1600/1920 = 0.833 → fontSize = max(14, min(16, 13.33)) = 14
    Object.defineProperty(window, 'innerWidth', { value: 1600, configurable: true });
    act(() => {
      window.dispatchEvent(new Event('resize'));
    });

    const fontSize = document.documentElement.style.fontSize;
    expect(parseFloat(fontSize)).toBe(14);
  });

  it('cycles zoom levels correctly on repeated Ctrl+=', () => {
    renderWithZoom(<ZoomConsumer />);

    // auto → 125
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true })));
    expect(screen.getByTestId('zoom-level').textContent).toBe('125');

    // 125 → 150
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true })));
    expect(screen.getByTestId('zoom-level').textContent).toBe('150');

    // 150 → 200
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true })));
    expect(screen.getByTestId('zoom-level').textContent).toBe('200');

    // 200 → 200 (stays at max)
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true })));
    expect(screen.getByTestId('zoom-level').textContent).toBe('200');
  });

  it('cycles zoom levels correctly on repeated Ctrl+-', () => {
    renderWithZoom(<ZoomConsumer />);

    // Set to 200 first
    fireEvent.click(screen.getByTestId('set-200'));

    // 200 → 150
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '-', ctrlKey: true })));
    expect(screen.getByTestId('zoom-level').textContent).toBe('150');

    // 150 → 125
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '-', ctrlKey: true })));
    expect(screen.getByTestId('zoom-level').textContent).toBe('125');

    // 125 → 100
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '-', ctrlKey: true })));
    expect(screen.getByTestId('zoom-level').textContent).toBe('100');

    // 100 → 100 (stays at min)
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '-', ctrlKey: true })));
    expect(screen.getByTestId('zoom-level').textContent).toBe('100');
  });
});
