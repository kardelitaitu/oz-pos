import { useState, useCallback } from 'react';
import { processRefund, type SaleDetail } from '@/api/sales';
import { useAuth } from '@/contexts/AuthContext';
import { formatMoney, type Money } from '@/types/domain';
import { Button } from '@/components/Button';
import './RefundModal.css';

interface RefundModalProps {
  open: boolean;
  sale: SaleDetail;
  onClose: () => void;
  onRefunded: () => void;
}

export default function RefundModal({ open, sale, onClose, onRefunded }: RefundModalProps) {
  const { session } = useAuth();
  const [selectedLines, setSelectedLines] = useState<Record<string, number>>({});
  const [reason, setReason] = useState('');
  const [note, setNote] = useState('');
  const [processing, setProcessing] = useState(false);
  const [result, setResult] = useState<{ refundId: string; totalMinor: number } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const toggleLine = useCallback((lineId: string, _sku: string, maxQty: number) => {
    setSelectedLines((prev) => {
      const current = prev[lineId] ?? 0;
      if (current > 0) {
        const next = { ...prev };
        delete next[lineId];
        return next;
      }
      return { ...prev, [lineId]: maxQty };
    });
  }, []);

  const updateQty = useCallback((lineId: string, qty: number) => {
    setSelectedLines((prev) => {
      if (qty <= 0) {
        const next = { ...prev };
        delete next[lineId];
        return next;
      }
      return { ...prev, [lineId]: qty };
    });
  }, []);

  const totalRefund = sale.lines.reduce((sum, line) => {
    const qty = selectedLines[line.id] ?? 0;
    return sum + (line.total_minor ?? 0) * qty / (line.qty ?? 1);
  }, 0);

  const hasSelection = Object.values(selectedLines).some((q) => q > 0);

  const handleRefund = useCallback(async () => {
    if (!session || !hasSelection || !reason.trim()) return;
    setProcessing(true);
    setError(null);
    try {
      const lines = sale.lines
        .filter((l) => (selectedLines[l.id] ?? 0) > 0)
        .map((l) => {
          const qty = selectedLines[l.id]!;
          const unitPriceMinor = Math.round((l.total_minor ?? 0) / (l.qty ?? 1));
          return {
            saleLineId: l.id,
            sku: l.sku,
            qty,
            unitPriceMinor,
            currency: sale.total.currency,
            lineTotalMinor: unitPriceMinor * qty,
          };
        });
      const res = await processRefund({
        saleId: sale.id,
        reason: reason.trim(),
        note: note.trim() || null,
        userId: session.user_id,
        lines,
      });
      setResult(res);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Refund failed');
    } finally {
      setProcessing(false);
    }
  }, [session, hasSelection, reason, note, selectedLines, sale]);

  const handleDone = useCallback(() => {
    onRefunded();
    onClose();
  }, [onRefunded, onClose]);

  if (!open) return null;

  return (
    <div className="refund-overlay" role="dialog" aria-modal="true" aria-label="Process refund">
      <div className="refund-modal">
        {result ? (
          <div className="refund-done">
            <h2 className="refund-done-title">Refund Processed</h2>
            <p className="refund-done-amount">
              Refunded: {formatMoney({ minor_units: result.totalMinor, currency: sale.total.currency } as Money)}
            </p>
            <Button variant="primary" onClick={handleDone}>
              Done
            </Button>
          </div>
        ) : (
          <>
            <div className="refund-header">
              <h2 className="refund-title">Process Refund</h2>
              <button type="button" className="refund-close" onClick={onClose} aria-label="Cancel refund">
                &times;
              </button>
            </div>

            <div className="refund-sale-info">
              <span>Sale: {sale.id.slice(0, 8)}&hellip;</span>
              <span>Total: {formatMoney(sale.total as Money)}</span>
              <span>Date: {new Date(sale.createdAt).toLocaleDateString()}</span>
            </div>

            <div className="refund-lines">
              <h3 className="refund-section-title">Select Items to Refund</h3>
              {sale.lines.map((line) => {
                const selectedQty = selectedLines[line.id] ?? 0;
                return (
                  <div key={line.id} className={`refund-line ${selectedQty > 0 ? 'refund-line-selected' : ''}`}>
                    <label className="refund-line-label">
                      <input
                        type="checkbox"
                        checked={selectedQty > 0}
                        onChange={() => toggleLine(line.id, line.sku, line.qty ?? 1)}
                        aria-label={`Refund ${line.sku}`}
                      />
                      <span className="refund-line-sku">{line.sku}</span>
                      <span className="refund-line-name">{line.name ?? line.sku}</span>
                    </label>
                    {selectedQty > 0 && (
                      <div className="refund-line-qty">
                        <button
                          type="button"
                          className="refund-qty-btn"
                          onClick={() => updateQty(line.id, selectedQty - 1)}
                          disabled={selectedQty <= 1}
                          aria-label="Decrease refund quantity"
                        >−</button>
                        <span className="refund-qty-value">{selectedQty}</span>
                        <button
                          type="button"
                          className="refund-qty-btn"
                          onClick={() => updateQty(line.id, selectedQty + 1)}
                          disabled={selectedQty >= (line.qty ?? 1)}
                          aria-label="Increase refund quantity"
                        >+</button>
                        <span className="refund-line-total">
                          {formatMoney({
                            minor_units: Math.round((line.total_minor ?? 0) * selectedQty / (line.qty ?? 1)),
                            currency: sale.total.currency,
                          } as Money)}
                        </span>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>

            <div className="refund-details">
              <label className="refund-field">
                <span>Reason *</span>
                <input
                  type="text"
                  className="refund-input"
                  value={reason}
                  onChange={(e) => setReason(e.target.value)}
                  placeholder="e.g. Customer changed mind"
                  aria-label="Refund reason"
                />
              </label>
              <label className="refund-field">
                <span>Note (internal)</span>
                <input
                  type="text"
                  className="refund-input"
                  value={note}
                  onChange={(e) => setNote(e.target.value)}
                  placeholder="Optional internal note"
                  aria-label="Refund note"
                />
              </label>
            </div>

            {error && <div className="refund-error">{error}</div>}

            <div className="refund-total-row">
              <span className="refund-total-label">Refund Total</span>
              <span className="refund-total-amount">
                {formatMoney({ minor_units: totalRefund, currency: sale.total.currency } as Money)}
              </span>
            </div>

            <div className="refund-actions">
              <Button variant="ghost" onClick={onClose} disabled={processing}>
                Cancel
              </Button>
              <Button
                variant="primary"
                loading={processing}
                disabled={!hasSelection || !reason.trim()}
                onClick={handleRefund}
              >
                Process Refund
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
