/**
 * Memoized topology node card.
 *
 * Extracted from the editor's inline `nodes.map` render so a hover or
 * selection change re-renders ONLY the affected card instead of every card
 * on the canvas. For React.memo to pay off, every prop must be
 * referentially stable across unrelated renders — the editor satisfies that
 * by passing useCallback'd handlers and per-node derived values computed in
 * useMemo. The l10n object from @fluent/react is stable by construction.
 */

import { memo, type ReactNode, type RefObject, type SetStateAction, type Dispatch } from 'react';
import type { ReactLocalization } from '@fluent/react';
import type { TopologyNodeData, PortName } from './NodeTopologyEditor';
import type { TopologyValidationError } from './topologyContract';
import {
  NODE_TYPE_ICON,
  leftPortLabelId,
  portAriaLabelId,
  portLabelId,
  topologyUiString,
  visiblePortsForNode,
} from './topologyCard';
import Tooltip from '../../frontend/shell/Tooltip';

interface TelemetryBadge {
  badge: string;
  status: 'online' | 'warning' | 'offline';
}

export interface TopologyNodeCardProps {
  node: TopologyNodeData;
  isSelected: boolean;
  isConnectingSource: boolean;
  connectingFromNodeId: string | null;
  connectingFromPort: PortName | null;
  /** Pre-computed per-port hover state: true when this card's left/right
   *  port is the current connection target. Derived in the parent's
   *  nodes.map so only cards whose boolean actually changes re-render
   *  (the old hoveredTarget object caused ALL cards to re-render on
   *  every hover change). */
  isLeftPortHovered: boolean;
  isRightPortHovered: boolean;
  nodeErrors: TopologyValidationError[];
  /** Compact excess-count chip (round 113): "N Stock Rooms — 1 allowed"
   *  / "N Branch Locations — 1 allowed", rendered inside the validation
   *  note on nodes pinned by a tier-limit / extra-branch error. Null on
   *  every other card keeps the memo boundary clean. */
  countBadge?: string | null;
  /** Non-destructive overlap indicator (round 143): true when this card's
   *  box intersects another card's — a saved diagram can load stacked even
   *  though movement paths can no longer create overlaps. Renders a small
   *  badge; derived from live geometry, so it disappears when the user
   *  drags the card clear. A stable boolean keeps the memo boundary clean. */
  hasOverlap: boolean;
  /** One-click "add stock wire" guidance (round 80): set while the
   *  validation panel action is guiding the user to route stock into this
   *  warehouse — renders an info chip on the card. The editor clears it the
   *  moment the missing-stock-routing error resolves. */
  stockWireHint: boolean;
  /** Branch-diff overlay marker (round 158): 'only-here' tints the card red
   *  (the workspace exists in this branch's saved diagram only),
   *  'differing' amber (shared but wired/named/retyped differently). Null
   *  on every other card keeps the memo boundary clean. */
  overlayMarker?: 'only-here' | 'differing' | null;
  isFresh: boolean;
  isDimmed: boolean;
  isRenameable: boolean;
  renaming: boolean;
  renameDraft: string;
  connectedPortId: string | undefined;
  l10n: Pick<ReactLocalization, 'getString'>;
  renameInputRef: RefObject<HTMLInputElement>;
  renameBaselineRef: { current: string | null };
  onSelect: (id: string) => void;
  onOpenNodeMenu: (e: React.MouseEvent, nodeId: string) => void;
  onCardMouseDown: (e: React.MouseEvent, nodeId: string) => void;
  onStartRename: (nodeId: string, currentName: string) => void;
  onCommitRename: (nodeId: string, fromKeyboard?: boolean) => void;
  onCancelRename: () => void;
  onRenameDraftChange: (draft: string) => void;
  onPersistRename: (nodeId: string, name: string) => void;
  onSetNodeName: (nodeId: string, name: string) => void;
  onSetNodeEnabled: (nodeId: string, enabled: boolean) => void;
  /** Dismiss the card's validation note (round 81). Only the
   *  missing-stock-routing prompt is semantically dismissable — the
   *  "intentionally empty" warehouse — so the affordance is gated on that
   *  error code alone. The editor persists the dismissal and the Apply
   *  gates (editor + screen) skip the resolved error. */
  onDismissNodeIssue?: (nodeId: string, messageId: string) => void;
  onPortClick: (e: React.MouseEvent, nodeId: string, port: PortName) => void;
  onHoverNode: Dispatch<SetStateAction<string | null>>;
  getTelemetry: (node: TopologyNodeData) => TelemetryBadge | null;
  isPortCompatible: (nodeId: string, port: PortName) => boolean;
}

