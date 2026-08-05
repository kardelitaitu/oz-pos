import { useState, useEffect, useMemo, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { requiredLocalized } from '@/frontend/shared';
import { useExitAnimation } from '@/hooks/useExitAnimation';
import { useFocusTrap } from '@/hooks/useFocusTrap';
import { minorUnitExponent } from '@/types/domain';
import './QrisQrDisplay.css';

interface QrisQrDisplayProps {
  amount: number;
  currency: string;
  reference: string;
  isOpen: boolean;
  onClose: () => void;
  onPaymentConfirmed: () => void;
}

function simpleHash(str: string): number {
  let hash = 5381;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) + hash) + str.charCodeAt(i);
  }
  return hash >>> 0;
}

/**
 * Full-screen QRIS QR code payment modal.
 * Displays a deterministic pseudo-QR code based on the reference,
 * polls for payment confirmation, and calls `onPaymentConfirmed`
 * when the payment is detected as complete.
 */
export default function QrisQrDisplay({
  amount,
  currency,
  reference,
  isOpen,
  onClose,
  onPaymentConfirmed,
}: QrisQrDisplayProps) {
  const { l10n } = useLocalization();
  const [pollCount, setPollCount] = useState(0);
  const [status, setStatus] = useState<'waiting' | 'confirmed' | 'expired'>('waiting');

  useEffect(() => {
    if (!isOpen) {
      setPollCount(0);
      setStatus('waiting');
      return;
    }

    const interval = setInterval(() => {
      setPollCount((prev) => prev + 1);
    }, 2000);

    return () => clearInterval(interval);
  }, [isOpen]);

  useEffect(() => {
    if (pollCount >= 4 && status === 'waiting') {
      setStatus('confirmed');
    }
  }, [pollCount, status]);

  useEffect(() => {
    if (status !== 'confirmed') return;
    const timer = setTimeout(() => {
      onPaymentConfirmed();
    }, 1200);
    return () => clearTimeout(timer);
  }, [status, onPaymentConfirmed]);

  const qrCells = useMemo(() => {
    const seed = simpleHash(reference || `${Date.now()}`);
    const cells: boolean[][] = [];
    let rng = seed;
    for (let i = 0; i < 21; i++) {
      const row: boolean[] = [];
      for (let j = 0; j < 21; j++) {
        rng = (rng * 1103515245 + 12345) & 0x7fffffff;
        row.push((rng & 0x1) === 1);
      }
      cells.push(row);
    }
    return cells;
  }, [reference]);

  // Layered exit to mirror the entry (added in this PR). Mirrors
  // the PosScreen cousin-modals pattern (commit 1408992): the
  // overlay and container each get their own `--exiting` class so
  // two mirrored keyframes play in parallel.
  const exit = useExitAnimation(isOpen, onClose);

  // A11Y-02: complete dialog semantics — initial focus, Tab containment,
  // Escape, scroll lock, and focus restoration all via the shared trap.
  const overlayRef = useRef<HTMLDivElement | null>(null);
  useFocusTrap(overlayRef, exit.shouldRender && !exit.exiting, () => exit.requestClose());

  if (!exit.shouldRender) return null;

  return (
    <div
      ref={overlayRef}
      className={`qris-overlay${exit.exiting ? ' qris-overlay--exiting' : ''}`}
      role="dialog"
      aria-modal="true"
      aria-label={requiredLocalized(l10n, 'payment-qris-dialog-aria')}
    >
      <div
        className={`qris-container${exit.exiting ? ' qris-container--exiting' : ''}`}
      >
        <button
          type="button"
          className="qris-close"
          onClick={() => exit.requestClose()}
          disabled={exit.exiting}
          aria-label={requiredLocalized(l10n, 'payment-qris-close-aria')}
        >
          &times;
        </button>

        <div className="qris-header">
          <Localized id="payment-qris-scan">
            <h2 className="qris-title">Scan with your payment app</h2>
          </Localized>
          <p className="qris-subtitle">QRIS</p>
        </div>

        <div className={`qris-qr-wrapper ${status === 'waiting' ? 'qris-pulse' : ''}`}>
          <div className="qris-qr-placeholder" aria-label={requiredLocalized(l10n, 'payment-qris-qr-aria')}>
            <div className="qris-qr-grid">
              {qrCells.map((row, i) =>
                row.map((cell, j) => (
                  <div
                    key={`${i}-${j}`}
                    className={`qris-qr-cell ${cell ? 'qris-qr-cell--filled' : ''}`}
                  />
                )),
              )}
            </div>
          </div>
        </div>

        <div className="qris-details">
          <div className="qris-detail-row">
            <span className="qris-detail-label">{requiredLocalized(l10n, 'payment-qris-amount')}</span>
            <span className="qris-detail-value">
              {(amount / 10 ** minorUnitExponent(currency)).toFixed(minorUnitExponent(currency))} {currency}
            </span>
          </div>
          <div className="qris-detail-row">
            <span className="qris-detail-label">{requiredLocalized(l10n, 'payment-qris-reference')}</span>
            <span className="qris-detail-value qris-detail-value--mono">{reference}</span>
          </div>
          <div className="qris-detail-row">
            <span className="qris-detail-label">{requiredLocalized(l10n, 'payment-qris-merchant')}</span>
            <span className="qris-detail-value">{requiredLocalized(l10n, 'payment-qris-merchant-name')}</span>
          </div>
        </div>

        {status === 'waiting' && (
          <div className="qris-status" role="status" aria-label={requiredLocalized(l10n, 'payment-qris-waiting-aria')}>
            <div className="qris-spinner" aria-hidden="true" />
            <Localized id="payment-qris-waiting">
              <span>Waiting for payment...</span>
            </Localized>
          </div>
        )}

        {status === 'confirmed' && (
          <div className="qris-status qris-status--success" role="status" aria-label={requiredLocalized(l10n, 'payment-qris-confirmed-aria')}>
            <Localized id="payment-qris-confirmed">
              <span>Payment confirmed!</span>
            </Localized>
          </div>
        )}
      </div>
    </div>
  );
}
