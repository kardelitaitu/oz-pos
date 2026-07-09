import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import type { ReactNode } from 'react';
import FeatureToggleScreen from '@/features/settings/FeatureToggleScreen';
import settingsFtl from '@/locales/settings.ftl?raw';
import salesFtl from '@/locales/sales.ftl?raw';
import { ToastProvider } from '@/hooks/useToast';

// ── Mock Tauri IPC ─────────────────────────────────────────────────

const mockFeatures = [
  { key: 'simple-retail', name: 'Simple Retail', description: 'Core retail POS', group: 'Core', enabled: true, dependencies: [] },
  { key: 'restaurant', name: 'Restaurant', description: 'Restaurant mode with tables', group: 'Core', enabled: false, dependencies: [] },
  { key: 'cash-payment', name: 'Cash', description: 'Accept cash payments', group: 'Payments', enabled: true, dependencies: [] },
  { key: 'card-payment', name: 'Card', description: 'Accept card payments', group: 'Payments', enabled: false, dependencies: [] },
  { key: 'multi-currency', name: 'Multi-Currency', description: 'Support multiple currencies', group: 'Payments', enabled: false, dependencies: [] },
  { key: 'kitchen-display', name: 'Kitchen Display', description: 'KDS for order routing', group: 'Restaurant', enabled: false, dependencies: ['restaurant'] },
  { key: 'table-management', name: 'Table Management', description: 'Interactive floor plan', group: 'Restaurant', enabled: false, dependencies: ['restaurant'] },
];

let currentFeatures = [...mockFeatures];

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn((cmd: string, args?: { args: { key: string; enabled: boolean } }) => {
    if (cmd === 'list_all_features') {
      return Promise.resolve({ features: currentFeatures });
    }
    if (cmd === 'set_feature' && args?.args) {
      const { key, enabled } = args.args;
      currentFeatures = currentFeatures.map((f) =>
        f.key === key ? { ...f, enabled } : f,
      );
      return Promise.resolve({ success: true, features: currentFeatures, auto_enabled: [] });
    }
    return Promise.reject(new Error(`Unknown command: ${cmd}`));
  }),
}));

function FluentWrapper({ children }: { children: ReactNode }) {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(new FluentResource(settingsFtl));
  bundle.addResource(new FluentResource(salesFtl));
  const l10n = new ReactLocalization([bundle]);
  return (
    <LocalizationProvider l10n={l10n}>
      <ToastProvider>{children}</ToastProvider>
    </LocalizationProvider>
  );
}

