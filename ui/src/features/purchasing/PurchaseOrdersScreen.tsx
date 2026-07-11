import { useState, useCallback, useEffect, useMemo } from 'react';
import { Localized } from '@fluent/react';
import {
  listPurchaseOrders,
  updatePoStatus,
  receivePurchaseOrder,
  type PurchaseOrderDto,
} from '@/api/purchasing';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import PurchaseOrderForm from './PurchaseOrderForm';
import './PurchaseOrdersScreen.css';

const STATUSES = ['draft', 'pending', 'approved', 'received', 'cancelled'];

function formatMinor(minor: number): string {
  return (minor / 100).toFixed(2);
}

export default function PurchaseOrdersScreen() {
  const [orders, setOrders] = useState<PurchaseOrderDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [statusFilter, setStatusFilter] = useState('all');
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await listPurchaseOrders();
      setOrders(data);
    } catch {
      // IPC unavailable
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (statusFilter === 'all') return orders;
    return orders.filter((o) => o.status === statusFilter);
  }, [orders, statusFilter]);

  const handleStatusChange = useCallback(async (id: string, status: string) => {
    setActionLoading(id);
    try {
      await updatePoStatus({ id, status });
      await load();
    } catch {
      // ignore
    } finally {
      setActionLoading(null);
    }
  }, [load]);

  const handleReceive = useCallback(async (id: string) => {
    setActionLoading(id);
    try {
      await receivePurchaseOrder(id);
      await load();
    } catch {
      // ignore
    } finally {
      setActionLoading(null);
    }
  }, [load]);

  const openCreate = useCallback(() => {
    setEditingId(null);
    setShowForm(true);
  }, []);

  const closeForm = useCallback(() => {
    setShowForm(false);
    setEditingId(null);
  }, []);

  const onSaved = useCallback(() => {
    closeForm();
    load();
  }, [closeForm, load]);

  return (
    <div className="po-screen">
      <div className="po-header">
        <Localized id="po-title">
          <h1 className="po-title">Purchase Orders</h1>
        </Localized>
        <Localized id="po-add">
          <Button onClick={openCreate}>New Purchase Order</Button>
        </Localized>
      </div>

      <div className="po-filters">
        {['all', ...STATUSES].map((s) => (
          <button
            key={s}
            type="button"
            className={`po-filter-btn ${statusFilter === s ? 'po-filter-btn--active' : ''}`}
            onClick={() => setStatusFilter(s)}
          >
            {s === 'all' ? 'All' : s.charAt(0).toUpperCase() + s.slice(1)}
          </button>
        ))}
      </div>

      {loading ? (
        <p className="po-loading">Loading purchase orders…</p>
      ) : filtered.length === 0 ? (
        <Card shadow="sm">
          <div className="po-empty">
            {statusFilter === 'all'
              ? <p>No purchase orders yet.</p>
              : <p>No purchase orders with status &quot;{statusFilter}&quot;.</p>}
            <Button variant="secondary" onClick={openCreate}>Create Purchase Order</Button>
          </div>
        </Card>
      ) : (
        <div className="po-table-wrap">
          <table className="po-table" aria-label="Purchase Orders">
            <thead>
              <tr>
                <th>PO #</th>
                <th>Supplier</th>
                <th>Status</th>
                <th>Order Date</th>
                <th>Expected</th>
                <th>Total</th>
                <th>Items</th>
                <th aria-label="Actions"> </th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((po) => (
                <tr key={po.id}>
                  <td className="po-cell-number">{po.po_number}</td>
                  <td>{po.supplier_name || po.supplier_id}</td>
                  <td>
                    <span className={`po-status po-status--${po.status}`}>{po.status}</span>
                  </td>
                  <td className="po-cell-date">{po.order_date.slice(0, 10)}</td>
                  <td className="po-cell-date">{po.expected_date ? po.expected_date.slice(0, 10) : '\u2014'}</td>
                  <td className="po-cell-total">{formatMinor(po.total_minor)}</td>
                  <td>{po.lines.length}</td>
                  <td className="po-cell-actions">
                    {po.status === 'draft' && (
                      <button
                        type="button"
                        className="po-action-btn"
                        disabled={actionLoading === po.id}
                        onClick={() => handleStatusChange(po.id, 'pending')}
                      >
                        Submit
                      </button>
                    )}
                    {po.status === 'pending' && (
                      <button
                        type="button"
                        className="po-action-btn"
                        disabled={actionLoading === po.id}
                        onClick={() => handleStatusChange(po.id, 'approved')}
                      >
                        Approve
                      </button>
                    )}
                    {po.status === 'approved' && (
                      <button
                        type="button"
                        className="po-action-btn po-action-btn--primary"
                        disabled={actionLoading === po.id}
                        onClick={() => handleReceive(po.id)}
                      >
                        Receive
                      </button>
                    )}
                    {(po.status === 'draft' || po.status === 'pending') && (
                      <button
                        type="button"
                        className="po-action-btn po-action-btn--danger"
                        disabled={actionLoading === po.id}
                        onClick={() => handleStatusChange(po.id, 'cancelled')}
                      >
                        Cancel
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {showForm && (
        <PurchaseOrderForm editingId={editingId} onClose={closeForm} onSaved={onSaved} />
      )}
    </div>
  );
}
