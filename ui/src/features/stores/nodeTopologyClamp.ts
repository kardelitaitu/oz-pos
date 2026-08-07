// View-relative node positioning clamp for the topology editor.
//
// Nodes live in canvas coordinates inside a viewport transformed with
// `translate(pan) scale(zoom)`; their screen position is `pan + pos * zoom`.
// clampNodeToViewport converts a "keep the node visible" constraint back
// into canvas space so dragging/nudging can never lose a node off-canvas.

// Estimated node card dimensions for wire endpoint positioning.
// Uniform card dimensions keep connector geometry deterministic. The card reserves
// a dedicated footer for left/right ports instead of letting labels overlap content.
export const NODE_WIDTH = 240;
export const NODE_HEIGHT = 240;
/** Height of the dedicated left/right connector rail (CSS `.node-port-sockets-group`). */
export const NODE_PORT_ROW_H = 32;
/** Diameter of the connector marker circles (CSS `.node-port-socket::before`). */
export const NODE_PORT_MARKER = 12;
/** Canvas-space centerline of the connector rail — the wire endpoints must
 *  coincide with the visible circle centers: row top + (row − marker)/2 + marker/2. */
export const NODE_PORT_Y = NODE_HEIGHT - NODE_PORT_ROW_H + NODE_PORT_ROW_H / 2;

/** Screen px of the node box that must remain visible at the canvas edge.
 *  The clamp keeps at least this much of a node on-screen so it can never
 *  be dragged into unreachable negative space (the old hard 20px floor). */
export const EDGE_MARGIN = 40;

export interface ViewportClampOptions {
  panX: number;
  panY: number;
  zoom: number;
  canvasW: number;
  canvasH: number;
  nodeW?: number;
  nodeH?: number;
  /** Screen px of the node box that must remain visible (default EDGE_MARGIN). */
  margin?: number;
}

/**
 * Clamp a node position (canvas coordinates) so the node's box always
 * intersects the visible canvas viewport, with `margin` screen px still
 * visible. Pan/zoom aware: the screen position of a node origin is
 * `pan + pos * zoom` under the viewport's translate(pan) scale(zoom), so
 * the bounds translate back into canvas space. At identity transform this
 * reduces to `x ∈ [-(NODE_WIDTH − margin), canvasW − margin]`. The node width is
 * intentionally 240px so connector labels and workspace names have room to breathe.
 *
 * When the canvas has no measured size (0 — jsdom, pre-layout), no
 * viewport constraint exists and the position is returned unchanged.
 */
export function clampNodeToViewport(
  x: number,
  y: number,
  opts: ViewportClampOptions,
): { x: number; y: number } {
  const { panX, panY, zoom, canvasW, canvasH } = opts;
  if (canvasW <= 0 || canvasH <= 0) return { x, y };
  const nodeW = opts.nodeW ?? NODE_WIDTH;
  const nodeH = opts.nodeH ?? NODE_HEIGHT;
  const margin = opts.margin ?? EDGE_MARGIN;
  // Node's right edge must stay `margin` inside the canvas left edge, and
  // its left edge `margin` inside the right edge. min/max are swapped-safe
  // so a degenerate tiny canvas still keeps the node visible, never lost.
  const minX = (margin - panX) / zoom - nodeW;
  const maxX = (canvasW - margin - panX) / zoom;
  const minY = (margin - panY) / zoom - nodeH;
  const maxY = (canvasH - margin - panY) / zoom;
  const loX = Math.min(minX, maxX);
  const hiX = Math.max(minX, maxX);
  const loY = Math.min(minY, maxY);
  const hiY = Math.max(minY, maxY);
  return {
    x: Math.min(Math.max(x, loX), hiX),
    y: Math.min(Math.max(y, loY), hiY),
  };
}
