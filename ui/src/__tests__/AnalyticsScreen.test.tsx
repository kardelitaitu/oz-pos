import { describe, expect, it, vi, beforeEach } from 'vitest';
import React from 'react';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import analyticsFtl from '@/locales/analytics.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

// --- mocks ---

vi.mock('echarts-for-react/lib/core', () => ({
  default: (props: Record<string, unknown>) =>
    React.createElement('div', { ...props, 'data-testid': 'echarts-mock' }),
}));

vi.mock('echarts/core', () => ({
  use: vi.fn(),
  init: vi.fn(() => ({ setOption: vi.fn(), dispose: vi.fn(), resize: vi.fn(), getOption: vi.fn(() => ({})), on: vi.fn(), off: vi.fn(), clear: vi.fn(), isDisposed: vi.fn(() => false), getWidth: vi.fn(() => 0), getHeight: vi.fn(() => 0), getDom: vi.fn(() => document.createElement('div')), showLoading: vi.fn(), hideLoading: vi.fn(), getDataURL: vi.fn(() => '') })),
  getInstanceByDom: vi.fn(() => null),
  dispose: vi.fn(),
  graphic: { LinearGradient: vi.fn() },
}));

vi.mock('echarts/charts', () => ({ BarChart: {}, LineChart: {}, PieChart: {}, HeatmapChart: {} }));
vi.mock('echarts/components', () => ({ GridComponent: {}, TooltipComponent: {}, LegendComponent: {}, VisualMapComponent: {} }));
vi.mock('echarts/renderers', () => ({ CanvasRenderer: {} }));

const mockGoToPicker = vi.fn();
vi.mock('@/hooks/useWorkspaceNav', () => ({
  useWorkspaceNav: () => ({ goToWorkspacePicker: mockGoToPicker }),
}));

import AnalyticsScreen from '@/features/analytics/AnalyticsScreen';
import { registerAnalyticsFeature } from '@/features/analytics/register';
import { registerStaffFeature } from '@/features/staff/register';
import { getEnabledPages, clearPages, hasGrantedPermission } from '@/platform/ui/page-registry';
import { getNavItems, clearNavItems } from '@/platform/ui/menu-registry';

// ────────────────────────────────────────────────────────────────────
// Layout shell tests
// ────────────────────────────────────────────────────────────────────

describe('AnalyticsScreen layout shell', () => {
  beforeEach(() => {
    mockGoToPicker.mockReset();
  });

  it('renders the three-area layout structure', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // Area 1 — header with back button and title
    expect(screen.getByRole('button', { name: '.aria-label = Back to home' })).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Analytics' })).toBeTruthy();
    expect(screen.getByText('Sales, products, and staff performance')).toBeTruthy();
  });

  it('renders the workspace selector defaulting to Retail', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const select = screen.getByRole('combobox', { name: '.aria-label = Select workspace type' });
    expect(select).toBeTruthy();
    expect((select as HTMLSelectElement).value).toBe('retail');
  });

  it('renders all five granularity buttons with daily active by default', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const daily = screen.getByRole('radio', { name: 'Daily' });
    const weekly = screen.getByRole('radio', { name: 'Weekly' });
    const monthly = screen.getByRole('radio', { name: 'Monthly' });
    const yearly = screen.getByRole('radio', { name: 'Yearly' });
    const custom = screen.getByRole('radio', { name: 'Custom' });

    expect(daily).toBeTruthy();
    expect(weekly).toBeTruthy();
    expect(monthly).toBeTruthy();
    expect(yearly).toBeTruthy();
    expect(custom).toBeTruthy();
    expect(daily.getAttribute('aria-checked')).toBe('true');
  });

  it('activates a different granularity on click', async () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const weekly = screen.getByRole('radio', { name: 'Weekly' });
    await userEvent.click(weekly);
    expect(weekly.getAttribute('aria-checked')).toBe('true');

    // Daily should no longer be active
    const daily = screen.getByRole('radio', { name: 'Daily' });
    expect(daily.getAttribute('aria-checked')).toBe('false');
  });

  it('switches workspace and resets granularity to daily', async () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // Click weekly first
    const weekly = screen.getByRole('radio', { name: 'Weekly' });
    await userEvent.click(weekly);
    expect(weekly.getAttribute('aria-checked')).toBe('true');

    // Switch to restaurant — should reset to daily
    const select = screen.getByRole('combobox', { name: '.aria-label = Select workspace type' });
    await userEvent.selectOptions(select, 'restaurant');
    expect((select as HTMLSelectElement).value).toBe('restaurant');

    const daily = screen.getByRole('radio', { name: 'Daily' });
    expect(daily.getAttribute('aria-checked')).toBe('true');
  });

  it('back button calls goToWorkspacePicker', async () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const backBtn = screen.getByRole('button', { name: '.aria-label = Back to home' });
    await userEvent.click(backBtn);
    expect(mockGoToPicker).toHaveBeenCalledTimes(1);
  });

  it('shows the custom date range popup when Custom granularity is selected', async () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // Before clicking Custom, the date pickers should not be visible
    expect(screen.queryByLabelText('From')).toBeNull();
    expect(screen.queryByLabelText('To')).toBeNull();

    // Click Custom
    const custom = screen.getByRole('radio', { name: 'Custom' });
    await userEvent.click(custom);

    // Now the date pickers appear
    expect(screen.getByLabelText('From')).toBeTruthy();
    expect(screen.getByLabelText('To')).toBeTruthy();
  });

  it('renders refresh, zoom out, and zoom in action buttons', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    expect(screen.getByRole('button', { name: 'Refresh data' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Zoom out' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Zoom in' })).toBeTruthy();
  });

  it('zooms the main grid in and out without affecting title or menu', async () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const grid = document.querySelector('.analytics-grid') as HTMLElement;
    expect(grid.style.zoom).toBe('1');

    const zoomIn = screen.getByRole('button', { name: 'Zoom in' });
    await userEvent.click(zoomIn);
    expect(grid.style.zoom).toBe('1.2');

    const zoomOut = screen.getByRole('button', { name: 'Zoom out' });
    await userEvent.click(zoomOut);
    await userEvent.click(zoomOut);
    expect(grid.style.zoom).toBe('0.8');
  });

  it('renders a smart heatmap that changes buckets with granularity', async () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const heatmap = () => document.querySelector('.analytics-heatmap');
    const cellCount = () => heatmap()?.querySelectorAll('.analytics-heat-cell').length ?? 0;

    // Wait for the initial recalculation skeleton to clear
    await new Promise((r) => setTimeout(r, 650));

    // Default: daily → 7 weekday buckets
    expect(cellCount()).toBe(7);

    // Weekly → 4 week buckets
    await userEvent.click(screen.getByRole('radio', { name: 'Weekly' }));
    await new Promise((r) => setTimeout(r, 650));
    expect(cellCount()).toBe(4);

    // Monthly → 12 month buckets
    await userEvent.click(screen.getByRole('radio', { name: 'Monthly' }));
    await new Promise((r) => setTimeout(r, 650));
    expect(cellCount()).toBe(12);

    // Yearly → 4 quarter buckets
    await userEvent.click(screen.getByRole('radio', { name: 'Yearly' }));
    await new Promise((r) => setTimeout(r, 650));
    expect(cellCount()).toBe(4);
  });

  it('renders the analytics card grid with workspace-appropriate titles', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // Shared cards appear for retail
    expect(screen.getByText('Revenue Overview')).toBeTruthy();
    expect(screen.getByText('Staff Performance')).toBeTruthy();
    // Retail-specific
    expect(screen.getByText('Top Products')).toBeTruthy();
    expect(screen.getByText('Sales by Category')).toBeTruthy();
    // Full-width
    expect(screen.getByText('Peak Hours')).toBeTruthy();
  });

  it('switches card titles when workspace changes to restaurant', async () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // Retail defaults
    expect(screen.getByText('Top Products')).toBeTruthy();
    expect(screen.getByText('Sales by Category')).toBeTruthy();

    // Switch to restaurant
    const select = screen.getByRole('combobox', { name: '.aria-label = Select workspace type' });
    await userEvent.selectOptions(select, 'restaurant');

    // Restaurant-specific cards replace retail ones
    expect(screen.getByText('Top Menu Items')).toBeTruthy();
    expect(screen.getByText('Table Turnover')).toBeTruthy();
  });
});

