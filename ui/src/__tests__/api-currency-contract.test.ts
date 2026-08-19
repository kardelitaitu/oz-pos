import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  getCurrencyInfo,
  listCurrencies,
  listCurrenciesScoped,
  getDefaultCurrency,
  setDefaultCurrency,
  listExchangeRates,
  createExchangeRate,
  deleteExchangeRate,
} from '@/api/currency';

describe('currency.ts API contract', () => {
  const TOKEN = 'tok_curr';

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('getCurrencyInfo calls correct command', async () => {
    mockInvoke.mockResolvedValue({ code: 'IDR', symbol: 'Rp', decimals: 0 });
    const result = await getCurrencyInfo('IDR');
    expect(mockInvoke).toHaveBeenCalledWith('currency_info', { code: 'IDR' });
    expect(result.code).toBe('IDR');
  });

  it('listCurrencies calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listCurrencies();
    expect(mockInvoke).toHaveBeenCalledWith('list_currencies');
  });

  it('listCurrenciesScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listCurrenciesScoped(TOKEN);
    expect(mockInvoke).toHaveBeenCalledWith('list_currencies_scoped', {
      sessionToken: TOKEN,
    });
  });

  it('getDefaultCurrency calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue('IDR');
    const result = await getDefaultCurrency();
    expect(mockInvoke).toHaveBeenCalledWith('get_default_currency');
    expect(result).toBe('IDR');
  });

  it('setDefaultCurrency calls correct command', async () => {
    const args = { code: 'USD' };
    mockInvoke.mockResolvedValue(undefined);
    await setDefaultCurrency(args);
    expect(mockInvoke).toHaveBeenCalledWith('set_default_currency', { args });
  });

  it('listExchangeRates calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listExchangeRates();
    expect(mockInvoke).toHaveBeenCalledWith('list_exchange_rates');
  });

  it('createExchangeRate calls correct command', async () => {
    const args = { from: 'USD', to: 'IDR', rate: 15700 };
    mockInvoke.mockResolvedValue({ id: 'er1', ...args });
    const result = await createExchangeRate(args);
    expect(mockInvoke).toHaveBeenCalledWith('create_exchange_rate', { args });
    expect(result.id).toBe('er1');
  });

  it('deleteExchangeRate calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteExchangeRate('er1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_exchange_rate', { id: 'er1' });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('unknown currency'));
    await expect(getCurrencyInfo('XYZ')).rejects.toThrow('unknown currency');
  });
});
