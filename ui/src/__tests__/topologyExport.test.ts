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

  it('round-trips the warehouse stock metadata shape losslessly', () => {
    const stocked = [
      ...nodes,
      {
        id: 'wh-1',
        type: 'warehouse' as const,
        name: 'Main Stock Room',
        x: 680,
        y: 140,
        metadata: { stock: 250, capacity: 1000, lowStockThreshold: 25 },
      },
    ];
    const out = deserializeTopology(serializeTopology(stocked, wires));
    expect(out).not.toBeNull();
    expect(out!.nodes).toHaveLength(3);
    expect(out!.nodes[2]).toMatchObject({
      id: 'wh-1',
      type: 'warehouse',
      name: 'Main Stock Room',
      metadata: { stock: 250, capacity: 1000, lowStockThreshold: 25 },
    });
  });

  it('rejects a warehouse whose stock metadata is not a finite number', () => {
    const malformed = [
      ...nodes,
      {
        id: 'wh-1',
        type: 'warehouse' as const,
        name: 'Main Stock Room',
        x: 680,
        y: 140,
        metadata: { capacity: '1000' as unknown as number, lowStockThreshold: 25 },
      },
    ];
    // A hand-edited or drifted document must not half-load: the string
    // capacity would silently drop through readNumber/metadataNumber.
    expect(deserializeTopology(serializeTopology(malformed, wires))).toBeNull();
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

  it('rejects a wire whose fromPort/toPort is not a canonical port name', () => {
    // The geometry reads ports raw (PORT_OFFSET[port]); a hand-edited value
    // like 123 or 'diagonal' would pass the old shape check and crash the
    // canvas with an undefined offset dereference. The strict contract must
    // reject it at parse time.
    const badFromPort = serializeTopology(
      nodes,
      [{ ...wires[0]!, fromPort: 123 as unknown as 'right' }],
    );
    expect(deserializeTopology(badFromPort)).toBeNull();

    const badToPort = serializeTopology(
      nodes,
      [{ ...wires[0]!, toPort: 'diagonal' as unknown as 'left' }],
    );
    expect(deserializeTopology(badToPort)).toBeNull();
  });

  it('rejects duplicate wire ids (two wires sharing one id behave as one)', () => {
    // Duplicate NODE ids already reject; wire ids were unchecked — a pasted
    // diagram with two wires under one id would make delete/cycle operations
    // hit BOTH wires, so the parser must refuse it like node ids.
    const dup = serializeTopology(nodes, [wires[0]!, { ...wires[0]!, id: 'w1' }]);
    expect(deserializeTopology(dup)).toBeNull();
  });

  it('rejects malformed bend shapes on a wire', () => {
    // The geometry reads bends RAW (`wire.bends.map(...)`): a hand-edited
    // non-array would CRASH the canvas render, and a bend entry without
    // finite x/y produces NaN-coordinate degenerate paths. The strict
    // contract refuses the whole payload instead of half-loading it.
    const badNonArray = serializeTopology(
      nodes,
      [{ ...wires[0]!, bends: 'oops' as unknown as Array<{ x: number; y: number }> }],
    );
    expect(deserializeTopology(badNonArray)).toBeNull();

    const badMissingY = serializeTopology(
      nodes,
      [{ ...wires[0]!, bends: [{ x: 300 }] as unknown as Array<{ x: number; y: number }> }],
    );
    expect(deserializeTopology(badMissingY)).toBeNull();

    const badStringCoords = serializeTopology(
      nodes,
      [{ ...wires[0]!, bends: [{ x: '300', y: 200 }] as unknown as Array<{ x: number; y: number }> }],
    );
    expect(deserializeTopology(badStringCoords)).toBeNull();

    const badNonObjectEntry = serializeTopology(
      nodes,
      [{ ...wires[0]!, bends: [42] as unknown as Array<{ x: number; y: number }> }],
    );
    expect(deserializeTopology(badNonObjectEntry)).toBeNull();
  });

  it('rejects a wire whose endpoint references a node missing from the payload', () => {
    // A dangling endpoint cannot render (the geometry skips it) and the
    // imported diagram immediately banners unknown-wire-endpoint — a
    // drifted document must be refused whole, like every other broken shape.
    const danglingFrom = serializeTopology(
      nodes,
      [{ ...wires[0]!, fromNodeId: 'ghost-node' }],
    );
    expect(deserializeTopology(danglingFrom)).toBeNull();

    const danglingTo = serializeTopology(
      nodes,
      [{ ...wires[0]!, toNodeId: 'ghost-node' }],
    );
    expect(deserializeTopology(danglingTo)).toBeNull();
  });

  it('still round-trips canonical ports and wires after the stricter validation', () => {
    const wired = [
      { id: 'w3', fromNodeId: 'store-1', fromPort: 'right' as const, toNodeId: 'ws-1', toPort: 'left' as const, direction: 'reverse' as const },
    ];
    const out = deserializeTopology(serializeTopology(nodes, wired));
    expect(out).not.toBeNull();
    expect(out!.wires).toEqual(wired);
  });

  it('keeps a canonical wire with a valid (and an empty) bends array lossless', () => {
    const bent = [
      { id: 'w-bent', fromNodeId: 'store-1', toNodeId: 'ws-1', direction: 'one-way' as const, bends: [{ x: 300, y: 200 }] },
      { id: 'w-plain', fromNodeId: 'store-1', toNodeId: 'ws-1', direction: 'one-way' as const, bends: [] },
    ];
    const out = deserializeTopology(serializeTopology(nodes, bent));
    expect(out).not.toBeNull();
    expect(out!.wires).toEqual(bent);
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
