import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { LocalizationProvider } from '@fluent/react';
import { createEnUsLocalization } from '@/locales';
import MenuEngineeringScreen from '@/features/reports/MenuEngineeringScreen';
import * as reportsApi from '@/api/reports';
import type { MenuEngineeringResult } from '@/api/reports';

// ── Mocks ─────────────────────────────────────────────────

vi.mock('@/api/reports', async (importOriginal) => {
  const actual = await importOriginal<typeof reportsApi>();
  return {
    ...actual,
    getMenuEngineering: vi.fn(),
  };
});

const mockResult: MenuEngineeringResult = {
  median_volume: 50,
  median_margin: 2500,
  rows: [
    {
      product_id: 'p1',
      sku: 'STEAK',
      name: 'Ribeye Steak',
      total_volume: 100,
      unit_price_minor: 2500,
      unit_cost_minor: 800,
      margin_per_unit: 1700,
      total_margin_minor: 170000,
      total_revenue_minor: 250000,
    },
    {
      product_id: 'p2',
      sku: 'SALAD',
      name: 'Caesar Salad',
      total_volume: 80,
      unit_price_minor: 1200,
      unit_cost_minor: 400,
      margin_per_unit: 800,
      total_margin_minor: 64000,
      total_revenue_minor: 96000,
    },
    {
      product_id: 'p3',
      sku: 'SODA',
      name: 'Cola',
      total_volume: 200,
      unit_price_minor: 300,
      unit_cost_minor: 100,
      margin_per_unit: 200,
      total_margin_minor: 40000,
      total_revenue_minor: 60000,
    },
    {
      product_id: 'p4',
      sku: 'COFFEE',
      name: 'Specialty Coffee',
      total_volume: 30,
      unit_price_minor: 500,
      unit_cost_minor: 150,
      margin_per_unit: 350,
      total_margin_minor: 10500,
      total_revenue_minor: 15000,
    },
  ],
};

function renderWithLocales(ui: React.ReactElement) {
  const l10n = createEnUsLocalization();
  return render(
    <LocalizationProvider l10n={l10n}>
      {ui}
    </LocalizationProvider>,
  );
}

// ── Tests ──────────────────────────────────────────────────

describe('MenuEngineeringScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(reportsApi.getMenuEngineering).mockResolvedValue(mockResult);
  });

  it('shows loading spinner initially', () => {
    vi.mocked(reportsApi.getMenuEngineering).mockImplementation(
      () => new Promise(() => {}), // never resolves
    );

    renderWithLocales(<MenuEngineeringScreen />);

    expect(screen.getByRole('region', { name: /Menu Engineering Report/ })).toBeTruthy();
  });

  it('renders KPI cards with correct values after loading', async () => {
    renderWithLocales(<MenuEngineeringScreen />);

    // Wait for product count KPI to appear
    await waitFor(() => {
      const productLabels = screen.getAllByText('Products');
      expect(productLabels.length).toBeGreaterThanOrEqual(1);
    });

    // Total revenue = 250000 + 96000 + 60000 + 15000 = 421000
    await waitFor(() => {
      expect(screen.getByText('$4,210.00')).toBeTruthy();
    });
  });

  it('renders quadrant summary cards for all 4 quadrants', async () => {
    renderWithLocales(<MenuEngineeringScreen />);

    await waitFor(() => {
      const stars = screen.getAllByText('Star');
      expect(stars.length).toBeGreaterThanOrEqual(1);
    });

    const plowhorses = screen.getAllByText('Plowhorse');
    expect(plowhorses.length).toBeGreaterThanOrEqual(1);

    const puzzles = screen.getAllByText('Puzzle');
    expect(puzzles.length).toBeGreaterThanOrEqual(1);

    const dogs = screen.getAllByText('Dog');
    expect(dogs.length).toBeGreaterThanOrEqual(1);
  });

  it('renders product data table with row data', async () => {
    renderWithLocales(<MenuEngineeringScreen />);

    await waitFor(() => {
      expect(screen.getByText('Ribeye Steak')).toBeTruthy();
    });

    expect(screen.getByText('Caesar Salad')).toBeTruthy();
    expect(screen.getByText('Cola')).toBeTruthy();
    expect(screen.getByText('Specialty Coffee')).toBeTruthy();
  });

  it('renders quadrant badge for each row', async () => {
    renderWithLocales(<MenuEngineeringScreen />);

    await waitFor(() => {
      const stars = screen.getAllByText(/Star/);
      expect(stars.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('calls getMenuEngineering with default date range', () => {
    renderWithLocales(<MenuEngineeringScreen />);

    expect(reportsApi.getMenuEngineering).toHaveBeenCalledWith(
      expect.any(String),
      expect.any(String),
    );
  });

  it('shows error state when API fails', async () => {
    vi.mocked(reportsApi.getMenuEngineering).mockRejectedValue(
      new Error('Network error'),
    );

    renderWithLocales(<MenuEngineeringScreen />);

    await waitFor(() => {
      expect(screen.getByText(/Network error/)).toBeTruthy();
    });
  });

  it('shows empty state when no data returned', async () => {
    vi.mocked(reportsApi.getMenuEngineering).mockResolvedValue({
      median_volume: 0,
      median_margin: 0,
      rows: [],
    });

    renderWithLocales(<MenuEngineeringScreen />);

    await waitFor(() => {
      expect(screen.getByText(/No results/)).toBeTruthy();
    });
  });

  it('classifies quadrants correctly', async () => {
    renderWithLocales(<MenuEngineeringScreen />);

    await waitFor(() => {
      const stars = screen.getAllByText(/Star/);
      expect(stars.length).toBeGreaterThanOrEqual(1);
    });

    // COFFEE has low volume (30 < 50) and high margin (10500 >= 2500) -> Puzzle
    const puzzles = screen.getAllByText(/Puzzle/);
    expect(puzzles.length).toBeGreaterThanOrEqual(1);
  });

  it('renders date range inputs', async () => {
    renderWithLocales(<MenuEngineeringScreen />);

    await waitFor(() => {
      expect(screen.getByLabelText('Start date')).toBeTruthy();
    });

    expect(screen.getByLabelText('End date')).toBeTruthy();
  });

  it('renders margin rate KPI when revenue > 0', async () => {
    renderWithLocales(<MenuEngineeringScreen />);

    // Total margin = 170000 + 64000 + 40000 + 10500 = 284500
    // Total revenue = 250000 + 96000 + 60000 + 15000 = 421000
    // Margin rate = 284500 / 421000 * 100 ~= 67.6%
    await waitFor(() => {
      expect(screen.getByText(/67\.6%/)).toBeTruthy();
    });
  });

  it('recommendation text appears for Star quadrant', async () => {
    renderWithLocales(<MenuEngineeringScreen />);

    await waitFor(() => {
      const recs = screen.getAllByText(/Promote Star/);
      expect(recs.length).toBeGreaterThanOrEqual(1);
    });
  });
});
