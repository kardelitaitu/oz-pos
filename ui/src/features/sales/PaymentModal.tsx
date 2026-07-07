import { useState, useMemo, useCallback, useEffect, useRef } from 'react';
import { useToast } from '@/frontend/shared/Toast';
import { Localized, useLocalization } from '@fluent/react';
import { startSale, addLine, completeSale, printSalesReceipt, getSale, setCartDiscount, holdCart, type SetCartDiscountArgs, type PaymentSplitArg, type SerialNumberArg } from '@/api/sales';
import { createKdsOrderFromSale } from '@/api/kds';
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
import { listCustomers, type CustomerDto } from '@/api/customers';
import { getLoyaltyAccount, redeemLoyaltyPoints, getPointsValue, type LoyaltyAccountWithDetails } from '@/api/loyalty';
import QrisQrDisplay from '@/components/QrisQrDisplay';
import { animDuration } from '@/utils/animation';
import './PaymentModal.css';

type PaymentMethod = 'cash' | 'card' | 'qris' | 'other' | 'open_bill' | 'credit';

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
  tableNumber?: string;
  selectedCustomer?: CustomerDto | null;
  onCustomerChange?: (customer: CustomerDto | null) => void;
  onComplete: () => void;
  onClose: () => void;
  /** Serial numbers captured per SKU for track_serial products. */
  serialNumbers?: Record<string, string>;
  /** Custom quick tender preset amounts (in minor units). Defaults to standard Rp denominations. */
  tenderPresets?: number[];
}

