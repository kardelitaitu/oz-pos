import { describe, expect, it } from 'vitest';
import type { TopologyNodeData } from '@/features/stores/NodeTopologyEditor';
import {
  isKdsNode,
  isInventoryNode,
  leftPortVariants,
  visiblePortsForNode,
  leftPortLabelId,
  portLabelId,
  portAriaLabelId,
  semanticPortId,
  settingsCardForTypeKey,
  topologyUiString,
  workspaceTypeLabel,
} from '@/features/stores/topologyCard';

const node = (overrides: Partial<TopologyNodeData>): TopologyNodeData => ({
  id: 'n-1',
  type: 'workspace',
  name: 'N',
  x: 0,
  y: 0,
  ...overrides,
});

/** identity-l10n: returns the key itself, exercising fallbacks. */
const identityL10n = {
  getString: (id: string, _vars?: unknown, fallback?: string) => fallback ?? id,
};

describe('topologyCard registry — per-node-type behavior', () => {
  it('detects KDS and inventory workspace variants by typeKey', () => {
    expect(isKdsNode(node({ metadata: { typeKey: 'kds' } }))).toBe(true);
    expect(isKdsNode(node({ metadata: { typeKey: 'store-pos' } }))).toBe(false);
    expect(isInventoryNode(node({ metadata: { typeKey: 'inventory' } }))).toBe(true);
    expect(isInventoryNode(node({ metadata: { typeKey: 'kds' } }))).toBe(false);
    expect(isKdsNode(node({ type: 'store' }))).toBe(false);
  });

  it('exposes only an output on stores, only an input on KDS, both on others', () => {
    expect(visiblePortsForNode(node({ type: 'store' }))).toEqual(['right']);
    expect(visiblePortsForNode(node({ type: 'workspace', metadata: { typeKey: 'kds' } }))).toEqual(['left']);
    expect(visiblePortsForNode(node({ type: 'workspace', metadata: { typeKey: 'store-pos' } }))).toEqual(['left', 'right']);
    expect(visiblePortsForNode(node({ type: 'warehouse' }))).toEqual(['left', 'right']);
    expect(visiblePortsForNode(node({ type: 'hardware' }))).toEqual(['left', 'right']);
  });

  it('resolves left-input semantic variants per type', () => {
    expect(leftPortVariants(node({ type: 'store' }))).toEqual([]);
    expect(leftPortVariants(node({ metadata: { typeKey: 'kds' } }))).toEqual(['operation-in']);
    expect(leftPortVariants(node({ metadata: { typeKey: 'store-pos' } }))).toEqual(['location-in']);
  });

  it('maps a store output to location-out and a KDS input to operation-in', () => {
    expect(semanticPortId(node({ type: 'store' }), 'right')).toBe('location-out');
    expect(semanticPortId(node({ metadata: { typeKey: 'kds' } }), 'left')).toBe('operation-in');
    expect(semanticPortId(node({ metadata: { typeKey: 'store-pos' } }), 'left')).toBe('location-in');
  });

  it('labels inventory input by its connected wire semantic', () => {
    const inv = node({ metadata: { typeKey: 'inventory' } });
    expect(leftPortLabelId(inv, 0)).toBe('topology-port-generic-in');
    expect(leftPortLabelId(inv, 0, 'location-in')).toBe('topology-port-location-in');
    expect(leftPortLabelId(inv, 0, 'operation-in')).toBe('topology-port-operation-in');
  });

  it('labels right ports by node type', () => {
    expect(portLabelId(node({ type: 'store' }), 'right')).toBe('topology-port-location-out');
    expect(portLabelId(node({ type: 'workspace' }), 'right')).toBe('topology-port-workspace-out');
    expect(portLabelId(node({ type: 'warehouse' }), 'right')).toBe('topology-port-stock-out');
    expect(portLabelId(node({ type: 'hardware' }), 'right')).toBe('topology-port-device-out');
    expect(portAriaLabelId(node({ type: 'store' }), 'right')).toBe('topology-port-location-out-aria');
  });

  it('falls back to friendly text when the bundle is missing a key', () => {
    expect(topologyUiString(identityL10n, 'topology-port-location-in')).toBe('Location');
    expect(topologyUiString(identityL10n, 'topology-field-name')).toBe('Name');
    // Unknown keys fall back to the key itself.
    expect(topologyUiString(identityL10n, 'no-such-key')).toBe('no-such-key');
  });

  it('resolves workspace type labels and settings cards by typeKey', () => {
    expect(workspaceTypeLabel('kds', identityL10n.getString.bind(identityL10n))).toBe('topology-ws-type-kds');
    expect(workspaceTypeLabel('unknown', identityL10n.getString.bind(identityL10n))).toBe('unknown');
    // Every workspace card type resolves to a concrete component.
    for (const key of ['store-pos', 'restaurant-pos', 'kds', 'inventory']) {
      expect(typeof settingsCardForTypeKey(key)).toBe('function');
    }
    // Unknown typeKeys fall back to the store-pos card.
    expect(settingsCardForTypeKey('mystery')).toBe(settingsCardForTypeKey('store-pos'));
  });
});
