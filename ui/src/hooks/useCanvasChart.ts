import { useRef, useEffect, useCallback } from 'react';

/** Options for the canvas chart hook. */
export interface CanvasChartOptions {
  /** Whether to enable DPR-aware rendering (default: true). */
  enableHiDpi?: boolean;
}

/** Return type for useCanvasChart. */
export interface CanvasChartApi {
  /** The canvas ref to attach to a <canvas> element. */
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  /** Force a redraw immediately. */
  redraw: () => void;
  /** Read a CSS custom property value from the canvas's computed style. */
  getCssVar: (name: string, fallback?: string) => string;
}

/**
 * Base hook for Canvas 2D chart rendering.
 *
 * Manages DPR scaling, container resizing via ResizeObserver, and
 * provides a `redraw()` trigger that consumers call after data changes.
 *
 * @example
 * ```tsx
 * const { canvasRef, redraw, getCssVar } = useCanvasChart();
 * useEffect(() => { redraw(); }, [data, redraw]);
 * // then in render: <canvas ref={canvasRef} ... />
 * ```
 */
export function useCanvasChart(
  draw: (ctx: CanvasRenderingContext2D, width: number, height: number) => void,
  deps: unknown[],
  options?: CanvasChartOptions,
): CanvasChartApi {
  const { enableHiDpi = true } = options ?? {};
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const drawRef = useRef(draw);
  drawRef.current = draw;

  // Store deps for the redraw callback to use in the effect
  const depsRef = useRef(deps);
  depsRef.current = deps;

  const scheduleDraw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const parent = canvas.parentElement;
    if (!parent) return;

    const rect = parent.getBoundingClientRect();
    const w = Math.floor(rect.width);
    const h = Math.floor(rect.height);

    if (enableHiDpi) {
      const dpr = window.devicePixelRatio || 1;
      canvas.width = w * dpr;
      canvas.height = h * dpr;
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
      ctx.scale(dpr, dpr);
    } else {
      canvas.width = w;
      canvas.height = h;
    }

    drawRef.current(ctx, w, h);
  }, [enableHiDpi]);

  useEffect(() => {
    // Initial draw after mount
    requestAnimationFrame(scheduleDraw);

    const canvas = canvasRef.current;
    const parent = canvas?.parentElement;
    if (!parent) return;

    const ro = new ResizeObserver(() => {
      requestAnimationFrame(scheduleDraw);
    });
    ro.observe(parent);

    return () => {
      ro.disconnect();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scheduleDraw, ...deps]);

  const getCssVar = useCallback((name: string, fallback = '#888'): string => {
    const el = canvasRef.current;
    if (!el) return fallback;
    return getComputedStyle(el).getPropertyValue(name).trim() || fallback;
  }, []);

  return { canvasRef, redraw: scheduleDraw, getCssVar };
}
