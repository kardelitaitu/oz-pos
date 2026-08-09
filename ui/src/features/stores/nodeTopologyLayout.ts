// Layered wire-direction layout engine for the topology editor.
//
// Auto-layout reorganizes a tangled diagram into ranked columns: sources
// (nodes nothing points to) rank 0, each wire target ranks one deeper via
// BFS, and every rank occupies a column with rows stacked in prior-y
// order. The result is translated so its bounding-box midpoint matches the
// original diagram's — the diagram reorganizes in place instead of
// sliding across the canvas. Pure and unit-testable; the editor's
// autoLayout callback is a thin wrapper around this (history, bend
// clearing, the live announcement).
import { NODE_HEIGHT, NODE_WIDTH } from './nodeTopologyClamp';

/** Canvas-space gap between layout columns (2⅔ grid steps). */
export const LAYOUT_GAP_X = 64;
/** Canvas-space gap between layout rows (2 grid steps). */
export const LAYOUT_GAP_Y = 48;

/** Minimal node shape the engine needs (id + current position). */
export interface LayoutNode {
  id: string;
  x: number;
  y: number;
}

/** Minimal wire shape — direction is implicit: from ranks before to. */
export interface LayoutWire {
  id: string;
  fromNodeId: string;
  toNodeId: string;
}

/** A placed node position (final canvas coordinates, anchored in place). */
export interface NodePlacement {
  id: string;
  x: number;
  y: number;
}

export function computeAutoLayout(
  nodes: LayoutNode[],
  wires: LayoutWire[],
): NodePlacement[] {
  if (nodes.length === 0) return [];

  // Rank by BFS along wire direction from the sources (nodes nothing
  // points to). Cycles keep their first-seen rank; a node never reached
  // (a pure cycle) falls back to rank 0.
  const rank = new Map<string, number>();
  const frontier = nodes.filter((n) => !wires.some((w) => w.toNodeId === n.id)).map((n) => n.id);
  for (const id of frontier) rank.set(id, 0);
  let depth = 0;
  let current = frontier;
  while (current.length > 0 && depth < nodes.length) {
    const next: string[] = [];
    for (const id of current) {
      for (const w of wires) {
        if (w.fromNodeId === id && !rank.has(w.toNodeId)) {
          rank.set(w.toNodeId, depth + 1);
          next.push(w.toNodeId);
        }
      }
    }
    current = next;
    depth += 1;
  }
  for (const n of nodes) if (!rank.has(n.id)) rank.set(n.id, 0);

  // Anchor: keep the placed bounding-box midpoint on the ORIGINAL one so
  // the layout reorganizes in place. Both midpoints use node origins —
  // for uniform boxes that equals the box-center midpoint (the same W/2
  // term cancels on both sides), and a single node stays exactly put.
  const midOf = (values: number[]) => (Math.min(...values) + Math.max(...values)) / 2;
  const oldCx = midOf(nodes.map((n) => n.x));
  const oldCy = midOf(nodes.map((n) => n.y));

  // Place each rank in a column; rows stack in prior-y order so cards keep
  // their relative reading order within a rank.
  const byRank = new Map<number, LayoutNode[]>();
  for (const n of nodes) {
    const r = rank.get(n.id)!;
    const col = byRank.get(r) ?? [];
    col.push(n);
    byRank.set(r, col);
  }
  const placed: NodePlacement[] = [];
  for (const r of [...byRank.keys()].sort((a, b) => a - b)) {
    const col = byRank.get(r)!;
    col.sort((a, b) => a.y - b.y);
    const colX = r * (NODE_WIDTH + LAYOUT_GAP_X);
    col.forEach((n, i) => placed.push({ id: n.id, x: colX, y: i * (NODE_HEIGHT + LAYOUT_GAP_Y) }));
  }
  const dx = oldCx - midOf(placed.map((p) => p.x));
  const dy = oldCy - midOf(placed.map((p) => p.y));
  return placed.map((p) => ({ id: p.id, x: Math.round(p.x + dx), y: Math.round(p.y + dy) }));
}
