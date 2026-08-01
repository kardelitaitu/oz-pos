import { useState, useEffect, useCallback } from 'react';
import { Button } from '@/components/Button';
import { ConfirmDialog } from '@/components/ConfirmDialog';
import { Localized, useLocalization } from '@fluent/react';
import { useToast } from '@/frontend/shared/Toast';
import { requiredLocalized } from '@/frontend/shared';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { listStockTransfers, getStockTransferLines, cancelStockTransfer, type StockTransfer, type StockTransferLine } from '@/api/stockTransfers';
import './TransitAuditScreen.css';

interface TransferWithLines {
  transfer: StockTransfer;
  lines: StockTransferLine[];
}

const TRANSIT_EXPIRY_HOURS = 24;

export default function TransitAuditScreen() {
  const [transfers, setTransfers] = useState<TransferWithLines[]>([]);
  const [loading, setLoading] = useState(true);
  const [reverseConfirmId, setReverseConfirmId] = useState<string | null>(null);
  const { addToast } = useToast();
  const { l10n } = useLocalization();
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken || '';

  const loadTransfers = useCallback(async () => {
    setLoading(true);
    try {
      const allTransfers = await listStockTransfers(sessionToken);
      const inTransit = allTransfers.filter(t => t.status === 'in_transit');
      
      const enriched = await Promise.all(
        inTransit.map(async (transfer) => {
          const lines = await getStockTransferLines(sessionToken, transfer.id);
          return { transfer, lines };
        })
      );
      setTransfers(enriched);
    } catch (err) {
      addToast({ message: err instanceof Error ? err.message : (requiredLocalized(l10n, 'inv-transit-error-load')), type: 'error' });
    } finally {
      setLoading(false);
    }
  }, [addToast, l10n, sessionToken]);

  useEffect(() => {
    loadTransfers();
  }, [loadTransfers]);

  const handleReverseClick = (id: string) => {
    setReverseConfirmId(id);
  };

  const handleReverseConfirm = async () => {
    if (!reverseConfirmId) return;
    try {
      await cancelStockTransfer(sessionToken, reverseConfirmId);
      setReverseConfirmId(null);
      await loadTransfers();
      addToast({ message: requiredLocalized(l10n, 'inv-transit-reversed-toast'), type: 'success' });
    } catch (err) {
      addToast({ message: err instanceof Error ? err.message : (requiredLocalized(l10n, 'inv-transit-error-reverse')), type: 'error' });
    }
  };

  const isOverdue = (sentAt: string | null) => {
    if (!sentAt) return false;
    const sentTime = new Date(sentAt).getTime();
    const now = Date.now();
    const diffHours = (now - sentTime) / 3600000;
    return diffHours > TRANSIT_EXPIRY_HOURS;
  };

  if (loading) {
    return (
      <div className="transit-audit-container">
        <div className="transit-empty">
          <Localized id="inv-loading">
            <span>Loading...</span>
          </Localized>
        </div>
      </div>
    );
  }

  return (
    <div className="transit-audit-container">
      <div className="transit-header">
        <Localized id="inv-transit-title">
          <h2 className="transit-title">Transit Stock Audit</h2>
        </Localized>
      </div>

      {transfers.length === 0 ? (
        <div className="transit-empty">
          <Localized id="inv-transit-no-overdue">
            <span>No transfers in transit.</span>
          </Localized>
        </div>
      ) : (
        <div className="transit-grid">
          {transfers.map(({ transfer, lines }) => {
            const overdue = isOverdue(transfer.sent_at);
            return (
              <div key={transfer.id} className={`transit-card ${overdue ? 'overdue' : ''}`}>
                <div className="transit-meta">
                  <div>
                    <span><Localized id="inv-transit-transfer-label">Transfer #</Localized></span>
                    <strong>{transfer.transfer_number}</strong>
                  </div>
                  <div>
                    <Localized id="inv-transit-col-source">
                      <span>Source</span>
                    </Localized>
                    : <strong>{transfer.source_location || 'Warehouse'}</strong>
                  </div>
                  <div>
                    <Localized id="inv-transit-col-dest">
                      <span>Destination</span>
                    </Localized>
                    : <strong>{transfer.destination_location || 'Store Front'}</strong>
                  </div>
                  <div>
                    <Localized id="inv-transit-col-sent">
                      <span>Sent At</span>
                    </Localized>
                    : <strong>{transfer.sent_at ? new Date(transfer.sent_at).toLocaleString() : (requiredLocalized(l10n, 'inv-transit-unknown'))}</strong>
                  </div>
                </div>

                <table className="transit-lines-table">
                  <thead>
                    <tr>
                      <Localized id="inv-transit-col-sku">
                        <th>SKU</th>
                      </Localized>
                      <Localized id="inv-transit-col-product">
                        <th>Product</th>
                      </Localized>
                      <Localized id="inv-transit-col-qty">
                        <th>Qty</th>
                      </Localized>
                    </tr>
                  </thead>
                  <tbody>{lines.map(line => (
                      <tr key={line.id}>
                        <td>{line.sku}</td>
                        <td>{line.product_name}</td>
                        <td>{line.qty}</td>
                      </tr>
                    ))}
</tbody>
                </table>

                <div className="transit-actions">
                  <Button variant="danger" size="sm" className="reverse-btn" onClick={() => handleReverseClick(transfer.id)}>
                    <Localized id="inv-transit-reverse-btn">
                      <span>Reverse Transfer</span>
                    </Localized>
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      <ConfirmDialog
        open={reverseConfirmId !== null}
        onCancel={() => setReverseConfirmId(null)}
        onConfirm={handleReverseConfirm}
        title={requiredLocalized(l10n, 'inv-transit-reverse-title')}
        message={requiredLocalized(l10n, 'inv-transit-reverse-message')}
        variant="danger"
        confirmLabel={requiredLocalized(l10n, 'inv-transit-reverse-confirm')}
      />
    </div>
  );
}
