import { useState, useCallback, useEffect, useRef } from 'react';
import { staffLogin } from '@/api/staff';
import { formatMoney, type Money } from '@/types/domain';

/** Props for the PriceOverrideModal — requires staff PIN verification before applying a manual price change. */
export interface PriceOverrideModalProps {
  open: boolean;
  lineDescription: string;
  currentPrice: Money;
  onConfirm: (newPriceMinor: number, userId: string) => Promise<void>;
  onClose: () => void;
}

/** Price override modal — two-step flow: enter new price, then authenticate with staff username + PIN before applying. */
export default function PriceOverrideModal({
  open,
  lineDescription,
  currentPrice,
  onConfirm,
  onClose,
}: PriceOverrideModalProps) {
  const [step, setStep] = useState<'price' | 'username' | 'pin'>('price');
  const [newPriceMinor, setNewPriceMinor] = useState<number>(currentPrice.minor_units);
  const [username, setUsername] = useState('');
  const [pin, setPin] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const usernameInputRef = useRef<HTMLInputElement>(null);
  const pinSubmitted = useRef(false);

  const MAX_PIN_LENGTH = 4;

  useEffect(() => {
    if (step === 'username') {
      usernameInputRef.current?.focus();
    }
  }, [step]);

  const attemptVerify = useCallback(async () => {
    if (pin.length === 0) return;
    setLoading(true);
    setError(null);
    try {
      const result = await staffLogin({ username: username.trim(), pin: pin.join('') });
      await onConfirm(newPriceMinor, result.session.user_id);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'PIN verification failed';
      setError(message);
      setPin([]);
      pinSubmitted.current = false;
    } finally {
      setLoading(false);
    }
  }, [pin, username, newPriceMinor, onConfirm]);

  useEffect(() => {
    if (pin.length === MAX_PIN_LENGTH && !loading && !pinSubmitted.current) {

      pinSubmitted.current = true;
      attemptVerify();
    }
    if (pin.length < MAX_PIN_LENGTH) {
      pinSubmitted.current = false;
    }
  }, [pin, loading, attemptVerify]);


  const handlePriceConfirm = useCallback(() => {
    if (newPriceMinor > 0) {
      setStep('username');
      setError(null);
    }
  }, [newPriceMinor]);

  const handleUsernameSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    if (username.trim()) {
      setStep('pin');
      setError(null);
    }
  }, [username]);

  const handlePinDigit = useCallback((digit: string) => {
    setPin((prev) => {
      if (prev.length >= MAX_PIN_LENGTH) return prev;
      return [...prev, digit];
    });
  }, []);

  const handlePinBackspace = useCallback(() => {
    setPin((prev) => prev.slice(0, -1));
  }, []);

  const handlePinClear = useCallback(() => {
    setPin([]);
    pinSubmitted.current = false;
  }, []);


  const handleGoBack = useCallback(() => {
    if (step === 'username') {
      setStep('price');
    } else if (step === 'pin') {
      setStep('username');
      setPin([]);
      pinSubmitted.current = false;
    }
  }, [step]);

  if (!open) return null;

  const renderPinDots = (length: number) => (
    <div className="price-override-pin-dots" aria-label={`PIN entry: ${length} of ${MAX_PIN_LENGTH} digits`}>
      {Array.from({ length: MAX_PIN_LENGTH }, (_, i) => (
        <span
          key={i}
          className={`price-override-pin-dot ${i < length ? 'price-override-pin-dot--filled' : ''}`}
          aria-hidden="true"
        />
      ))}
    </div>
  );

  const renderPinPad = () => (
    <div className="price-override-pin-pad" role="group" aria-label="Numeric keypad">
      {[7, 8, 9].map((d) => (
        <button key={d} className="price-override-pin-key" onClick={() => handlePinDigit(String(d))} disabled={loading}>{d}</button>
      ))}
      {[4, 5, 6].map((d) => (
        <button key={d} className="price-override-pin-key" onClick={() => handlePinDigit(String(d))} disabled={loading}>{d}</button>
      ))}
      {[1, 2, 3].map((d) => (
        <button key={d} className="price-override-pin-key" onClick={() => handlePinDigit(String(d))} disabled={loading}>{d}</button>
      ))}
      <button className="price-override-pin-key price-override-pin-key--clear" onClick={handlePinClear} disabled={loading || pin.length === 0}>Clear</button>
      <button className="price-override-pin-key" onClick={() => handlePinDigit('0')} disabled={loading}>0</button>
      <button className="price-override-pin-key price-override-pin-key--backspace" onClick={handlePinBackspace} disabled={loading || pin.length === 0}>⌫</button>
    </div>
  );

  return (
    <div className="price-override-overlay" role="dialog" aria-modal="true" aria-label="Price override">
      <div className="price-override-modal">
        <button
          type="button"
          className="price-override-close"
          onClick={onClose}
          aria-label="Close"
        >
          &times;
        </button>

        <h2 className="price-override-title">Price Override</h2>
        <p className="price-override-item">{lineDescription}</p>

        {step === 'price' && (
          <div className="price-override-price-step">
            <div className="price-override-current">
              <span className="price-override-current-label">Current price</span>
              <span className="price-override-current-value">{formatMoney(currentPrice)}</span>
            </div>
            <label className="price-override-new-label" htmlFor="price-override-input">
              New price (in minor units)
            </label>
            <input
              id="price-override-input"
              className="price-override-input"
              type="number"
              min="1"
              value={newPriceMinor}
              onChange={(e) => setNewPriceMinor(Math.max(1, parseInt(e.target.value, 10) || 0))}
              aria-label="Enter new price in minor units"
            />
            <div className="price-override-actions">
              <button type="button" className="price-override-cancel-btn" onClick={onClose}>Cancel</button>
              <button
                type="button"
                className="price-override-next-btn"
                onClick={handlePriceConfirm}
                disabled={newPriceMinor <= 0}
              >
                Next
              </button>
            </div>
          </div>
        )}

        {step === 'username' && (
          <form onSubmit={handleUsernameSubmit} className="price-override-username-step">
            <p className="price-override-step-label">Enter manager username</p>
            <input
              ref={usernameInputRef}
              className="price-override-username-input"
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Username"
              autoComplete="username"
              aria-label="Manager username"
              disabled={loading}
            />
            {error && <div className="price-override-error" role="alert">{error}</div>}
            <div className="price-override-actions">
              <button type="button" className="price-override-cancel-btn" onClick={handleGoBack} disabled={loading}>Back</button>
              <button
                type="submit"
                className="price-override-next-btn"
                disabled={!username.trim() || loading}
              >
                Next
              </button>
            </div>
          </form>
        )}

        {step === 'pin' && (
          <div className="price-override-pin-step">
            <p className="price-override-step-label">Enter manager PIN</p>
            {renderPinDots(pin.length)}
            {renderPinPad()}
            {error && <div className="price-override-error" role="alert">{error}</div>}
            {loading && <div className="price-override-loading" role="status">Verifying…</div>}
            <div className="price-override-actions">
              <button type="button" className="price-override-cancel-btn" onClick={handleGoBack} disabled={loading}>Back</button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
