import { useEffect, useMemo } from 'react';
import { useCanvasChart } from '@/hooks/useCanvasChart';
import './charts.css';

/** A single heatmap cell. */
export interface HeatmapCell {
  /** 0=Sunday … 6=Saturday */
  dayOfWeek: number;
  /** 0 … 23 */
  hour: number;
  value: number;
}

interface CanvasHeatmapProps {
  data: HeatmapCell[];
  /** Formatter for cell values in aria-label. */
  formatValue?: (v: number) => string;
  /** Minimum height of the canvas container. */
  minHeight?: string;
  /** Color stop for low values (CSS color). */
  colorLow?: string;
  /** Color stop for high values (CSS color). */
  colorHigh?: string;
}

const DAY_LABELS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

/**
 * Canvas 2D hourly heatmap — renders a 7×24 grid with
 * intensity-based coloring, day/hour labels, and DPR-aware rendering.
 */
export default function CanvasHeatmap({
  data,
  formatValue,
  minHeight = '160px',
  colorLow,
  colorHigh,
}: CanvasHeatmapProps) {
  // Build a 7×24 grid
  const grid = useMemo(() => {
    const g: number[][] = Array.from({ length: 7 }, () => Array(24).fill(0));
    for (const cell of data) {
      if (cell.dayOfWeek >= 0 && cell.dayOfWeek < 7 && cell.hour >= 0 && cell.hour < 24) {
        g[cell.dayOfWeek]![cell.hour] = cell.value;
      }
    }
    return g;
  }, [data]);

  const maxVal = useMemo(() => Math.max(...grid.flat(), 1), [grid]);

  const draw = (ctx: CanvasRenderingContext2D, w: number, h: number) => {
    const pad = { top: 8, right: 8, bottom: 8, left: 32 };
    const chartW = w - pad.left - pad.right;
    const chartH = h - pad.top - pad.bottom;

    if (chartW <= 0 || chartH <= 0) return;

    const cellW = chartW / 24;
    const cellH = chartH / 7;
    const gap = 1;

    const lowColor = colorLow ?? (getComputedStyle(ctx.canvas).getPropertyValue('--color-bg-hover').trim() || '#f3f4f6');
    const highColor = colorHigh ?? (getComputedStyle(ctx.canvas).getPropertyValue('--color-accent').trim() || '#4f46e5');
    const textColor = getComputedStyle(ctx.canvas).getPropertyValue('--color-fg-tertiary').trim() || '#9ca3af';
    const labelColor = getComputedStyle(ctx.canvas).getPropertyValue('--color-fg-secondary').trim() || '#6b7280';
    // Read bg for the background color
    const bgColor = getComputedStyle(ctx.canvas).getPropertyValue('--color-bg').trim() || '#ffffff';

    ctx.clearRect(0, 0, w, h);

    // Row labels (day names)
    ctx.font = '9px system-ui, sans-serif';
    ctx.fillStyle = labelColor;
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';
    for (let d = 0; d < 7; d++) {
      const y = pad.top + d * cellH + cellH / 2;
      ctx.fillText(DAY_LABELS[d]!, pad.left - 4, y);
    }

    // Column headers (hours)
    ctx.fillStyle = textColor;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'bottom';
    const maxHourLabels = Math.max(1, Math.floor(chartW / 30));
    const hourStep = Math.max(1, Math.ceil(24 / maxHourLabels));
    for (let h = 0; h < 24; h += hourStep) {
      const x = pad.left + h * cellW + cellW / 2;
      ctx.fillText(String(h), x, pad.top - 2);
    }

    // Cells
    for (let d = 0; d < 7; d++) {
      for (let h = 0; h < 24; h++) {
        const x = pad.left + h * cellW + gap;
        const y = pad.top + d * cellH + gap;
        const cw = cellW - gap * 2;
        const ch = cellH - gap * 2;
        const val = grid[d]![h]!;
        const intensity = val / maxVal;

        if (val > 0) {
          // Interpolate between lowColor and highColor
          ctx.fillStyle = interpolateColor(lowColor, highColor, intensity);
        } else {
          ctx.fillStyle = bgColor;
        }

        ctx.beginPath();
        ctx.roundRect(x, y, cw, ch, 2);
        ctx.fill();

        // Value text (only for higher intensity cells)
        if (intensity > 0.5 && cw > 20 && ch > 14) {
          ctx.fillStyle = intensity > 0.7 ? '#fff' : labelColor;
          ctx.font = '8px system-ui, sans-serif';
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillText(formatValue ? formatValue(val) : val >= 1000 ? `${(val / 1000).toFixed(1)}k` : String(Math.round(val)), x + cw / 2, y + ch / 2);
        }
      }
    }
  };

  const { canvasRef, redraw } = useCanvasChart(draw, [grid, maxVal, colorLow, colorHigh, formatValue]);

  useEffect(() => { redraw(); }, [grid, redraw]);

  return (
    <div className="canvas-chart-container" style={{ minHeight, width: '100%' }}>
      <canvas ref={canvasRef as React.Ref<HTMLCanvasElement>} className="canvas-chart" aria-label="Hourly heatmap" role="img" />
    </div>
  );
}

/** Simple 2-color hex interpolation. */
function interpolateColor(from: string, to: string, t: number): string {
  const f = parseHex(from);
  const t2 = parseHex(to);
  if (!f || !t2) return to;
  const r = Math.round(f.r + (t2.r - f.r) * t);
  const g = Math.round(f.g + (t2.g - f.g) * t);
  const b = Math.round(f.b + (t2.b - f.b) * t);
  return `rgb(${r},${g},${b})`;
}

function parseHex(hex: string): { r: number; g: number; b: number } | null {
  const cleaned = hex.replace('#', '').trim();
  if (cleaned.length === 3) {
    const r = parseInt(cleaned[0]! + cleaned[0], 16);
    const g = parseInt(cleaned[1]! + cleaned[1], 16);
    const b = parseInt(cleaned[2]! + cleaned[2], 16);
    if (isNaN(r) || isNaN(g) || isNaN(b)) return null;
    return { r, g, b };
  }
  if (cleaned.length === 6) {
    const r = parseInt(cleaned.slice(0, 2), 16);
    const g = parseInt(cleaned.slice(2, 4), 16);
    const b = parseInt(cleaned.slice(4, 6), 16);
    if (isNaN(r) || isNaN(g) || isNaN(b)) return null;
    return { r, g, b };
  }
  // Handle rgb/rgba strings
  const rgbMatch = cleaned.match(/^rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)$/);
  if (rgbMatch) {
    return { r: parseInt(rgbMatch[1]!), g: parseInt(rgbMatch[2]!), b: parseInt(rgbMatch[3]!) };
  }
  return null;
}
