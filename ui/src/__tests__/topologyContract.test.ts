import { describe, expect, it } from 'vitest';
import type { TopologyNodeData, TopologyWireData } from '@/features/stores/NodeTopologyEditor';
import {
  TOPOLOGY_SCHEMA_VERSION,
  normalizeTopologyGraph,
  normalizeWireDirection,
  validateTopologyGraph,
} from '@/features/stores/topologyContract';

const branch = (id = 'branch-1'): TopologyNodeData => ({
  id,
  type: 'store',
  name: 'Downtown Branch',
  storeProfileId: id,
  x: 0,
  y: 0,
});

const workspace = (id: string): TopologyNodeData => ({
  id,
  type: 'workspace',
  name: id,
  x: 100,
  y: 100,
});

const warehouse = (id: string): TopologyNodeData => ({
  id,
  type: 'warehouse',
  name: id,
  x: 200,
  y: 200,
});

const ownershipWire = (id: string, workspaceId: string): TopologyWireData => ({
  id,
  fromNodeId: 'branch-1',
  fromPort: 'right',
  fromPortId: 'location-out',
  toNodeId: workspaceId,
  toPort: 'left',
  toPortId: 'location-in',
  relationshipType: 'location',
  direction: 'one-way',
});

function graph(nodes: TopologyNodeData[], wires: TopologyWireData[]) {
  return normalizeTopologyGraph(nodes, wires);
}

