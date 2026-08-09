import { describe, expect, it } from 'vitest';
import type { TopologyNodeData } from '@/features/stores/NodeTopologyEditor';
import {
  isKdsNode,
  leftPortVariants,
  visiblePortsForNode,
  leftPortLabelId,
  portLabelId,
  portAriaLabelId,
  semanticPortId,
  gatingSemanticId,
  socketSemanticIds,
  canSemanticPortsConnect,
  isSemanticWireCompatible,
  wireRelationshipOptions,
  settingsCardForTypeKey,
  topologyUiString,
  workspaceTypeLabel,
  sanitizeCopiedNode,
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
  it('detects KDS workspace variants by typeKey', () => {
    expect(isKdsNode(node({ metadata: { typeKey: 'kds' } }))).toBe(true);
    expect(isKdsNode(node({ metadata: { typeKey: 'store-pos' } }))).toBe(false);
    expect(isKdsNode(node({ type: 'store' }))).toBe(false);
  });

  it('exposes only an output on stores, left+right on KDS and others', () => {
    expect(visiblePortsForNode(node({ type: 'store' }))).toEqual(['right']);
    expect(visiblePortsForNode(node({ type: 'workspace', metadata: { typeKey: 'kds' } }))).toEqual(['left', 'right']);
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

  it('treats a legacy Inventory workspace like a plain workspace — no flexible input', () => {
    // Inventory Management was removed from the topology (round 67); an
    // inventory node left over in a saved diagram degrades to the generic
    // workspace card: fixed Location input, store-pos settings card, and
    // ordinary workspace semantics.
    const legacy = node({ metadata: { typeKey: 'inventory' } });
    expect(leftPortLabelId(legacy, 0)).toBe('topology-port-location-in');
    // A wire never changes the label — the flexible Input/Operation
    // behavior is gone with the inventory card.
    expect(leftPortLabelId(legacy, 0, 'operation-in')).toBe('topology-port-location-in');
    expect(settingsCardForTypeKey('inventory')).toBe(settingsCardForTypeKey('store-pos'));
    expect(gatingSemanticId(legacy, 'right')).toBe('stock-out');
    expect(socketSemanticIds(legacy, 'right')).toEqual(['stock-out', 'transfer-out']);
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
    // Every remaining workspace card type resolves to a concrete component.
    for (const key of ['store-pos', 'restaurant-pos', 'kds']) {
      expect(typeof settingsCardForTypeKey(key)).toBe('function');
    }
    // Unknown typeKeys fall back to the store-pos card.
    expect(settingsCardForTypeKey('mystery')).toBe(settingsCardForTypeKey('store-pos'));
  });
});

describe('typed connection pairing (ADR #34 first slice)', () => {
  it('pairs only the legal semantic port combinations', () => {
    // Ownership: a Branch Location output feeds the Location input only.
    // KDS operation inputs require a Restaurant POS operation feed.
    expect(canSemanticPortsConnect('location-out', 'location-in')).toBe(true);
    expect(canSemanticPortsConnect('location-out', 'operation-in')).toBe(false);
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

  it('validates persisted semantic wires against the same pairing matrix', () => {
    expect(isSemanticWireCompatible('stock-out', 'stock-in', 'stock-routing')).toBe(true);
    expect(isSemanticWireCompatible('ticket-out', 'ticket-in', 'ticket-routing')).toBe(true);
    expect(isSemanticWireCompatible('device-out', 'generic-in', 'hardware-connection')).toBe(true);
    expect(isSemanticWireCompatible('stock-out', 'location-in', 'stock-routing')).toBe(false);
    expect(isSemanticWireCompatible('ticket-out', 'ticket-in', 'generic')).toBe(false);
  });

  it('resolves the full typed socket map for gating (outputs + non-workspace inputs)', () => {
    // Outputs.
    expect(gatingSemanticId(node({ type: 'store' }), 'right')).toBe('location-out');
    expect(gatingSemanticId(node({ metadata: { typeKey: 'store-pos' } }), 'right')).toBe('stock-out');
    expect(gatingSemanticId(node({ metadata: { typeKey: 'restaurant-pos' } }), 'right')).toBe('operation-out');
    expect(gatingSemanticId(node({ metadata: { typeKey: 'kds' } }), 'right')).toBe('ticket-out');
    expect(gatingSemanticId(node({ type: 'warehouse' }), 'right')).toBe('stock-out');
    expect(gatingSemanticId(node({ type: 'hardware' }), 'right')).toBe('device-out');
    // Inputs: warehouses take stock, hardware takes generic, workspaces take
    // location (KDS takes the operation feed).
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

describe('relationship options (ADR #34 multi-semantic slice)', () => {
  it('resolves the multi-semantic socket map: workspaces output stock OR transfer', () => {
    // Workspaces can emit either a stock-routing feed or a transfer feed.
    // Restaurant POS additionally emits an operational feed for KDS on the
    // same output socket.
    expect(socketSemanticIds(node({ type: 'workspace', metadata: { typeKey: 'store-pos' } }), 'right')).toEqual(['stock-out', 'transfer-out']);
    expect(socketSemanticIds(node({ metadata: { typeKey: 'restaurant-pos' } }), 'right')).toEqual(['operation-out', 'stock-out', 'transfer-out']);
    // A warehouse INPUT likewise accepts both: stock-in or transfer-in.
    expect(socketSemanticIds(node({ type: 'warehouse' }), 'left')).toEqual(['stock-in', 'transfer-in']);
    // Every other socket keeps its single semantic.
    expect(socketSemanticIds(node({ type: 'store' }), 'right')).toEqual(['location-out']);
    expect(socketSemanticIds(node({ type: 'warehouse' }), 'right')).toEqual(['stock-out']);
    expect(socketSemanticIds(node({ type: 'hardware' }), 'right')).toEqual(['device-out']);
    expect(socketSemanticIds(node({ metadata: { typeKey: 'kds' } }), 'right')).toEqual(['ticket-out']);
    expect(socketSemanticIds(node({ type: 'hardware' }), 'left')).toEqual(['generic-in', 'ticket-in']);
    expect(socketSemanticIds(node({ metadata: { typeKey: 'store-pos' } }), 'left')).toEqual(['location-in']);
    expect(socketSemanticIds(node({ metadata: { typeKey: 'kds' } }), 'left')).toEqual(['operation-in']);
    expect(socketSemanticIds(node({ type: 'store' }), 'left')).toEqual([]);
  });

  it('gatingSemanticId stays the PRIMARY semantic of each socket', () => {
    expect(gatingSemanticId(node({ metadata: { typeKey: 'store-pos' } }), 'right')).toBe('stock-out');
    expect(gatingSemanticId(node({ type: 'warehouse' }), 'left')).toBe('stock-in');
    expect(gatingSemanticId(node({ type: 'store' }), 'right')).toBe('location-out');
  });

  it('extends the pairing table with the transfer relationship', () => {
    expect(canSemanticPortsConnect('transfer-out', 'transfer-in')).toBe(true);
    // Cross-pairing between stock and transfer is still rejected.
    expect(canSemanticPortsConnect('transfer-out', 'stock-in')).toBe(false);
    expect(canSemanticPortsConnect('stock-out', 'transfer-in')).toBe(false);
  });

  it('admits BOTH relationships for a workspace→warehouse drop, stock first', () => {
    const ws = node({ type: 'workspace', metadata: { typeKey: 'store-pos' } });
    const wh = node({ type: 'warehouse' });
    expect(wireRelationshipOptions(ws, 'right', wh, 'left')).toEqual([
      {
        fromPortId: 'stock-out',
        toPortId: 'stock-in',
        relationshipType: 'stock-routing',
        labelId: 'topology-relationship-stock-routing',
      },
      {
        fromPortId: 'transfer-out',
        toPortId: 'transfer-in',
        relationshipType: 'inventory-transfer',
        labelId: 'topology-relationship-inventory-transfer',
      },
    ]);
  });

  it('allows Restaurant POS to route operational tickets into a KDS', () => {
    const resto = node({ type: 'workspace', metadata: { typeKey: 'restaurant-pos' } });
    const kds = node({ type: 'workspace', metadata: { typeKey: 'kds' } });

    expect(wireRelationshipOptions(resto, 'right', kds, 'left')).toEqual([
      {
        fromPortId: 'operation-out',
        toPortId: 'operation-in',
        relationshipType: 'generic',
        labelId: 'topology-relationship-operation',
      },
    ]);
  });

  it('yields exactly one option for single-semantic drops', () => {
    const store = node({ type: 'store' });
    const ws = node({ type: 'workspace', metadata: { typeKey: 'store-pos' } });
    const wh = node({ type: 'warehouse' });
    expect(wireRelationshipOptions(store, 'right', ws, 'left')).toEqual([
      {
        fromPortId: 'location-out',
        toPortId: 'location-in',
        relationshipType: 'location',
        labelId: 'topology-relationship-location',
      },
    ]);
    expect(wireRelationshipOptions(wh, 'right', wh, 'left')).toEqual([
      {
        fromPortId: 'stock-out',
        toPortId: 'stock-in',
        relationshipType: 'stock-routing',
        labelId: 'topology-relationship-stock-routing',
      },
    ]);
  });

  it('labels a warehouse input by its attached relationship (Stock In / Transfer In)', () => {
    const wh = node({ type: 'warehouse' });
    expect(leftPortVariants(wh)).toEqual(['stock-in']);
    // Unwired (or stock-wired): Stock In. Transfer-wired: Transfer In.
    expect(leftPortLabelId(wh, 0)).toBe('topology-port-stock-in');
    expect(leftPortLabelId(wh, 0, 'stock-in')).toBe('topology-port-stock-in');
    expect(leftPortLabelId(wh, 0, 'transfer-in')).toBe('topology-port-transfer-in');
  });

  it('yields zero options for pairs outside the pairing table', () => {
    const ws = node({ type: 'workspace', metadata: { typeKey: 'store-pos' } });
    const wh = node({ type: 'warehouse' });
    const hw = node({ type: 'hardware' });
    // Workspace → workspace (no location/stock combination), store →
    // warehouse (location vs stock), and a workspace → hardware all have
    // no admissible pairing. KDS → hardware is authorable (ticket) and is
    // covered by its own test below.
    expect(wireRelationshipOptions(ws, 'right', ws, 'left')).toEqual([]);
    expect(wireRelationshipOptions(node({ type: 'store' }), 'right', wh, 'left')).toEqual([]);
    expect(wireRelationshipOptions(ws, 'right', hw, 'left')).toEqual([]);
  });

  it('authorizes a KDS ticket-out to a hardware ticket-in feed (load-only gap closed)', () => {
    // The Resto preset's kds→printer wire records ticket-out/ticket-in.
    // Hardware inputs now admit the ticket-in semantic alongside
    // generic-in, so the pair resolves to exactly one ticket-routing
    // option — authorable without a picker, and recorded in the exact
    // format the preset persists.
    const kds = node({ metadata: { typeKey: 'kds' } });
    const hw = node({ type: 'hardware' });
    expect(wireRelationshipOptions(kds, 'right', hw, 'left')).toEqual([
      {
        fromPortId: 'ticket-out',
        toPortId: 'ticket-in',
        relationshipType: 'ticket-routing',
        labelId: 'topology-relationship-ticket-routing',
      },
    ]);
    expect(canSemanticPortsConnect('ticket-out', 'ticket-in')).toBe(true);
    // A workspace stock/transfer feed must NOT leak into a hardware input
    // — only the ticket semantic is admissible there.
    expect(wireRelationshipOptions(node({ metadata: { typeKey: 'store-pos' } }), 'right', hw, 'left')).toEqual([]);
  });

  it('labels a KDS right socket as Ticket Out with a dedicated aria', () => {
    const kds = node({ metadata: { typeKey: 'kds' } });
    expect(portLabelId(kds, 'right')).toBe('topology-port-ticket-out');
    expect(portAriaLabelId(kds, 'right')).toBe('topology-port-ticket-out-aria');
  });

  it('labels a hardware left input by its attached wire: Ticket In for a ticket feed, Input otherwise', () => {
    const hw = node({ type: 'hardware' });
    // A ticket wire attached → the input reads Ticket In (matching the
    // preset format); unwired / generic feeds keep the neutral Input. The
    // aria is the generic port aria — never a Location label (a hardware
    // input is not a branch location).
    expect(leftPortLabelId(hw, 0, 'ticket-in')).toBe('topology-port-ticket-in');
    expect(leftPortLabelId(hw, 0)).toBe('topology-port-generic-in');
    expect(leftPortVariants(hw)).toEqual(['generic-in']);
    expect(portAriaLabelId(hw, 'left')).toBe('topology-port-aria');
  });
});

describe('sanitizeCopiedNode — Branch Location identity strip', () => {
  // The graph keeps EXACTLY one Branch Location, so a duplicated store card
  // must not carry the original's canonical store identity: the copy is a
  // diagram-only card (same model as a palette-spawned store) instead of a
  // second card impersonating the real branch. Every duplicate path
  // (Ctrl+D / Ctrl+V / Alt+drag / mid-drag conversion) routes through this.
  it('strips storeProfileId from a duplicated Branch Location copy', () => {
    const branch = node({ type: 'store', storeProfileId: 'loc-1' });

    const copy = sanitizeCopiedNode(branch);

    expect(copy.storeProfileId).toBeUndefined();
    expect(copy.id).toBe('n-1');
    expect(copy.type).toBe('store');
    expect(copy.name).toBe('N');
    expect(copy.x).toBe(0);
    expect(copy.y).toBe(0);
  });

  it('leaves a store with no canonical identity untouched', () => {
    const store = node({ type: 'store' });
    expect(sanitizeCopiedNode(store)).toEqual(store);
  });

  it('leaves non-store node copies untouched (same reference)', () => {
    const ws = node({ type: 'workspace', metadata: { typeKey: 'store-pos' } });
    const copy = sanitizeCopiedNode(ws);
    expect(copy).toBe(ws);
    expect(copy.metadata).toEqual({ typeKey: 'store-pos' });
  });
});