function TopologyNodeCardImpl({
  node,
  isSelected,
  isConnectingSource,
  connectingFromNodeId,
  connectingFromPort,
  isLeftPortHovered,
  isRightPortHovered,
  nodeErrors,
  countBadge,
  hasOverlap,
  stockWireHint,
  overlayMarker,
  isFresh,
  isDimmed,
  isRenameable,
  renaming,
  renameDraft,
  connectedPortId,
  l10n,
  renameInputRef,
  renameBaselineRef,
  onSelect,
  onOpenNodeMenu,
  onCardMouseDown,
  onStartRename,
  onCommitRename,
  onCancelRename,
  onRenameDraftChange,
  onPersistRename,
  onSetNodeName,
  onSetNodeEnabled,
  onDismissNodeIssue,
  onPortClick,
  onHoverNode,
  getTelemetry,
  isPortCompatible,
}: TopologyNodeCardProps): ReactNode {
  return (
    // eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions -- the canvas card is an interactive unit: selectable, draggable, keyboard-selectable (Enter/Space) and context-menuable; these are its purpose, not incidental handlers
    <div
      onContextMenu={(e) => onOpenNodeMenu(e, node.id)}
      onDoubleClick={() => {
        if (isRenameable) onStartRename(node.id, node.name);
      }}
      data-node-id={node.id}
      className={`topology-node node-type-${node.type} ${isSelected ? 'node-selected' : ''} ${isConnectingSource ? 'node-connecting-source' : ''}${isFresh ? ' node-fresh' : ''}${isDimmed ? ' node-dimmed' : ''}${overlayMarker ? ` topology-node--overlay-${overlayMarker}` : ''}`}
      style={{ left: `${node.x}px`, top: `${node.y}px` }}
      // role=group — NOT aria-selected: group supports no selection state
      // and axe flagged every card (critical aria-allowed-attr). The card
      // also contains real controls (rename, enabled, port sockets), which
      // no aria-selected role (option/treeitem/gridcell) allows nested.
      // Selection is announced through the canvas live region instead.
      role="group"
      // eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- the selectable card is focusable so Enter/Space select it from the keyboard
      tabIndex={0}
      aria-label={node.name}
      onMouseEnter={() => onHoverNode(node.id)}
      onMouseLeave={() => onHoverNode((prev) => (prev === node.id ? null : prev))}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect(node.id); } }}
      // Keep the body selectable/draggable for existing canvas
      // workflows, while nested controls explicitly opt out.
      onMouseDown={(e) => {
        const target = e.target as Element;
        if (target.closest('input, button, select, textarea, [data-no-node-drag]')) return;
        onCardMouseDown(e, node.id);
      }}
    >
      <div className="node-header node-titlebar">
        <div className="node-title-wrapper">
          <span className="node-type-icon">
            {(() => { const Icon = NODE_TYPE_ICON[node.type]; return <Icon size={16} />; })()}
          </span>
          {node.type === 'store' && (
            <span className="node-anchor-chip" title="Branch Location — permanent anchor, cannot be deleted">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="10" height="10"><circle cx="12" cy="5" r="2" /><path d="M12 7v10" /><path d="M8 21h8" /></svg>
            </span>
          )}
          {isRenameable && renaming ? (
            <input
              ref={renameInputRef}
              className="node-card-rename-input"
              value={renameDraft}
              onChange={(e) => onRenameDraftChange(e.target.value)}
              onMouseDown={(e) => e.stopPropagation()}
              onKeyDown={(e) => {
                if (e.key === 'Enter') { e.preventDefault(); void onCommitRename(node.id, true); }
                if (e.key === 'Escape') { e.preventDefault(); onCancelRename(); }
              }}
              onBlur={() => void onCommitRename(node.id)}
              aria-label={topologyUiString(l10n, node.type === 'store' ? 'topology-branch-rename-placeholder' : 'topology-workspace-rename-placeholder')}
            />
          ) : (
            <span className="node-title">{node.name}</span>
          )}
        </div>
        {nodeErrors && nodeErrors.length > 0 && (
          <>
            <Tooltip
              content={(
                <span className="node-validation-tip">
                <span className="node-validation-text">{l10n.getString(nodeErrors[0]!.messageId)}</span>
                {countBadge && <span className="node-validation-count-badge">{countBadge}</span>}
                {nodeErrors[0]!.code === 'warehouse-missing-stock-routing' && onDismissNodeIssue && (
                  <button
                    type="button"
                    className="node-validation-note-dismiss"
                    aria-label={topologyUiString(l10n, 'topology-validation-dismiss', null)}
                    title={topologyUiString(l10n, 'topology-validation-dismiss', null)}
                    onMouseDown={(e) => e.stopPropagation()}
                    onClick={() => onDismissNodeIssue(node.id, nodeErrors[0]!.messageId)}
                  >
                    <span aria-hidden="true">×</span>
                  </button>
                )}
              </span>
            )}
            position="bottom"
            portal
            showDelay={300}
          >
            {/* eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions */}
            <span
              className="node-validation-note node-validation-chip"
              role="status"
              title={nodeErrors.map((e) => l10n.getString(e.messageId)).join('\n')}
              onMouseDown={(e) => e.stopPropagation()}
            >
              <span className="node-validation-icon" aria-hidden="true">!</span>
            </span>
            </Tooltip>
            <span className="node-validation-sr">{l10n.getString(nodeErrors[0]!.messageId)}</span>
          </>
        )}
      </div>

      <div className="node-body">
        <div className="node-body-meta">
          <span className="node-type-accent node-body-accent" aria-hidden="true" />
          <span className="node-grip" aria-hidden="true" title={l10n.getString('topology-node-drag-hint')}>
            <svg viewBox="0 0 24 24" width="10" height="10" fill="currentColor" aria-hidden="true">
              <circle cx="9" cy="6" r="1.5" /><circle cx="15" cy="6" r="1.5" />
              <circle cx="9" cy="12" r="1.5" /><circle cx="15" cy="12" r="1.5" />
              <circle cx="9" cy="18" r="1.5" /><circle cx="15" cy="18" r="1.5" />
            </svg>
          </span>
          <span className="node-subtitle">{node.subtitle}</span>
        </div>
        <div className="node-body-status">
          {(() => {
            const peerGroup = node.metadata?.['peerGroup'] as string | undefined;
            if (peerGroup) {
              return (
                <span className="node-peer-group-badge" aria-hidden="true" title={topologyUiString(l10n, 'topology-peer-group-badge', { group: peerGroup })}>
                  {peerGroup}
                </span>
              );
            }
            return null;
          })()}
          {(() => {
            const telemetry = getTelemetry(node);
            if (!telemetry) return null;
            return (
              <span className={`node-telemetry-badge telemetry-${telemetry.status}`} aria-hidden="true">
                {telemetry.badge}
              </span>
            );
          })()}
          {hasOverlap && (
            // eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions -- stopPropagation keeps a click on the badge from starting a node drag
            <span
              className="node-overlap-badge"
              role="status"
              title={topologyUiString(l10n, 'topology-overlap-badge', null)}
              onMouseDown={(e) => e.stopPropagation()}
            >
              {topologyUiString(l10n, 'topology-overlap-badge', null)}
            </span>
          )}
          {isRenameable && !renaming && (
            <button
              type="button"
              className="node-card-rename-btn"
              onMouseDown={(e) => e.stopPropagation()}
              onClick={() => onStartRename(node.id, node.name)}
              aria-label={topologyUiString(l10n, node.type === 'store' ? 'topology-branch-rename-label' : 'topology-workspace-rename-label')}
              title={topologyUiString(l10n, node.type === 'store' ? 'topology-branch-rename-label' : 'topology-workspace-rename-label')}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
              </svg>
            </button>
          )}
        </div>
        {node.type === 'workspace' && (
          <div className="node-config-row">
            <label htmlFor={`node-name-${node.id}`} className="node-config-label">
              {topologyUiString(l10n, 'topology-field-name')}
            </label>
            <input
              id={`node-name-${node.id}`}
              className="node-config-input"
              onMouseDown={(e) => e.stopPropagation()}
              type="text"
              value={node.name}
              aria-label={topologyUiString(l10n, 'topology-field-name-aria', { name: node.name })}
              onChange={(e) => onSetNodeName(node.id, e.target.value)}
              onFocus={() => { renameBaselineRef.current = node.name; }}
              onBlur={() => void onPersistRename(node.id, node.name)}
              onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); void onPersistRename(node.id, node.name); } }}
            />
          </div>
        )}
        {node.type === 'workspace' && (
          <label className="node-config-row node-config-toggle">
            <span className="node-config-label">{topologyUiString(l10n, 'topology-field-enabled')}</span>
            <input
              type="checkbox"
              onMouseDown={(e) => e.stopPropagation()}
              checked={node.metadata?.['enabled'] !== false}
              aria-label={topologyUiString(l10n, 'topology-field-enabled-aria', { name: node.name })}
              onChange={(e) => onSetNodeEnabled(node.id, e.target.checked)}
            />
          </label>
        )}
      </div>

      {stockWireHint && (
        <div className="node-stock-wire-hint" role="status">
          <span className="node-stock-wire-hint-icon" aria-hidden="true">→</span>
          <span className="node-stock-wire-hint-text">
            {topologyUiString(l10n, 'topology-node-stock-wire-hint', null)}
          </span>
        </div>
      )}

      <div className="node-port-sockets-group">
        {visiblePortsForNode(node).map((port) => {
          const isActive = connectingFromNodeId === node.id && connectingFromPort === port;
          const isHovered = port === 'left' ? isLeftPortHovered : isRightPortHovered;
          const compatible = isPortCompatible(node.id, port);
          const showHighlight = connectingFromNodeId && connectingFromNodeId !== node.id && isHovered && compatible;
          // Inventory's single input is flexible: its label follows
          // the wire actually attached ('location-in' → Location,
          // 'operation-in' → Operation, nothing → Input).
          const labelId = port === 'left'
            ? leftPortLabelId(node, 0, connectedPortId)
            : portLabelId(node, port);
          return (
            <button
              key={port}
              className={`node-port-socket port-${port} ${isActive ? 'port-active' : ''} ${showHighlight ? 'port-highlight' : ''} ${compatible ? 'port-compatible' : ''} ${connectingFromNodeId && !compatible ? 'port-incompatible' : ''}`}
              onClick={(e) => onPortClick(e, node.id, port)}
              aria-label={topologyUiString(
                l10n,
                portAriaLabelId(node, port),
                { name: node.name || '', port },
              )}
              title={topologyUiString(l10n, labelId)}
            >
              <span className={`node-port-label node-port-label-${port}`}>
                {topologyUiString(l10n, labelId)}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const TopologyNodeCard = memo(TopologyNodeCardImpl);
