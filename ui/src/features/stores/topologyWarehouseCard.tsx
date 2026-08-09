/**
 * Warehouse (Stock Room) settings card for the node inspector.
 *
 * The warehouse is diagram-level — unlike the workspace settings cards,
 * which read and write live backend settings, this card edits per-node
 * properties (capacity and low-stock threshold) that persist in the
 * diagram's node metadata. The editor writes them through the stable
 * handleSetNodeMetadata callback so edits flow through beginInspectorEdit
 * and the normal dirty/save cycle (canvasStateEqual projects the keys).
 */

import { Localized } from '@fluent/react';
import type { TopologyNodeData } from './NodeTopologyEditor';

export interface WarehouseSettingsCardProps {
  node: TopologyNodeData;
  onChange: (nodeId: string, patch: Record<string, unknown>) => void;
}

function readNumber(node: TopologyNodeData, key: string): number | undefined {
  const v = node.metadata?.[key];
  return typeof v === 'number' && Number.isFinite(v) ? v : undefined;
}

export function WarehouseSettingsCard({ node, onChange }: WarehouseSettingsCardProps) {
  const capacity = readNumber(node, 'capacity');
  const lowStockThreshold = readNumber(node, 'lowStockThreshold');

  return (
    <div className="inspector-section" data-testid="warehouse-inspector">
      <h4>
        <Localized id="topology-warehouse-settings-title">Stock Room Settings</Localized>
      </h4>
      {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- text is provided by <Localized> child */}
      <label className="inspector-field">
        <span><Localized id="topology-warehouse-capacity">Capacity</Localized></span>
        <input
          type="number"
          min={0}
          value={capacity ?? ''}
          onChange={(e) => {
            const parsed = parseInt(e.target.value, 10);
            onChange(node.id, { capacity: Number.isNaN(parsed) ? 0 : Math.max(0, parsed) });
          }}
        />
        <span className="inspector-hint">
          <Localized id="topology-warehouse-capacity-desc">Max items this Stock Room can hold</Localized>
        </span>
      </label>
      {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- text is provided by <Localized> child */}
      <label className="inspector-field">
        <span><Localized id="topology-warehouse-low-stock-threshold">Low-Stock Threshold</Localized></span>
        <input
          type="number"
          min={0}
          value={lowStockThreshold ?? ''}
          onChange={(e) => {
            const parsed = parseInt(e.target.value, 10);
            onChange(node.id, { lowStockThreshold: Number.isNaN(parsed) ? 0 : Math.max(0, parsed) });
          }}
        />
        <span className="inspector-hint">
          <Localized id="topology-warehouse-low-stock-desc">Alert when stored stock drops to or below this count</Localized>
        </span>
      </label>
    </div>
  );
}
