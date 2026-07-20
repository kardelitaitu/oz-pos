import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/__tests__/test-utils/render';
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
    currentInstanceId: 'inst-1',
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
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    });
  });

  // ── Opens dropdown on click ───────────────────────────────────

  it('opens dropdown when trigger is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);

    await waitFor(() => {
      expect(screen.getByRole('listbox')).toBeInTheDocument();
    });
    expect(screen.getByText('Store Front')).toBeInTheDocument();
    expect(screen.getByText('In Transit')).toBeInTheDocument();
  });

  // ── Closes on outside click ─────────────────────────────────────

  it('closes dropdown on outside click', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);
    await waitFor(() => {
      expect(screen.getByRole('listbox')).toBeInTheDocument();
    });

    // Click outside the dropdown
    await user.click(document.body);
    await waitFor(() => {
      expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    });
  });

  // ── Closes on Escape ────────────────────────────────────────────

  it('closes dropdown on Escape key', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);
    await waitFor(() => {
      expect(screen.getByRole('listbox')).toBeInTheDocument();
    });

    await user.keyboard('{Escape}');
    await waitFor(() => {
      expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    });
  });

  // ── Calls onChange with new location ────────────────────────────

  it('calls onChange with new location when option is clicked', async () => {
    const user = userEvent.setup();
    const handleChange = vi.fn();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={handleChange} />,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);

    await waitFor(() => {
      expect(screen.getByText('Store Front')).toBeInTheDocument();
    });

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
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);

    await waitFor(() => {
      expect(screen.getByRole('option', { name: /warehouse a warehouse/i })).toBeInTheDocument();
    });

    const sameOption = screen.getByRole('option', { name: /warehouse a warehouse/i });
    await user.click(sameOption);

    expect(handleChange).not.toHaveBeenCalled();
  });

  // ── Shows type metadata in dropdown ─────────────────────────────

  it('displays location type metadata in dropdown options', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);

    await waitFor(() => {
      expect(screen.getByText('warehouse')).toBeInTheDocument();
      expect(screen.getByText('store')).toBeInTheDocument();
      expect(screen.getByText('transit')).toBeInTheDocument();
    });
  });

  // ── Highlights active location with aria-selected ──────────────

  it('marks active location with aria-selected', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
    );

    await waitFor(() => {
      expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    });

    const trigger = screen.getByRole('button', { name: /select inventory location/i });
    await user.click(trigger);

    await waitFor(() => {
      const options = screen.getAllByRole('option');
      const activeOption = options.find((opt) => opt.getAttribute('aria-selected') === 'true');
      expect(activeOption).toBeDefined();
      expect(activeOption).toHaveTextContent('Warehouse A');
    });
  });

  // ── Hides when no locations are loaded ─────────────────────────

  it('renders nothing when locations fail to load', async () => {
    mockListLocations.mockResolvedValue([]);
    renderWithProviders(
      <LocationPicker value="loc-warehouse" onChange={vi.fn()} />,
    );

    await new Promise((r) => setTimeout(r, 100));
    // The component returns null when locations is empty
    expect(screen.queryByRole('button', { name: /select inventory location/i })).not.toBeInTheDocument();
  });
});
