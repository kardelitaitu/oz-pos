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
  gatingSemanticId,
  canSemanticPortsConnect,
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

describe('typed connection pairing (ADR #34 first slice)', () => {
  it('pairs only the legal semantic port combinations', () => {
    // Ownership: a Branch Location output feeds any workspace input
    // (KDS and inventory inputs are flexible and accept the Operation feed).
    expect(canSemanticPortsConnect('location-out', 'location-in')).toBe(true);
    expect(canSemanticPortsConnect('location-out', 'operation-in')).toBe(true);
    // Stock routing, ticket routing, operation feeds, hardware + generic.
    expect(canSemanticPortsConnect('stock-out', 'stock-in')).toBe(true);
    expect(canSemanticPortsConnect('ticket-out', 'ticket-in')).toBe(true);
    expect(canSemanticPortsConnect('operation-out', 'operation-in')).toBe(true);
    expect(canSemanticPortsConnect('device-out', 'generic-in')).toBe(true);
    expect(canSemanticPortsConnect('generic-out', 'generic-in')).toBe(true);
  });

  it('rejects mismatched and reversed pairings', () => {
    expect(canSemanticPortsConnect('location-out', 'stock-in')).toBe(false);
    expect(canSemanticPortsConnect('stock-out', 'location-in')).toBe(false);
    expect(canSemanticPortsConnect('ticket-out', 'stock-in')).toBe(false);
    expect(canSemanticPortsConnect('operation-out', 'stock-in')).toBe(false);
    expect(canSemanticPortsConnect('device-out', 'location-in')).toBe(false);
    // Inputs are never sources.
    expect(canSemanticPortsConnect('location-in', 'location-in')).toBe(false);
    expect(canSemanticPortsConnect('stock-in', 'stock-out')).toBe(false);
  });

  it('resolves the full typed socket map for gating (outputs + non-workspace inputs)', () => {
    // Outputs.
    expect(gatingSemanticId(node({ type: 'store' }), 'right')).toBe('location-out');
    expect(gatingSemanticId(node({ metadata: { typeKey: 'store-pos' } }), 'right')).toBe('stock-out');
    expect(gatingSemanticId(node({ metadata: { typeKey: 'restaurant-pos' } }), 'right')).toBe('stock-out');
    expect(gatingSemanticId(node({ metadata: { typeKey: 'inventory' } }), 'right')).toBe('stock-out');
    expect(gatingSemanticId(node({ metadata: { typeKey: 'kds' } }), 'right')).toBe('ticket-out');
    expect(gatingSemanticId(node({ type: 'warehouse' }), 'right')).toBe('stock-out');
    expect(gatingSemanticId(node({ type: 'hardware' }), 'right')).toBe('device-out');
    // Inputs: warehouses take stock, hardware takes generic, workspaces take
    // location (KDS takes the operation feed, inventory is flexible).
    expect(gatingSemanticId(node({ type: 'warehouse' }), 'left')).toBe('stock-in');
    expect(gatingSemanticId(node({ type: 'hardware' }), 'left')).toBe('generic-in');
    expect(gatingSemanticId(node({ metadata: { typeKey: 'store-pos' } }), 'left')).toBe('location-in');
    expect(gatingSemanticId(node({ metadata: { typeKey: 'kds' } }), 'left')).toBe('operation-in');
    // Sockets that carry no typed meaning stay untyped.
    expect(gatingSemanticId(node({ type: 'store' }), 'left')).toBeUndefined();
  });

  it('keeps the recording-side semanticPortId contract stable (wire creation unaffected)', () => {
    // semanticPortId (the wire-RECORDING resolver) is deliberately unchanged
    // by the gating map: outputs and non-workspace inputs that are not
    // recorded today stay undefined there, so persisted wire semantics and
    // the duplicate-detection fallbacks are untouched.
    expect(semanticPortId(node({ metadata: { typeKey: 'store-pos' } }), 'right')).toBeUndefined();
    expect(semanticPortId(node({ type: 'warehouse' }), 'left')).toBeUndefined();
    expect(semanticPortId(node({ type: 'hardware' }), 'left')).toBeUndefined();
    // The cases the recording side DOES resolve are unchanged.
    expect(semanticPortId(node({ type: 'store' }), 'right')).toBe('location-out');
    expect(semanticPortId(node({ metadata: { typeKey: 'kds' } }), 'left')).toBe('operation-in');
    expect(semanticPortId(node({ metadata: { typeKey: 'store-pos' } }), 'left')).toBe('location-in');
  });
});