describe('FeatureToggleScreen', () => {
  beforeEach(() => {
    // Reset mock state.
    currentFeatures = [...mockFeatures];
  });

  // ── Initial render ─────────────────────────────────────────────

  it('renders the feature toggle screen title and subtitle', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    expect(await screen.findByText('Feature Toggles')).toBeInTheDocument();
    // The subtitle shows count via Fluent, check it contains '2' and '7'.
    expect(screen.getByText((content) => content.includes('2') && content.includes('7') && content.includes('enabled'))).toBeInTheDocument();
  });

  it('renders feature groups', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    await screen.findByText('Feature Toggles');

    // Core group (2 features) — use getAllByText since 'Core' also appears elsewhere.
    expect(screen.getAllByText('Core').length).toBeGreaterThanOrEqual(1);
    // Payments group
    expect(screen.getAllByText('Payments').length).toBeGreaterThanOrEqual(1);
    // Restaurant appears as both group title and feature name — check via getAllByText.
    expect(screen.getAllByText('Restaurant').length).toBeGreaterThanOrEqual(1);
  });

  it('renders the Live Setup Preview', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    expect(await screen.findByText('Feature Preview')).toBeInTheDocument();
  });

  // ── Search bar ─────────────────────────────────────────────────

  it('renders the search input', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    await screen.findByText('Feature Toggles');

    expect(
      screen.getByRole('searchbox', { name: /search features/i }),
    ).toBeInTheDocument();
  });

  it('filters features by key when searching', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    await screen.findByText('Feature Toggles');

    const searchInput = screen.getByRole('searchbox', { name: /search features/i });
    await userEvent.type(searchInput, 'cash');

    // Cash payment should match (key: 'cash-payment')
    expect(screen.getByText('Cash')).toBeInTheDocument();

    // Non-matching features in the same group should be hidden.
    expect(screen.queryByText('Card')).not.toBeInTheDocument();
    expect(screen.queryByText('Multi-Currency')).not.toBeInTheDocument();

    // Non-matching groups entirely should be hidden.
    expect(screen.queryByText('Restaurant')).not.toBeInTheDocument();
  });

  it('filters features by name when searching', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    await screen.findByText('Feature Toggles');

    const searchInput = screen.getByRole('searchbox', { name: /search features/i });
    await userEvent.type(searchInput, 'Display');

    // Kitchen Display should match (name contains 'Display')
    // Use getAllByText since it may appear in feature list and group header.
    expect(screen.getAllByText('Kitchen Display').length).toBeGreaterThanOrEqual(1);

    // Restaurant group header should appear (Kitchen Display is in it).
    expect(screen.getAllByText('Restaurant').length).toBeGreaterThanOrEqual(1);

    // Others should be hidden.
    expect(screen.queryByText('Core')).not.toBeInTheDocument();
    expect(screen.queryByText('Payments')).not.toBeInTheDocument();
  });

  it('filters features by description when searching', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    await screen.findByText('Feature Toggles');

    const searchInput = screen.getByRole('searchbox', { name: /search features/i });
    await userEvent.type(searchInput, 'floor plan');

    // Table Management description contains 'floor plan'
    expect(screen.getByText('Table Management')).toBeInTheDocument();

    // Other features should be hidden.
    expect(screen.queryByText('Simple Retail')).not.toBeInTheDocument();
    expect(screen.queryByText('Cash')).not.toBeInTheDocument();
  });

  it('shows empty state when search matches nothing', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    await screen.findByText('Feature Toggles');

    const searchInput = screen.getByRole('searchbox', { name: /search features/i });
    await userEvent.type(searchInput, 'zzzzz');

    expect(
      screen.getByText('No features match your search.'),
    ).toBeInTheDocument();
  });

  it('clears search when clear button is clicked', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    await screen.findByText('Feature Toggles');

    const searchInput = screen.getByRole('searchbox', { name: /search features/i });
    await userEvent.type(searchInput, 'cash');

    // Clear button should appear.
    const clearBtn = screen.getByLabelText('Clear search');
    await userEvent.click(clearBtn);

    // After clearing, all features should be visible again.
    expect(searchInput).toHaveValue('');
    expect(screen.getByText('Simple Retail')).toBeInTheDocument();
    expect(screen.getByText('Cash')).toBeInTheDocument();
    // Kitchen Display may appear multiple times — use getAllByText.
    expect(screen.getAllByText('Kitchen Display').length).toBeGreaterThanOrEqual(1);
  });

  // ── Bulk actions ───────────────────────────────────────────────

  it('renders Enable All and Disable All buttons per group', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    await screen.findByText('Feature Toggles');

    const enableButtons = screen.getAllByText('Enable All');
    const disableButtons = screen.getAllByText('Disable All');

    // 3 groups visible: Core, Payments, Restaurant.
    expect(enableButtons).toHaveLength(3);
    expect(disableButtons).toHaveLength(3);
  });

  it('Enable All toggles all features in a group on', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    await screen.findByText('Feature Toggles');

    // Payments group has: Cash (on), Card (off), Multi-Currency (off).
    const enableBtns = screen.getAllByText('Enable All');

    // Click the Enable All for the Payments group (index 1 — Core is 0).
    await userEvent.click(enableBtns[1]!);

    // The toast should confirm (use flexible text matcher).
    const toast = await screen.findByText((content) =>
      content.includes('Payments') && content.includes('enabled'),
    );
    expect(toast).toBeInTheDocument();
  });

  it('Disable All toggles all features in a group off', async () => {
    render(<FeatureToggleScreen />, { wrapper: FluentWrapper });

    await screen.findByText('Feature Toggles');

    // Core group has: Simple Retail (on), Restaurant (off).
    const disableBtns = screen.getAllByText('Disable All');

    // Click the Disable All for the Core group (index 0).
    await userEvent.click(disableBtns[0]!);

    // The toast should confirm (use flexible text matcher).
    const toast = await screen.findByText((content) =>
      content.includes('Core') && content.includes('disabled'),
    );
    expect(toast).toBeInTheDocument();
  });
});
