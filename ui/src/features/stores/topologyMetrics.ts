// ── Node card metrics ─────────────────────────────────────────────
//
// Adaptive-height node boxes (round 174): the card is a three-region flex
// column — fixed header, content-driven main, port-driven footer — so a
// Warehouse with four stacked inputs grows while a Store stays compact.
//
// The geometry layer (wires, overlap, auto-layout, minimap, alignment)
// computes in TS canvas-space and can never measure the DOM, so every
// height must be a PURE function of the node's type and its port rows.
// This module is that single source of truth; CSS mirrors it, and the
// wire endpoints use portRowCenterY for the exact socket the wire
// attaches to (replacing the old constant NODE_PORT_Y rail).
//
// Rows are top-aligned within each column: the footer height is the
// taller of the two columns, and both left and right stacks start at the
// top of that band.

import type { TopologyNodeData, PortName } from './NodeTopologyEditor';
import { socketSemanticIds } from './topologyCard';

/** Fixed card header height (CSS `.node-header`). */
export const NODE_HEADER_H = 48;
/** Height of one stacked port row in the footer (CSS `.node-port-row`). */
export const PORT_ROW_H = 20;
/** Height of one main-content row (subtitle / status / config). */
export const MAIN_ROW_H = 24;
/** Minimum main-body height so a content-light card never collapses. */
export const MAIN_MIN_H = 56;

/** Main-content rows per node kind. Workspaces carry two extra config
 *  rows (name input + enabled toggle) on top of the shared meta/status. */
export function mainRowCount(node: TopologyNodeData): number {
  return node.type === 'workspace' ? 4 : 2;
}

/** Height of the card's main region (between header and footer). */
export function mainHeight(node: TopologyNodeData): number {
  return Math.max(MAIN_MIN_H, mainRowCount(node) * MAIN_ROW_H);
}

/** Number of stacked port rows in a node's left column. */
export function leftPortRowCount(node: TopologyNodeData): number {
  return socketSemanticIds(node, 'left').length;
}

/** Number of stacked port rows in a node's right column. */
export function rightPortRowCount(node: TopologyNodeData): number {
  return socketSemanticIds(node, 'right').length;
}

/** Port-row count of the taller column — the footer's height band. */
export function portRowCount(node: TopologyNodeData): number {
  return Math.max(leftPortRowCount(node), rightPortRowCount(node));
}

/** Height of the card's footer region (the port stack). */
export function footerHeight(node: TopologyNodeData): number {
  return portRowCount(node) * PORT_ROW_H;
}

/** Total adaptive height of a node card, in canvas units. */
export function nodeHeight(node: TopologyNodeData): number {
  return NODE_HEADER_H + mainHeight(node) + footerHeight(node);
}

/** Canvas-space Y of a port row's CENTER, relative to the card's top.
 *  Left and right columns are both top-aligned; a column's row i sits at
 *  header + main + i*PORT_ROW_H, centered in its PORT_ROW_H band. */
export function portRowCenterY(node: TopologyNodeData, rowIndex: number): number {
  return NODE_HEADER_H + mainHeight(node) + PORT_ROW_H / 2 + rowIndex * PORT_ROW_H;
}

/** Row index of a specific semantic within a node's port column. Used to
 *  resolve a wire's recorded from_port_id / to_port_id back to the exact
 *  socket row it attaches to. Falls back to 0 (the primary) for a semantic
 *  the column does not currently expose. */
export function semanticRowIndex(node: TopologyNodeData, port: PortName, semanticId: string | undefined): number {
  const idx = socketSemanticIds(node, port).indexOf(semanticId as never);
  return idx === -1 ? 0 : idx;
}
