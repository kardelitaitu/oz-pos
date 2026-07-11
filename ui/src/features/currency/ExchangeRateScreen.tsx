import { useState, useCallback, useEffect } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listExchangeRates,
  createExchangeRate,
  deleteExchangeRate,
  listCurrencies,
  type ExchangeRateDto,
  type CurrencyDto,
} from '@/api/currency';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './ExchangeRateScreen.css';

function todayStr(): string {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${d.getFullYear()}-${m}-${day}`;
}

interface FormData {
  fromCurrency: string;
  toCurrency: string;
  rate: string;
  source: string;
  effectiveDate: string;
}

const EMPTY_FORM: FormData = {
  fromCurrency: '',
  toCurrency: '',
  rate: '',
  source: '',
  effectiveDate: todayStr(),
};

/** Exchange rate management screen — create and delete currency exchange rates for multi-currency support. */
export default function ExchangeRateScreen() {
  const { l10n } = useLocalization();
  const [rates, setRates] = useState<ExchangeRateDto[]>([]);
  const [currencies, setCurrencies] = useState<CurrencyDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showModal, setShowModal] = useState(false);
  const [form, setForm] = useState<FormData>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [items, currs] = await Promise.all([
        listExchangeRates(),
        listCurrencies(),
      ]);
      setRates(items);
      setCurrencies(currs);
    } catch {
      setError(l10n.getString('currency-load-error'));
    } finally {
      setLoading(false);
    }
  }, [l10n]);

  useEffect(() => { load(); }, [load]);

  const openCreate = useCallback(() => {
    setForm(EMPTY_FORM);
    setShowModal(true);
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const rate = parseFloat(form.rate);
      if (Number.isNaN(rate) || rate <= 0) return;

      const args: Parameters<typeof createExchangeRate>[0] = {
        fromCurrency: form.fromCurrency,
        toCurrency: form.toCurrency,
        rate,
      };
      if (form.source) args.source = form.source;
      if (form.effectiveDate) args.effectiveDate = form.effectiveDate;
      await createExchangeRate(args);
      setShowModal(false);
      await load();
    } catch {
      // Error handling.
    } finally {
      setSaving(false);
    }
  }, [form, load]);

  const confirmDelete = useCallback(async (id: string) => {
    setDeleting(id);
    try {
      await deleteExchangeRate(id);
      setDeleting(null);
      await load();
    } catch {
      setDeleting(null);
    }
  }, [load]);

  const currencyOptions = currencies.map((c) => (
    <option key={c.code} value={c.code}>
      {c.code} — {c.name}
    </option>
  ));

  const formValid =
    form.fromCurrency &&
    form.toCurrency &&
    form.fromCurrency !== form.toCurrency &&
    form.rate.trim() &&
    !Number.isNaN(parseFloat(form.rate)) &&
    parseFloat(form.rate) > 0;

  return (
    <div className="exchange-rate-config">
      <div className="exchange-rate-header">
        <Localized id="currency-title">
          <h1 className="exchange-rate-title">Exchange Rates</h1>
        </Localized>
        <Localized id="currency-btn-add">
          <Button onClick={openCreate}>Add</Button>
        </Localized>
      </div>

      {loading ? (
        <Localized id="currency-loading">
          <p className="exchange-rate-loading">Loading exchange rates&hellip;</p>
        </Localized>
      ) : error ? (
        <Card shadow="sm">
          <div className="exchange-rate-error">
            <p>{error}</p>
            <Button variant="secondary" onClick={load}>
              <Localized id="error-state-retry"><span>Retry</span></Localized>
            </Button>
          </div>
        </Card>
      ) : rates.length === 0 ? (
        <Card shadow="sm">
          <div className="exchange-rate-empty">
            <Localized id="currency-empty">
              <p>No exchange rates configured</p>
            </Localized>
            <Localized id="currency-btn-add">
              <Button variant="secondary" onClick={openCreate}>Add</Button>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="exchange-rate-table-wrap">
          <table className="exchange-rate-table" aria-label={l10n.getString('currency-table-label')}>
            <thead>
              <tr>
                <Localized id="currency-col-from"><th>From</th></Localized>
                <Localized id="currency-col-to"><th>To</th></Localized>
                <Localized id="currency-col-rate"><th>Rate</th></Localized>
                <Localized id="currency-col-source"><th>Source</th></Localized>
                <Localized id="currency-col-effective"><th>Effective Date</th></Localized>
                <th aria-label={l10n.getString('currency-table-actions')}> </th>
              </tr>
            </thead>
            <tbody>
              {rates.map((r) => (
                <tr key={r.id}>
                  <td>{r.from_currency}</td>
                  <td>{r.to_currency}</td>
                  <td>{r.rate}</td>
                  <td>{r.source === 'manual' ? <Localized id="currency-source-manual"><span>manual</span></Localized> : r.source}</td>
                  <td>{r.effective_date}</td>
                  <td className="exchange-rate-cell-actions">
                    <button
                      type="button"
                      className="exchange-rate-action-btn exchange-rate-action-btn--danger"
                      onClick={() => confirmDelete(r.id)}
                      disabled={deleting === r.id}
                      aria-label={l10n.getString('currency-delete-label', { from: r.from_currency, to: r.to_currency })}
                    >
                      <Localized id="currency-delete">
                        <span>Delete</span>
                      </Localized>
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {showModal && (
        <div className="exchange-rate-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('currency-modal-add-label')}>
          <div className="exchange-rate-modal">
            <div className="exchange-rate-modal-header">
              <Localized id="currency-modal-title">
                <h2>Add Exchange Rate</h2>
              </Localized>
              <button
                type="button"
                className="exchange-rate-modal-close"
                onClick={() => setShowModal(false)}
                aria-label={l10n.getString('close')}
              >
                &times;
              </button>
            </div>

            <div className="exchange-rate-modal-body">
              <label className="exchange-rate-field" htmlFor="er-field-from" aria-label={l10n.getString('currency-field-from')}>
                <Localized id="currency-field-from">
                  <span className="exchange-rate-label">From Currency</span>
                </Localized>
                <select
                  className="exchange-rate-input exchange-rate-select"
                  id="er-field-from"
                  value={form.fromCurrency}
                  onChange={(e) => setForm({ ...form, fromCurrency: e.target.value })}
                >
                  <Localized id="currency-select-placeholder">
                    <option value="">Select currency&hellip;</option>
                  </Localized>
                  {currencyOptions}
                </select>
              </label>

              <label className="exchange-rate-field" htmlFor="er-field-to" aria-label={l10n.getString('currency-field-to')}>
                <Localized id="currency-field-to">
                  <span className="exchange-rate-label">To Currency</span>
                </Localized>
                <select
                  className="exchange-rate-input exchange-rate-select"
                  id="er-field-to"
                  value={form.toCurrency}
                  onChange={(e) => setForm({ ...form, toCurrency: e.target.value })}
                >
                  <Localized id="currency-select-placeholder">
                    <option value="">Select currency&hellip;</option>
                  </Localized>
                  {currencyOptions}
                </select>
              </label>

              <label className="exchange-rate-field" htmlFor="er-field-rate" aria-label={l10n.getString('currency-field-rate')}>
                <Localized id="currency-field-rate">
                  <span className="exchange-rate-label">Rate</span>
                </Localized>
                <Localized id="currency-rate-placeholder" attrs={{ placeholder: true }}>
                  <input
                    className="exchange-rate-input"
                    type="number"
                    id="er-field-rate"
                    min="0"
                    step="any"
                    value={form.rate}
                    onChange={(e) => setForm({ ...form, rate: e.target.value })}
                    placeholder="1.25"
                  />
                </Localized>
              </label>

              <label className="exchange-rate-field" htmlFor="er-field-source" aria-label={l10n.getString('currency-field-source')}>
                <Localized id="currency-field-source">
                  <span className="exchange-rate-label">Source (optional)</span>
                </Localized>
                <Localized id="currency-source-placeholder" attrs={{ placeholder: true }}>
                  <input
                    className="exchange-rate-input"
                    type="text"
                    id="er-field-source"
                    value={form.source}
                    onChange={(e) => setForm({ ...form, source: e.target.value })}
                    placeholder="e.g. ECB"
                  />
                </Localized>
              </label>

              <label className="exchange-rate-field" htmlFor="er-field-date" aria-label={l10n.getString('currency-field-date')}>
                <Localized id="currency-field-date">
                  <span className="exchange-rate-label">Effective Date</span>
                </Localized>
                <input
                  className="exchange-rate-input"
                  type="date"
                  id="er-field-date"
                  value={form.effectiveDate}
                  onChange={(e) => setForm({ ...form, effectiveDate: e.target.value })}
                />
              </label>
            </div>

            <div className="exchange-rate-modal-actions">
              <Localized id="currency-btn-cancel">
                <Button variant="ghost" onClick={() => setShowModal(false)} disabled={saving}>Cancel</Button>
              </Localized>
              <Button
                variant="primary"
                loading={saving}
                disabled={!formValid}
                onClick={handleSave}
              >
                <Localized id="currency-btn-save">
                  <span>Save</span>
                </Localized>
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