// ────────────────────────────────────────────────────────────────────
// Role gate tests (unchanged — registration, not component)
// ────────────────────────────────────────────────────────────────────

describe('analytics page role gate (0046 taxonomy)', () => {
  beforeEach(() => {
    clearPages();
    clearNavItems();
    registerAnalyticsFeature();
    registerStaffFeature();
  });

  it('is visible to owner, admin, and manager', () => {
    for (const role of ['owner', 'admin', 'manager', 'role-admin', 'role-manager']) {
      const pages = getEnabledPages(undefined, role);
      expect(pages.some((p) => p.route === 'analytics')).toBe(true);
      const nav = getNavItems(undefined, role);
      expect(nav.some((n) => n.route === 'analytics')).toBe(true);
    }
  });

  it('is hidden from staff and auditor (no analytics:view grant)', () => {
    for (const role of ['staff', 'role-staff', 'auditor', 'role-auditor']) {
      const pages = getEnabledPages(undefined, role);
      expect(pages.some((p) => p.route === 'analytics')).toBe(false);
      const nav = getNavItems(undefined, role);
      expect(nav.some((n) => n.route === 'analytics')).toBe(false);
    }
  });

  it('manager-gated pages stay visible to admin (admin is manager-level)', () => {
    const pages = getEnabledPages(undefined, 'admin');
    expect(pages.some((p) => p.route === 'staff')).toBe(true);
  });

  it('permission gate is authoritative when the session carries granted keys', () => {
    const granted = getEnabledPages(undefined, 'staff', ['sales:process', 'analytics:view']);
    expect(granted.some((p) => p.route === 'analytics')).toBe(true);
    const denied = getEnabledPages(undefined, 'manager', ['sales:process', 'sales:view']);
    expect(denied.some((p) => p.route === 'analytics')).toBe(false);
    const owner = getEnabledPages(undefined, 'owner', ['*']);
    expect(owner.some((p) => p.route === 'analytics')).toBe(true);
    const empty = getEnabledPages(undefined, 'owner', []);
    expect(empty.some((p) => p.route === 'analytics')).toBe(false);
  });
});

describe('hasGrantedPermission (backend has_permission mirror)', () => {
  it('matches exact keys, the global wildcard, and domain wildcards', () => {
    expect(hasGrantedPermission(['analytics:view'], 'analytics:view')).toBe(true);
    expect(hasGrantedPermission(['*'], 'analytics:view')).toBe(true);
    expect(hasGrantedPermission(['analytics:*'], 'analytics:view')).toBe(true);
    expect(hasGrantedPermission(['sales:*'], 'analytics:view')).toBe(false);
    expect(hasGrantedPermission(['analytics:view'], 'sales:create')).toBe(false);
    expect(hasGrantedPermission(undefined, 'analytics:view')).toBe(false);
  });
});
