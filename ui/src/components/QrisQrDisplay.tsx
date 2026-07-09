import { useState, useEffect, useMemo } from 'react';
import { useExitAnimation } from '@/hooks/useExitAnimation';
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

export default function QrisQrDisplay({
  amount,
  currency,
  reference,
  isOpen,
  onClose,
  onPaymentConfirmed,
}: QrisQrDisplayProps) {
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

  if (!exit.shouldRender) return null;

  return (
    <div
      className={`qris-overlay${exit.exiting ? ' qris-overlay--exiting' : ''}`}
      role="dialog"
      aria-modal="true"
      aria-label="QRIS QR payment"
    >
      <div
        className={`qris-container${exit.exiting ? ' qris-container--exiting' : ''}`}
      >
        <button
          type="button"
          className="qris-close"
          onClick={() => exit.requestClose()}
          disabled={exit.exiting}
          aria-label="Close QR payment"
        >
          &times;
        </button>

        <div className="qris-header">
          <h2 className="qris-title">Scan with your payment app</h2>
          <p className="qris-subtitle">QRIS</p>
        </div>

        <div className={`qris-qr-wrapper ${status === 'waiting' ? 'qris-pulse' : ''}`}>
          <div className="qris-qr-placeholder" aria-label="QR code">
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
            <span className="qris-detail-label">Amount</span>
            <span className="qris-detail-value">{(amount / 100).toFixed(2)} {currency}</span>
          </div>
          <div className="qris-detail-row">
            <span className="qris-detail-label">Reference</span>
            <span className="qris-detail-value qris-detail-value--mono">{reference}</span>
          </div>
          <div className="qris-detail-row">
            <span className="qris-detail-label">Merchant</span>
            <span className="qris-detail-value">OZ-POS Store</span>
          </div>
        </div>

        {status === 'waiting' && (
          <div className="qris-status" role="status" aria-label="Waiting for payment">
            <div className="qris-spinner" aria-hidden="true" />
            <span>Waiting for payment...</span>
          </div>
        )}

        {status === 'confirmed' && (
          <div className="qris-status qris-status--success" role="status" aria-label="Payment confirmed">
            <span>Payment confirmed!</span>
          </div>
        )}
      </div>
    </div>
  );
}
