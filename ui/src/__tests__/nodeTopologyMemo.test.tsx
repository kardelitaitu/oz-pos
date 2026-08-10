import { type ComponentProps } from 'react';
import type { TopologyNodeCard as NodeCardComponent } from '../features/stores/topologyNodeCard';
import type { TopologyWireGroup as WireGroupComponent } from '../features/stores/topologyWireGroup';
import { fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import NodeTopologyEditor from '../features/stores/NodeTopologyEditor';
import { loadTopology } from '@/api/topology';
import multiStoreFtl from '@/locales/multi-store.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

// Render-count probes. Each mock wraps the REAL memo'd component with a
// counting boundary that (like the production React.memo) re-renders only
// when its props change. A flat count for an unrelated card/wire proves the
// editor hands the layers referentially-stable props — the useCallback
// contract the memo depends on — so hover/selection re-renders only the
// affected element instead of the whole canvas.
const { nodeCounts, wireCounts, stableL10n, stableSettings } = vi.hoisted(() => ({
  nodeCounts: {} as Record<string, number>,
  wireCounts: {} as Record<string, number>,
  // Production @fluent/react returns a referentially-stable Localization
  // object; the mock must mirror that or the l10n prop itself defeats the
  // memo every render.
  stableL10n: { getString: (id: string) => id },
  // Same for SettingsContext: the editor's getTelemetry takes `settings` as
  // a useCallback dep, so a fresh object per call would churn every card.
  stableSettings: {
    receipt: { showCurrency: false, decimalSeparator: 'dot', showTax: true, footer: '', paperWidth: 'standard', showTableNumber: false, marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0 },
    store: { name: 'Test Store', address: '', taxId: '', currency: 'IDR', branch: '' },
    sync: { serverUrl: null, hasApiKey: false, enabled: false },
    brand: { colour: '#10b981', storeName: 'Test Store' },
    preferences: { cardSize: 0, fontSize: 0, fontSmoothing: 'antialiased' },
    currencies: [],
    appVersion: '0.0.19',
  },
}));

vi.mock('../features/stores/topologyNodeCard', async (importOriginal) => {
  // The mocked module's type shape (only the component export is read).
  const actual = await importOriginal<{ TopologyNodeCard: typeof NodeCardComponent }>();
  const { memo: memoize } = await import('react');
  const RealCard = actual.TopologyNodeCard;
  return {
    ...actual,
    TopologyNodeCard: memoize((props: ComponentProps<typeof RealCard>) => {
      nodeCounts[props.node.id] = (nodeCounts[props.node.id] ?? 0) + 1;
      return <RealCard {...props} />;
    }),
  };
});

vi.mock('../features/stores/topologyWireGroup', async (importOriginal) => {
  const actual = await importOriginal<{ TopologyWireGroup: typeof WireGroupComponent }>();
  const { memo: memoize } = await import('react');
  const RealWire = actual.TopologyWireGroup;
  return {
    ...actual,
    TopologyWireGroup: memoize((props: ComponentProps<typeof RealWire>) => {
      wireCounts[props.wire.id] = (wireCounts[props.wire.id] ?? 0) + 1;
      return <RealWire {...props} />;
    }),
  };
});

vi.mock('@/api/topology', () => ({
  loadTopology: vi.fn(() => Promise.resolve(null)),
}));

vi.mock('@fluent/react', async () => {
  const actual = await vi.importActual('@fluent/react');
  return {
    ...actual,
    Localized: ({ children }: { id: string; children: React.ReactNode }) => <>{children}</>,
    useLocalization: () => ({
      l10n: stableL10n,
    }),
  };
});

vi.mock('@/contexts/SettingsContext', () => ({
  useSettings: () => ({
    settings: stableSettings,
    loading: false,
    error: null,
    hasPartialError: false,
    refetch: vi.fn(),
    lastChangedKeys: [],
    markSettingsUpdated: vi.fn(),
  }),
}));

const mockLoadTopology = vi.mocked(loadTopology);

/** Baseline = the counts accumulated during mount; tests assert interactions
 *  only change the elements they target. */
const snapshot = () => ({ ...nodeCounts, ...wireCounts });

const nodeCount = (id: string, base: Record<string, number>) => (nodeCounts[id] ?? 0) - (base[id] ?? 0);
const wireCount = (id: string, base: Record<string, number>) => (wireCounts[id] ?? 0) - (base[id] ?? 0);

describe('topology memoized render layers — hover/selection touch only the affected element', () => {
  // The load effect can fire more than once on mount (the editor shows a
  // placeholder preset while the async load resolves), so the mock ALWAYS
  // returns this diagram — every call resolves to the same nodes/wires.
  const renderWithPreset = async () => {
    mockLoadTopology.mockResolvedValue({
      nodes: [
        { id: 'store-1', type: 'store', name: 'Branch', x: 80, y: 140 },
        { id: 'ws-1', type: 'workspace', name: 'POS 1', x: 380, y: 80 },
        { id: 'ws-2', type: 'workspace', name: 'POS 2', x: 380, y: 260 },
      ],
      wires: [
        { id: 'w1', from_node_id: 'store-1', to_node_id: 'ws-1', direction: 'one-way' },
        { id: 'w2', from_node_id: 'store-1', to_node_id: 'ws-2', direction: 'one-way' },
      ],
    } as never);
    renderWithProvidersSync(<NodeTopologyEditor currentTier="standard" />, multiStoreFtl, sharedFtl);
    // Wait on a node id only THIS preset has (the placeholder is a retail
    // preset with wh-1, not ws-2), so the counts baseline is post-load.
    await waitFor(() =>
      expect(document.querySelector('.topology-node[data-node-id="ws-2"]')).not.toBeNull(),
    );
    return snapshot();
  };

  const nodeEl = (id: string) => document.querySelector(`.topology-node[data-node-id="${id}"]`) as HTMLElement;
  const wireEl = (id: string) => document.querySelector(`.wire-hitbox[data-wire-id="${id}"]`) as HTMLElement;

  it('pointer hover over a card re-renders no card and no wire', async () => {
    const base = await renderWithPreset();

    fireEvent.mouseEnter(nodeEl('ws-1'));
    fireEvent.mouseLeave(nodeEl('ws-1'));

    // Hovering dims only the elements NOT connected to ws-1: the non-neighbor
    // ws-2 and its wire w2 (2 renders each — dim on enter, restore on leave).
    // The hovered card, its neighbor store-1, and the connected wire w1 get
    // no prop change — the memo boundary absorbs them entirely.
    expect(nodeCount('store-1', base)).toBe(0);
    expect(nodeCount('ws-1', base)).toBe(0);
    expect(nodeCount('ws-2', base)).toBe(2);
    expect(wireCount('w1', base)).toBe(0);
    expect(wireCount('w2', base)).toBe(2);
  });

  it('selecting a card re-renders only that card, not its neighbors or wires', async () => {
    const base = await renderWithPreset();

    fireEvent.mouseDown(nodeEl('ws-1'), { button: 0 });
    fireEvent.mouseUp(nodeEl('ws-1'), { button: 0 });

    expect(nodeCount('ws-1', base)).toBe(1);
    expect(nodeCount('store-1', base)).toBe(0);
    expect(nodeCount('ws-2', base)).toBe(0);
    expect(wireCount('w1', base)).toBe(0);
    expect(wireCount('w2', base)).toBe(0);
  });

  it('cycling a wire direction re-renders only that wire, not the other wire or any card', async () => {
    const base = await renderWithPreset();

    fireEvent.click(wireEl('w1'));

    // The clicked wire's data changed (direction cycle) — it re-renders.
    expect(wireCount('w1', base)).toBe(1);
    // The sibling wire keeps object identity; cards are untouched by a wire edit.
    expect(wireCount('w2', base)).toBe(0);
    expect(nodeCount('store-1', base)).toBe(0);
    expect(nodeCount('ws-1', base)).toBe(0);
    expect(nodeCount('ws-2', base)).toBe(0);
  });
});
