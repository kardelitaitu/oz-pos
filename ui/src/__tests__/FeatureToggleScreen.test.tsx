// ── FeatureToggleScreen component tests ─────────────────────────────
//
// Covers: loading state, error state with retry, empty state (no features),
// search with no results, feature list rendering, single toggle, bulk
// enable/disable, dependency display, and group headers.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluent } from '@/__tests__/test-utils/render';
import settingsFtl from '@/locales/settings.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import { ToastProvider } from '@/frontend/shared/Toast';
import FeatureToggleScreen from '@/features/settings/FeatureToggleScreen';

// ── Mock @tauri-apps/api/core ────────────────────────────────────

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Mock useAuth ─────────────────────────────────────────────────

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Admin', role_name: 'admin' },
  }),
}));



// ── Sample data ───────────────────────────────────────────────────

const sampleFeatures = [
  { key: 'cash_payment', name: 'Cash Payment', description: 'Accept cash payments', group: 'Core', enabled: true, dependencies: [] },
  { key: 'card_payment', name: 'Card Payment', description: 'Accept card payments', group: 'Payments', enabled: false, dependencies: ['cash_payment'] },
  { key: 'inventory_tracking', name: 'Inventory Tracking', description: 'Track stock levels', group: 'Products', enabled: true, dependencies: [] },
  { key: 'staff_login', name: 'Staff Login', description: 'PIN-based staff login', group: 'Staff', enabled: true, dependencies: [] },
  { key: 'barcode_scanning', name: 'Barcode Scanner', description: 'USB barcode scanning', group: 'Hardware', enabled: false, dependencies: [] },
];

const sampleFeaturesResult = { features: sampleFeatures };

// ── Tests ─────────────────────────────────────────────────────────

