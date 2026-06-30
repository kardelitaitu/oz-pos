import { useState, useMemo, useCallback, useEffect } from 'react';
import { startSale, addLine, completeSale, setCartDiscount, printSalesReceipt, getSale, type SetCartDiscountArgs, type PaymentSplitArg } from '@/api/sales';
import { Button } from '@/components/Button';
import { formatMoney, type Money, type CartLine } from '@/types/domain';
import { useFeatures, FEATURES } from '@/hooks/useFeatures';
import {
  listCurrencies,
  listExchangeRates,
  getDefaultCurrency,
  type CurrencyDto,
  type ExchangeRateDto,
} from '@/api/currency';
import QrisQrDisplay from '@/components/QrisQrDisplay';
import './PaymentModal.css';

type PaymentMethod = 'cash' | 'card' | 'qris' | 'other';

interface SplitRow {
  id: number;
  method: PaymentMethod;
  otherLabel: string;
  amountMinor: string;
}

export interface PaymentModalProps {
  open: boolean;
  lineItems: CartLine[];
  total: Money;
  discountPercent?: number;
  discountLabel?: string;
  userId: string;
  onComplete: () => void;
  onClose: () => void;
}

export default function PaymentModal({
  open,
  lineItems,
  total,
  discountPercent = 0,
  discountLabel,
  userId,
  onComplete,
  onClose,
}: PaymentModalProps) {
  const [method, setMethod] = useState<PaymentMethod>('cash');
  const [otherLabel, setOtherLabel] = useState('');
  const [tendered, setTendered] = useState('');
  const [processing, setProcessing] = useState(false);
  const [done, setDone] = useState(false);
  const [changeDue, setChangeDue] = useState<Money | null>(null);

  const [showQr, setShowQr] = useState(false);
  const [qrReference, setQrReference] = useState('');

  const [splitMode, setSplitMode] = useState(false);
  const [splits, setSplits] = useState<SplitRow[]>([
    { id: 1, method: 'cash', otherLabel: '', amountMinor: '' },
    { id: 2, method: 'card', otherLabel: '', amountMinor: '' },
  ]);
  let nextSplitId = 3;

  const { isEnabled } = useFeatures();
  const multiCurrency = isEnabled(FEATURES.MULTI_CURRENCY);

  const [currencies, setCurrencies] = useState<CurrencyDto[]>([]);
  const [exchangeRates, setExchangeRates] = useState<ExchangeRateDto[]>([]);
  const [selectedCurrency, setSelectedCurrency] = useState(total.currency);
  const [baseCurrency, setBaseCurrency] = useState(total.currency);

  useEffect(() => {
    if (open && multiCurrency) {
      Promise.all([
        listCurrencies(),
        listExchangeRates(),
        getDefaultCurrency(),
      ])
        .then(([currs, rates, base]) => {
          setCurrencies(currs);
          setExchangeRates(rates);
          if (base) setBaseCurrency(base);
        })
        .catch(() => {});
    }
  }, [open, multiCurrency]);

  const exchangeRateInfo = useMemo(() => {
    if (selectedCurrency === total.currency) return null;
    const rate = exchangeRates.find(
      (r) => r.from_currency === total.currency && r.to_currency === selectedCurrency,
    );
    if (rate) return rate;
    const inverse = exchangeRates.find(
      (r) => r.from_currency === selectedCurrency && r.to_currency === total.currency,
    );
    if (inverse) {
      return {
        ...inverse,
        rate: 1 / inverse.rate,
        from_currency: total.currency,
        to_currency: selectedCurrency,
      };
    }
    return null;
  }, [selectedCurrency, total.currency, exchangeRates]);

  useEffect(() => {
    if (open) {
      setMethod('cash');
      setOtherLabel('');
      setTendered('');
      setProcessing(false);
      setDone(false);
      setChangeDue(null);
      setSplitMode(false);
      setShowQr(false);
      setQrReference('');
      setSelectedCurrency(total.currency);
      setSplits([
        { id: 1, method: 'cash', otherLabel: '', amountMinor: '' },
        { id: 2, method: 'card', otherLabel: '', amountMinor: '' },
      ]);
    }
  }, [open, total.currency]);

  const totalMinor = useMemo(() => BigInt(total.minor_units), [total.minor_units]);

  const tenderedMinor = useMemo(() => {
    const num = parseFloat(tendered);
    if (Number.isNaN(num) || num < 0) return 0n;
    const known: Record<string, number> = {
      JPY: 0, KRW: 0, VND: 0, CLP: 0, ISK: 0, HUF: 0,
      KWD: 3, OMR: 3, BHD: 3, JOD: 3, TND: 3,
    };
    const exp = known[total.currency] ?? 2;
    return BigInt(Math.round(num * 10 ** exp));
  }, [tendered, total.currency]);

  const handleQrPay = useCallback(() => {
    const ref = `QR-${Date.now()}-${Math.random().toString(36).slice(2, 8).toUpperCase()}`;
    setQrReference(ref);
    setShowQr(true);
  }, []);

  const handleQrConfirmed = useCallback(async () => {
    setShowQr(false);
    setProcessing(true);

    try {
      const { cartId } = await startSale({ currency: total.currency });

      if (discountPercent > 0) {
        const discountArgs: SetCartDiscountArgs = { cartId, percent: discountPercent };
        if (discountLabel) discountArgs.label = discountLabel;
        await setCartDiscount(discountArgs);
      }

      for (const line of lineItems) {
        await addLine({
          cartId,
          sku: line.sku,
          qty: line.qty,
          unitPriceMinor: line.unit_price.minor_units,
        });
      }

      const saleResult = await completeSale({
        cartId,
        paymentMethod: 'QRIS',
        tenderedMinor: null,
        userId,
        paymentSplits: [
          {
            method: 'QRIS',
            amountMinor: total.minor_units,
            gatewayReference: qrReference,
            gatewayStatus: 'completed',
            gatewayResponse: 'QRIS payment confirmed',
          },
        ],
      });

      try {
        const completedSale = await getSale(saleResult.saleId);

        await printSalesReceipt({
          date: new Date().toLocaleDateString('en-US', {
            year: 'numeric', month: 'short', day: 'numeric',
          }),
          receiptNumber: `SALE-${saleResult.saleId}`,
          items: lineItems.map((line, i) => {
            const computedLine = completedSale?.lines[i];
            const tax = computedLine?.tax_amount
              ? { minorUnits: computedLine.tax_amount.minor_units, currency: computedLine.tax_amount.currency }
              : null;
            return {
              name: line.name ?? line.sku,
              quantity: line.qty,
              unitPrice: { minorUnits: line.unit_price.minor_units, currency: line.unit_price.currency },
              totalPrice: {
                minorUnits: line.unit_price.minor_units * line.qty,
                currency: line.unit_price.currency,
              },
              ...(tax ? { taxAmount: tax } : {}),
            };
          }),
          subtotal: completedSale
            ? { minorUnits: completedSale.subtotal.minor_units, currency: total.currency }
            : { minorUnits: saleResult.total?.minor_units ?? total.minor_units, currency: total.currency },
          ...(completedSale && completedSale.taxTotal.minor_units > 0
            ? { tax: { minorUnits: completedSale.taxTotal.minor_units, currency: total.currency } }
            : {}),
          total: { minorUnits: saleResult.total?.minor_units ?? total.minor_units, currency: total.currency },
          payments: [
            {
              method: 'QRIS',
              amount: { minorUnits: total.minor_units, currency: total.currency },
              change: null,
            },
          ],
        });
      } catch {
        // Printer may not be connected.
      }

      setDone(true);
    } catch (err) {
      console.error('QR payment failed:', err);
    } finally {
      setProcessing(false);
    }
  }, [lineItems, total, discountPercent, discountLabel, userId, qrReference]);

  const { sufficient, change } = useMemo(() => {
    if (method !== 'cash') return { sufficient: true, change: null };
    if (tenderedMinor < totalMinor) return { sufficient: false, change: null };
    const diff = Number(tenderedMinor - totalMinor);
    return {
      sufficient: true,
      change: { minor_units: diff, currency: total.currency } as Money,
    };
  }, [method, total, tenderedMinor, totalMinor]);

  const parseSplitMinor = useCallback((val: string): bigint => {
    const num = parseFloat(val);
    if (Number.isNaN(num) || num < 0) return 0n;
    const known: Record<string, number> = {
      JPY: 0, KRW: 0, VND: 0, CLP: 0, ISK: 0, HUF: 0,
      KWD: 3, OMR: 3, BHD: 3, JOD: 3, TND: 3,
    };
    const exp = known[total.currency] ?? 2;
    return BigInt(Math.round(num * 10 ** exp));
  }, [total.currency]);

  const splitTotals = useMemo(() => {
    let splitSum = 0n;
    for (const s of splits) {
      splitSum += parseSplitMinor(s.amountMinor);
    }
    return { splitSum, remaining: totalMinor - splitSum };
  }, [splits, parseSplitMinor, totalMinor]);

  const splitComplete = useMemo(() => {
    if (splitTotals.remaining !== 0n) return false;
    return splits.every((s) => {
      if (s.method === 'other' && !s.otherLabel.trim()) return false;
      return parseSplitMinor(s.amountMinor) > 0n;
    });
  }, [splits, splitTotals, parseSplitMinor]);

  const addSplit = useCallback(() => {
    setSplits((prev) => [
      ...prev,
      { id: nextSplitId++, method: 'cash', otherLabel: '', amountMinor: '' },
    ]);
  }, []);

  const removeSplit = useCallback((id: number) => {
    setSplits((prev) => {
      if (prev.length <= 1) return prev;
      return prev.filter((s) => s.id !== id);
    });
  }, []);

  const updateSplit = useCallback((id: number, patch: Partial<SplitRow>) => {
    setSplits((prev) => prev.map((s) => (s.id === id ? { ...s, ...patch } : s)));
  }, []);

  const autoSplitEvenly = useCallback(() => {
    const count = splits.length;
    if (count === 0) return;
    const each = Number(totalMinor) / count;
    const known: Record<string, number> = {
      JPY: 0, KRW: 0, VND: 0, CLP: 0, ISK: 0, HUF: 0,
      KWD: 3, OMR: 3, BHD: 3, JOD: 3, TND: 3,
    };
    const exp = known[total.currency] ?? 2;
    const eachFormatted = (each / 10 ** exp).toFixed(exp);
    const remainderCents = Number(totalMinor % BigInt(count));
    setSplits((prev) =>
      prev.map((s, i) => {
        const val = exp === 0 ? parseFloat(eachFormatted).toFixed(0) : eachFormatted;
        return {
          ...s,
          amountMinor: i === prev.length - 1
            ? (parseFloat(val) + remainderCents / 10 ** exp).toFixed(exp)
            : val,
        };
      }),
    );
  }, [splits.length, totalMinor, total.currency]);

  const canComplete = useMemo(() => {
    if (splitMode) return splitComplete;
    if (method === 'other' && !otherLabel.trim()) return false;
    if (method === 'cash') return sufficient;
    if (method === 'qris') return true;
    return true;
  }, [splitMode, splitComplete, method, otherLabel, sufficient]);

  const complete = useCallback(async () => {
    setProcessing(true);

    try {
      const { cartId } = await startSale({ currency: total.currency });

      if (discountPercent > 0) {
        const discountArgs: SetCartDiscountArgs = { cartId, percent: discountPercent };
        if (discountLabel) discountArgs.label = discountLabel;
        await setCartDiscount(discountArgs);
      }

      for (const line of lineItems) {
        await addLine({
          cartId,
          sku: line.sku,
          qty: line.qty,
          unitPriceMinor: line.unit_price.minor_units,
        });
      }

      let paymentSplits: PaymentSplitArg[] | undefined;

      if (splitMode) {
        const known: Record<string, number> = {
          JPY: 0, KRW: 0, VND: 0, CLP: 0, ISK: 0, HUF: 0,
          KWD: 3, OMR: 3, BHD: 3, JOD: 3, TND: 3,
        };
        const exp = known[total.currency] ?? 2;
        paymentSplits = splits.map((s) => ({
          method: s.method === 'other' ? s.otherLabel.trim() || 'OTHER' : s.method.toUpperCase(),
          amountMinor: Math.round(parseFloat(s.amountMinor || '0') * 10 ** exp),
        }));
      }

      const methodLabel = splitMode
        ? 'split'
        : method === 'other'
          ? otherLabel.trim() || 'OTHER'
          : method.toUpperCase();

      const saleResult = await completeSale({
        cartId,
        paymentMethod: methodLabel,
        tenderedMinor: method === 'cash' && !splitMode ? Number(tenderedMinor) : null,
        userId,
        ...(paymentSplits ? { paymentSplits } : {}),
      });

      try {
        // Fetch the completed sale to get computed tax data.
        const completedSale = await getSale(saleResult.saleId);

        await printSalesReceipt({
          date: new Date().toLocaleDateString('en-US', {
            year: 'numeric', month: 'short', day: 'numeric',
          }),
          receiptNumber: `SALE-${saleResult.saleId}`,
          items: lineItems.map((line, i) => {
            const computedLine = completedSale?.lines[i];
            const tax = computedLine?.tax_amount
              ? { minorUnits: computedLine.tax_amount.minor_units, currency: computedLine.tax_amount.currency }
              : null;
            return {
              name: line.name ?? line.sku,
              quantity: line.qty,
              unitPrice: { minorUnits: line.unit_price.minor_units, currency: line.unit_price.currency },
              totalPrice: {
                minorUnits: line.unit_price.minor_units * line.qty,
                currency: line.unit_price.currency,
              },
              ...(tax ? { taxAmount: tax } : {}),
            };
          }),
          subtotal: completedSale
            ? { minorUnits: completedSale.subtotal.minor_units, currency: total.currency }
            : { minorUnits: saleResult.total?.minor_units ?? total.minor_units, currency: total.currency },
          ...(completedSale && completedSale.taxTotal.minor_units > 0
            ? { tax: { minorUnits: completedSale.taxTotal.minor_units, currency: total.currency } }
            : {}),
          total: { minorUnits: saleResult.total?.minor_units ?? total.minor_units, currency: total.currency },
          payments: paymentSplits
            ? paymentSplits.map((ps) => ({
                method: ps.method,
                amount: { minorUnits: ps.amountMinor, currency: total.currency },
                change: null,
              }))
            : [
                {
                  method: methodLabel,
                  amount: { minorUnits: total.minor_units, currency: total.currency },
                  change: change
                    ? { minorUnits: change.minor_units, currency: change.currency }
                    : null,
                },
              ],
        });
      } catch {
        // Printer may not be connected.
      }

      if (change) setChangeDue(change);
      setDone(true);
    } catch (err) {
      console.error('Sale failed:', err);
    } finally {
      setProcessing(false);
    }
  }, [splitMode, splits, method, otherLabel, lineItems, total, discountPercent, discountLabel, change, userId, tenderedMinor]);

  useEffect(() => {
    if (!done) return;
    const timer = setTimeout(() => {
      onComplete();
    }, changeDue ? 3000 : 1500);
    return () => clearTimeout(timer);
  }, [done, changeDue, onComplete]);

  if (!open) return null;

  return (
    <div className="payment-overlay" role="dialog" aria-modal="true" aria-label="Payment">
      <QrisQrDisplay
        amount={total.minor_units}
        currency={total.currency}
        reference={qrReference}
        isOpen={showQr}
        onClose={() => setShowQr(false)}
        onPaymentConfirmed={handleQrConfirmed}
      />

      <div className="payment-modal">
        {done ? (
          <div className="payment-done">
            <h2 className="payment-done-title">Sale Complete</h2>
            {changeDue && (
              <div className="payment-change">
                <span className="payment-change-label">Change due</span>
                <span className="payment-change-amount">
                  {formatMoney(changeDue)}
                </span>
              </div>
            )}
            <p className="payment-done-note">Receipt printed</p>
          </div>
        ) : (
          <>
            <div className="payment-header">
              <h2 className="payment-title">Complete Sale</h2>
              <button
                type="button"
                className="payment-close"
                onClick={onClose}
                aria-label="Cancel payment"
              >
                &times;
              </button>
            </div>

            <div className="payment-total-row">
              <span className="payment-total-label">Total Due</span>
              <span className="payment-total-amount">{formatMoney(total)}</span>
            </div>

            {multiCurrency && (
              <div className="payment-currency-selector">
                <label htmlFor="payment-currency-select" aria-label="Charge currency">
                  <span className="payment-currency-label">Charge Currency</span>
                  <select
                    id="payment-currency-select"
                    className="payment-currency-select"
                    value={selectedCurrency}
                    onChange={(e) => setSelectedCurrency(e.target.value)}
                    aria-label="Select charge currency"
                  >
                    {currencies.length === 0 && (
                      <option value={total.currency}>{total.currency}</option>
                    )}
                    {currencies.map((c) => (
                      <option key={c.code} value={c.code}>
                        {c.code} — {c.name}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
            )}

            {selectedCurrency !== total.currency && exchangeRateInfo && (
              <div className="payment-exchange-notice" aria-label="Exchange rate information">
                <div className="payment-exchange-row">
                  <span>Exchange rate</span>
                  <span>
                    1 {exchangeRateInfo.from_currency} = {exchangeRateInfo.rate.toFixed(6)} {exchangeRateInfo.to_currency}
                  </span>
                </div>
                <div className="payment-exchange-row">
                  <span>Rate source</span>
                  <span>{exchangeRateInfo.source || 'manual'}</span>
                </div>
                <div className="payment-exchange-row">
                  <span>Rate timestamp</span>
                  <span>{exchangeRateInfo.effective_date}</span>
                </div>
              </div>
            )}

            {selectedCurrency !== total.currency && (
              <div className="payment-receipt-currency" aria-label="Receipt currency information">
                <div className="payment-receipt-currency-row">
                  <span>Charged in</span>
                  <span>{selectedCurrency}</span>
                </div>
                <div className="payment-receipt-currency-row">
                  <span>Default currency</span>
                  <span>{baseCurrency}</span>
                </div>
                <div className="payment-receipt-currency-row">
                  <span>Base amount</span>
                  <span>{formatMoney(total)}</span>
                </div>
                <div className="payment-receipt-currency-row">
                  <span>Charge amount</span>
                  <span>
                    {formatMoney({
                      minor_units: Math.round(total.minor_units * (exchangeRateInfo?.rate ?? 1)),
                      currency: selectedCurrency,
                    } as Money)}
                  </span>
                </div>
              </div>
            )}

            {!splitMode && (
              <>
                <fieldset className="payment-methods">
                  <legend className="payment-section-title">Payment Method</legend>
                  <div className="payment-method-options">
                    {(['cash', 'card', 'qris'] as const).map((m) => (
                      <label key={m} className="payment-method-label">
                        <input
                          type="radio"
                          name="payment-method"
                          value={m}
                          checked={method === m}
                          onChange={() => setMethod(m)}
                        />
                        <span className="payment-method-name">
                          {m === 'cash' ? 'Cash' : m === 'card' ? 'Card' : 'QRIS'}
                        </span>
                      </label>
                    ))}
                    <label className="payment-method-label">
                      <input
                        type="radio"
                        name="payment-method"
                        value="other"
                        checked={method === 'other'}
                        onChange={() => setMethod('other')}
                      />
                      <input
                        type="text"
                        className="payment-other-input"
                        placeholder="Other..."
                        value={otherLabel}
                        onChange={(e) => {
                          setMethod('other');
                          setOtherLabel(e.target.value);
                        }}
                        disabled={method !== 'other'}
                        aria-label="Other payment method name"
                      />
                    </label>
                  </div>
                </fieldset>

                {method === 'cash' && (
                  <div className="payment-cash-section">
                    <label className="payment-tendered-label">
                      <span>Amount Tendered</span>
                      <input
                        type="text"
                        className="payment-tendered-input"
                        inputMode="decimal"
                        placeholder="0.00"
                        value={tendered}
                        onChange={(e) => setTendered(e.target.value)}
                        aria-label="Amount tendered"
                      />
                    </label>

                    <div className="payment-quick-cash">
                      {[5, 10, 20, 50, 100].map((amount) => {
                        const totalNum = Number(total.minor_units) / 100;
                        const quickVal = Math.ceil(totalNum / amount) * amount;
                        return (
                          <button
                            key={amount}
                            type="button"
                            className="payment-quick-btn"
                            onClick={() => setTendered(quickVal.toFixed(2))}
                            aria-label={`Tender $${quickVal.toFixed(2)}`}
                          >
                            ${quickVal}
                          </button>
                        );
                      })}
                      <button
                        type="button"
                        className="payment-quick-btn"
                        onClick={() => setTendered((Number(total.minor_units) / 100).toFixed(2))}
                        aria-label="Tend exact amount"
                      >
                        Exact
                      </button>
                    </div>

                    {tendered.length > 0 && (
                      <div className="payment-change-preview">
                        <span className="payment-change-label">Change</span>
                        <span
                          className={`payment-change-amount ${!sufficient ? 'payment-change-insufficient' : ''}`}
                        >
                          {sufficient
                            ? formatMoney(change!)
                            : 'Insufficient amount'}
                        </span>
                      </div>
                    )}
                  </div>
                )}

                {method === 'qris' && (
                  <div className="payment-qris-section">
                    <p className="payment-qris-description">
                      Generate a QRIS QR code for the customer to scan with their payment app.
                    </p>
                    <button
                      type="button"
                      className="payment-qris-btn"
                      onClick={handleQrPay}
                      disabled={processing}
                      aria-label="Generate QRIS QR code"
                    >
                      Pay with QR
                    </button>
                  </div>
                )}
              </>
            )}

            {splitMode && (
              <div className="payment-split-section">
                <div className="payment-split-header">
                  <span className="payment-section-title">Split Payments</span>
                  <div className="payment-split-actions">
                    <button
                      type="button"
                      className="payment-split-btn"
                      onClick={autoSplitEvenly}
                      aria-label="Split evenly"
                    >
                      Split Evenly
                    </button>
                    <button
                      type="button"
                      className="payment-split-btn"
                      onClick={addSplit}
                      aria-label="Add split"
                    >
                      + Add Split
                    </button>
                  </div>
                </div>

                <div className="payment-split-rows">
                  {splits.map((s) => (
                    <div key={s.id} className="payment-split-row">
                      <div className="payment-split-method-group">
                        {(['cash', 'card'] as const).map((m) => (
                          <label key={m} className="payment-split-radio-label">
                            <input
                              type="radio"
                              name={`split-method-${s.id}`}
                              value={m}
                              checked={s.method === m}
                              onChange={() => updateSplit(s.id, { method: m, otherLabel: '' })}
                            />
                            <span>{m === 'cash' ? 'Cash' : 'Card'}</span>
                          </label>
                        ))}
                        <label className="payment-split-radio-label">
                          <input
                            type="radio"
                            name={`split-method-${s.id}`}
                            value="other"
                            checked={s.method === 'other'}
                            onChange={() => updateSplit(s.id, { method: 'other' })}
                          />
                          <input
                            type="text"
                            className="payment-split-other-input"
                            placeholder="Other"
                            value={s.otherLabel}
                            onChange={(e) => updateSplit(s.id, { otherLabel: e.target.value })}
                            disabled={s.method !== 'other'}
                            aria-label="Other payment method name"
                          />
                        </label>
                      </div>
                      <div className="payment-split-amount-group">
                        <span className="payment-split-currency">$</span>
                        <input
                          type="text"
                          className="payment-split-amount-input"
                          inputMode="decimal"
                          placeholder="0.00"
                          value={s.amountMinor}
                          onChange={(e) => updateSplit(s.id, { amountMinor: e.target.value })}
                          aria-label="Split amount"
                        />
                      </div>
                      <button
                        type="button"
                        className="payment-split-remove"
                        onClick={() => removeSplit(s.id)}
                        disabled={splits.length <= 1}
                        aria-label="Remove split"
                      >
                        &times;
                      </button>
                    </div>
                  ))}
                </div>

                <div className="payment-split-remaining">
                  <span className="payment-split-remaining-label">Remaining</span>
                  <span
                    className={`payment-split-remaining-amount ${
                      splitTotals.remaining !== 0n ? 'payment-split-remaining-positive' : ''
                    }`}
                  >
                    {formatMoney({
                      minor_units: Number(splitTotals.remaining),
                      currency: total.currency,
                    } as Money)}
                  </span>
                </div>
              </div>
            )}

            <div className="payment-split-toggle">
              <label className="payment-split-toggle-label">
                <input
                  type="checkbox"
                  checked={splitMode}
                  onChange={(e) => setSplitMode(e.target.checked)}
                />
                <span>Split payment across methods</span>
              </label>
            </div>

            <div className="payment-actions">
              <Button variant="ghost" onClick={onClose} disabled={processing}>
                Cancel
              </Button>
              <Button
                variant="primary"
                loading={processing}
                disabled={!canComplete}
                onClick={complete}
              >
                Complete Sale
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
