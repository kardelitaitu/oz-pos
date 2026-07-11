import { useState, useCallback } from 'react';
import { useLocalization } from '@fluent/react';
import { getGiftCardBalance, redeemGiftCard } from '@/api/giftCards';
import { Button } from '@/components/Button';


export interface GiftCardPaymentProps {
  /** Total amount to pay in minor units. */
  totalMinor: number;
  /** Currency code. */
  currency: string;
  /** The current sale id. */
  saleId: string;
  /** Called with the amount that the gift card should cover (minor units). */
  onApplied: (amountMinor: number, cardNumber: string) => void;
  /** Called to show an error / toast message. */
  onError: (message: string) => void;
  /** Called to close / cancel the gift card flow. */
  onCancel: () => void;
  /** Called when redemption is complete. */
  onComplete: () => void;
}

/** Gift card payment flow — look up gift card number, check balance, and apply gift card amount toward the current sale. */
export default function GiftCardPayment({
  totalMinor,
  currency,
  saleId,
  onApplied,
  onError,
  onCancel,
  onComplete,
}: GiftCardPaymentProps) {
  const { l10n } = useLocalization();
  const [cardInput, setCardInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [cardBalance, setCardBalance] = useState<{ minor: number; currency: string; status: string } | null>(null);
  const [error, setError] = useState('');

  const handleLookup = useCallback(async () => {
    const code = cardInput.trim();
    if (!code) {
      setError(l10n.getString('gift-cards-payment-enter-number'));
      return;
    }
    setLoading(true);
    setError('');
    try {
      const result = await getGiftCardBalance(code);
      if (!result) {
        setError(l10n.getString('gift-cards-payment-not-found'));
        setCardBalance(null);
        return;
      }
      if (result.status !== 'active') {
        setError(l10n.getString('gift-cards-payment-not-active'));
        setCardBalance(null);
        return;
      }
      setCardBalance({ minor: result.balance_minor, currency: result.currency, status: result.status });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Lookup failed');
    } finally {
      setLoading(false);
    }
  }, [cardInput, l10n]);

  const handleApply = useCallback(async () => {
    const code = cardInput.trim();
    if (!code || !cardBalance) return;

    const amountToRedeem = Math.min(totalMinor, cardBalance.minor);
    if (amountToRedeem <= 0) {
      onError(l10n.getString('gift-cards-payment-insufficient'));
      return;
    }

    setLoading(true);
    try {
      await redeemGiftCard(code, amountToRedeem, saleId);
      onApplied(amountToRedeem, code);
      onComplete();
    } catch (err) {
      onError(err instanceof Error ? err.message : 'Redemption failed');
    } finally {
      setLoading(false);
    }
  }, [cardInput, cardBalance, totalMinor, saleId, onApplied, onComplete, onError, l10n]);

  const formatMoney = (minor: number, cur: string): string => {
    const known: Record<string, number> = { JPY: 0, KRW: 0, VND: 0, IDR: 2 };
    const exp = known[cur] ?? 2;
    const val = (minor / 10 ** exp).toLocaleString(undefined, {
      minimumFractionDigits: exp,
      maximumFractionDigits: exp,
    });
    return `${cur} ${val}`;
  };

  return (
    <div className="gift-card-payment">
      <div className="gift-card-payment-header">
        <h3 className="gift-card-payment-title">Gift Card</h3>
        <p className="gift-card-payment-subtitle">
          Total due: {formatMoney(totalMinor, currency)}
        </p>
      </div>

      <div className="gift-card-payment-input-row">
        <input
          type="text"
          className="gift-card-payment-input"
          placeholder="Scan or enter gift card number"
          value={cardInput}
          onChange={(e) => { setCardInput(e.target.value); setCardBalance(null); setError(''); }}
          disabled={loading}
          aria-label="Gift card number"
        />
        <Button variant="primary" onClick={handleLookup} disabled={loading || !cardInput.trim()}>
          Check
        </Button>
      </div>

      {error && <div className="gift-card-payment-error">{error}</div>}

      {cardBalance && (
        <div className="gift-card-payment-balance">
          <div className="gift-card-payment-balance-row">
            <span>Available Balance</span>
            <span className="gift-card-payment-balance-amount">
              {formatMoney(cardBalance.minor, cardBalance.currency)}
            </span>
          </div>
          <div className="gift-card-payment-balance-row">
            <span>To Apply</span>
            <span className="gift-card-payment-balance-amount">
              {formatMoney(Math.min(totalMinor, cardBalance.minor), currency)}
            </span>
          </div>
        </div>
      )}

      <div className="gift-card-payment-actions">
        <Button variant="ghost" onClick={onCancel} disabled={loading}>
          Cancel
        </Button>
        {cardBalance && (
          <Button variant="primary" onClick={handleApply} disabled={loading || cardBalance.minor <= 0}>
            {loading ? 'Processing...' : 'Apply Gift Card'}
          </Button>
        )}
      </div>
    </div>
  );
}