describe('semantic topology contract', () => {
  it('normalizes legacy Store → Workspace geometry into semantic ownership ports', () => {
    const normalized = graph(
      [branch(), workspace('ws-1')],
      [{ id: 'wire-1', fromNodeId: 'branch-1', toNodeId: 'ws-1', direction: 'one-way', fromPort: 'right', toPort: 'left' }],
    );

    expect(normalized.schemaVersion).toBe(TOPOLOGY_SCHEMA_VERSION);
    expect(normalized.wires[0]).toMatchObject({
      fromPortId: 'location-out',
      toPortId: 'location-in',
      relationshipType: 'location',
      legacyInferred: true,
    });
    expect(validateTopologyGraph(normalized)).toEqual([]);
  });

  it('rejects ambiguous legacy workspace-to-workspace wires with a repairable error', () => {
    const normalized = graph(
      [branch(), workspace('ws-1'), workspace('ws-2')],
      [
        { id: 'wire-owner', fromNodeId: 'branch-1', toNodeId: 'ws-1', fromPort: 'right', toPort: 'left', direction: 'one-way' },
        { id: 'wire-ambiguous', fromNodeId: 'ws-1', toNodeId: 'ws-2', fromPort: 'right', toPort: 'left', direction: 'one-way' },
      ],
    );

    expect(validateTopologyGraph(normalized)).toEqual(expect.arrayContaining([
      expect.objectContaining({
        code: 'ambiguous-legacy-wire',
        wireId: 'wire-ambiguous',
      }),
    ]));
  });

  it('normalizes corrupt or missing wire directions to one-way', () => {
    // Direction is presentation-only, but a corrupt/undefined value must
    // still normalize to a legal value at the contract boundary — the
    // editor renderer and location validation both assume a well-formed
    // direction, and a garbage value would flow straight through
    // otherwise (undefined survives JSON round-trips in legacy data).
    const normalized = graph(
      [branch(), workspace('ws-1'), workspace('ws-2')],
      [
        { id: 'wire-bad', fromNodeId: 'branch-1', toNodeId: 'ws-1', fromPort: 'right', toPort: 'left', direction: 'backwards' as never },
        // Legacy payloads may omit `direction` entirely (type-level cast
        // simulates pre-normalization data).
        { id: 'wire-missing', fromNodeId: 'branch-1', toNodeId: 'ws-2', fromPort: 'right', toPort: 'left', direction: undefined as never },
      ],
    );

    expect(normalized.wires[0]!.direction).toBe('one-way');
    expect(normalized.wires[1]!.direction).toBe('one-way');
    // And the graph validates cleanly — corrupt direction is not a
    // validation error, it is a normalization concern.
    expect(validateTopologyGraph(normalized)).toEqual([]);
  });

  it('re-derives a legal relationship type when the stored value is corrupt', () => {
    // relationshipType is a closed union (SemanticRelationshipType) and
    // every consumer — locationWires() filtering, the renderer's label
    // priority, the Apply boundary — assumes a well-formed value. The
    // early-return path of inferredWire accepted any truthy value, so a
    // garbage string (manual edit, stale JSON) flowed into the semantic
    // graph un-normalized. Corrupt values must be treated like missing
    // ones: fall through to legacy inference, which re-derives the legal
    // type from node identity (branch → workspace = location, else
    // generic) instead of trusting the stored value.
    const normalized = graph(
      [branch(), workspace('ws-1'), workspace('ws-2')],
      [
        // Corrupt type on a Store → Workspace wire with location ports:
        // identity re-derives 'location', the ownership is preserved.
        { id: 'wire-bad', fromNodeId: 'branch-1', toNodeId: 'ws-1', fromPort: 'right', toPort: 'left', fromPortId: 'location-out', toPortId: 'location-in', relationshipType: 'banana' as never, direction: 'one-way' },
        // Corrupt type on a workspace → workspace wire (no identity rule):
        // falls to the last-resort generic default.
        { id: 'wire-generic', fromNodeId: 'ws-1', toNodeId: 'ws-2', fromPort: 'right', toPort: 'left', fromPortId: 'legacy-out', toPortId: 'legacy-in', relationshipType: 'banana' as never, direction: 'one-way' },
      ],
    );

    expect(normalized.wires[0]!.relationshipType).toBe('location');
    expect(normalized.wires[0]!.legacyInferred).toBe(true);
    expect(normalized.wires[1]!.relationshipType).toBe('generic');
    expect(normalized.wires[1]!.legacyInferred).toBe(true);
  });

  it('re-derives legal port ids and types when stored wire fields are corrupt', () => {
    // The early-return of inferredWire guards only relationshipType —
    // fromPortId/toPortId pass through verbatim when truthy, and the
    // workspace → warehouse branch still uses ?? for the type. Port ids
    // are a closed union too (SemanticPortId), so corrupt values must
    // fall through to identity inference just like types do.
    const normalized = graph(
      [branch(), workspace('ws-1'), workspace('ws-2'), warehouse('wh-1')],
      [
        // Corrupt ports on a Store → Workspace wire with location ports:
        // identity re-derives location-out/location-in, ownership kept.
        { id: 'wire-bad', fromNodeId: 'branch-1', toNodeId: 'ws-1', fromPort: 'right', toPort: 'left', fromPortId: 'banana' as never, toPortId: 'cabbage' as never, relationshipType: 'location', direction: 'one-way' },
        // Corrupt ports + corrupt type on a workspace → warehouse wire:
        // identity re-derives stock-out/stock-in/stock-routing.
        { id: 'wire-wh', fromNodeId: 'ws-1', toNodeId: 'wh-1', fromPort: 'right', toPort: 'left', fromPortId: 'banana' as never, toPortId: 'cabbage' as never, relationshipType: 'banana' as never, direction: 'one-way' },
        // Corrupt ports on a workspace → workspace wire (no identity rule):
        // falls to the last-resort legacy placeholders + generic type.
        { id: 'wire-generic', fromNodeId: 'ws-1', toNodeId: 'ws-2', fromPort: 'right', toPort: 'left', fromPortId: 'banana' as never, toPortId: 'cabbage' as never, relationshipType: 'banana' as never, direction: 'one-way' },
      ],
    );

    expect(normalized.wires[0]).toMatchObject({
      fromPortId: 'location-out',
      toPortId: 'location-in',
      relationshipType: 'location',
      legacyInferred: true,
    });
    expect(normalized.wires[1]).toMatchObject({
      fromPortId: 'stock-out',
      toPortId: 'stock-in',
      relationshipType: 'stock-routing',
      legacyInferred: true,
    });
    expect(normalized.wires[2]).toMatchObject({
      fromPortId: 'legacy-out',
      toPortId: 'legacy-in',
      relationshipType: 'generic',
      legacyInferred: true,
    });
  });

  it('keeps legal non-stock ports on a workspace → warehouse wire (no over-fold)', () => {
    // The warehouse branch folds corrupt ports to stock, but a LEGAL
    // port (ticket-out/ticket-in, device-out, …) must survive unchanged —
    // the whitelist is the SemanticPortId union, not a stock-only list.
    const normalized = graph(
      [branch(), workspace('ws-1'), warehouse('wh-1')],
      [
        { id: 'wire-ticket', fromNodeId: 'ws-1', toNodeId: 'wh-1', fromPort: 'right', toPort: 'left', fromPortId: 'ticket-out', toPortId: 'ticket-in', relationshipType: 'ticket-routing', direction: 'one-way' },
      ],
    );

    expect(normalized.wires[0]).toMatchObject({
      fromPortId: 'ticket-out',
      toPortId: 'ticket-in',
      relationshipType: 'ticket-routing',
      legacyInferred: false,
    });
  });

  it('normalizeWireDirection keeps only the three legal flow states', () => {
    expect(normalizeWireDirection('one-way')).toBe('one-way');
    expect(normalizeWireDirection('reverse')).toBe('reverse');
    expect(normalizeWireDirection('two-way')).toBe('two-way');
    // Corrupt / legacy / missing values fold to the historical default.
    expect(normalizeWireDirection('bidirectional')).toBe('one-way');
    expect(normalizeWireDirection('banana')).toBe('one-way');
    expect(normalizeWireDirection(undefined)).toBe('one-way');
  });

  it('normalizes an unknown node kind to a legal value instead of passing it through', () => {
    // SEMANTIC_NODE_DEFINITIONS documents "unknown node kinds are not
    // accepted", but nodeKind returned node.type verbatim — a corrupt
    // type (manual edit, stale JSON) flowed into the semantic graph as an
    // opaque kind that validateTopologyGraph never sees (it filters only
    // branch-location and workspace), so the node silently passed
    // validation AND could round-trip to Apply. Fold it to a legal kind
    // so the ownership checks surface it (missing-location-input).
    const normalized = graph(
      [{ id: 'kiosk-1', type: 'kiosk' as never, name: 'Kiosk', x: 0, y: 0 }],
      [],
    );

    expect(normalized.nodes[0]!.kind).toBe('workspace');
    // The unknown-kind node is now a workspace with no Location In — the
    // corrupt data surfaces as a real validation error instead of passing.
    const errors = validateTopologyGraph(normalized);
    expect(errors.some((e) => e.code === 'missing-location-input' && e.nodeId === 'kiosk-1')).toBe(true);
  });

  it('rejects duplicate wire ids across the whole graph (mirrors duplicate-node)', () => {
    // Node ids are guarded (seenNodeIds → duplicate-node) but wire ids
    // were never checked: the existing duplicate-wire error only fires
    // for location-ownership tuples sharing the same 4-tuple. Two wires
    // with the SAME id (UUID collision from a manual edit or a stale JSON
    // merge) pass validation silently even when their endpoints differ —
    // breaking the editor's React keys, click-cycle-by-id, and
    // delete-by-id, and round-tripping to Apply.
    const errors = validateTopologyGraph(graph(
      [branch(), workspace('ws-1'), workspace('ws-2')],
      [
        ownershipWire('wire-x', 'ws-1'),
        // Same id, DIFFERENT endpoints — no 4-tuple clash, only an id clash.
        ownershipWire('wire-x', 'ws-2'),
      ],
    ));

    expect(errors).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: 'duplicate-wire', wireId: 'wire-x' }),
    ]));
  });

  it('rejects a non-location wire that references a node missing from the graph', () => {
    // Endpoint existence is only enforced for LOCATION wires (via
    // invalid-location-connection). A stock-routing / ticket-routing /
    // generic wire pointing at a ghost node id passed validation
    // silently — inferredWire saw undefined nodes, fell to the last-resort
    // legacy branch, and the wire round-tripped to Apply referencing a
    // node that does not exist. Every wire endpoint must resolve.
    const errors = validateTopologyGraph(graph(
      [branch(), workspace('ws-1')],
      [
        { id: 'wire-ghost', fromNodeId: 'ghost-1', toNodeId: 'ws-1', fromPort: 'right', toPort: 'left', fromPortId: 'stock-out', toPortId: 'stock-in', relationshipType: 'stock-routing', direction: 'one-way' },
      ],
    ));

    expect(errors).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: 'unknown-wire-endpoint', wireId: 'wire-ghost' }),
    ]));
  });

  it('allows one Branch Location output to fan out to many workspaces', () => {
    const normalized = graph(
      [branch(), workspace('ws-a'), workspace('ws-b')],
      [ownershipWire('wire-a', 'ws-a'), ownershipWire('wire-b', 'ws-b')],
    );

    expect(validateTopologyGraph(normalized)).toEqual([]);
  });

  it('rejects a non-location wire whose semantic ports and relationship disagree', () => {
    const storePos = { ...workspace('store-pos'), metadata: { typeKey: 'store-pos' } };
    const warehouseNode = warehouse('warehouse-1');
    const normalized = graph(
      [branch(), storePos, warehouseNode],
      [
        ownershipWire('wire-store-location', 'store-pos'),
        {
          id: 'wire-invalid-pair',
          fromNodeId: 'store-pos',
          fromPortId: 'stock-out',
          toNodeId: 'warehouse-1',
          toPortId: 'location-in',
          relationshipType: 'stock-routing',
          direction: 'one-way',
        },
      ],
    );

    expect(validateTopologyGraph(normalized)).toEqual(expect.arrayContaining([
      expect.objectContaining({
        code: 'invalid-semantic-connection',
        wireId: 'wire-invalid-pair',
      }),
    ]));
  });

  it('accepts a valid stock-routing wire through the semantic contract', () => {
    const storePos = { ...workspace('store-pos'), metadata: { typeKey: 'store-pos' } };
    const warehouseNode = warehouse('warehouse-1');
    const normalized = graph(
      [branch(), storePos, warehouseNode],
      [
        ownershipWire('wire-store-location', 'store-pos'),
        {
          id: 'wire-stock',
          fromNodeId: 'store-pos',
          fromPortId: 'stock-out',
          toNodeId: 'warehouse-1',
          toPortId: 'stock-in',
          relationshipType: 'stock-routing',
          direction: 'one-way',
        },
      ],
    );

    expect(validateTopologyGraph(normalized)).toEqual([]);
  });

  it('rejects a paired ticket wire when the endpoints are not KDS and hardware', () => {
    const storePos = { ...workspace('store-pos'), metadata: { typeKey: 'store-pos' } };
    const printer = { id: 'printer-1', type: 'hardware' as const, name: 'Printer', x: 200, y: 200 };
    const normalized = graph(
      [branch(), storePos, printer],
      [
        ownershipWire('wire-store-location', 'store-pos'),
        {
          id: 'wire-invalid-ticket-source',
          fromNodeId: 'store-pos',
          fromPortId: 'ticket-out',
          toNodeId: 'printer-1',
          toPortId: 'ticket-in',
          relationshipType: 'ticket-routing',
          direction: 'one-way',
        },
      ],
    );

    expect(validateTopologyGraph(normalized)).toEqual(expect.arrayContaining([
      expect.objectContaining({
        code: 'invalid-semantic-connection',
        wireId: 'wire-invalid-ticket-source',
      }),
    ]));
  });

  it('accepts a KDS operationally connected to Restaurant POS', () => {
    const resto = { ...workspace('resto-pos'), metadata: { typeKey: 'restaurant-pos' } };
    const kds = { ...workspace('kds'), metadata: { typeKey: 'kds' } };
    const errors = validateTopologyGraph(graph(
      [branch(), resto, kds],
      [
        ownershipWire('wire-resto-location', 'resto-pos'),
        {
          id: 'wire-resto-kds',
          fromNodeId: 'resto-pos',
          fromPortId: 'operation-out',
          toNodeId: 'kds',
          toPortId: 'operation-in',
          relationshipType: 'generic',
          direction: 'one-way',
        },
      ],
    ));

    expect(errors).toEqual([]);
  });

  it('rejects an operation feed into KDS when the source is not Restaurant POS', () => {
    const storePos = { ...workspace('store-pos'), metadata: { typeKey: 'store-pos' } };
    const kds = { ...workspace('kds'), metadata: { typeKey: 'kds' } };
    const errors = validateTopologyGraph(graph(
      [branch(), storePos, kds],
      [
        ownershipWire('wire-pos-location', 'store-pos'),
        {
          id: 'wire-invalid-operation-source',
          fromNodeId: 'store-pos',
          fromPortId: 'operation-out',
          toNodeId: 'kds',
          toPortId: 'operation-in',
          relationshipType: 'generic',
          direction: 'one-way',
        },
      ],
    ));

    expect(errors).toEqual(expect.arrayContaining([
      expect.objectContaining({
        code: 'invalid-operation-source',
        nodeId: 'kds',
        wireId: 'wire-invalid-operation-source',
      }),
    ]));
  });

  it('infers a legacy Restaurant POS → KDS wire as the required operation feed', () => {
    const resto = { ...workspace('resto-pos'), metadata: { typeKey: 'restaurant-pos' } };
    const kds = { ...workspace('kds'), metadata: { typeKey: 'kds' } };
    const normalized = graph(
      [branch(), resto, kds],
      [
        ownershipWire('wire-resto-location', 'resto-pos'),
        {
          id: 'wire-resto-kds-legacy',
          fromNodeId: 'resto-pos',
          toNodeId: 'kds',
          fromPort: 'right',
          toPort: 'left',
          direction: 'one-way',
        },
      ],
    );

    expect(normalized.wires[1]).toMatchObject({
      fromPortId: 'operation-out',
      toPortId: 'operation-in',
      relationshipType: 'generic',
      legacyInferred: true,
    });
    expect(validateTopologyGraph(normalized)).toEqual([]);
  });

  it('rejects a workspace without the required Location In connection', () => {
    const errors = validateTopologyGraph(graph([branch(), workspace('ws-unowned')], []));

    expect(errors).toEqual(expect.arrayContaining([
      expect.objectContaining({
        code: 'missing-location-input',
        nodeId: 'ws-unowned',
        portId: 'location-in',
      }),
    ]));
  });

  it('rejects multiple parents and duplicate semantic wires', () => {
    const errors = validateTopologyGraph(graph(
      [branch(), branch('branch-2'), workspace('ws-1')],
      [ownershipWire('wire-a', 'ws-1'), ownershipWire('wire-a-duplicate', 'ws-1')],
    ));

    expect(errors).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: 'multiple-branch-locations' }),
      expect.objectContaining({ code: 'multiple-location-inputs', nodeId: 'ws-1' }),
      expect.objectContaining({ code: 'duplicate-wire', wireId: 'wire-a-duplicate' }),
    ]));
  });

  it('rejects a location wire that targets a non-workspace or uses the wrong ports', () => {
    const normalized = graph(
      [branch(), { ...workspace('warehouse-1'), type: 'warehouse' }],
      [
        {
          id: 'wire-invalid',
          fromNodeId: 'branch-1',
          fromPortId: 'location-in',
          toNodeId: 'warehouse-1',
          toPortId: 'location-out',
          relationshipType: 'location',
          direction: 'one-way',
        },
      ],
    );

    expect(validateTopologyGraph(normalized)).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: 'invalid-location-connection', wireId: 'wire-invalid' }),
    ]));
  });

  it('rejects a Branch Location without a canonical store identity', () => {
    const normalized = graph(
      [{ ...branch(), storeProfileId: '' }, workspace('ws-1')],
      [ownershipWire('wire-1', 'ws-1')],
    );

    expect(validateTopologyGraph(normalized)).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: 'branch-location-missing-identity', nodeId: 'branch-1' }),
    ]));
  });

  it('rejects directed operational cycles', () => {
    const normalized = graph(
      [branch(), workspace('ws-1'), workspace('ws-2')],
      [
        ownershipWire('wire-owner-1', 'ws-1'),
        ownershipWire('wire-owner-2', 'ws-2'),
        {
          id: 'wire-cycle-a',
          fromNodeId: 'ws-1',
          fromPortId: 'generic-out',
          toNodeId: 'ws-2',
          toPortId: 'generic-in',
          relationshipType: 'generic',
          direction: 'one-way',
        },
        {
          id: 'wire-cycle-b',
          fromNodeId: 'ws-2',
          fromPortId: 'generic-out',
          toNodeId: 'ws-1',
          toPortId: 'generic-in',
          relationshipType: 'generic',
          direction: 'one-way',
        },
      ],
    );

    expect(validateTopologyGraph(normalized)).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: 'cycle-detected' }),
    ]));
  });

  it('rejects duplicate node IDs', () => {
    const normalized = graph([branch(), branch(), workspace('ws-1')], []);

    expect(validateTopologyGraph(normalized)).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: 'duplicate-node', nodeId: 'branch-1' }),
    ]));
  });

  it('rejects an unsupported schema version before validating graph contents', () => {
    const normalized = graph([branch(), workspace('ws-1')], []);
    normalized.schemaVersion = TOPOLOGY_SCHEMA_VERSION + 1;

    expect(validateTopologyGraph(normalized)).toEqual([
      expect.objectContaining({ code: 'unsupported-schema-version' }),
    ]);
  });
});