export default function PaymentModal({
  open,
  lineItems,
  total,
  discountPercent = 0,
  discountLabel,
  userId,
  tableNumber,
  selectedCustomer: selectedCustomerProp,
  onCustomerChange,
  onComplete,
  onClose,
  serialNumbers,
  tenderPresets,
}: PaymentModalProps) {
  const { l10n } = useLocalization();
  const { addToast } = useToast();
  const [method, setMethod] = useState<PaymentMethod>('cash');
  const [otherLabel, setOtherLabel] = useState('');
  const [tendered, setTendered] = useState('');
  const [processing, setProcessing] = useState(false);
  const [done, setDone] = useState(false);
  const [changeDue, setChangeDue] = useState<Money | null>(null);
  const [customerName, setCustomerName] = useState('');
  const [selectedCustomer, setSelectedCustomer] = useState<CustomerDto | null>(selectedCustomerProp ?? null);
  const [showCustomerSearch, setShowCustomerSearch] = useState(false);
  const [loyaltyAccount, setLoyaltyAccount] = useState<LoyaltyAccountWithDetails | null>(null);
  const [redeemPoints, setRedeemPoints] = useState(false);
  const [loyaltyDiscount, setLoyaltyDiscount] = useState(0n);
  const [pointsToRedeem, setPointsToRedeem] = useState(0);
  const [pointsWorthMinor, setPointsWorthMinor] = useState<number | null>(null);

  const notifyCustomerChange = useCallback(
    (c: CustomerDto | null) => {
      setSelectedCustomer(c);
      onCustomerChange?.(c);
    },
    [onCustomerChange],
  );
  const [customerSearchQuery, setCustomerSearchQuery] = useState('');
  const [customerSearchResults, setCustomerSearchResults] = useState<CustomerDto[]>([]);
  const [loadingCustomers, setLoadingCustomers] = useState(false);
  const [leaving, setLeaving] = useState(false);
  const leaveCb = useRef<(() => void) | null>(null);

  const MS_200 = animDuration(200);

  const animateLeave = useCallback((done: () => void) => {
    setLeaving(true);
    leaveCb.current = done;
  }, []);

  const handleLeaveEnd = useCallback(() => {
    if (!leaving) return;
    leaveCb.current?.();
    leaveCb.current = null;
    setLeaving(false);
  }, [leaving]);

  const [showQr, setShowQr] = useState(false);
  const [qrReference, setQrReference] = useState('');

  const [splitMode, setSplitMode] = useState(false);
  const [splits, setSplits] = useState<SplitRow[]>([
    { id: 1, method: 'cash', otherLabel: '', amountMinor: '' },
    { id: 2, method: 'card', otherLabel: '', amountMinor: '' },
  ]);
  const nextSplitId = useRef(3);

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
        .catch(() => addToast({ message: 'Failed to load currency data', type: 'error' }));
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
      notifyCustomerChange(null);
      setCustomerSearchQuery('');
      setCustomerSearchResults([]);
      setSplits([
        { id: 1, method: 'cash', otherLabel: '', amountMinor: '' },
        { id: 2, method: 'card', otherLabel: '', amountMinor: '' },
      ]);
    }
  }, [open, total.currency]);

  useEffect(() => {
    if (!showCustomerSearch) return;
    setLoadingCustomers(true);
    listCustomers()
      .then((customers) => {
        const q = customerSearchQuery.trim().toLowerCase();
        if (!q) {
          setCustomerSearchResults(customers);
        } else {
          setCustomerSearchResults(
            customers.filter(
              (c) =>
                c.name.toLowerCase().includes(q) ||
                (c.phone && c.phone.includes(q)) ||
                (c.email && c.email.toLowerCase().includes(q)),
            ),
          );
        }
      })
      .catch(() => setCustomerSearchResults([]))
      .finally(() => setLoadingCustomers(false));
  }, [showCustomerSearch, customerSearchQuery]);

  const totalMinor = useMemo(() => BigInt(total.minor_units), [total.minor_units]);

  useEffect(() => {
    if (selectedCustomer) {
      getLoyaltyAccount(selectedCustomer.id)
        .then((account) => {
          setLoyaltyAccount(account);
          if (account && account.account.points > 0) {
            setRedeemPoints(false);
            setLoyaltyDiscount(0n);
          }
        })
        .catch(() => setLoyaltyAccount(null));
    } else {
      setLoyaltyAccount(null);
      setRedeemPoints(false);
      setLoyaltyDiscount(0n);
    }
  }, [selectedCustomer]);

  useEffect(() => {
    if (loyaltyAccount && loyaltyAccount.account.points > 0) {
      getPointsValue(loyaltyAccount.account.points)
        .then(setPointsWorthMinor)
        .catch(() => setPointsWorthMinor(null));
    } else {
      setPointsWorthMinor(null);
    }
  }, [loyaltyAccount]);

  useEffect(() => {
    if (!redeemPoints || pointsToRedeem <= 0) {
      setLoyaltyDiscount(0n);
      return;
    }
    let cancelled = false;
    getPointsValue(pointsToRedeem)
      .then((val) => {
        if (!cancelled) {
          const discount = BigInt(val);
          setLoyaltyDiscount(discount > totalMinor ? totalMinor : discount);
        }
      })
      .catch(() => {});
    return () => { cancelled = true; };
  }, [pointsToRedeem, redeemPoints, totalMinor]);

  const effectiveTotal = useMemo(() => {
    const base = totalMinor;
    const discount = loyaltyDiscount;
    return base - discount >= 0n ? base - discount : 0n;
  }, [totalMinor, loyaltyDiscount]);

  const effectiveTotalMoney = useMemo<Money>(() => ({
    minor_units: Number(effectiveTotal),
    currency: total.currency,
  }), [effectiveTotal, total.currency]);

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
        const discountArgs: SetCartDiscountArgs = { cartId, percent: discountPercent, userId };
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

      const serialNumberArgs: SerialNumberArg[] | undefined = serialNumbers
        ? Object.entries(serialNumbers)
            .filter(([_, s]) => s.trim().length > 0)
            .map(([sku, serial]) => ({ sku, serial }))
        : undefined;
      const saleResult = await completeSale({
        cartId,
        paymentMethod: 'QRIS',
        tenderedMinor: null,
        userId,
        ...(selectedCustomer ? { customerId: selectedCustomer.id } : {}),
        ...(serialNumberArgs && serialNumberArgs.length > 0 ? { serialNumbers: serialNumberArgs } : {}),
        paymentSplits: [
          {
            method: 'QRIS',
            amountMinor: Number(effectiveTotal),
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
          ...(tableNumber ? { tableNumber } : {}),
        });
      } catch {
        // Printer may not be connected.
      }

      try {
        await createKdsOrderFromSale(saleResult.saleId);
      } catch {
        // KDS may not be configured — non-blocking.
      }

      if (loyaltyAccount && redeemPoints && loyaltyDiscount > 0n) {
        try {
          await redeemLoyaltyPoints(
            selectedCustomer!.id,
            Number(loyaltyDiscount),
            saleResult.saleId,
          );
        } catch {
          // non-blocking
        }
      }

      setDone(true);
    } catch (err) {
      console.error('QR payment failed:', err);
    } finally {
      setProcessing(false);
    }
  }, [lineItems, total, discountPercent, discountLabel, userId, qrReference, selectedCustomer, effectiveTotal, loyaltyAccount, redeemPoints, loyaltyDiscount]);

  const { sufficient, change } = useMemo(() => {
    if (method !== 'cash') return { sufficient: true, change: null };
    if (tenderedMinor < effectiveTotal) return { sufficient: false, change: null };
    const diff = Number(tenderedMinor - effectiveTotal);
    return {
      sufficient: true,
      change: { minor_units: diff, currency: total.currency } as Money,
    };
  }, [method, total, tenderedMinor, effectiveTotal]);

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
    return { splitSum, remaining: effectiveTotal - splitSum };
  }, [splits, parseSplitMinor, effectiveTotal]);

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
      { id: nextSplitId.current++, method: 'cash', otherLabel: '', amountMinor: '' },
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
    if (method === 'open_bill') return customerName.trim().length > 0;
    if (method === 'credit') return customerName.trim().length > 0;
    if (method === 'cash') return sufficient;
    if (method === 'qris') return qrReference.length > 0;
    return true;
  }, [splitMode, splitComplete, method, otherLabel, sufficient, customerName]);

  const subtotalMinor = useMemo(() => {
    return lineItems.reduce((acc, l) => acc + l.unit_price.minor_units * l.qty, 0);
  }, [lineItems]);

  const complete = useCallback(async () => {
    setProcessing(true);
    console.log('[Sale] Starting sale...');

    try {
      // ── Open Bill: save cart without payment ──────────────
      if (method === 'open_bill') {
        const cartData = JSON.stringify({
          lines: lineItems.map((l) => ({
            sku: l.sku,
            name: l.name,
            qty: l.qty,
            unit_price: l.unit_price,
          })),
          discountPercent,
          discountLabel,
          tableNumber,
        });
        await holdCart({
          label: customerName.trim() || `Open Bill #${Date.now()}`,
          cart_data: cartData,
          item_count: lineItems.length,
          total_minor: total.minor_units,
          currency: total.currency,
          bill_type: 'open_bill',
          customer_name: customerName.trim(),
        });
        console.log('[Sale] Open bill saved');
        setDone(true);
        return;
      }

      console.log('[Sale] Creating cart...');
      const { cartId } = await startSale({ currency: total.currency });
      console.log('[Sale] Cart created:', cartId);

      if (discountPercent > 0) {
        console.log('[Sale] Setting discount:', discountPercent);
        const discountArgs: SetCartDiscountArgs = { cartId, percent: discountPercent, userId };
        if (discountLabel) discountArgs.label = discountLabel;
        await setCartDiscount(discountArgs);
      }

      for (const line of lineItems) {
        console.log('[Sale] Adding line:', line.sku, 'qty:', line.qty);
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

      console.log('[Sale] Completing sale...');
      const serialNumberArgs: SerialNumberArg[] | undefined = serialNumbers
        ? Object.entries(serialNumbers)
            .filter(([_, s]) => s.trim().length > 0)
            .map(([sku, serial]) => ({ sku, serial }))
        : undefined;
      const saleResult = await completeSale({
        cartId,
        paymentMethod: methodLabel,
        tenderedMinor: method === 'cash' && !splitMode ? Number(tenderedMinor) : null,
        userId,
        ...(selectedCustomer ? { customerId: selectedCustomer.id } : {}),
        ...(paymentSplits ? { paymentSplits } : {}),
        ...(method === 'credit' && customerName.trim() ? { customerName: customerName.trim() } : {}),
        ...(serialNumberArgs && serialNumberArgs.length > 0 ? { serialNumbers: serialNumberArgs } : {}),
      });
      console.log('[Sale] Sale completed:', saleResult.saleId);

      try {
        console.log('[Sale] Fetching completed sale...');
        const completedSale = await getSale(saleResult.saleId);

        console.log('[Sale] Printing receipt...');
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
          ...(tableNumber ? { tableNumber } : {}),
        });
      } catch (e) {
        console.warn('[Sale] Receipt/KDS step failed (non-blocking):', e);
      }

      try {
        await createKdsOrderFromSale(saleResult.saleId);
      } catch {
        // KDS may not be configured — non-blocking.
      }

      if (loyaltyAccount && redeemPoints && loyaltyDiscount > 0n) {
        try {
          await redeemLoyaltyPoints(
            selectedCustomer!.id,
            Number(loyaltyDiscount),
            saleResult.saleId,
          );
          console.log('[Sale] Loyalty points redeemed');
        } catch (e) {
          console.warn('[Sale] Loyalty redemption failed (non-blocking):', e);
        }
      }

      if (change) setChangeDue(change);
      console.log('[Sale] Done');
      setDone(true);
    } catch (err) {
      console.error('[Sale] FAILED:', err);
    } finally {
      setProcessing(false);
    }
  }, [method, customerName, lineItems, subtotalMinor, total, discountPercent, discountLabel, splitMode, splits, otherLabel, change, userId, tenderedMinor, selectedCustomer, loyaltyAccount, redeemPoints, loyaltyDiscount]);

  useEffect(() => {
    if (!done) return;
    const timer = setTimeout(() => {
      animateLeave(onComplete);
    }, changeDue ? 3000 : 1500);
    return () => clearTimeout(timer);
  }, [done, changeDue, onComplete, animateLeave]);

  // Auto-dismiss after leave animation completes
  useEffect(() => {
    if (!leaving) return;
    const timer = setTimeout(handleLeaveEnd, MS_200);
    return () => clearTimeout(timer);
  }, [leaving, handleLeaveEnd]);

  if (!open && !leaving) return null;

  const stateClass = leaving ? 'payment-overlay--exit' : 'payment-overlay--enter';
  const modalStateClass = leaving ? 'payment-modal--exit' : 'payment-modal--enter';

  return (
    <Localized id="payment-dialog-aria" attrs={{ 'aria-label': true }}>
    <div className={`payment-overlay ${stateClass}`} role="dialog" aria-modal="true" aria-label={l10n.getString('payment-dialog-aria')}>
      <QrisQrDisplay
        amount={total.minor_units}
        currency={total.currency}
        reference={qrReference}
        isOpen={showQr}
        onClose={() => setShowQr(false)}
        onPaymentConfirmed={handleQrConfirmed}
      />

      <div className={`payment-modal ${modalStateClass}`}>
        {done ? (
          <div className="payment-done">
            <svg className="payment-done-checkmark" viewBox="0 0 64 64" aria-hidden="true">
              <circle className="payment-done-checkmark-circle" cx="32" cy="32" r="26" />
              <path className="payment-done-checkmark-path" d="M20 32 l8 8 l16 -16" />
            </svg>
            <Localized id="payment-done-title">
              <h2 className="payment-done-title">Sale Complete</h2>
            </Localized>
            {changeDue && (
              <div className="payment-change">
                <Localized id="payment-change-label">
                  <span className="payment-change-label">Change due</span>
                </Localized>
                <span className="payment-change-amount">
                  {formatMoney(changeDue)}
                </span>
              </div>
            )}
            <Localized id="payment-done-receipt">
              <p className="payment-done-note">Receipt printed</p>
            </Localized>
          </div>
        ) : (
          <>
            <div className="payment-header">
              <Localized id="payment-title">
                <h2 className="payment-title">Complete Sale</h2>
              </Localized>
              <Localized id="payment-close-aria" attrs={{ 'aria-label': true }}>
              <button
                type="button"
                className="payment-close"
                onClick={() => animateLeave(onClose)}
                aria-label={l10n.getString('payment-close-aria')}
              >
                &times;
              </button>
              </Localized>
            </div>

            <div className="payment-total-row">
              <Localized id="payment-total-due">
                <span className="payment-total-label">Total Due</span>
              </Localized>
              <span className="payment-total-amount">
                {loyaltyDiscount > 0n ? formatMoney(effectiveTotalMoney) : formatMoney(total)}
              </span>
            </div>

            {multiCurrency && (
              <div className="payment-currency-selector">
                <Localized id="payment-currency-aria" attrs={{ 'aria-label': true }}>
                  <label htmlFor="payment-currency-select" aria-label={l10n.getString('payment-currency-aria')}>
                    <Localized id="payment-currency-label">
                      <span className="payment-currency-label">Charge Currency</span>
                    </Localized>
                    <Localized id="payment-currency-select-aria" attrs={{ 'aria-label': true }}>
                      <select
                        id="payment-currency-select"
                        className="payment-currency-select"
                        value={selectedCurrency}
                        onChange={(e) => setSelectedCurrency(e.target.value)}
                        aria-label={l10n.getString('payment-currency-select-aria')}
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
                    </Localized>
                  </label>
                </Localized>
              </div>
            )}

            {selectedCurrency !== total.currency && exchangeRateInfo && (
              <Localized id="payment-exchange-aria" attrs={{ 'aria-label': true }}>
              <div className="payment-exchange-notice" aria-label={l10n.getString('payment-exchange-aria')}>
                <div className="payment-exchange-row">
                  <Localized id="payment-exchange-rate">
                    <span>Exchange rate</span>
                  </Localized>
                  <span>
                    1 {exchangeRateInfo.from_currency} = {exchangeRateInfo.rate.toFixed(6)} {exchangeRateInfo.to_currency}
                  </span>
                </div>
                <div className="payment-exchange-row">
                  <Localized id="payment-rate-source">
                    <span>Rate source</span>
                  </Localized>
                  <span>{exchangeRateInfo.source || l10n.getString('payment-rate-source-manual')}</span>
                </div>
                <div className="payment-exchange-row">
                  <Localized id="payment-rate-timestamp">
                    <span>Rate timestamp</span>
                  </Localized>
                  <span>{exchangeRateInfo.effective_date}</span>
                </div>
              </div>
              </Localized>
            )}

            {selectedCurrency !== total.currency && (
              <Localized id="payment-receipt-currency-aria" attrs={{ 'aria-label': true }}>
              <div className="payment-receipt-currency" aria-label={l10n.getString('payment-receipt-currency-aria')}>
                <div className="payment-receipt-currency-row">
                  <Localized id="payment-charged-in">
                    <span>Charged in</span>
                  </Localized>
                  <span>{selectedCurrency}</span>
                </div>
                <div className="payment-receipt-currency-row">
                  <Localized id="payment-default-currency">
                    <span>Default currency</span>
                  </Localized>
                  <span>{baseCurrency}</span>
                </div>
                <div className="payment-receipt-currency-row">
                  <Localized id="payment-base-amount">
                    <span>Base amount</span>
                  </Localized>
                  <span>{formatMoney(total)}</span>
                </div>
                <div className="payment-receipt-currency-row">
                  <Localized id="payment-charge-amount">
                    <span>Charge amount</span>
                  </Localized>
                  <span>
                    {formatMoney({
                      minor_units: Math.round(total.minor_units * (exchangeRateInfo?.rate ?? 1)),
                      currency: selectedCurrency,
                    } as Money)}
                  </span>
                </div>
              </div>
              </Localized>
            )}

            {!splitMode && (
              <>
                <fieldset className="payment-methods">
                  <Localized id="payment-method-label">
                    <legend className="payment-section-title">Payment Method</legend>
                  </Localized>
                  <div className="payment-method-options">
                    {(['cash', 'card', 'qris', 'credit'] as const).map((m) => (
                      <label key={m} className="payment-method-label">
                        <input
                          type="radio"
                          name="payment-method"
                          value={m}
                          checked={method === m}
                          onChange={() => setMethod(m)}
                        />
                        <span className="payment-method-name">
                          {m === 'cash' ? l10n.getString('payment-method-cash') : m === 'card' ? l10n.getString('payment-method-card') : m === 'qris' ? l10n.getString('payment-method-qris') : 'Credit'}
                        </span>
                      </label>
                    ))}
                    <div className="payment-method-label">
                      <input
                        type="radio"
                        name="payment-method"
                        value="other"
                        checked={method === 'other'}
                        onChange={() => setMethod('other')}
                      />
                      <Localized id="payment-other-aria" attrs={{ 'aria-label': true }}>
                      <Localized id="payment-other-placeholder" attrs={{ placeholder: true }}>
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
                        />
                      </Localized>
                      </Localized>
                    </div>
                    <label className="payment-method-label">
                      <input
                        type="radio"
                        name="payment-method"
                        value="open_bill"
                        checked={method === 'open_bill'}
                        onChange={() => setMethod('open_bill')}
                      />
                      <span className="payment-method-name">
                        Open Bill
                      </span>
                    </label>
                  </div>
                </fieldset>

                {(method === 'open_bill' || method === 'credit') && (
                  <div className="payment-open-bill-section">
                    <label className="payment-customer-label">
                      {method === 'credit' ? (
                        <span>Customer Name (for credit)</span>
                      ) : (
                        <Localized id="payment-customer-name">
                          <span>Customer Name</span>
                        </Localized>
                      )}
                      <Localized id="payment-customer-name-aria" attrs={{ 'aria-label': true }}>
                      <input
                        type="text"
                        className="payment-customer-input"
                        placeholder="e.g. John Doe"
                        value={customerName}
                        onChange={(e) => setCustomerName(e.target.value)}
                      />
                      </Localized>
                    </label>
                  </div>
                )}

                {method === 'cash' && (
                  <div className="payment-cash-section">
                    <div className="payment-tendered-label">
                      <Localized id="payment-amount-tendered">
                        <span>Amount Tendered</span>
                      </Localized>
                      <Localized id="payment-tendered-input" attrs={{ placeholder: true, 'aria-label': true }}>
                        <input
                          type="text"
                          className="payment-tendered-input"
                          inputMode="decimal"
                          placeholder="0.00"
                          value={tendered}
                          onChange={(e) => setTendered(e.target.value)}
                        />
                      </Localized>
                    </div>

                    <div className="payment-quick-cash">
                      {(tenderPresets ?? [5000, 10000, 20000, 50000, 100000]).map((amount) => {
                        const totalNum = Number(total.minor_units) / 100;
                        const quickVal = Math.ceil(totalNum / amount) * amount;
                        return (
                          <Localized key={amount} id="payment-quick-tender-aria" attrs={{ 'aria-label': true }} vars={{ amount: quickVal.toFixed(2) }}>
                          <button
                            type="button"
                            className="payment-quick-btn"
                            onClick={() => setTendered(quickVal.toFixed(2))}
                          >
                            Rp {quickVal.toLocaleString('id-ID')}
                          </button>
                          </Localized>
                        );
                      })}
                      <Localized id="payment-tender-exact-aria" attrs={{ 'aria-label': true }}>
                      <button
                        type="button"
                        className="payment-quick-btn"
                        onClick={() => setTendered((Number(total.minor_units) / 100).toFixed(2))}
                      >
                        <Localized id="payment-tender-exact">
                          <span>Exact</span>
                        </Localized>
                      </button>
                      </Localized>
                    </div>

                    {tendered.length > 0 && (
                      <div className="payment-change-preview">
                        <Localized id="payment-change">
                          <span className="payment-change-label">Change</span>
                        </Localized>
                        <span
                          className={`payment-change-amount ${!sufficient ? 'payment-change-insufficient' : ''}`}
                        >
                          {sufficient
                            ? formatMoney(change!)
                            : l10n.getString('payment-insufficient')}
                        </span>
                      </div>
                    )}
                  </div>
                )}

                {method === 'qris' && (
                  <div className="payment-qris-section">
                    <Localized id="payment-qris-description">
                      <p className="payment-qris-description">
                        Generate a QRIS QR code for the customer to scan with their payment app.
                      </p>
                    </Localized>
                    <Localized id="payment-qris-btn-aria" attrs={{ 'aria-label': true }}>
                    <button
                      type="button"
                      className="payment-qris-btn"
                      onClick={handleQrPay}
                      disabled={processing}
                    >
                      <Localized id="payment-qris-pay">
                        <span>Pay with QR</span>
                      </Localized>
                    </button>
                    </Localized>
                  </div>
                )}
              </>
            )}

            {splitMode && (
              <div className="payment-split-section">
                <div className="payment-split-header">
                  <Localized id="payment-split-title">
                    <span className="payment-section-title">Split Payments</span>
                  </Localized>
                  <div className="payment-split-actions">
                    <Localized id="payment-split-evenly-aria" attrs={{ 'aria-label': true }}>
                    <button
                      type="button"
                      className="payment-split-btn"
                      onClick={autoSplitEvenly}
                    >
                      <Localized id="payment-split-evenly">
                        <span>Split Evenly</span>
                      </Localized>
                    </button>
                    </Localized>
                    <Localized id="payment-split-add-aria" attrs={{ 'aria-label': true }}>
                    <button
                      type="button"
                      className="payment-split-btn"
                      onClick={addSplit}
                    >
                      <Localized id="payment-split-add">
                        <span>+ Add Split</span>
                      </Localized>
                    </button>
                    </Localized>
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
                            <span>{m === 'cash' ? l10n.getString('payment-split-method-cash') : l10n.getString('payment-split-method-card')}</span>
                          </label>
                        ))}
                        <div className="payment-split-radio-label">
                          <input
                            type="radio"
                            name={`split-method-${s.id}`}
                            value="other"
                            checked={s.method === 'other'}
                            onChange={() => updateSplit(s.id, { method: 'other' })}
                          />
                          <Localized id="payment-split-other-aria" attrs={{ 'aria-label': true }}>
                          <Localized id="payment-split-other-placeholder" attrs={{ placeholder: true }}>
                            <input
                              type="text"
                              className="payment-split-other-input"
                              placeholder="Other"
                              value={s.otherLabel}
                              onChange={(e) => updateSplit(s.id, { otherLabel: e.target.value })}
                              disabled={s.method !== 'other'}
                            />
                          </Localized>
                          </Localized>
                        </div>
                      </div>
                      <div className="payment-split-amount-group">
                        <span className="payment-split-currency">$</span>
                        <Localized id="payment-split-amount-aria" attrs={{ 'aria-label': true }}>
                        <Localized id="payment-split-amount-placeholder" attrs={{ placeholder: true }}>
                          <input
                            type="text"
                            className="payment-split-amount-input"
                            inputMode="decimal"
                            placeholder="0.00"
                            value={s.amountMinor}
                            onChange={(e) => updateSplit(s.id, { amountMinor: e.target.value })}
                          />
                        </Localized>
                        </Localized>
                      </div>
                      <Localized id="payment-split-remove-aria" attrs={{ 'aria-label': true }}>
                      <button
                        type="button"
                        className="payment-split-remove"
                        onClick={() => removeSplit(s.id)}
                        disabled={splits.length <= 1}
                      >
                        &times;
                      </button>
                      </Localized>
                    </div>
                  ))}
                </div>

                <div className="payment-split-remaining">
                  <Localized id="payment-split-remaining">
                    <span className="payment-split-remaining-label">Remaining</span>
                  </Localized>
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
                {l10n.getString('payment-split-toggle')}
              </label>
            </div>

            <div className="payment-customer-section">
              {selectedCustomer ? (
                <div className="payment-customer-badge">
                  <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
                    <path d="M10 10a4 4 0 100-8 4 4 0 000 8zm-7 8a7 7 0 1114 0H3z" />
                  </svg>
                  <span className="payment-customer-name">{selectedCustomer.name}</span>
                  <button
                    type="button"
                    className="payment-customer-change"
                    onClick={() => setShowCustomerSearch(true)}
                  >
                    Change
                  </button>
                  <button
                    type="button"
                    className="payment-customer-remove"
                    onClick={() => notifyCustomerChange(null)}
                  >
                    &times;
                  </button>
                </div>
              ) : (
                <button
                  type="button"
                  className="payment-customer-select-btn"
                  onClick={() => setShowCustomerSearch(true)}
                >
                  <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
                    <path d="M10 10a4 4 0 100-8 4 4 0 000 8zm-7 8a7 7 0 1114 0H3z" />
                  </svg>
                  Select Customer
                </button>
              )}
            </div>

            {isEnabled(FEATURES.LOYALTY_PROGRAM) && loyaltyAccount && (
              <div className="payment-loyalty-section">
                <div className="payment-loyalty-balance">
                  <span className="payment-loyalty-label">
                    Points: {loyaltyAccount.account.points}
                  </span>
                  <span className="payment-loyalty-value">
                    {pointsWorthMinor !== null
                      ? `(${formatMoney({ minor_units: pointsWorthMinor, currency: total.currency } as Money)})`
                      : '…'}
                  </span>
                </div>
                {loyaltyAccount.account.points > 0 && !redeemPoints && (
                  <button
                    type="button"
                    className="payment-loyalty-redeem-btn"
                    onClick={() => {
                      setRedeemPoints(true);
                      setPointsToRedeem(loyaltyAccount.account.points);
                    }}
                  >
                    Use Points
                  </button>
                )}
                {redeemPoints && (
                  <div className="payment-loyalty-active">
                    <div className="payment-loyalty-input-row">
                      <span className="payment-loyalty-input-label">Points</span>
                      <input
                        type="number"
                        className="payment-loyalty-input"
                        value={pointsToRedeem}
                        onChange={(e) => setPointsToRedeem(Math.max(0, parseInt(e.target.value, 10) || 0))}
                        min={0}
                        max={loyaltyAccount.account.points}
                        aria-label="Points"
                      />
                      <span className="payment-loyalty-input-hint">
                        / {loyaltyAccount.account.points}
                      </span>
                    </div>
                    <span className="payment-loyalty-discount-label">
                      Discount: -{formatMoney({
                        minor_units: Number(loyaltyDiscount),
                        currency: total.currency,
                      } as Money)}
                    </span>
                    <button
                      type="button"
                      className="payment-loyalty-cancel-btn"
                      onClick={() => {
                        setRedeemPoints(false);
                        setPointsToRedeem(0);
                        setLoyaltyDiscount(0n);
                      }}
                    >
                      Cancel
                    </button>
                  </div>
                )}
              </div>
            )}

            {showCustomerSearch && (
              <div className="payment-customer-search-overlay" role="button" tabIndex={0} onClick={() => setShowCustomerSearch(false)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowCustomerSearch(false); } }}>
                <div className="payment-customer-search-modal" role="button" tabIndex={0} onClick={(e) => e.stopPropagation()} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); } }}>
                  <h3 className="payment-customer-search-heading">Select Customer</h3>
                  <input
                    className="payment-customer-search-input"
                    type="text"
                    placeholder="Search by name, phone, or email..."
                    value={customerSearchQuery}
                    onChange={(e) => setCustomerSearchQuery(e.target.value)}
                  />
                  <div className="payment-customer-search-list">
                    {loadingCustomers ? (
                      <div className="payment-customer-search-loading">Loading...</div>
                    ) : customerSearchResults.length === 0 ? (
                      <div className="payment-customer-search-empty">No customers found</div>
                    ) : (
                      customerSearchResults.map((c) => (
                        <button
                          key={c.id}
                          className="payment-customer-search-item"
                          onClick={() => {
                            notifyCustomerChange(c);
                            setShowCustomerSearch(false);
                            setCustomerSearchQuery('');
                          }}
                        >
                          <span className="payment-customer-search-item-name">{c.name}</span>
                          {(c.phone || c.email) && (
                            <span className="payment-customer-search-item-detail">
                              {c.phone || c.email}
                            </span>
                          )}
                        </button>
                      ))
                    )}
                  </div>
                  <button
                    className="payment-customer-search-close"
                    onClick={() => setShowCustomerSearch(false)}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            )}

            <div className="payment-actions">
              <Localized id="payment-cancel">
                <Button variant="ghost" onClick={() => animateLeave(onClose)} disabled={processing}>
                  Cancel
                </Button>
              </Localized>
              <Button
                variant="primary"
                loading={processing}
                disabled={!canComplete}
                onClick={complete}
              >
                {method === 'open_bill' ? (
                  <Localized id="payment-open-bill">
                    <span>Open Bill</span>
                  </Localized>
                ) : method === 'credit' ? (
                  <span>Credit Sale</span>
                ) : (
                  <Localized id="payment-complete">
                    <span>Complete Sale</span>
                  </Localized>
                )}
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
    </Localized>
  );
}
