import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { act, fireEvent, screen } from '@testing-library/react';
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

import AnalyticsScreen, { nextExpandedKey, daysInCurrentMonth, monthCalendarGrid, smartScale } from '@/features/analytics/AnalyticsScreen';
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
    localStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  /**
   * Fire any pending recalculation timer instantly. Only meaningful when
   * fake timers are enabled (the tests that need it call `vi.useFakeTimers()`).
   */
  const flushRecalc = () => {
    act(() => {
      vi.advanceTimersByTime(700);
    });
  };

  it('renders the three-area layout structure', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // Area 1 — header with back button and title
    expect(screen.getByRole('button', { name: 'Back to home' })).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Analytics' })).toBeTruthy();
    expect(screen.getByText('Sales, products, and staff performance')).toBeTruthy();
  });

  it('renders the workspace selector defaulting to Retail', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const select = screen.getByRole('combobox', { name: 'Select workspace type' });
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
    const select = screen.getByRole('combobox', { name: 'Select workspace type' });
    await userEvent.selectOptions(select, 'restaurant');
    expect((select as HTMLSelectElement).value).toBe('restaurant');

    const daily = screen.getByRole('radio', { name: 'Daily' });
    expect(daily.getAttribute('aria-checked')).toBe('true');
  });

  it('back button calls goToWorkspacePicker', async () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const backBtn = screen.getByRole('button', { name: 'Back to home' });
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

  it('shows a zoom badge that resets zoom on click', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const grid = document.querySelector('.analytics-grid') as HTMLElement;
    const badge = screen.getByRole('button', { name: 'Reset zoom to 100%' });
    expect(badge.textContent).toBe('100%');

    fireEvent.click(screen.getByRole('button', { name: 'Zoom in' }));
    expect(badge.textContent).toBe('120%');
    expect(grid.style.zoom).toBe('1.2');

    fireEvent.click(badge);
    expect(badge.textContent).toBe('100%');
    expect(grid.style.zoom).toBe('1');
  });

  it('disables zoom buttons at their limits', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const zoomOut = screen.getByRole('button', { name: 'Zoom out' }) as HTMLButtonElement;
    const zoomIn = screen.getByRole('button', { name: 'Zoom in' }) as HTMLButtonElement;

    // At 100%, zoom out is enabled, zoom in is not yet at the max
    expect(zoomOut.disabled).toBe(false);
    expect(zoomIn.disabled).toBe(false);

    // Zoom out to the floor (0.6) — button becomes disabled
    for (let i = 0; i < 10; i++) fireEvent.click(zoomOut);
    expect(zoomOut.disabled).toBe(true);

    // Zoom in back to the ceiling (1.6) — button becomes disabled
    for (let i = 0; i < 10; i++) fireEvent.click(screen.getByRole('button', { name: 'Zoom in' }));
    expect(zoomIn.disabled).toBe(true);
  });

  it('shows the view status bar with card count and workspace', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    expect(screen.getByText('13 cards')).toBeTruthy();
    // Status shows workspace · granularity (scoped to the status bar)
    const status = document.querySelector('.analytics-status');
    expect(status?.textContent).toContain('Retail');
    expect(status?.textContent).toContain('Daily');
  });

  it('opens the command palette with Ctrl+K and runs a filtered action', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(screen.getByRole('dialog', { name: 'Quick actions' })).toBeTruthy();

    // Filter to granularity items
    fireEvent.change(screen.getByRole('textbox', { name: 'Search actions…' }), { target: { value: 'month' } });
    fireEvent.keyDown(window, { key: 'Enter' });

    // Monthly became active and the palette closed
    expect(screen.getByRole('radio', { name: 'Monthly' }).getAttribute('aria-checked')).toBe('true');
    expect(screen.queryByRole('dialog', { name: 'Quick actions' })).toBeNull();
  });

  it('switches workspace from the command palette', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    fireEvent.change(screen.getByRole('textbox', { name: 'Search actions…' }), { target: { value: 'restaurant' } });
    fireEvent.keyDown(window, { key: 'Enter' });

    const select = screen.getByRole('combobox', { name: 'Select workspace type' }) as HTMLSelectElement;
    expect(select.value).toBe('restaurant');
    expect(screen.queryByRole('dialog', { name: 'Quick actions' })).toBeNull();
  });

  it('closes the palette with Escape and keeps shortcuts dormant while open', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(screen.getByRole('dialog', { name: 'Quick actions' })).toBeTruthy();

    // Shortcuts are ignored while the palette is open
    fireEvent.keyDown(window, { key: '3' });
    expect(screen.getByRole('radio', { name: 'Monthly' }).getAttribute('aria-checked')).toBe('false');

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Quick actions' })).toBeNull();
  });

  it('handles keyboard shortcuts for granularity and escape', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // '3' selects Monthly
    fireEvent.keyDown(window, { key: '3' });
    expect(screen.getByRole('radio', { name: 'Monthly' }).getAttribute('aria-checked')).toBe('true');

    // Escape closes the shortcuts popover if open
    fireEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    expect(screen.getByRole('dialog')).toBeTruthy();
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('ignores keyboard shortcuts while typing in an input', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    fireEvent.click(screen.getByRole('radio', { name: 'Custom' }));
    const from = screen.getByLabelText('From') as HTMLInputElement;

    // Typing digits inside the date input must not switch granularity
    fireEvent.keyDown(from, { key: '2' });
    expect(screen.getByRole('radio', { name: 'Weekly' }).getAttribute('aria-checked')).toBe('false');
  });

  it('opens the shortcuts help popover and closes it', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    expect(screen.queryByRole('dialog')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    expect(screen.getByRole('dialog')).toBeTruthy();
    expect(screen.getByText(/Time range/)).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('shows a reset-layout button after reordering and restores defaults', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    expect(screen.queryByRole('button', { name: 'Reset layout' })).toBeNull();

    // Reorder: drag Heat Map onto Staff Performance
    const cards = () => [...document.querySelectorAll('.analytics-card')];
    const heat = cards()[0]!;
    const staff = cards().find((c) => c.querySelector('.analytics-card-title')?.textContent === 'Staff Performance')!;
    fireEvent.dragStart(heat);
    fireEvent.dragOver(staff);
    fireEvent.drop(staff);
    fireEvent.dragEnd(heat);

    expect(screen.getByRole('button', { name: 'Reset layout' })).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'Reset layout' }));
    expect(screen.queryByRole('button', { name: 'Reset layout' })).toBeNull();
    expect(cards()[0]!.querySelector('.analytics-card-title')?.textContent).toBe('Heat Map');
  });

  it('reorders cards by drag and persists the layout', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // First card is the wide Heat Map (spans 2 columns)
    const cards = () => [...document.querySelectorAll('.analytics-card')];
    const titles = () => cards().map((c) => c.querySelector('.analytics-card-title')?.textContent);
    expect(titles()[0]).toBe('Heat Map');

    // Drag Revenue Overview onto Staff Performance's slot
    const heat = cards()[0]!;
    const staff = cards().find((c) => c.querySelector('.analytics-card-title')?.textContent === 'Staff Performance')!;
    fireEvent.dragStart(heat);
    fireEvent.dragOver(staff);
    fireEvent.drop(staff);
    fireEvent.dragEnd(heat);

    // Order changed: Staff Performance moved before Heat Map
    expect(titles().indexOf('Staff Performance')).toBeLessThan(titles().indexOf('Heat Map'));

    // Layout persisted to localStorage
    const saved = JSON.parse(localStorage.getItem('oz-analytics-card-order-retail')!);
    expect(saved.indexOf('staff-shared')).toBeLessThan(saved.indexOf('heatmap-shared'));
  });

  it('applies quick range presets to the custom date pickers', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    fireEvent.click(screen.getByRole('radio', { name: 'Custom' }));
    const from = screen.getByLabelText('From') as HTMLInputElement;
    const to = screen.getByLabelText('To') as HTMLInputElement;

    fireEvent.click(screen.getByRole('button', { name: 'Last 7 days' }));

    const expectedFrom = new Date();
    expectedFrom.setDate(expectedFrom.getDate() - 6);
    expect(from.value).toBe(expectedFrom.toISOString().slice(0, 10));
    expect(to.value).toBe(new Date().toISOString().slice(0, 10));
  });

  it('collapses all card bodies with the toggle and restores them', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    expect(document.querySelectorAll('.analytics-card--collapsed').length).toBe(0);

    fireEvent.click(screen.getByRole('button', { name: 'Collapse all cards' }));
    expect(document.querySelectorAll('.analytics-card--collapsed').length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole('button', { name: 'Expand all cards' }));
    expect(document.querySelectorAll('.analytics-card--collapsed').length).toBe(0);
  });

  it('shows the custom range in the status bar', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    fireEvent.click(screen.getByRole('radio', { name: 'Custom' }));

    const from = screen.getByLabelText('From') as HTMLInputElement;
    const to = screen.getByLabelText('To') as HTMLInputElement;
    expect(from.value).toBeTruthy();
    expect(screen.getByText(`${from.value} – ${to.value}`)).toBeTruthy();
  });

  it('shows the scroll-to-top button after scrolling the main area', () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const main = document.querySelector('.analytics-main') as HTMLElement;
    expect(screen.queryByRole('button', { name: 'Back to top' })).toBeNull();

    // Scroll the main area down past the threshold
    Object.defineProperty(main, 'scrollTop', { value: 400, configurable: true });
    fireEvent.scroll(main);
    expect(screen.getByRole('button', { name: 'Back to top' })).toBeTruthy();

    // Scroll back up — button hides
    Object.defineProperty(main, 'scrollTop', { value: 0, configurable: true });
    fireEvent.scroll(main);
    expect(screen.queryByRole('button', { name: 'Back to top' })).toBeNull();
  });

  it('renders a smart heatmap that changes buckets with granularity', () => {
    vi.useFakeTimers();
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    const heatmap = () => document.querySelector('.analytics-heatmap');
    const cellCount = () => heatmap()?.querySelectorAll('.analytics-heat-cell').length ?? 0;

    // Skip the initial recalculation skeleton instantly
    flushRecalc();

    // Default: daily → 7 weekday buckets
    expect(cellCount()).toBe(7);

    // Weekly → 7 day rows × 24 hour columns
    fireEvent.click(screen.getByRole('radio', { name: 'Weekly' }));
    flushRecalc();
    expect(cellCount()).toBe(168);
    expect(heatmap()?.querySelectorAll('.analytics-weekly-row').length).toBe(8); // header + 7 days

    // Monthly → real calendar: day 1 starts on its actual weekday,
    // empty cells pad the first/last rows to complete weeks
    fireEvent.click(screen.getByRole('radio', { name: 'Monthly' }));
    flushRecalc();
    const filled = heatmap()?.querySelectorAll('.analytics-heat-cell[data-intensity]').length ?? 0;
    const total = cellCount();
    expect(filled).toBe(daysInCurrentMonth());
    expect(filled).toBeGreaterThanOrEqual(28);
    expect(filled).toBeLessThanOrEqual(31);
    expect(total % 7).toBe(0); // complete calendar weeks

    // Yearly → 12 month columns × 4 week rows = 48 cells
    fireEvent.click(screen.getByRole('radio', { name: 'Yearly' }));
    flushRecalc();
    expect(cellCount()).toBe(48);
    expect(heatmap()?.querySelectorAll('.analytics-heat-column').length).toBe(12);
  });

  it('expands a card to fill the main area and restores it', () => {
    vi.useFakeTimers();
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // Skip the initial recalculation skeleton instantly
    flushRecalc();

    // All cards visible before expanding
    expect(screen.getByText('Revenue Overview')).toBeTruthy();

    // Expand the Revenue card (first expand button)
    const expandButtons = screen.getAllByRole('button', { name: 'Expand card' });
    expect(expandButtons.length).toBeGreaterThan(1);
    fireEvent.click(expandButtons[1]!);

    // Only the expanded card remains visible, with a restore button
    expect(screen.getByRole('button', { name: 'Restore card' })).toBeTruthy();
    expect(screen.getByText('Revenue Overview')).toBeTruthy();

    // Restore brings the grid back
    fireEvent.click(screen.getByRole('button', { name: 'Restore card' }));
    expect(screen.getAllByRole('button', { name: 'Expand card' }).length).toBeGreaterThan(0);
  });

  it('expands exactly one card — expanding another while one is open is ignored', () => {
    vi.useFakeTimers();
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // Skip the initial recalculation skeleton instantly
    flushRecalc();

    // Expand the first card
    const expandButtons = screen.getAllByRole('button', { name: 'Expand card' });
    const first = expandButtons[0]!;
    fireEvent.click(first);

    // Only the expanded card is rendered — exactly one restore action,
    // and no other expand buttons exist to open a different card
    expect(screen.getByRole('button', { name: 'Restore card' })).toBeTruthy();
    expect(screen.queryAllByRole('button', { name: 'Expand card' }).length).toBe(0);

    // Restore
    fireEvent.click(screen.getByRole('button', { name: 'Restore card' }));
    expect(screen.getAllByRole('button', { name: 'Expand card' }).length).toBeGreaterThan(0);
  });

  it('smart-expands every card — each one fills the grid and restores', () => {
    vi.useFakeTimers();
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // Skip the initial recalculation skeleton instantly
    flushRecalc();

    const cardCount = screen.getAllByRole('button', { name: 'Expand card' }).length;
    expect(cardCount).toBeGreaterThan(1);

    // Loop over every card: expand it, verify it is the only expanded card,
    // then restore it before moving to the next one
    for (let i = 0; i < cardCount; i++) {
      fireEvent.click(screen.getAllByRole('button', { name: 'Expand card' })[i]!);

      expect(document.querySelectorAll('.analytics-card--expanded').length).toBe(1);
      // The expanded card always carries the scaled content wrapper
      const content = document.querySelector('.analytics-card--expanded .analytics-card-content');
      expect(content).toBeTruthy();

      fireEvent.click(screen.getByRole('button', { name: 'Restore card' }));
      expect(screen.getAllByRole('button', { name: 'Expand card' }).length).toBe(cardCount);
    }

    // Nothing stays expanded after the loop
    expect(document.querySelectorAll('.analytics-card--expanded').length).toBe(0);
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
    expect(screen.getByText('Heat Map')).toBeTruthy();
  });

  it('switches card titles when workspace changes to restaurant', async () => {
    renderWithFluentSync(<AnalyticsScreen />, analyticsFtl, sharedFtl);

    // Retail defaults
    expect(screen.getByText('Top Products')).toBeTruthy();
    expect(screen.getByText('Sales by Category')).toBeTruthy();

    // Switch to restaurant
    const select = screen.getByRole('combobox', { name: 'Select workspace type' });
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

describe('nextExpandedKey — single-expansion invariant', () => {
  it('expands a card when nothing is open', () => {
    expect(nextExpandedKey(null, 'revenue-shared')).toBe('revenue-shared');
  });

  it('restores the expanded card when clicked again', () => {
    expect(nextExpandedKey('revenue-shared', 'revenue-shared')).toBe(null);
  });

  it('ignores expanding a different card while one is open', () => {
    expect(nextExpandedKey('revenue-shared', 'heatmap-shared')).toBe('revenue-shared');
  });

  it('never yields a different card than the one currently expanded', () => {
    for (const current of ['a', 'b', 'c']) {
      for (const cid of ['a', 'b', 'c', 'd']) {
        const next = nextExpandedKey(current, cid);
        expect(next === null || next === current).toBe(true);
      }
    }
  });
});

describe('monthCalendarGrid — monthly heatmap calendar layout', () => {
  it('day 1 does not always start on the first cell', () => {
    const grid = monthCalendarGrid();
    expect(grid.leading).toBeGreaterThanOrEqual(0);
    expect(grid.leading).toBeLessThanOrEqual(6);
  });

  it('has 28–31 day cells', () => {
    const grid = monthCalendarGrid();
    expect(grid.days).toBeGreaterThanOrEqual(28);
    expect(grid.days).toBeLessThanOrEqual(31);
  });

  it('pads with leading/trailing empties so weeks are complete', () => {
    const grid = monthCalendarGrid();
    expect((grid.leading + grid.days + grid.trailing) % 7).toBe(0);
    expect(grid.trailing).toBeGreaterThanOrEqual(0);
    expect(grid.trailing).toBeLessThan(7);
  });
});

describe('smartScale — expanded card fills the available area', () => {
  it('returns 1 when layout has not been measured', () => {
    expect(smartScale({ w: 800, h: 600 }, { w: 0, h: 0 })).toBe(1);
    expect(smartScale({ w: 0, h: 0 }, { w: 200, h: 150 })).toBe(1);
  });

  it('fills both axes when content is smaller than the area', () => {
    expect(smartScale({ w: 800, h: 600 }, { w: 200, h: 150 })).toBe(4);
  });

  it('is constrained by the narrower axis', () => {
    // Width allows 2x, height allows 6x → width wins
    expect(smartScale({ w: 800, h: 600 }, { w: 400, h: 100 })).toBe(2);
    // Height allows 1.5x, width allows 8x → height wins
    expect(smartScale({ w: 800, h: 600 }, { w: 100, h: 400 })).toBe(1.5);
  });

  it('caps the scale at the max to avoid absurd blow-ups', () => {
    expect(smartScale({ w: 1000, h: 1000 }, { w: 100, h: 100 }, 4)).toBe(4);
    expect(smartScale({ w: 1000, h: 1000 }, { w: 100, h: 100 }, 2)).toBe(2);
  });

  it('never shrinks content below 1x', () => {
    expect(smartScale({ w: 400, h: 300 }, { w: 2000, h: 1500 })).toBe(1);
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
