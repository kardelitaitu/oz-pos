import { useEffect } from 'react';
import { useCanvasChart } from '@/hooks/useCanvasChart';
import './charts.css';

/** A single data point for the line chart. */
export interface LineChartPoint {
  label: string;
  value: number;
}

interface CanvasLineChartProps {
  data: LineChartPoint[];
  /** Accent color CSS var, e.g. '--color-accent' */
  colorVar?: string;
  /** Formatter for Y-axis labels. */
  formatValue?: (v: number) => string;
  /** Number of Y-axis grid lines (default: 4). */
  gridLines?: number;
  /** Minimum height of the canvas container (CSS value). */
  minHeight?: string;
}

/** Canvas-based line chart with gradient fill, grid lines, and DPR-aware rendering. */
export default function CanvasLineChart({
  data,
  colorVar = '--color-accent',
  formatValue,
  gridLines = 4,
  minHeight = '220px',
}: CanvasLineChartProps) {
  const draw = (ctx: CanvasRenderingContext2D, w: number, h: number) => {
    const pad = { top: 16, right: 16, bottom: 32, left: 56 };
    const chartW = w - pad.left - pad.right;
    const chartH = h - pad.top - pad.bottom;

    if (chartW <= 0 || chartH <= 0 || data.length === 0) return;

    const values = data.map((d) => d.value);
    const maxVal = Math.max(...values, 1);
    const minVal = Math.min(...values, 0);
    const range = maxVal - minVal || 1;

    const xs = data.map((_, i) => pad.left + (i / Math.max(data.length - 1, 1)) * chartW);
    const ys = data.map((d) => pad.top + chartH - ((d.value - minVal) / range) * chartH);

    const accent = ctx.canvas ? getComputedStyle(ctx.canvas).getPropertyValue(colorVar).trim() || '#4f46e5' : '#4f46e5';
    const gridColor = getComputedStyle(ctx.canvas).getPropertyValue('--color-border').trim() || '#e5e7eb';
    const textColor = getComputedStyle(ctx.canvas).getPropertyValue('--color-fg-secondary').trim() || '#6b7280';
    const bgColor = getComputedStyle(ctx.canvas).getPropertyValue('--color-bg').trim() || '#ffffff';

    // Clear
    ctx.clearRect(0, 0, w, h);

    // Grid lines & Y-axis labels
    ctx.strokeStyle = gridColor;
    ctx.lineWidth = 1;
    ctx.font = '10px system-ui, sans-serif';
    ctx.fillStyle = textColor;
    ctx.textAlign = 'right';

    for (let i = 0; i <= gridLines; i++) {
      const y = pad.top + (i / gridLines) * chartH;
      const val = maxVal - (i / gridLines) * range;
      ctx.beginPath();
      ctx.moveTo(pad.left, y);
      ctx.lineTo(w - pad.right, y);
      ctx.stroke();
      ctx.fillText(formatValue ? formatValue(val) : String(Math.round(val)), pad.left - 6, y + 3);
    }

    // X-axis labels (show a subset to avoid crowding)
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    const maxLabels = Math.max(1, Math.floor(chartW / 60));
    const step = Math.max(1, Math.floor(data.length / maxLabels));
    for (let i = 0; i < data.length; i += step) {
      ctx.fillText(data[i]!.label, xs[i]!, h - pad.bottom + 8);
    }
    // Always show last label
    if (data.length > 1 && (data.length - 1) % step !== 0) {
      ctx.fillText(data[data.length - 1]!.label, xs[xs.length - 1]!, h - pad.bottom + 8);
    }
    ctx.textBaseline = 'alphabetic';

    // Area fill (always drawn for visual appeal)
    const grad = ctx.createLinearGradient(0, pad.top, 0, h - pad.bottom);
    grad.addColorStop(0, accent + '33');
    grad.addColorStop(1, accent + '08');
    ctx.beginPath();
    ctx.moveTo(xs[0]!, h - pad.bottom);
    for (let i = 0; i < data.length; i++) {
      ctx.lineTo(xs[i]!, ys[i]!);
    }
    ctx.lineTo(xs[xs.length - 1]!, h - pad.bottom);
    ctx.closePath();
    ctx.fillStyle = grad;
    ctx.fill();

    // Line path
    ctx.beginPath();
    ctx.moveTo(xs[0]!, ys[0]!);
    for (let i = 1; i < data.length; i++) {
      ctx.lineTo(xs[i]!, ys[i]!);
    }
    ctx.strokeStyle = accent;
    ctx.lineWidth = 2;
    ctx.lineJoin = 'round';
    ctx.lineCap = 'round';
    ctx.stroke();

    // Data points
    for (let i = 0; i < data.length; i++) {
      ctx.beginPath();
      ctx.arc(xs[i]!, ys[i]!, 3, 0, Math.PI * 2);
      ctx.fillStyle = bgColor;
      ctx.fill();
      ctx.strokeStyle = accent;
      ctx.lineWidth = 2;
      ctx.stroke();
    }
  };

  const { canvasRef, redraw } = useCanvasChart(draw, [data, colorVar, formatValue, gridLines]);

  useEffect(() => { redraw(); }, [data, redraw]);

  return (
    <div className="canvas-chart-container" style={{ minHeight, width: '100%' }}>
      <canvas ref={canvasRef as React.Ref<HTMLCanvasElement>} className="canvas-chart" aria-label="Line chart" role="img" />
    </div>
  );
}
