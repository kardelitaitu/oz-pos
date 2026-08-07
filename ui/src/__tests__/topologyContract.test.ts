import { describe, expect, it } from 'vitest';
import type { TopologyNodeData, TopologyWireData } from '@/features/stores/NodeTopologyEditor';
import {
  TOPOLOGY_SCHEMA_VERSION,
  normalizeTopologyGraph,
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

  it('allows one Branch Location output to fan out to many workspaces', () => {
    const normalized = graph(
      [branch(), workspace('ws-a'), workspace('ws-b')],
      [ownershipWire('wire-a', 'ws-a'), ownershipWire('wire-b', 'ws-b')],
    );

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
