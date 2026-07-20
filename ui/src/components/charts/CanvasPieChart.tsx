import { useEffect } from 'react';
import { useCanvasChart } from '@/hooks/useCanvasChart';
import './charts.css';

/** A slice in the pie chart. */
export interface PieSlice {
  name: string;
  value: number;
  /** Optional explicit color. If omitted, auto-assigned from palette. */
  color?: string;
}

interface CanvasPieChartProps {
  data: PieSlice[];
  /** Inner radius ratio (0 = pie, >0 = donut). Default 0.45. */
  innerRadiusRatio?: number;
  /** Minimum height of the canvas container. */
  minHeight?: string;
  /** Formatter for values shown in tooltip area. */
  formatValue?: (v: number) => string;
}

const DEFAULT_COLORS = [
  '#4f46e5', '#06b6d4', '#10b981', '#f59e0b', '#ef4444',
  '#8b5cf6', '#ec4899', '#14b8a6', '#f97316', '#6366f1',
  '#84cc16', '#d946ef',
];

/**
 * Canvas 2D pie/donut chart with labels, theme-aware colors via CSS
 * variables, and DPR-aware rendering.
 */
export default function CanvasPieChart({
  data,
  innerRadiusRatio = 0.45,
  minHeight = '220px',
  formatValue,
}: CanvasPieChartProps) {
  const draw = (ctx: CanvasRenderingContext2D, w: number, h: number) => {
    const total = data.reduce((s, d) => s + d.value, 0);
    if (total === 0 || data.length === 0) return;

    const size = Math.min(w, h) - 32;
    const cx = w / 2;
    const cy = h / 2;
    const outerR = size / 2;
    const innerR = outerR * innerRadiusRatio;

    const textColor = getComputedStyle(ctx.canvas).getPropertyValue('--color-fg-secondary').trim() || '#6b7280';
    const labelColor = getComputedStyle(ctx.canvas).getPropertyValue('--color-fg').trim() || '#111';

    ctx.clearRect(0, 0, w, h);

    let startAngle = -Math.PI / 2;

    // Draw slices
    for (let i = 0; i < data.length; i++) {
      const slice = data[i]!;
      const sliceAngle = (slice.value / total) * Math.PI * 2;
      const endAngle = startAngle + sliceAngle;
      const color = slice.color ?? DEFAULT_COLORS[i % DEFAULT_COLORS.length]!;

      ctx.beginPath();
      if (innerR > 0) {
        // Donut: two arcs
        ctx.arc(cx, cy, outerR, startAngle, endAngle);
        ctx.arc(cx, cy, innerR, endAngle, startAngle, true);
      } else {
        // Pie
        ctx.moveTo(cx, cy);
        ctx.arc(cx, cy, outerR, startAngle, endAngle);
        ctx.closePath();
      }
      ctx.fillStyle = color;
      ctx.fill();

      // Stroke between slices
      ctx.strokeStyle = getComputedStyle(ctx.canvas).getPropertyValue('--color-bg').trim() || '#fff';
      ctx.lineWidth = 2;
      ctx.stroke();

      // Label line and text for slices > 5%
      const midAngle = startAngle + sliceAngle / 2;
      const pct = (slice.value / total) * 100;
      if (pct > 5) {
        const labelR = outerR * 0.7;
        const lx = cx + Math.cos(midAngle) * labelR;
        const ly = cy + Math.sin(midAngle) * labelR;
        ctx.fillStyle = '#fff';
        ctx.font = 'bold 11px system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(`${pct.toFixed(0)}%`, lx, ly);
      }

      // Outer label for slices > 8%
      if (pct > 8) {
        const extR = outerR + 14;
        const ex = cx + Math.cos(midAngle) * extR;
        const ey = cy + Math.sin(midAngle) * extR;

        ctx.fillStyle = textColor;
        ctx.font = '10px system-ui, sans-serif';
        ctx.textAlign = midAngle > Math.PI / 2 && midAngle < (3 * Math.PI) / 2 ? 'right' : 'left';
        ctx.textBaseline = 'middle';
        ctx.fillText(slice.name, ex, ey);
      }

      startAngle = endAngle;
    }

    // Center text (donut only)
    if (innerR > 0) {
      ctx.fillStyle = labelColor;
      ctx.font = 'bold 18px system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      const totalFmt = formatValue ? formatValue(total) : String(total);
      ctx.fillText(totalFmt, cx, cy - 6);
      ctx.font = '10px system-ui, sans-serif';
      ctx.fillStyle = textColor;
      ctx.fillText('Total', cx, cy + 14);
    }
  };

  const { canvasRef, redraw } = useCanvasChart(draw, [data, innerRadiusRatio, formatValue]);

  useEffect(() => { redraw(); }, [data, redraw]);

  return (
    <div className="canvas-chart-container" style={{ minHeight, width: '100%' }}>
      <canvas ref={canvasRef as React.Ref<HTMLCanvasElement>} className="canvas-chart" aria-label="Pie chart" role="img" />
    </div>
  );
}
