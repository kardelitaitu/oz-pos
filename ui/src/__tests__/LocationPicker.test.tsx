import { describe, expect, it, vi, beforeEach } from 'vitest';
import { useState } from 'react';
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/__tests__/test-utils/render';
import inventoryFtl from '@/locales/inventory.ftl?raw';
import inventoryIdFtl from '@/locales/inventory.id.ftl?raw';
import LocationPicker from '@/features/inventory/LocationPicker';

// ── Mock auth and workspace contexts ───────────────────────────
vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Test User', role_name: 'cashier', session_token: 'mock-session-token' },
  }),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    sessionToken: 'mock-session-token',
    activeInstance: { instance_id: 'inst-1' },
    swapSessionToken: vi.fn(),
  }),
}));

// ── Mock API module ──────────────────────────────────────────────

const mockLocations = [
  { id: 'loc-warehouse', name: 'Warehouse A', type: 'warehouse', description: 'Main warehouse', is_active: true, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
  { id: 'loc-store', name: 'Store Front', type: 'store', description: 'Retail store', is_active: true, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
  { id: 'loc-transit', name: 'In Transit', type: 'transit', description: 'Transit hub', is_active: true, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
];

const mockListLocations = vi.fn();

vi.mock('@/api/inventory', async (importOriginal) => {
  // eslint-disable-next-line @typescript-eslint/consistent-type-imports
  const actual = await importOriginal<typeof import('@/api/inventory')>();
  return {
    ...actual,
    listInventoryLocations: (...args: unknown[]) => mockListLocations(...args),
  };
});

describe('LocationPicker', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mockListLocations.mockResolvedValue(mockLocations);
  });

  // ── Renders trigger with current value ──────────────────────────

  it('renders the currently selected location name', async () => {
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  // ── Opens dropdown on click ───────────────────────────────────

  it('opens dropdown when trigger is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);

    await waitFor(() => {
      expect(screen.getByRole('listbox')).toBeInTheDocument();
    }, { timeout: 5000 });
    expect(screen.getByText('Store Front')).toBeInTheDocument();
    expect(screen.getByText('In Transit')).toBeInTheDocument();
  });

  // ── Closes on outside click ─────────────────────────────────────

  it('closes dropdown on outside click', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);
    await waitFor(() => {
      expect(screen.getByRole('listbox')).toBeInTheDocument();
    }, { timeout: 5000 });

    // Click outside the dropdown
    await user.click(document.body);
    await waitFor(() => {
      expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    }, { timeout: 5000 });
  });

  // ── Closes on Escape ────────────────────────────────────────────

  it('closes dropdown on Escape key', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);
    await waitFor(() => {
      expect(screen.getByRole('listbox')).toBeInTheDocument();
    }, { timeout: 5000 });

    await user.keyboard('{Escape}');
    await waitFor(() => {
      expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    }, { timeout: 5000 });
  });

  // ── Calls onChange with new location ────────────────────────────

  it('calls onChange with new location when option is clicked', async () => {
    const user = userEvent.setup();
    const handleChange = vi.fn();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={handleChange} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);

    await waitFor(() => {
      expect(screen.getByText('Store Front')).toBeInTheDocument();
    }, { timeout: 5000 });

    const storeOption = screen.getByRole('option', { name: /store front store/i });
    await user.click(storeOption);

    expect(handleChange).toHaveBeenCalledWith('loc-store', 'Store Front');
  });

  // ── Does not call onChange when same location is re-selected ──

  it('does not call onChange when same location is clicked', async () => {
    const user = userEvent.setup();
    const handleChange = vi.fn();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={handleChange} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);

    await waitFor(() => {
      expect(screen.getByRole('option', { name: /warehouse a warehouse/i })).toBeInTheDocument();
    }, { timeout: 5000 });

    const sameOption = screen.getByRole('option', { name: /warehouse a warehouse/i });
    await user.click(sameOption);

    expect(handleChange).not.toHaveBeenCalled();
  });

  // ── Shows type metadata in dropdown ─────────────────────────────

  it('displays localized location type metadata in dropdown options', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);

    // Raw machine values (warehouse/store/transit) must never leak — the
    // en bundle renders capitalized Fluent labels instead (LOC-05).
    await waitFor(() => {
      expect(screen.getByText('Warehouse')).toBeInTheDocument();
      expect(screen.getByText('Store')).toBeInTheDocument();
      expect(screen.getByText('Transit')).toBeInTheDocument();
    }, { timeout: 5000 });
    expect(screen.queryByText('warehouse')).not.toBeInTheDocument();
    expect(screen.queryByText('store')).not.toBeInTheDocument();
  });

  it('renders Indonesian type labels from the id bundle', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryIdFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    // The id bundle renders the trigger aria-label in Indonesian, so query by
    // the Indonesian label text rather than the English regex.
    const trigger = screen.getByRole('button', { name: /pilih lokasi inventaris/i });
    await user.click(trigger);

    // Indonesian bundle: warehouse → Gudang, store → Toko.
    await waitFor(() => {
      expect(screen.getByText('Gudang')).toBeInTheDocument();
      expect(screen.getByText('Toko')).toBeInTheDocument();
    }, { timeout: 5000 });
    expect(screen.queryByText('warehouse')).not.toBeInTheDocument();
  });

  // ── Highlights active location with aria-selected ──────────────

  it('marks active location with aria-selected', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);

    await waitFor(() => {
      const options = screen.getAllByRole('option');
      const activeOption = options.find((opt) => opt.getAttribute('aria-selected') === 'true');
      expect(activeOption).toBeDefined();
      expect(activeOption).toHaveTextContent('Warehouse A');
    }, { timeout: 5000 });
  });

  // ── Hides when no locations are loaded ─────────────────────────

  it('renders nothing when locations fail to load', async () => {
    mockListLocations.mockResolvedValue([]);
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryFtl,
    );

    // Wait for the (empty) locations fetch to settle, then confirm the component hides itself
    await waitFor(() => {
      expect(mockListLocations).toHaveBeenCalled();
    }, { timeout: 5000 });
    expect(screen.queryByRole('button', { name: /select inventory location/i })).not.toBeInTheDocument();
  });

  // ── Durable error state (INV-08) ───────────────────────────────

  it('shows a persistent error with retry when the locations fetch fails', async () => {
    mockListLocations.mockRejectedValue(new Error('boom'));
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryFtl,
    );

    // The error message (loc-picker-error-load = "Failed to load locations")
    // must render persistently instead of silently returning null.
    // Timeout 10s (was 5s): flaked once under parallel-worker load during the
    // full 8-file inventory sweep — the alert render raced the 5s budget.
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    }, { timeout: 10000 });
    expect(screen.getByText('Failed to load locations')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });

  it('recovers when Retry succeeds after a failed load', async () => {
    const user = userEvent.setup();
    mockListLocations.mockRejectedValueOnce(new Error('boom')).mockResolvedValueOnce(mockLocations);
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    }, { timeout: 10000 });

    await user.click(screen.getByRole('button', { name: 'Retry' }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /select inventory location/i })).toBeInTheDocument();
    }, { timeout: 10000 });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  // ── Listbox keyboard navigation (LOC-04) ───────────────────────

  it('moves the highlighted option with ArrowDown and selects with Enter', async () => {
    const user = userEvent.setup();
    const handleChange = vi.fn();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={handleChange} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);
    await waitFor(() => {
      expect(screen.getByRole('listbox')).toBeInTheDocument();
    }, { timeout: 5000 });

    // Open focuses the listbox; the current value is pre-highlighted (index 0).
    const listbox = screen.getByRole('listbox');
    expect(listbox.getAttribute('aria-activedescendant')).toBe('location-picker-option-loc-warehouse');

    // ArrowDown → Store Front (index 1)
    await user.keyboard('{ArrowDown}');
    expect(listbox.getAttribute('aria-activedescendant')).toBe('location-picker-option-loc-store');

    // Enter selects it
    await user.keyboard('{Enter}');
    expect(handleChange).toHaveBeenCalledWith('loc-store', 'Store Front');
  });

  it('wraps ArrowUp and jumps with Home/End', async () => {
    const user = userEvent.setup();
    const handleChange = vi.fn();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={handleChange} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);
    await waitFor(() => {
      expect(screen.getByRole('listbox')).toBeInTheDocument();
    }, { timeout: 5000 });

    const listbox = screen.getByRole('listbox');
    // Current value loc-warehouse is index 0 → pre-highlighted.
    expect(listbox.getAttribute('aria-activedescendant')).toBe('location-picker-option-loc-warehouse');

    // ArrowUp from the first option wraps to the last (index 2 = loc-transit)
    await user.keyboard('{ArrowUp}');
    expect(listbox.getAttribute('aria-activedescendant')).toBe('location-picker-option-loc-transit');

    // Home jumps to the first option
    await user.keyboard('{Home}');
    expect(listbox.getAttribute('aria-activedescendant')).toBe('location-picker-option-loc-warehouse');

    // End jumps to the last option
    await user.keyboard('{End}');
    expect(listbox.getAttribute('aria-activedescendant')).toBe('location-picker-option-loc-transit');

    // Space selects the highlighted option
    await user.keyboard(' ');
    expect(handleChange).toHaveBeenCalledWith('loc-transit', 'In Transit');
  });

  // ── LOC-07: invalidation + out-of-order response guard ──────────
  //
  // These tests drive `refreshKey` through a stateful harness instead of RTL
  // `rerender()`: rerender replaces the WHOLE root element, which would drop
  // the Fluent LocalizationProvider / DefaultProviders wrapper and crash the
  // component (useLocalization has no context). The harness bumps a stateful
  // refreshKey the same way a real consumer would after a location mutation.

  // Hoisted so LocationPicker (a memo() component) never sees a fresh onChange
  // reference on every harness re-render (which would defeat memo short-circuit).
  const noopChange = vi.fn();

  function RefreshKeyHarness({
    onChange = noopChange,
  }: {
    onChange?: (locationId: string, locationName: string) => void;
  }) {
    const [refreshKey, setRefreshKey] = useState(0);
    return (
      <>
        <button type="button" onClick={() => setRefreshKey((k) => k + 1)}>
          bump-refresh
        </button>
        <LocationPicker
          value="loc-warehouse"
          onChange={onChange}
          refreshKey={refreshKey}
        />
      </>
    );
  }

  it('refetches locations when refreshKey is bumped (external invalidation)', async () => {
    const user = userEvent.setup();
    renderWithProviders(<RefreshKeyHarness />, inventoryFtl);

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });
    expect(mockListLocations).toHaveBeenCalledTimes(1);

    // A location is renamed elsewhere; the consumer bumps refreshKey and the
    // picker must reload to pick up the fresh list.
    mockListLocations.mockResolvedValue([
      { id: 'loc-warehouse', name: 'Warehouse A (renamed)', type: 'warehouse', description: '', is_active: true, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
      ...mockLocations.slice(1),
    ]);
    await user.click(screen.getByRole('button', { name: 'bump-refresh' }));

    await waitFor(() => {
      expect(mockListLocations).toHaveBeenCalledTimes(2);
    }, { timeout: 5000 });
    await waitFor(() => {
      expect(screen.getByText('Warehouse A (renamed)')).toBeInTheDocument();
    }, { timeout: 5000 });
    expect(screen.queryByText('Warehouse A')).not.toBeInTheDocument();
  });

  it('guards against out-of-order responses (stale load never overwrites fresh)', async () => {
    const user = userEvent.setup();
    // Two overlapping loads: the SECOND resolves first, the FIRST resolves
    // last. Only the second (newest) result may be applied.
    let resolveFirst!: (v: typeof mockLocations) => void;
    let resolveSecond!: (v: typeof mockLocations) => void;
    const first = new Promise<typeof mockLocations>((r) => { resolveFirst = r; });
    const second = new Promise<typeof mockLocations>((r) => { resolveSecond = r; });

    mockListLocations
      .mockReturnValueOnce(first)
      .mockReturnValueOnce(second);

    renderWithProviders(<RefreshKeyHarness />, inventoryFtl);

    // First load is in flight.
    await waitFor(() => {
      expect(mockListLocations).toHaveBeenCalledTimes(1);
    }, { timeout: 5000 });

    // Bump refreshKey → second load starts.
    await user.click(screen.getByRole('button', { name: 'bump-refresh' }));
    await waitFor(() => {
      expect(mockListLocations).toHaveBeenCalledTimes(2);
    }, { timeout: 5000 });

    // The NEW request resolves first with the fresh list.
    await act(async () => {
      resolveSecond(mockLocations);
    });
    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    // The STALE first request resolves last with an outdated payload — the
    // seq guard must drop it.
    await act(async () => {
      resolveFirst([
        { id: 'loc-stale', name: 'Stale Warehouse', type: 'warehouse', description: '', is_active: true, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
      ]);
    });
    expect(screen.queryByText('Stale Warehouse')).not.toBeInTheDocument();
    // The fresh list is still what's shown.
    expect(screen.getByText('Warehouse A')).toBeInTheDocument();
  });

  it('restores focus to the trigger and closes on Escape', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
      inventoryFtl,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    }, { timeout: 5000 });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);
    await waitFor(() => {
      expect(screen.getByRole('listbox')).toBeInTheDocument();
    }, { timeout: 5000 });

    await user.keyboard('{Escape}');
    await waitFor(() => {
      expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    }, { timeout: 5000 });
    expect(trigger).toHaveFocus();
  });
});
