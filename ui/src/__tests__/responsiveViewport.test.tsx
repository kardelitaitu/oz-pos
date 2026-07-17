import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { act } from 'react';
import { ZoomProvider, useAppZoom } from '@/contexts/ZoomContext';
import type { ReactNode } from 'react';

// ── Test helper ────────────────────────────────────────────────────

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

describe('responsive viewport sizing', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.style.fontSize = '';
  });

  afterEach(() => {
    localStorage.clear();
    document.documentElement.style.fontSize = '';
  });

  // ── Auto zoom at various viewport widths ──────────────────────

  it('clamps font-size to 16px at 1920px viewport', () => {
    Object.defineProperty(window, 'innerWidth', {
      value: 1920,
      configurable: true,
    });
    renderWithZoom(<ZoomConsumer />);

    const fontSize = parseFloat(document.documentElement.style.fontSize);
    expect(fontSize).toBe(16);
  });

  it('clamps font-size to 14px at 1366px viewport (minimum)', () => {
    Object.defineProperty(window, 'innerWidth', {
      value: 1366,
      configurable: true,
    });
    renderWithZoom(<ZoomConsumer />);

    const fontSize = parseFloat(document.documentElement.style.fontSize);
    expect(fontSize).toBe(14);
  });

  it('clamps font-size to 16px at viewports wider than 1920px', () => {
    Object.defineProperty(window, 'innerWidth', {
      value: 2560,
      configurable: true,
    });
    renderWithZoom(<ZoomConsumer />);

    const fontSize = parseFloat(document.documentElement.style.fontSize);
    expect(fontSize).toBe(16); // Never scales up above 1920px
  });

  it('clamps font-size to 14px at very narrow viewports (1024px)', () => {
    Object.defineProperty(window, 'innerWidth', {
      value: 1024,
      configurable: true,
    });
    renderWithZoom(<ZoomConsumer />);

    const fontSize = parseFloat(document.documentElement.style.fontSize);
    expect(fontSize).toBe(14); // Clamped at minimum
  });

  it('calculates proportional font-size at 1600px viewport', () => {
    Object.defineProperty(window, 'innerWidth', {
      value: 1600,
      configurable: true,
    });
    renderWithZoom(<ZoomConsumer />);

    // scale = 1600/1920 = 0.833, fontSize = 16 * 0.833 = 13.33, clamped to min 14
    const fontSize = parseFloat(document.documentElement.style.fontSize);
    expect(fontSize).toBe(14);
  });

  it('calculates proportional font-size at 1800px viewport', () => {
    Object.defineProperty(window, 'innerWidth', {
      value: 1800,
      configurable: true,
    });
    renderWithZoom(<ZoomConsumer />);

    // scale = 1800/1920 = 0.9375, fontSize = 16 * 0.9375 = 15
    const fontSize = parseFloat(document.documentElement.style.fontSize);
    expect(fontSize).toBe(15);
  });

  // ── Fixed zoom levels interact with viewport ─────────────────

  it('uses fixed 125% zoom regardless of viewport width', () => {
    // Small viewport
    Object.defineProperty(window, 'innerWidth', {
      value: 1366,
      configurable: true,
    });
    localStorage.setItem('app-zoom-level', '125');
    renderWithZoom(<ZoomConsumer />);

    // 125%: fontSize = 16 * 1.25 = 20
    const fontSize = parseFloat(document.documentElement.style.fontSize);
    expect(fontSize).toBe(20);
  });

  it('uses fixed 150% zoom at any viewport size', () => {
    // Wide viewport
    Object.defineProperty(window, 'innerWidth', {
      value: 1920,
      configurable: true,
    });
    localStorage.setItem('app-zoom-level', '150');
    renderWithZoom(<ZoomConsumer />);

    // 150%: fontSize = 16 * 1.5 = 24
    const fontSize = parseFloat(document.documentElement.style.fontSize);
    expect(fontSize).toBe(24);
  });

  it('uses fixed 200% zoom at any viewport size', () => {
    localStorage.setItem('app-zoom-level', '200');
    renderWithZoom(<ZoomConsumer />);

    // 200%: fontSize = 16 * 2 = 32
    const fontSize = parseFloat(document.documentElement.style.fontSize);
    expect(fontSize).toBe(32);
  });

  // ── Viewport resize triggers recalculation ────────────────────

  it('recalculates font-size when viewport is resized in auto mode', () => {
    Object.defineProperty(window, 'innerWidth', {
      value: 1920,
      configurable: true,
    });
    renderWithZoom(<ZoomConsumer />);
    expect(parseFloat(document.documentElement.style.fontSize)).toBe(16);

    // Resize to 1366
    Object.defineProperty(window, 'innerWidth', {
      value: 1366,
      configurable: true,
    });
    act(() => {
      window.dispatchEvent(new Event('resize'));
    });

    expect(parseFloat(document.documentElement.style.fontSize)).toBe(14);
  });

  it('does not recalculate for fixed zoom when viewport resizes', () => {
    localStorage.setItem('app-zoom-level', '125');
    Object.defineProperty(window, 'innerWidth', {
      value: 1920,
      configurable: true,
    });
    renderWithZoom(<ZoomConsumer />);
    expect(parseFloat(document.documentElement.style.fontSize)).toBe(20);

    // Resize — should still be 125% = 20px
    Object.defineProperty(window, 'innerWidth', {
      value: 1366,
      configurable: true,
    });
    act(() => {
      window.dispatchEvent(new Event('resize'));
    });

    expect(parseFloat(document.documentElement.style.fontSize)).toBe(20);
  });

  // ── Transition between zoom levels recalculates ───────────────

  it('recalculates font-size when switching from auto to fixed zoom', () => {
    Object.defineProperty(window, 'innerWidth', {
      value: 1366,
      configurable: true,
    });
    renderWithZoom(<ZoomConsumer />);
    expect(parseFloat(document.documentElement.style.fontSize)).toBe(14); // auto clamp

    // Switch to 150%
    act(() => {
      screen.getByTestId('set-150').click();
    });
    expect(parseFloat(document.documentElement.style.fontSize)).toBe(24); // 16 * 1.5
  });

  it('recalculates font-size when switching from fixed to auto zoom', () => {
    localStorage.setItem('app-zoom-level', '200');
    Object.defineProperty(window, 'innerWidth', {
      value: 1800,
      configurable: true,
    });
    renderWithZoom(<ZoomConsumer />);
    expect(parseFloat(document.documentElement.style.fontSize)).toBe(32); // 200%

    // Switch to auto — 1800/1920 = 0.9375, 16 * 0.9375 = 15
    act(() => {
      screen.getByTestId('set-auto').click();
    });
    expect(parseFloat(document.documentElement.style.fontSize)).toBe(15);
  });

  // ── Edge cases ────────────────────────────────────────────────

  it('handles rapid resize events without error', () => {
    renderWithZoom(<ZoomConsumer />);

    // Dispatch many resize events rapidly
    for (const width of [1920, 1600, 1366, 1920, 1800, 1500]) {
      Object.defineProperty(window, 'innerWidth', {
        value: width,
        configurable: true,
      });
      act(() => {
        window.dispatchEvent(new Event('resize'));
      });
    }

    // Should end up with correct value for last width (1500)
    // 1500/1920 = 0.78125, 16 * 0.78125 = 12.5, clamped to min 14
    expect(parseFloat(document.documentElement.style.fontSize)).toBe(14);
  });

  it('maintains zoom level after multiple Ctrl+ keyboard interactions', () => {
    renderWithZoom(<ZoomConsumer />);

    // Ctrl+= 4 times should stay at max (200%)
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true }))); // auto → 125
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true }))); // 125 → 150
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true }))); // 150 → 200
    act(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', ctrlKey: true }))); // stays 200

    expect(screen.getByTestId('zoom-level').textContent).toBe('200');
    expect(parseFloat(document.documentElement.style.fontSize)).toBe(32);
  });

  it('persists zoom level across remounts', () => {
    // First mount: set to 125%
    const { unmount } = renderWithZoom(<ZoomConsumer />);
    act(() => {
      screen.getByTestId('set-125').click();
    });
    expect(localStorage.getItem('app-zoom-level')).toBe('125');
    unmount();

    // Second mount: should restore 125%
    renderWithZoom(<ZoomConsumer />);
    expect(screen.getByTestId('zoom-level').textContent).toBe('125');
    expect(parseFloat(document.documentElement.style.fontSize)).toBe(20);
  });
});
