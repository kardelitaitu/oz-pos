import { describe, it, expect, beforeEach } from 'vitest';
import {
  TOPOLOGY_EXPORT_FORMAT,
  TOPOLOGY_EXPORT_VERSION,
  serializeTopology,
  deserializeTopology,
  saveTemplate,
  loadTemplate,
  listTemplates,
  deleteTemplate,
} from '../features/stores/topologyExport';

const nodes = [
  { id: 'store-1', type: 'store' as const, name: 'Downtown', x: 80, y: 140, subtitle: 'Branch' },
  { id: 'ws-1', type: 'workspace' as const, name: 'POS #1', x: 400, y: 80, metadata: { typeKey: 'store-pos' } },
];
const wires = [{ id: 'w1', fromNodeId: 'store-1', toNodeId: 'ws-1', direction: 'one-way' as const, label: 'Stock' }];

describe('serializeTopology / deserializeTopology (versioned export envelope)', () => {
  it('round-trips nodes and wires losslessly', () => {
    const json = serializeTopology(nodes, wires);
    const out = deserializeTopology(json);
    expect(out).not.toBeNull();
    expect(out!.nodes).toEqual(nodes);
    expect(out!.wires).toEqual(wires);
  });

  it('carries the versioned envelope format', () => {
    const json = serializeTopology(nodes, wires);
    const parsed = JSON.parse(json);
    expect(parsed.format).toBe(TOPOLOGY_EXPORT_FORMAT);
    expect(parsed.version).toBe(TOPOLOGY_EXPORT_VERSION);
  });

  it('preserves optional fields (bends, labels, metadata) on the way through', () => {
    const bent = [
      { id: 'w2', fromNodeId: 'store-1', toNodeId: 'ws-1', direction: 'two-way' as const, bends: [{ x: 300, y: 200 }] },
    ];
    const out = deserializeTopology(serializeTopology(nodes, bent));
    expect(out!.wires).toEqual(bent);
  });

  it('rejects garbage, wrong format, and wrong version', () => {
    expect(deserializeTopology('not json')).toBeNull();
    expect(deserializeTopology('42')).toBeNull();
    expect(deserializeTopology(JSON.stringify({ format: 'other', version: 1, nodes, wires }))).toBeNull();
    expect(deserializeTopology(JSON.stringify({ format: TOPOLOGY_EXPORT_FORMAT, version: 99, nodes, wires }))).toBeNull();
    expect(deserializeTopology(JSON.stringify({ format: TOPOLOGY_EXPORT_FORMAT, version: 1 }))).toBeNull();
  });

  it('rejects a malformed node, a malformed wire, and duplicate ids', () => {
    const badNode = serializeTopology([{ ...nodes[0]!, id: '' }], wires);
    expect(deserializeTopology(badNode)).toBeNull();

    const badWire = serializeTopology(nodes, [{ ...wires[0]!, direction: 'sideways' as unknown as 'one-way' }]);
    expect(deserializeTopology(badWire)).toBeNull();

    const dup = serializeTopology(
      [nodes[0]!, { ...nodes[1]!, id: 'store-1' }],
      wires,
    );
    expect(deserializeTopology(dup)).toBeNull();
  });
});

describe('diagram templates (localStorage)', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('saves, lists, loads, and deletes a template', () => {
    expect(listTemplates()).toEqual([]);

    const key = saveTemplate('Main Floor', nodes, wires);
    expect(key).toBe('oz-topology-template:Main Floor');
    expect(listTemplates()).toEqual(['Main Floor']);

    const loaded = loadTemplate('Main Floor');
    expect(loaded).not.toBeNull();
    expect(loaded!.nodes).toEqual(nodes);

    deleteTemplate('Main Floor');
    expect(listTemplates()).toEqual([]);
    expect(loadTemplate('Main Floor')).toBeNull();
  });

  it('rejects an empty name and sorts the list', () => {
    expect(saveTemplate('   ', nodes, wires)).toBeNull();
    saveTemplate('B', nodes, wires);
    saveTemplate('A', nodes, wires);
    expect(listTemplates()).toEqual(['A', 'B']);
  });

  it('returns null for a corrupt stored template', () => {
    localStorage.setItem('oz-topology-template:broken', 'not json');
    expect(loadTemplate('broken')).toBeNull();
  });
});
