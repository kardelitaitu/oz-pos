import { useState, useCallback, useEffect } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listStockTransfers,
  getStockTransfer,
  createStockTransfer,
  sendStockTransfer,
  receiveStockTransfer,
  cancelStockTransfer,
  addStockTransferLine as _addStockTransferLine,
  removeStockTransferLine as _removeStockTransferLine,
  type StockTransfer,
  type StockTransferLine,
  type ReceivedLineInput,
} from '@/api/stockTransfers';
import { listProducts, type ProductDto } from '@/api/products';
import { listTerminals, type TerminalDto } from '@/api/terminals';
import { useAuth } from '@/contexts/AuthContext';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './StockTransfersScreen.css';

const STATUS_FILTERS = ['all', 'draft', 'pending', 'in_transit', 'received', 'cancelled'] as const;

function statusLabel(status: string): string {
  return status.charAt(0).toUpperCase() + status.slice(1).replace('_', ' ');
}

function formatDate(iso: string | null): string {
  if (!iso) return '—';
  return new Date(iso).toLocaleDateString(undefined, {
    year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit',
  });
}

interface LineFormEntry {
  sku: string;
  productName: string;
  qty: string;
}

/** Stock transfers screen — create, send, receive, and cancel stock transfers between store locations or terminals. */
export default function StockTransfersScreen() {
  const { l10n } = useLocalization();
  const { session } = useAuth();
  const [transfers, setTransfers] = useState<StockTransfer[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [statusFilter, setStatusFilter] = useState<string>('all');

  // Detail view
  const [detailId, setDetailId] = useState<string | null>(null);
  const [detail, setDetail] = useState<{ transfer: StockTransfer; lines: StockTransferLine[] } | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  // Create modal
  const [showCreate, setShowCreate] = useState(false);
  const [createSourceLoc, setCreateSourceLoc] = useState('');
  const [createDestLoc, setCreateDestLoc] = useState('');
  const [createSourceTerminalId, setCreateSourceTerminalId] = useState('');
  const [createDestTerminalId, setCreateDestTerminalId] = useState('');
  const [createNotes, setCreateNotes] = useState('');
  const [createLines, setCreateLines] = useState<LineFormEntry[]>([]);
  const [createSaving, setCreateSaving] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // Receive modal
  const [receiveTransferId, setReceiveTransferId] = useState<string | null>(null);
  const [receiveLines, setReceiveLines] = useState<Record<string, string>>({});
  const [receiveSaving, setReceiveSaving] = useState(false);

  // Cancel state
  const [cancelling, setCancelling] = useState<string | null>(null);

  // Products & terminals for dropdowns
  const [products, setProducts] = useState<ProductDto[]>([]);
  const [terminals, setTerminals] = useState<TerminalDto[]>([]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [data, prodData, termData] = await Promise.all([
        listStockTransfers(),
        listProducts().catch(() => []),
        listTerminals().catch(() => []),
      ]);
      setTransfers(data);
      setProducts(prodData);
      setTerminals(termData);
    } catch {
      setError(l10n.getString('stock-transfers-error-load'));
    } finally {
      setLoading(false);
    }
  }, [l10n]);

  useEffect(() => { load(); }, [load]);

  const openDetail = useCallback(async (id: string) => {
    setDetailId(id);
    setDetailLoading(true);
    try {
      const data = await getStockTransfer(id);
      if (data) setDetail(data);
    } catch {
      setError(l10n.getString('stock-transfers-error-load'));
    } finally {
      setDetailLoading(false);
    }
  }, [l10n]);

  const closeDetail = useCallback(() => {
    setDetailId(null);
    setDetail(null);
  }, []);

  const openSend = useCallback(async () => {
    if (!detailId) return;
    try {
      await sendStockTransfer(detailId);
      await load();
      if (detailId) openDetail(detailId);
    } catch {
      setError(l10n.getString('stock-transfers-error-send'));
    }
  }, [detailId, load, openDetail, l10n]);

  const openReceiveModal = useCallback(() => {
    if (!detailId || !detail) return;
    setReceiveTransferId(detailId);
    const init: Record<string, string> = {};
    detail.lines.forEach((l) => { init[l.id] = String(l.qty); });
    setReceiveLines(init);
  }, [detailId, detail]);

  const handleReceive = useCallback(async () => {
    if (!receiveTransferId || !session?.user_id) return;
    setReceiveSaving(true);
    try {
      const receivedLines: ReceivedLineInput[] = Object.entries(receiveLines).map(
        ([lineId, qtyStr]) => ({
          line_id: lineId,
          received_qty: parseInt(qtyStr, 10) || 0,
        }),
      );
      await receiveStockTransfer(receiveTransferId, session.user_id, receivedLines);
      setReceiveTransferId(null);
      await load();
      if (detailId) openDetail(detailId);
    } catch {
      setError(l10n.getString('stock-transfers-error-receive'));
    } finally {
      setReceiveSaving(false);
    }
  }, [receiveTransferId, receiveLines, session, detailId, load, openDetail, l10n]);

  const handleCancel = useCallback(async (id: string) => {
    setCancelling(id);
    try {
      await cancelStockTransfer(id);
      await load();
      if (detailId === id) closeDetail();
    } catch {
      setError(l10n.getString('stock-transfers-error-cancel'));
    } finally {
      setCancelling(null);
    }
  }, [load, detailId, closeDetail, l10n]);

  const addLineEntry = useCallback(() => {
    setCreateLines([...createLines, { sku: '', productName: '', qty: '1' }]);
  }, [createLines]);

  const updateLineEntry = useCallback((index: number, field: keyof LineFormEntry, value: string) => {
    const updated = [...createLines];
    if (field === 'sku') {
      const match = products.find((p) => p.sku === value);
      updated[index] = { ...updated[index], sku: value, productName: match?.name ?? value } as LineFormEntry;
    } else {
      updated[index] = { ...updated[index], [field]: value } as LineFormEntry;
    }
    setCreateLines(updated);
  }, [createLines, products]);

  const removeLineEntry = useCallback((index: number) => {
    setCreateLines(createLines.filter((_, i) => i !== index));
  }, [createLines]);
  const resetCreateForm = useCallback(() => {
    setCreateSourceLoc('');
    setCreateDestLoc('');
    setCreateSourceTerminalId('');
    setCreateDestTerminalId('');
    setCreateNotes('');
    setCreateLines([]);
    setCreateError(null);
  }, []);

  const handleCreate = useCallback(async () => {
    if (!session?.user_id) return;
    setCreateSaving(true);
    setCreateError(null);
    try {
      const lines = createLines
        .filter((l) => l.sku.trim() && parseInt(l.qty, 10) > 0)
        .map((l) => ({
          id: '',
          transfer_id: '',
          sku: l.sku.trim(),
          product_name: l.productName,
          qty: parseInt(l.qty, 10),
          received_qty: 0,
        }));
      if (lines.length === 0) {
        setCreateError(l10n.getString('stock-transfers-error-no-lines'));
        setCreateSaving(false);
        return;
      }
      await createStockTransfer(
        createSourceLoc || null,
        createDestLoc || null,
        createSourceTerminalId || null,
        createDestTerminalId || null,
        createNotes,
        session.user_id,
        lines,
      );
      setShowCreate(false);
      resetCreateForm();
      await load();
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : l10n.getString('stock-transfers-error-create'));
    } finally {
      setCreateSaving(false);
    }
  }, [session, createLines, createSourceLoc, createDestLoc, createSourceTerminalId, createDestTerminalId, createNotes, l10n, load, resetCreateForm]);


  const openCreate = useCallback(() => {
    resetCreateForm();
    setShowCreate(true);
  }, [resetCreateForm]);

  const closeCreate = useCallback(() => {
    setShowCreate(false);
    resetCreateForm();
  }, [resetCreateForm]);

  const filtered = statusFilter === 'all'
    ? transfers
    : transfers.filter((t) => t.status === statusFilter);

  return (
    <div className="stock-transfers">
      <div className="stock-transfers-header">
        <Localized id="stock-transfers-title">
          <h1 className="stock-transfers-title">Stock Transfers</h1>
        </Localized>
        <Localized id="stock-transfers-create">
          <Button onClick={openCreate}>New Transfer</Button>
        </Localized>
      </div>

      <div className="stock-transfers-filters" role="tablist" aria-label={l10n.getString('stock-transfers-filter-aria')}>
        {STATUS_FILTERS.map((s) => (
          <button
            key={s}
            type="button"
            role="tab"
            aria-selected={statusFilter === s}
            className={`stock-transfers-filter-btn${statusFilter === s ? ' active' : ''}`}
            onClick={() => setStatusFilter(s)}
          >
            <Localized id={`stock-transfers-status-${s}`}>
              <span>{statusLabel(s)}</span>
            </Localized>
          </button>
        ))}
      </div>

      {loading ? (
        <Localized id="loading">
          <p className="stock-transfers-loading">Loading…</p>
        </Localized>
      ) : error ? (
        <Card shadow="sm">
          <div className="stock-transfers-empty">
            <p className="stock-transfers-error-text">{error}</p>
            <Button variant="secondary" onClick={load}>
              <Localized id="retry">Retry</Localized>
            </Button>
          </div>
        </Card>
      ) : filtered.length === 0 ? (
        <Card shadow="sm">
          <div className="stock-transfers-empty">
            <Localized id="stock-transfers-empty">
              <p>No stock transfers found</p>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="stock-transfers-table-wrap">
          <table className="stock-transfers-table" aria-label={l10n.getString('stock-transfers-table-aria')}>
            <thead>
              <tr>
                <Localized id="stock-transfers-number"><th>Transfer #</th></Localized>
                <Localized id="stock-transfers-status"><th>Status</th></Localized>
                <Localized id="stock-transfers-source"><th>Source</th></Localized>
                <Localized id="stock-transfers-destination"><th>Destination</th></Localized>
                <Localized id="stock-transfers-created"><th>Created</th></Localized>
                <Localized id="stock-transfers-actions" attrs={{ 'aria-label': true }}>
                  <th aria-label="Actions"> </th>
                </Localized>
              </tr>
            </thead>
            <tbody>
              {filtered.map((t) => (
                <tr key={t.id}>
                  <td className="stock-transfers-cell-number">
                    <button
                      type="button"
                      className="stock-transfers-link"
                      onClick={() => openDetail(t.id)}
                    >
                      {t.transfer_number}
                    </button>
                  </td>
                  <td>
                    <span className={`stock-transfers-badge stock-transfers-badge--${t.status}`}>
                      {statusLabel(t.status)}
                    </span>
                  </td>
                  <td>{t.source_location ?? t.source_terminal_id ?? '—'}</td>
                  <td>{t.destination_location ?? t.destination_terminal_id ?? '—'}</td>
                  <td className="stock-transfers-cell-date">{formatDate(t.created_at)}</td>
                  <td className="stock-transfers-cell-actions">
                    <Localized id="stock-transfers-view">
                      <button
                        type="button"
                        className="stock-transfers-action-btn"
                        onClick={() => openDetail(t.id)}
                      >
                        View
                      </button>
                    </Localized>
                    {(t.status === 'draft' || t.status === 'pending') && (
                      <button
                        type="button"
                        className="stock-transfers-action-btn stock-transfers-action-btn--danger"
                        disabled={cancelling === t.id}
                        onClick={() => handleCancel(t.id)}
                      >
                        <Localized id="stock-transfers-cancel">
                          <span>Cancel</span>
                        </Localized>
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* ── Detail Modal ─────────────────────────────────────────── */}
      {detailId && (
        <div className="stock-transfers-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('stock-transfers-detail-aria')}>
          <div className="stock-transfers-modal stock-transfers-modal--wide">
            <div className="stock-transfers-modal-header">
              <Localized id="stock-transfers-detail-title">
                <h2>Transfer Details</h2>
              </Localized>
              <button type="button" className="stock-transfers-modal-close" onClick={closeDetail} aria-label={l10n.getString('close')}>&times;</button>
            </div>
            {detailLoading ? (
              <Localized id="loading"><p className="stock-transfers-loading">Loading…</p></Localized>
            ) : detail ? (
              <div className="stock-transfers-detail">
                <div className="stock-transfers-detail-info">
                  <div className="stock-transfers-detail-field">
                    <Localized id="stock-transfers-number"><span className="stock-transfers-detail-label">Transfer #</span></Localized>
                    <span>{detail.transfer.transfer_number}</span>
                  </div>
                  <div className="stock-transfers-detail-field">
                    <Localized id="stock-transfers-status"><span className="stock-transfers-detail-label">Status</span></Localized>
                    <span className={`stock-transfers-badge stock-transfers-badge--${detail.transfer.status}`}>{statusLabel(detail.transfer.status)}</span>
                  </div>
                  <div className="stock-transfers-detail-field">
                    <Localized id="stock-transfers-source"><span className="stock-transfers-detail-label">Source</span></Localized>
                    <span>{detail.transfer.source_location ?? detail.transfer.source_terminal_id ?? '—'}</span>
                  </div>
                  <div className="stock-transfers-detail-field">
                    <Localized id="stock-transfers-destination"><span className="stock-transfers-detail-label">Destination</span></Localized>
                    <span>{detail.transfer.destination_location ?? detail.transfer.destination_terminal_id ?? '—'}</span>
                  </div>
                  <div className="stock-transfers-detail-field">
                    <Localized id="stock-transfers-notes"><span className="stock-transfers-detail-label">Notes</span></Localized>
                    <span>{detail.transfer.notes || '—'}</span>
                  </div>
                  <div className="stock-transfers-detail-field">
                    <Localized id="stock-transfers-created"><span className="stock-transfers-detail-label">Created</span></Localized>
                    <span>{formatDate(detail.transfer.created_at)}</span>
                  </div>
                  {detail.transfer.sent_at && (
                    <div className="stock-transfers-detail-field">
                      <Localized id="stock-transfers-sent-at"><span className="stock-transfers-detail-label">Sent</span></Localized>
                      <span>{formatDate(detail.transfer.sent_at)}</span>
                    </div>
                  )}
                  {detail.transfer.received_at && (
                    <div className="stock-transfers-detail-field">
                      <Localized id="stock-transfers-received-at"><span className="stock-transfers-detail-label">Received</span></Localized>
                      <span>{formatDate(detail.transfer.received_at)}</span>
                    </div>
                  )}
                </div>

                <table className="stock-transfers-lines-table" aria-label={l10n.getString('stock-transfers-lines-aria')}>
                  <thead>
                    <tr>
                      <Localized id="stock-transfers-sku"><th>SKU</th></Localized>
                      <Localized id="stock-transfers-product"><th>Product</th></Localized>
                      <Localized id="stock-transfers-qty"><th>Qty</th></Localized>
                      <Localized id="stock-transfers-received"><th>Received</th></Localized>
                    </tr>
                  </thead>
                  <tbody>
                    {detail.lines.map((l) => (
                      <tr key={l.id}>
                        <td>{l.sku}</td>
                        <td>{l.product_name}</td>
                        <td>{l.qty}</td>
                        <td>{l.received_qty}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>

                <div className="stock-transfers-detail-actions">
                  {(detail.transfer.status === 'draft' || detail.transfer.status === 'pending') && (
                    <>
                      <Localized id="stock-transfers-send">
                        <Button variant="primary" onClick={openSend}>Send Transfer</Button>
                      </Localized>
                      <Localized id="stock-transfers-cancel">
                        <Button variant="danger" onClick={() => handleCancel(detail.transfer.id)}>Cancel Transfer</Button>
                      </Localized>
                    </>
                  )}
                  {detail.transfer.status === 'in_transit' && (
                    <Localized id="stock-transfers-receive">
                      <Button variant="primary" onClick={openReceiveModal}>Receive Transfer</Button>
                    </Localized>
                  )}
                  <Localized id="close">
                    <Button variant="ghost" onClick={closeDetail}>Close</Button>
                  </Localized>
                </div>
              </div>
            ) : (
              <Localized id="stock-transfers-not-found">
                <p>Transfer not found</p>
              </Localized>
            )}
          </div>
        </div>
      )}

      {/* ── Create Modal ─────────────────────────────────────────── */}
      {showCreate && (
        <div className="stock-transfers-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('stock-transfers-create-aria')}>
          <div className="stock-transfers-modal stock-transfers-modal--wide">
            <div className="stock-transfers-modal-header">
              <Localized id="stock-transfers-create-title">
                <h2>New Stock Transfer</h2>
              </Localized>
              <button type="button" className="stock-transfers-modal-close" onClick={closeCreate} aria-label={l10n.getString('close')}>&times;</button>
            </div>
            <div className="stock-transfers-modal-body">
              <div className="stock-transfers-form-row">
                <div className="stock-transfers-field">
                  <Localized id="stock-transfers-source"><span className="stock-transfers-label">Source</span></Localized>
                  <input id="st-source-location" className="stock-transfers-input" type="text" value={createSourceLoc} onChange={(e) => setCreateSourceLoc(e.target.value)} placeholder={l10n.getString('stock-transfers-source-placeholder')} aria-label={l10n.getString('stock-transfers-source')} />
                </div>
                <div className="stock-transfers-field">
                  <Localized id="stock-transfers-destination"><span className="stock-transfers-label">Destination</span></Localized>
                  <input id="st-dest-location" className="stock-transfers-input" type="text" value={createDestLoc} onChange={(e) => setCreateDestLoc(e.target.value)} placeholder={l10n.getString('stock-transfers-destination-placeholder')} aria-label={l10n.getString('stock-transfers-destination')} />
                </div>
              </div>
              <div className="stock-transfers-form-row">
                <label className="stock-transfers-field" htmlFor="st-source-terminal">
                  <Localized id="stock-transfers-source-terminal"><span className="stock-transfers-label">Source Terminal (optional)</span></Localized>
                  <select id="st-source-terminal" className="stock-transfers-input" value={createSourceTerminalId} onChange={(e) => setCreateSourceTerminalId(e.target.value)}>
                    <option value="">—</option>
                    {terminals.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
                  </select>
                </label>
                <label className="stock-transfers-field" htmlFor="st-dest-terminal">
                  <Localized id="stock-transfers-destination-terminal"><span className="stock-transfers-label">Destination Terminal (optional)</span></Localized>
                  <select id="st-dest-terminal" className="stock-transfers-input" value={createDestTerminalId} onChange={(e) => setCreateDestTerminalId(e.target.value)}>
                    <option value="">—</option>
                    {terminals.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
                  </select>
                </label>
              </div>
              <div className="stock-transfers-field">
                <Localized id="stock-transfers-notes"><span className="stock-transfers-label">Notes</span></Localized>
                <textarea id="st-notes" className="stock-transfers-input stock-transfers-textarea" value={createNotes} onChange={(e) => setCreateNotes(e.target.value)} rows={2} aria-label={l10n.getString('stock-transfers-notes')} />
              </div>

              <div className="stock-transfers-lines-section">
                <div className="stock-transfers-lines-header">
                  <Localized id="stock-transfers-lines">
                    <span className="stock-transfers-label">Line Items</span>
                  </Localized>
                  <Localized id="stock-transfers-add-line">
                    <Button variant="secondary" size="sm" onClick={addLineEntry}>Add Line</Button>
                  </Localized>
                </div>
                {createLines.map((line, i) => (
                  <div key={i} className="stock-transfers-line-row">
                    <input
                      className="stock-transfers-input stock-transfers-line-sku"
                      type="text"
                      list="product-skus"
                      value={line.sku}
                      onChange={(e) => updateLineEntry(i, 'sku', e.target.value)}
                      placeholder={l10n.getString('stock-transfers-sku-placeholder')}
                      aria-label={l10n.getString('stock-transfers-sku')}
                    />
                    <input
                      className="stock-transfers-input stock-transfers-line-name"
                      type="text"
                      value={line.productName}
                      onChange={(e) => updateLineEntry(i, 'productName', e.target.value)}
                      placeholder={l10n.getString('stock-transfers-product-placeholder')}
                      aria-label={l10n.getString('stock-transfers-product')}
                    />
                    <input
                      className="stock-transfers-input stock-transfers-line-qty"
                      type="number"
                      min="1"
                      value={line.qty}
                      onChange={(e) => updateLineEntry(i, 'qty', e.target.value)}
                      aria-label={l10n.getString('stock-transfers-qty')}
                    />
                    <button type="button" className="stock-transfers-line-remove" onClick={() => removeLineEntry(i)} aria-label={l10n.getString('stock-transfers-remove-line')}>&times;</button>
                  </div>
                ))}
                <datalist id="product-skus">
                  {products.map((p) => <option key={p.sku} value={p.sku} />)}
                </datalist>
              </div>

              {createError && <div className="stock-transfers-error" role="alert">{createError}</div>}
            </div>
            <div className="stock-transfers-modal-actions">
              <Localized id="cancel">
                <Button variant="ghost" onClick={closeCreate} disabled={createSaving}>Cancel</Button>
              </Localized>
              <Button variant="primary" loading={createSaving} onClick={handleCreate}>
                <Localized id="stock-transfers-create-action"><span>Create Transfer</span></Localized>
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* ── Receive Modal ────────────────────────────────────────── */}
      {receiveTransferId && detail && (
        <div className="stock-transfers-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('stock-transfers-receive-aria')}>
          <div className="stock-transfers-modal">
            <div className="stock-transfers-modal-header">
              <Localized id="stock-transfers-receive-title">
                <h2>Receive Transfer</h2>
              </Localized>
              <button type="button" className="stock-transfers-modal-close" onClick={() => setReceiveTransferId(null)} aria-label={l10n.getString('close')}>&times;</button>
            </div>
            <div className="stock-transfers-modal-body">
              <Localized id="stock-transfers-receive-instruction">
                <p>Enter the quantity received for each line item.</p>
              </Localized>
              {detail.lines.map((l) => (
                <label key={l.id} className="stock-transfers-field">
                  <span className="stock-transfers-label">{l.sku} — {l.product_name} (ordered: {l.qty})</span>
                  <input
                    className="stock-transfers-input"
                    type="number"
                    min="0"
                    max={l.qty}
                    value={receiveLines[l.id] ?? String(l.qty)}
                    onChange={(e) => setReceiveLines({ ...receiveLines, [l.id]: e.target.value })}
                    aria-label={`${l.sku} received quantity`}
                  />
                </label>
              ))}
            </div>
            <div className="stock-transfers-modal-actions">
              <Localized id="cancel">
                <Button variant="ghost" onClick={() => setReceiveTransferId(null)} disabled={receiveSaving}>Cancel</Button>
              </Localized>
              <Button variant="primary" loading={receiveSaving} onClick={handleReceive}>
                <Localized id="stock-transfers-receive-action"><span>Confirm Receipt</span></Localized>
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
