import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import currencyFtl from '@/locales/currency.ftl?raw';

import ExchangeRateScreen from '@/features/currency/ExchangeRateScreen';

// ── Mocks ────────────────────────────────────────────────────────────

const mockListExchangeRates = vi.fn();
const mockListCurrencies = vi.fn();
const mockCreateExchangeRate = vi.fn();
const mockDeleteExchangeRate = vi.fn();

vi.mock('@/api/currency', () => ({
  listExchangeRates: (...args: unknown[]) => mockListExchangeRates(...args),
  listCurrencies: (...args: unknown[]) => mockListCurrencies(...args),
  createExchangeRate: (...args: unknown[]) => mockCreateExchangeRate(...args),
  deleteExchangeRate: (...args: unknown[]) => mockDeleteExchangeRate(...args),
}));

// ── Helpers ───────────────────────────────────────────────────────────

function makeRate(overrides: Record<string, unknown> = {}) {
  return {
    id: 'rate-1',
    from_currency: 'USD',
    to_currency: 'IDR',
    rate: 16000,
    source: 'manual',
    effective_date: '2025-07-07',
    created_at: '2025-07-07T00:00:00.000Z',
    ...overrides,
  };
}

function makeCurrency(code: string, name: string) {
  return { code, name, minor_exponent: 2, symbol: code };
}

const wrap = (children: React.ReactNode) =>
  withFluent(children, currencyFtl);

function renderScreen() {
  return render(wrap(<ExchangeRateScreen />));
}

// ── Tests ─────────────────────────────────────────────────────────────

describe('ExchangeRateScreen', () => {
  beforeEach(() => {
    mockListExchangeRates.mockReset();
    mockListCurrencies.mockReset();
    mockCreateExchangeRate.mockReset();
    mockDeleteExchangeRate.mockReset();
  });

  it('renders the title', async () => {
    mockListExchangeRates.mockResolvedValue([makeRate()]);
    mockListCurrencies.mockResolvedValue([makeCurrency('USD', 'US Dollar')]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Exchange Rates')).toBeTruthy();
    });
  });

  it('renders the Add button', async () => {
    mockListExchangeRates.mockResolvedValue([makeRate()]);
    mockListCurrencies.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      const addBtns = screen.getAllByText('Add');
      expect(addBtns.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('shows loading state initially', () => {
    mockListExchangeRates.mockImplementation(() => new Promise(() => {}));
    mockListCurrencies.mockImplementation(() => new Promise(() => {}));
    renderScreen();

    expect(screen.getByText('Loading exchange rates…')).toBeTruthy();
  });

  it('shows error state with retry', async () => {
    mockListExchangeRates.mockRejectedValue(new Error('Failed'));
    mockListCurrencies.mockRejectedValue(new Error('Failed'));
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Retry')).toBeTruthy();
    });
  });

  it('shows empty state when no rates exist', async () => {
    mockListExchangeRates.mockResolvedValue([]);
    mockListCurrencies.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('No exchange rates configured')).toBeTruthy();
    });
  });

  it('renders a table with rate rows', async () => {
    mockListExchangeRates.mockResolvedValue([
      makeRate({ id: 'r1', from_currency: 'USD', to_currency: 'IDR', rate: 16000 }),
      makeRate({ id: 'r2', from_currency: 'EUR', to_currency: 'IDR', rate: 17000 }),
    ]);
    mockListCurrencies.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('From')).toBeTruthy();
      expect(screen.getByText('To')).toBeTruthy();
      expect(screen.getByText('Rate')).toBeTruthy();
    });

    // IDR appears twice (as to_currency for both rows), use getAllByText
    const idrEls = screen.getAllByText('IDR');
    expect(idrEls.length).toBe(2);
    expect(screen.getByText('USD')).toBeTruthy();
    expect(screen.getByText('EUR')).toBeTruthy();
    expect(screen.getByText('16000')).toBeTruthy();
  });

  it('shows manual source label', async () => {
    mockListExchangeRates.mockResolvedValue([makeRate({ source: 'manual' })]);
    mockListCurrencies.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('manual')).toBeTruthy();
    });
  });

  it('each row has a Delete button', async () => {
    mockListExchangeRates.mockResolvedValue([makeRate()]);
    mockListCurrencies.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      const deleteBtns = screen.getAllByText('Delete');
      expect(deleteBtns.length).toBe(1);
    });
  });

  it('opens the add modal when Add is clicked', async () => {
    mockListExchangeRates.mockResolvedValue([]);
    mockListCurrencies.mockResolvedValue([
      makeCurrency('USD', 'US Dollar'),
      makeCurrency('IDR', 'Indonesian Rupiah'),
    ]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('No exchange rates configured')).toBeTruthy();
    });

    const user = userEvent.setup();
    const addBtns = screen.getAllByText('Add');
    await user.click(addBtns[0]!);

    await waitFor(() => {
      expect(screen.getByText('Add Exchange Rate')).toBeTruthy();
    });
  });

  it('closes the add modal with Cancel', async () => {
    mockListExchangeRates.mockResolvedValue([]);
    mockListCurrencies.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('No exchange rates configured')).toBeTruthy();
    });

    const user = userEvent.setup();
    await user.click(screen.getAllByText('Add')[0]!);

    await waitFor(() => {
      expect(screen.getByText('Cancel')).toBeTruthy();
    });

    await user.click(screen.getByText('Cancel'));

    await waitFor(() => {
      expect(screen.queryByText('Add Exchange Rate')).toBeNull();
    });
  });

  it('saves a new exchange rate via the modal', async () => {
    mockListExchangeRates.mockResolvedValue([]);
    mockListCurrencies.mockResolvedValue([
      makeCurrency('USD', 'US Dollar'),
      makeCurrency('IDR', 'Indonesian Rupiah'),
    ]);
    mockCreateExchangeRate.mockResolvedValue(makeRate());

    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('No exchange rates configured')).toBeTruthy();
    });

    const user = userEvent.setup();
    await user.click(screen.getAllByText('Add')[0]!);

    await waitFor(() => {
      expect(screen.getByText('Add Exchange Rate')).toBeTruthy();
    });

    // Fill the rate field
    const rateInput = document.querySelector('#er-field-rate') as HTMLInputElement;
    await user.type(rateInput, '16000');

    // Select From currency
    const fromSelect = document.querySelector('#er-field-from') as HTMLSelectElement;
    await user.selectOptions(fromSelect, 'USD');

    // Select To currency
    const toSelect = document.querySelector('#er-field-to') as HTMLSelectElement;
    await user.selectOptions(toSelect, 'IDR');

    await user.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockCreateExchangeRate).toHaveBeenCalled();
    });
  });

  it('deletes a rate on Delete click', async () => {
    mockListExchangeRates.mockResolvedValueOnce([makeRate()]);
    mockListExchangeRates.mockResolvedValueOnce([]);
    mockListCurrencies.mockResolvedValue([]);
    mockDeleteExchangeRate.mockResolvedValue(undefined);

    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Delete')).toBeTruthy();
    });

    const user = userEvent.setup();
    await user.click(screen.getByText('Delete'));

    await waitFor(() => {
      expect(mockDeleteExchangeRate).toHaveBeenCalledWith('rate-1');
    });
  });
});