describe('FeatureToggleScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── Loading state ────────────────────────────────────────────

  it('renders loading spinner while fetching features', async () => {
    mockInvoke.mockReturnValue(new Promise(() => {}));

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    expect(screen.getByText(/loading features/i)).toBeInTheDocument();
  });

  // ── Error state ──────────────────────────────────────────────

  it('renders error with retry button when load fails', async () => {
    mockInvoke.mockRejectedValue(new Error('IPC error'));

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('IPC error')).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
  });

  it('calls load again when retry is clicked', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('IPC error'));
    mockInvoke.mockResolvedValueOnce(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /retry/i }));

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });
  });

  // ── Empty state ──────────────────────────────────────────────

  it('renders empty state when no features exist', async () => {
    mockInvoke.mockResolvedValue({ features: [] });

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText(/No features found/i)).toBeInTheDocument();
    });
  });

  // ── Main render ──────────────────────────────────────────────

  it('renders the title and subtitle with feature count', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Feature Toggles')).toBeInTheDocument();
    });
    // 3 of 5 enabled
    expect(screen.getByText(/3 \/ 5 enabled/)).toBeInTheDocument();
  });

  it('renders all feature groups with correct headers', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });
    // Use heading role to disambiguate from LSP nav chips that share group names
    expect(screen.getByRole('heading', { name: /core/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /payments/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /products/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /staff/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /hardware/i })).toBeInTheDocument();
  });

  it('renders feature names and descriptions', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });
    expect(screen.getByText('Accept cash payments')).toBeInTheDocument();
    expect(screen.getByText('Card Payment')).toBeInTheDocument();
  });

  it('renders group-level enabled/total count in each heading', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });

    // Core: cash_payment is enabled → "1/1".
    const coreHeading = screen.getByRole('heading', { name: /core/i });
    expect(within(coreHeading).getByText('1/1')).toBeInTheDocument();

    // Hardware: barcode_scanning is disabled → "0/1".
    const hardwareHeading = screen.getByRole('heading', { name: /hardware/i });
    expect(within(hardwareHeading).getByText('0/1')).toBeInTheDocument();
  });

  it('renders Enable All / Disable All bulk buttons per group', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });

    // Each group with features gets Enable All / Disable All buttons
    const enableButtons = screen.getAllByText(/enable all/i);
    const disableButtons = screen.getAllByText(/disable all/i);
    expect(enableButtons.length).toBe(5);
    expect(disableButtons.length).toBe(5);
  });

  // ── Search ────────────────────────────────────────────────────

  it('renders a search input', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });

    expect(screen.getByPlaceholderText(/search features/i)).toBeInTheDocument();
  });

  it('filters features when searching', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });

    const searchInput = screen.getByPlaceholderText(/search features/i);
    await userEvent.type(searchInput, 'card');

    await waitFor(() => {
      expect(screen.getByText('Card Payment')).toBeInTheDocument();
    });
    expect(screen.queryByText('Cash Payment')).not.toBeInTheDocument();
  });

  it('shows search-no-results message when query has no matches', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });

    const searchInput = screen.getByPlaceholderText(/search features/i);
    await userEvent.type(searchInput, 'zzzznotfound');

    await waitFor(() => {
      expect(screen.getByText(/No features match your search/i)).toBeInTheDocument();
    });
  });

  it('shows clear button when search has a value', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });

    const searchInput = screen.getByPlaceholderText(/search features/i);
    await userEvent.type(searchInput, 'card');

    const clearBtn = screen.getByRole('button', { name: /clear search/i });
    expect(clearBtn).toBeInTheDocument();

    await userEvent.click(clearBtn);
    expect(searchInput).toHaveValue('');
  });

  // ── Single toggle ─────────────────────────────────────────────

  it('toggles a feature on when clicked', async () => {
    const toggledFeatures = sampleFeatures.map((f) =>
      f.key === 'card_payment' ? { ...f, enabled: true } : f,
    );
    mockInvoke
      .mockResolvedValueOnce(sampleFeaturesResult)
      .mockResolvedValueOnce({ success: true, features: toggledFeatures, auto_enabled: [] });

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Card Payment')).toBeInTheDocument();
    });

    // Find the Card Payment toggle checkbox and click it
    const cardRow = screen.getByText('Card Payment').closest('.feature-toggle-item') as HTMLElement;
    const toggle = cardRow.querySelector<HTMLInputElement>('input[type="checkbox"]');
    expect(toggle).not.toBeNull();
    expect(toggle!.checked).toBe(false);

    await userEvent.click(toggle!);

    await waitFor(() => {
      // After toggle, the feature list updates and card_payment should be enabled
      expect(mockInvoke).toHaveBeenCalledWith(
        'set_feature',
        expect.objectContaining({ args: { key: 'card_payment', enabled: true } }),
      );
    });
  });

  it('shows auto-enabled dependency toast when toggle enables dependencies', async () => {
    const toggledFeatures = sampleFeatures.map((f) =>
      f.key === 'card_payment' ? { ...f, enabled: true } : f,
    );
    mockInvoke
      .mockResolvedValueOnce(sampleFeaturesResult)
      .mockResolvedValueOnce({
        success: true,
        features: toggledFeatures,
        auto_enabled: ['cash_payment'],
      });

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Card Payment')).toBeInTheDocument();
    });

    const autoCardRow = screen.getByText('Card Payment').closest('.feature-toggle-item') as HTMLElement;
    const autoToggle = autoCardRow.querySelector<HTMLInputElement>('input[type="checkbox"]');
    expect(autoToggle).not.toBeNull();
    await userEvent.click(autoToggle!);

    await waitFor(() => {
      expect(screen.getByText(/auto-enabled dependencies/i)).toBeInTheDocument();
    });
  });

  // ── Bulk toggle ───────────────────────────────────────────────

  it('bulk-enables all features in a group', async () => {
    const allHardwareEnabled = sampleFeatures.map((f) =>
      f.group === 'Hardware' ? { ...f, enabled: true } : f,
    );
    mockInvoke
      .mockResolvedValueOnce(sampleFeaturesResult)
      .mockResolvedValueOnce({ features: allHardwareEnabled });

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Barcode Scanner')).toBeInTheDocument();
    });

    // Find Hardware group's Enable All button and click it
    const hardwareGroup = screen.getByRole('heading', { name: /hardware/i }).closest('.feature-toggle-group') as HTMLElement;
    const enableAllBtn = within(hardwareGroup).getByText(/enable all/i);
    await userEvent.click(enableAllBtn);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'set_features_bulk',
        expect.objectContaining({
          args: expect.objectContaining({ keys: ['barcode_scanning'], enabled: true }),
        }),
      );
    });
  });

  it('bulk-disables all features in a group', async () => {
    const allCoreDisabled = sampleFeatures.map((f) =>
      f.group === 'Core' ? { ...f, enabled: false } : f,
    );
    mockInvoke
      .mockResolvedValueOnce(sampleFeaturesResult)
      .mockResolvedValueOnce({ features: allCoreDisabled });

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });

    // Find Core group's Disable All button and click it
    const coreGroup = screen.getByRole('heading', { name: /core/i }).closest('.feature-toggle-group') as HTMLElement;
    const disableAllBtn = within(coreGroup).getByText(/disable all/i);
    await userEvent.click(disableAllBtn);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'set_features_bulk',
        expect.objectContaining({
          args: expect.objectContaining({ keys: ['cash_payment'], enabled: false }),
        }),
      );
    });
  });

  // ── Dependency display ────────────────────────────────────────

  it('shows dependency info for features with dependencies', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Card Payment')).toBeInTheDocument();
    });

    // Card Payment depends on Cash Payment
    expect(screen.getByText(/requires: cash payment/i)).toBeInTheDocument();
  });

  it('does not show dependency info for features without dependencies', async () => {
    mockInvoke.mockResolvedValue(sampleFeaturesResult);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });

    // Cash Payment has no dependencies — the requires text should not appear in its row
    const cashRow = screen.getByText('Cash Payment').closest('.feature-toggle-item') as HTMLElement;
    expect(cashRow.querySelector('.feature-toggle-item-deps')).toBeNull();
  });

  // ── Disabled state on toggle ──────────────────────────────────

  it('disables toggle input while toggling is in progress', async () => {
    // Don't resolve the setFeature call — keeps toggling=true
    const togglePromise = new Promise(() => {});
    mockInvoke
      .mockResolvedValueOnce(sampleFeaturesResult)
      .mockReturnValueOnce(togglePromise);

    await renderWithFluent(<ToastProvider><FeatureToggleScreen /></ToastProvider>, settingsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Cash Payment')).toBeInTheDocument();
    });

    const disabledCashRow = screen.getByText('Cash Payment').closest('.feature-toggle-item') as HTMLElement;
    const disabledToggle = disabledCashRow.querySelector<HTMLInputElement>('input[type="checkbox"]');
    expect(disabledToggle).not.toBeNull();

    // Click to start toggling
    await userEvent.click(disabledToggle!);

    // Toggle should be disabled while in progress
    expect(disabledToggle!).toBeDisabled();
  });
});
