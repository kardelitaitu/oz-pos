import { describe, expect, it, vi, beforeEach } from 'vitest';
import React from 'react';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import analyticsFtl from '@/locales/analytics.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

const {
  mockGetSummary,
  mockGetDaily,
} = vi.hoisted(() => ({
  mockGetSummary: vi.fn(),
  mockGetDaily: vi.fn(),
}));

vi.mock('@/api/analytics', () => ({
  getStaffAnalyticsScoped: (...args: unknown[]) => mockGetSummary(...args),
  getStaffAnalyticsDailyScoped: (...args: unknown[]) => mockGetDaily(...args),
}));

// Mock echarts-for-react — jsdom has no Canvas
vi.mock('echarts-for-react/lib/core', () => ({
  default: (props: Record<string, unknown>) => {
    const { option, notMerge, echarts, style, ...rest } = props;
    return React.createElement('div', { ...rest, 'data-testid': 'echarts-mock', style });
  },
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

// Mock echarts-for-react — jsdom has no Canvas, so return a placeholder div
vi.mock('echarts-for-react/lib/core', () => ({
  default: (props: Record<string, unknown>) => {
    const { option, notMerge, echarts, style, ...rest } = props;
    return React.createElement('div', { ...rest, 'data-testid': 'echarts-mock', style });
  },
}));

// Mock the echarts core modules used by the component
vi.mock('echarts/core', () => ({
  use: vi.fn(),
  init: vi.fn(() => ({
    setOption: vi.fn(), dispose: vi.fn(), resize: vi.fn(),
    getOption: vi.fn(() => ({})), on: vi.fn(), off: vi.fn(),
    clear: vi.fn(), isDisposed: vi.fn(() => false),
    getWidth: vi.fn(() => 0), getHeight: vi.fn(() => 0),
    getDom: vi.fn(() => document.createElement('div')),
    showLoading: vi.fn(), hideLoading: vi.fn(), getDataURL: vi.fn(() => ''),
  })),
  getInstanceByDom: vi.fn(() => null),
  dispose: vi.fn(),
  graphic: { LinearGradient: vi.fn() },
}));

vi.mock('echarts/charts', () => ({ BarChart: {}, LineChart: {}, PieChart: {}, HeatmapChart: {} }));
vi.mock('echarts/components', () => ({
  GridComponent: {}, TooltipComponent: {}, LegendComponent: {}, VisualMapComponent: {},
}));
vi.mock('echarts/renderers', () => ({ CanvasRenderer: {} }));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({ sessionToken: 'mock-session-token' }),
}));

vi.mock('@/contexts/CurrencyContext', () => ({
  useCurrency: () => ({ currency: 'IDR', setCurrency: vi.fn(), loading: false }),
}));

import AnalyticsScreen from '@/features/analytics/AnalyticsScreen';
import { registerAnalyticsFeature } from '@/features/analytics/register';
import { registerStaffFeature } from '@/features/staff/register';
import { getEnabledPages, clearPages, hasGrantedPermission } from '@/platform/ui/page-registry';
import { getNavItems, clearNavItems } from '@/platform/ui/menu-registry';

const summaryRows = [
  {
    user_id: 'user-staff-1',
    display_name: 'Ayu',
    shift_count: 3,
    closed_shift_count: 2,
    shift_sales_minor: 300000,
    sale_count: 12,
    sale_total_minor: 240000,
  },
  {
    user_id: 'user-staff-2',
    display_name: 'Budi',
    shift_count: 1,
    closed_shift_count: 1,
    shift_sales_minor: 100000,
    sale_count: 5,
    sale_total_minor: 95000,
  },
];

const dailyRows = [
  { day: '2026-07-10', sale_count: 7, sale_total_minor: 140000, shift_count: 1, shift_sales_minor: 150000 },
  { day: '2026-07-11', sale_count: 5, sale_total_minor: 100000, shift_count: 2, shift_sales_minor: 150000 },
];

describe('AnalyticsScreen', () => {
  beforeEach(() => {
    mockGetSummary.mockReset();
    mockGetDaily.mockReset();
  });

  it('renders the per-staff summary from the scoped API', async () => {
    mockGetSummary.mockResolvedValue(summaryRows);
    mockGetDaily.mockResolvedValue([]); // all staff daily calls return empty
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('Ayu').length).toBeGreaterThan(0);
    });
    expect(screen.getAllByText('Budi').length).toBeGreaterThan(0);
    expect(mockGetSummary).toHaveBeenCalledWith(
      'mock-session-token',
      expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
      expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
    );
  });

  it('loads the daily series when a staff member is selected', async () => {
    mockGetSummary.mockResolvedValue(summaryRows);
    // Daily data: Ayu has real data, Budi empty
    mockGetDaily.mockImplementation((_token: string, userId: string) => {
      if (userId === 'user-staff-1') return Promise.resolve(dailyRows);
      return Promise.resolve([]);
    });
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('Ayu').length).toBeGreaterThan(0);
    });

    // Click the Ayu table cell to select
    const ayuCells = screen.getAllByText('Ayu');
    const ayuTableRow = ayuCells.find((el) => el.tagName === 'TD');
    expect(ayuTableRow).toBeTruthy();
    await userEvent.click(ayuTableRow!);

    // After selection, the deep-dive section appears
    await waitFor(() => {
      expect(screen.getByText('Ayu — Daily Detail')).toBeTruthy();
    });
  });

  it('shows the empty state when there is no staff activity', async () => {
    mockGetSummary.mockResolvedValue([]);
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('No staff activity in this period.')).toBeTruthy();
    });
  });
});

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
    // A custom staff-role user WITH the grant sees analytics (0046 registry
    // is the source of truth, not the role name).
    const granted = getEnabledPages(undefined, 'staff', ['sales:process', 'analytics:view']);
    expect(granted.some((p) => p.route === 'analytics')).toBe(true);
    // A manager WITHOUT the grant is denied even though 'management' role
    // would admit them — the permission check overrides the role fallback.
    const denied = getEnabledPages(undefined, 'manager', ['sales:process', 'sales:view']);
    expect(denied.some((p) => p.route === 'analytics')).toBe(false);
    // Owner's global wildcard satisfies the key.
    const owner = getEnabledPages(undefined, 'owner', ['*']);
    expect(owner.some((p) => p.route === 'analytics')).toBe(true);
    // An explicit empty key list is authoritative (no implicit role grant).
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
