import { useState, useEffect, useCallback } from 'react';
import { Button } from '@/components/Button';
import { ConfirmDialog } from '@/components/ConfirmDialog';
import { Localized, useLocalization } from '@fluent/react';
import { useToast } from '@/frontend/shared/Toast';
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

  const loadTransfers = useCallback(async () => {
    setLoading(true);
    try {
      const allTransfers = await listStockTransfers();
      const inTransit = allTransfers.filter(t => t.status === 'in_transit');
      
      const enriched = await Promise.all(
        inTransit.map(async (transfer) => {
          const lines = await getStockTransferLines(transfer.id);
          return { transfer, lines };
        })
      );
      setTransfers(enriched);
    } catch (err) {
      addToast({ message: err instanceof Error ? err.message : (l10n.getString('inv-transit-error-load') || 'Failed to load transit stock'), type: 'error' });
    } finally {
      setLoading(false);
    }
  }, [addToast, l10n]);

  useEffect(() => {
    loadTransfers();
  }, [loadTransfers]);

  const handleReverseClick = (id: string) => {
    setReverseConfirmId(id);
  };

  const handleReverseConfirm = async () => {
    if (!reverseConfirmId) return;
    try {
      await cancelStockTransfer(reverseConfirmId);
      setReverseConfirmId(null);
      await loadTransfers();
      addToast({ message: l10n.getString('inv-transit-reversed-toast') || 'Stock transfer reversed successfully', type: 'success' });
    } catch (err) {
      addToast({ message: err instanceof Error ? err.message : (l10n.getString('inv-transit-error-reverse') || 'Failed to reverse transfer'), type: 'error' });
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
                    : <strong>{transfer.sent_at ? new Date(transfer.sent_at).toLocaleString() : (l10n.getString('inv-transit-unknown') || 'Unknown')}</strong>
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
        title={l10n.getString('inv-transit-reverse-title') || 'Reverse Transfer?'}
        message={l10n.getString('inv-transit-reverse-message') || 'Are you sure you want to reverse this stock transfer? Stock will be returned to the source location. This action cannot be undone.'}
        variant="danger"
        confirmLabel={l10n.getString('inv-transit-reverse-confirm') || 'Reverse'}
      />
    </div>
  );
}
