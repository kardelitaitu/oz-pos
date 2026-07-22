import { useState, useEffect } from 'react';
import { Localized } from '@fluent/react';
import { useToast } from '@/frontend/shared/Toast';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import {
  listInventoryTransactions,
  listInventoryLocations,
  getInventoryTransaction,
  type InventoryTransaction,
  type InventoryLocation,
  type InventoryTransactionLine,
} from '@/api/inventory';
import './TransactionLogScreen.css';

export default function TransactionLogScreen() {
  const { sessionToken } = useWorkspace();
  const { addToast } = useToast();

  const [transactions, setTransactions] = useState<InventoryTransaction[]>([]);
  const [locations, setLocations] = useState<InventoryLocation[]>([]);
  const [loading, setLoading] = useState(true);

  // Expanded row tracking
  const [expandedTxId, setExpandedTxId] = useState<string | null>(null);
  const [expandedLines, setExpandedLines] = useState<InventoryTransactionLine[]>([]);
  const [loadingLines, setLoadingLines] = useState(false);

  // Filters state
  const [filterLocation, setFilterLocation] = useState('');
  const [filterStaff, setFilterStaff] = useState('');
  const [filterType, setFilterType] = useState('');
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');

  useEffect(() => {
    if (!sessionToken) return;

    setLoading(true);
    Promise.all([
      listInventoryTransactions(sessionToken),
      listInventoryLocations(sessionToken),
    ])
      .then(([txs, locs]) => {
        setTransactions(txs);
        setLocations(locs);
      })
      .catch((err) => addToast({ message: err instanceof Error ? err.message : 'Failed to load transactions', type: 'error' }))
      .finally(() => setLoading(false));
  }, [sessionToken, addToast]);

  const handleRowClick = async (txId: string) => {
    if (!sessionToken) return;
    if (expandedTxId === txId) {
      setExpandedTxId(null);
      setExpandedLines([]);
      return;
    }

    setExpandedTxId(txId);
    setLoadingLines(true);
    try {
      const detail = await getInventoryTransaction(sessionToken, txId);
      if (detail) {
        setExpandedLines(detail[1]);
      }
    } catch (err) {
      addToast({ message: err instanceof Error ? err.message : 'Failed to load transaction details', type: 'error' });
    } finally {
      setLoadingLines(false);
    }
  };

  // Extract unique staff IDs for filter
  const uniqueStaffIds = Array.from(new Set(transactions.map(tx => tx.staff_id)));

  // Filtered transactions
  const filteredTxs = transactions.filter(tx => {
    if (filterLocation && tx.location_id !== filterLocation) return false;
    if (filterStaff && tx.staff_id !== filterStaff) return false;
    if (filterType && tx.type !== filterType) return false;
    
    if (startDate) {
      const txTime = new Date(tx.created_at).getTime();
      const startTime = new Date(startDate).getTime();
      if (txTime < startTime) return false;
    }
    if (endDate) {
      const txTime = new Date(tx.created_at).getTime();
      // Set end time to the end of that day (23:59:59)
      const endTime = new Date(endDate).getTime() + 86400000 - 1;
      if (txTime > endTime) return false;
    }
    return true;
  });

  return (
    <div className="log-container">
      <div className="log-header">
        <Localized id="inv-log-title">
          <h2 className="log-title">Inventory Transaction Log</h2>
        </Localized>
      </div>

      <div className="log-filters">
        <div className="log-filter-group">
          <Localized id="inv-log-filter-location">
            <label htmlFor="filter-location">Location</label>
          </Localized>
          <select
            id="filter-location"
            className="log-select"
            value={filterLocation}
            onChange={e => setFilterLocation(e.target.value)}
          >
            <option value="">All</option>
            {locations.map(loc => (
              <option key={loc.id} value={loc.id}>
                {loc.name}
              </option>
            ))}
          </select>
        </div>

        <div className="log-filter-group">
          <Localized id="inv-log-filter-staff">
            <label htmlFor="filter-staff">Staff</label>
          </Localized>
          <select
            id="filter-staff"
            className="log-select"
            value={filterStaff}
            onChange={e => setFilterStaff(e.target.value)}
          >
            <option value="">All</option>
            {uniqueStaffIds.map(id => (
              <option key={id} value={id}>
                {id}
              </option>
            ))}
          </select>
        </div>

        <div className="log-filter-group">
          <Localized id="inv-log-filter-type">
            <label htmlFor="filter-type">Type</label>
          </Localized>
          <select
            id="filter-type"
            className="log-select"
            value={filterType}
            onChange={e => setFilterType(e.target.value)}
          >
            <option value="">All</option>
            <option value="sale">Sale</option>
            <option value="void">Void</option>
            <option value="refund">Refund</option>
            <option value="transfer">Transfer</option>
            <option value="purchase-order-receive">PO Receive</option>
            <option value="stock-count">Stock Count</option>
            <option value="manual-adjustment">Manual Adjustment</option>
          </select>
        </div>

        <div className="log-filter-group">
          <label htmlFor="filter-start-date">Start Date</label>
          <input
            id="filter-start-date"
            type="date"
            className="log-input"
            value={startDate}
            onChange={e => setStartDate(e.target.value)}
          />
        </div>

        <div className="log-filter-group">
          <label htmlFor="filter-end-date">End Date</label>
          <input
            id="filter-end-date"
            type="date"
            className="log-input"
            value={endDate}
            onChange={e => setEndDate(e.target.value)}
          />
        </div>
      </div>

      {loading ? (
        <div className="transit-empty">
          <Localized id="inv-loading">
            <span>Loading...</span>
          </Localized>
        </div>
      ) : (
        <div aria-live="polite" aria-relevant="additions text">
        <table className="log-table">
          <thead>
            <tr>
              <th>Date / Time</th>
              <th>Type</th>
              <th>Location</th>
              <th>Staff</th>
              <th>Action</th>
            </tr>
          </thead>
          <tbody>
            {filteredTxs.map(tx => {
              const locationName = locations.find(l => l.id === tx.location_id)?.name || tx.location_id;
              const isExpanded = expandedTxId === tx.id;
              return (
                <>
                  <tr
                    key={tx.id}
                    className="log-row-expandable"
                    onClick={() => handleRowClick(tx.id)}
                  >
                    <td>{new Date(tx.created_at).toLocaleString()}</td>
                    <td>
                      <span className={`badge badge-${tx.type}`}>
                        {tx.type.replace('-', ' ')}
                      </span>
                    </td>
                    <td>{locationName}</td>
                    <td>{tx.staff_id}</td>
                    <td>
                      <button className="shift-btn shift-btn-primary" style={{ padding: '4px 10px' }}>
                        <Localized id="inv-log-expand-btn">
                          <span>Details</span>
                        </Localized>
                      </button>
                    </td>
                  </tr>
                  {isExpanded && (
                    <tr className="log-row-expanded">
                      <td colSpan={5}>
                        <div className="log-details-container">
                          {tx.notes && (
                            <div className="details-notes">
                              <strong>Notes:</strong> {tx.notes}
                            </div>
                          )}
                          <div aria-live="polite">
                          {loadingLines ? (
                            <span>Loading lines...</span>
                          ) : (
                            <table className="details-table">
                              <thead>
                                <tr>
                                  <Localized id="inv-transit-col-sku">
                                    <th>SKU</th>
                                  </Localized>
                                  <Localized id="inv-transit-col-product">
                                    <th>Product Name</th>
                                  </Localized>
                                  <Localized id="inv-transit-col-qty">
                                    <th>Qty Change</th>
                                  </Localized>
                                  <Localized id="inv-log-col-barcode">
                                    <th>Barcode Scanned</th>
                                  </Localized>
                                </tr>
                              </thead>
                              <tbody>
                                {expandedLines.map(line => (
                                  <tr key={line.id}>
                                    <td>{line.sku}</td>
                                    <td>{line.product_name}</td>
                                    <td style={{ color: line.qty >= 0 ? '#22c55e' : '#ef4444' }}>
                                      {line.qty >= 0 ? `+${line.qty}` : line.qty}
                                    </td>
                                    <td>{line.barcode_scanned || '-'}</td>
                                  </tr>
                                ))}
                              </tbody>
                            </table>
                          )}
                        </div>
                        </div>
                      </td>
                    </tr>
                  )}
                </>
              );
            })}
          </tbody>
        </table>
        </div>
      )}
    </div>
  );
}
