// ── Editor revision-conflict recovery through the REAL dev-mock IPC ──
// (round 139)
//
// The round-137 recovery test mocked the API at the editor boundary
// (onSave rejects with the typed shape) and the round-138 test pinned the
// dev-mock's revision gate in isolation. This test stitches the PRODUCTION
// chain together: the editor calls the real `@/api/topology` wrappers,
// which invoke through the REAL dev-mock handlers — the same alias the
// Vite dev server applies in serve mode (vite.config.ts). It proves the
// browser preview's conflict recovery works end-to-end: a concurrent
// writer bumps the revision, the stale editor's Apply is rejected by the
// mock gate, and the editor reloads the authoritative diagram.

import { screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import NodeTopologyEditor from '../features/stores/NodeTopologyEditor';
import { loadTopology, applyTopologyDiff } from '@/api/topology';
import type { TopologyNodePayload, TopologyWirePayload } from '@/api/topology';
import multiStoreFtl from '@/locales/multi-store.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

// Route the REAL @tauri-apps/api/core invoke to the REAL dev-mock handlers —
// the same alias the Vite dev server applies in serve mode (vite.config.ts).
// jsdom has no window.__TAURI_INTERNALS__, so the dev-mock's invoke routes
// to its in-memory handlers. This is the production browser-preview chain:
// editor → @/api/topology → loggedInvoke → dev-mock invoke → handler.
vi.mock('@tauri-apps/api/core', async () => {
  const { invoke } = await import('@/dev-mock/tauri-api');
  return { invoke };
});

vi.mock('@fluent/react', async () => {
  const actual = await vi.importActual('@fluent/react');
  // Only keys this test asserts on need English text; everything else
  // falls back to the key string (tests assert on English text).
  const EN: Record<string, string> = {
    'topology-new-store': 'New Store',
    'topology-toast-revision-conflict': 'The topology changed elsewhere — loaded the latest version. Re-apply your changes.',
  };
  return {
    ...actual,
    Localized: ({ children }: { id: string; children: React.ReactNode }) => <>{children}</>,
    useLocalization: () => ({
      l10n: {
        getString: (id: string) => EN[id] ?? id,
      },
    }),
  };
});

vi.mock('@/contexts/SettingsContext', () => ({
  useSettings: () => ({
    settings: {
      receipt: {
        showCurrency: false,
        decimalSeparator: 'dot',
        showTax: true,
        footer: '',
        paperWidth: 'standard',
        showTableNumber: false,
        marginTop: 0,
        marginBottom: 0,
        marginLeft: 0,
        marginRight: 0,
      },
      store: { name: 'Test Store', address: '', taxId: '', currency: 'IDR', branch: '' },
      sync: { serverUrl: null, hasApiKey: false, enabled: false },
      brand: { colour: '#10b981', storeName: 'Test Store' },
      preferences: { cardSize: 0, fontSize: 0, fontSmoothing: 'antialiased' },
      currencies: [],
      appVersion: '0.0.25',
    },
    loading: false,
    error: null,
    hasPartialError: false,
    refetch: vi.fn(),
    lastChangedKeys: [],
    markSettingsUpdated: vi.fn(),
  }),
}));

// Wire the editor's Apply to the REAL API → dev-mock chain, mirroring
// TopologyScreen's handleTopologySave (minus the screen's diff/validation
// layer, which has its own coverage): pass the editor's base revision
// through so the dev-mock gate sees exactly what the backend would.
const handleSave = async (
  nodes: unknown,
  wires: unknown,
  baseRevision?: number,
) =>
  applyTopologyDiff(
    'test-session-token',
    [],
    [],
    [],
    nodes as TopologyNodePayload[],
    wires as TopologyWirePayload[],
    undefined,
    baseRevision,
  );

const renderEditor = () =>
  renderWithProvidersSync(
    <NodeTopologyEditor currentTier="standard" onSave={handleSave} />,
    multiStoreFtl,
    sharedFtl,
  );

const getNodeCount = () => document.querySelectorAll('.topology-node').length;

// The dev-mock logs every invoke; keep the test output clean.
beforeEach(() => {
  vi.spyOn(console, 'log').mockImplementation(() => {});
  vi.spyOn(console, 'warn').mockImplementation(() => {});
});

describe('editor revision-conflict recovery through the real dev-mock IPC', () => {
  it('reloads the authoritative diagram when the dev-mock rejects a stale Apply', async () => {
    // Snapshot the seeded dev-mock state (persisted to localStorage, so
    // read the CURRENT revision/nodes to self-heal across re-runs).
    const seeded = await loadTopology();
    expect(seeded).not.toBeNull();
    const seedNodes = seeded?.nodes ?? [];
    const seedWires = seeded?.wires ?? [];
    const seedRevision = seeded?.revision ?? 0;

    renderEditor();

    // The editor loads the seeded diagram through the real chain.
    await waitFor(() => expect(getNodeCount()).toBe(seedNodes.length));

    // A concurrent writer (another tab / the sync daemon) applies a NEWER
    // diagram on top of the current revision — the editor is now stale.
    const authoritativeNodes: TopologyNodePayload[] = [
      ...seedNodes,
      { id: 'store-auth', type: 'store', name: 'Authoritative Branch', x: 520, y: 520 },
    ];
    const bumped = await applyTopologyDiff(
      'test-session-token',
      [],
      [],
      [],
      authoritativeNodes,
      seedWires,
      undefined,
      seedRevision,
    );
    expect(bumped.revision).toBe(seedRevision + 1);

    // The stale editor user makes an edit on the old revision.
    fireEvent.click(screen.getByText('+ Store Node'));
    await waitFor(() => expect(screen.getByText('New Store')).toBeInTheDocument());

    // Apply — the dev-mock gate rejects the stale baseRevision, and the
    // editor must adopt the authoritative diagram (round 137 recovery).
    fireEvent.click(screen.getByText('Apply Topology Changes'));

    await waitFor(() => expect(screen.getByText('Authoritative Branch')).toBeInTheDocument());
    // The stale canvas (including the user's spawned node) is replaced.
    expect(screen.queryByText('New Store')).not.toBeInTheDocument();
    expect(getNodeCount()).toBe(authoritativeNodes.length);
    // The conflict toast explains the reload.
    expect(screen.getByText('The topology changed elsewhere — loaded the latest version. Re-apply your changes.')).toBeInTheDocument();

    // Self-heal: restore the seed diagram for watch-mode re-runs.
    await applyTopologyDiff(
      'test-session-token',
      [],
      [],
      [],
      seedNodes,
      seedWires,
      undefined,
      seedRevision + 1,
    );
  });
});
