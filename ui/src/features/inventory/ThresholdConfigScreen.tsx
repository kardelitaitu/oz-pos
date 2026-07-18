import { useState, useEffect } from 'react';
import { Localized } from '@fluent/react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { listProductsScoped, type ProductDto } from '@/api/products';
import {
  listInventoryLocations,
  getStockThresholds,
  setStockThreshold,
  deleteStockThreshold,
  type StockThreshold,
  type InventoryLocation,
} from '@/api/inventory';
import './ThresholdConfigScreen.css';

export default function ThresholdConfigScreen() {
  const { sessionToken } = useWorkspace();

  const [products, setProducts] = useState<ProductDto[]>([]);
  const [locations, setLocations] = useState<InventoryLocation[]>([]);
  const [thresholds, setThresholds] = useState<StockThreshold[]>([]);
  const [loading, setLoading] = useState(true);

  // Filter thresholds by location
  const [selectedLocationFilter, setSelectedLocationFilter] = useState<string>('all');

  // Dialog / Edit state
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [selectedProductId, setSelectedProductId] = useState('');
  const [selectedLocationId, setSelectedLocationId] = useState<string>('');
  const [thresholdVal, setThresholdVal] = useState('5');
  const [enabled, setEnabled] = useState(true);

  const loadData = async () => {
    if (!sessionToken) return;
    setLoading(true);
    try {
      const [prods, locs, thresh] = await Promise.all([
        listProductsScoped(sessionToken),
        listInventoryLocations(sessionToken),
        getStockThresholds(sessionToken, null), // Fetch all thresholds
      ]);
      setProducts(prods);
      setLocations(locs);
      setThresholds(thresh);
    } catch (err) {
      console.error('Failed to load threshold data:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, [sessionToken]);

  const handleOpenAddDialog = () => {
    setEditingId(null);
    if (products.length > 0) {
      // Find the first product that has SKU or is tracking stock
      setSelectedProductId(products[0].sku); // Product DTO uses SKU as id or we map by SKU
    }
    setSelectedLocationId(''); // Global
    setThresholdVal('5');
    setEnabled(true);
    setIsDialogOpen(true);
  };

  const handleOpenEditDialog = (t: StockThreshold) => {
    setEditingId(t.id);
    setSelectedProductId(t.product_id);
    setSelectedLocationId(t.location_id || '');
    setThresholdVal(t.threshold.toString());
    setEnabled(t.enabled);
    setIsDialogOpen(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!sessionToken || !selectedProductId) return;

    try {
      const locId = selectedLocationId === '' ? null : selectedLocationId;
      const numVal = parseInt(thresholdVal, 10);
      if (isNaN(numVal) || numVal < 0) {
        alert('Threshold must be a valid non-negative integer');
        return;
      }

      await setStockThreshold(sessionToken, selectedProductId, locId, numVal, enabled);
      setIsDialogOpen(false);
      await loadData();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to save threshold');
    }
  };

  const handleDelete = async (id: string) => {
    if (!sessionToken) return;
    if (!confirm('Are you sure you want to delete this threshold alert boundary?')) return;

    try {
      await deleteStockThreshold(sessionToken, id);
      await loadData();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete threshold');
    }
  };

  const filteredThresholds = thresholds.filter(t => {
    if (selectedLocationFilter === 'all') return true;
    if (selectedLocationFilter === 'global') return t.location_id === null;
    return t.location_id === selectedLocationFilter;
  });

  return (
    <div className="threshold-container">
      <div className="threshold-header">
        <Localized id="inv-threshold-title">
          <h2 className="threshold-title">Stock Threshold Configuration</h2>
        </Localized>
        <button className="shift-btn shift-btn-primary" onClick={handleOpenAddDialog}>
          <Localized id="inv-threshold-add-btn">
            <span>+ Add Threshold</span>
          </Localized>
        </button>
      </div>

      <div className="log-filters">
        <div className="log-filter-group">
          <Localized id="inv-transit-col-dest">
            <label htmlFor="filter-location">Filter by Location</label>
          </Localized>
          <select
            id="filter-location"
            className="log-select"
            value={selectedLocationFilter}
            onChange={e => setSelectedLocationFilter(e.target.value)}
          >
            <option value="all">All Locations</option>
            <option value="global">Global Fallback Only</option>
            {locations.map(loc => (
              <option key={loc.id} value={loc.id}>
                {loc.name}
              </option>
            ))}
          </select>
        </div>
      </div>

      {loading ? (
        <div className="transit-empty">
          <Localized id="inv-loading">
            <span>Loading...</span>
          </Localized>
        </div>
      ) : (
        <table className="threshold-table">
          <thead>
            <tr>
              <Localized id="inv-threshold-col-sku">
                <th>SKU</th>
              </Localized>
              <Localized id="inv-threshold-col-product">
                <th>Product Name</th>
              </Localized>
              <Localized id="inv-threshold-col-location">
                <th>Location</th>
              </Localized>
              <Localized id="inv-threshold-col-threshold">
                <th>Threshold</th>
              </Localized>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {filteredThresholds.map(t => {
              // The backend stored product_id is actually the product's SKU or DB ID.
              // Let's resolve the product name by matching product_id with product.sku.
              const prod = products.find(p => p.sku === t.product_id);
              const loc = locations.find(l => l.id === t.location_id);
              return (
                <tr key={t.id}>
                  <td>{t.product_id}</td>
                  <td>{prod ? prod.name : 'Unknown Product'}</td>
                  <td>{loc ? loc.name : 'Global (All Locations)'}</td>
                  <td>{t.threshold}</td>
                  <td>
                    <span className={`badge ${t.enabled ? 'badge-purchase-order-receive' : 'badge-void'}`}>
                      {t.enabled ? 'Enabled' : 'Disabled'}
                    </span>
                  </td>
                  <td className="threshold-actions">
                    <button className="shift-btn shift-btn-primary" style={{ padding: '4px 10px' }} onClick={() => handleOpenEditDialog(t)}>
                      <span>Edit</span>
                    </button>
                    <button className="shift-btn shift-btn-danger" style={{ padding: '4px 10px' }} onClick={() => handleDelete(t.id)}>
                      <span>Delete</span>
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}

      {isDialogOpen && (
        <div className="threshold-dialog-overlay">
          <form className="threshold-dialog" role="dialog" aria-modal="true" onSubmit={handleSave}>
            <Localized id="inv-threshold-dialog-title">
              <h3>Configure Threshold</h3>
            </Localized>

            <div className="form-group">
              <Localized id="inv-transit-col-product">
                <label htmlFor="dialog-product">Product</label>
              </Localized>
              <select
                id="dialog-product"
                className="threshold-select"
                value={selectedProductId}
                onChange={e => setSelectedProductId(e.target.value)}
                disabled={editingId !== null}
                required
              >
                {products.map(p => (
                  <option key={p.sku} value={p.sku}>
                    {p.name} ({p.sku})
                  </option>
                ))}
              </select>
            </div>

            <div className="form-group">
              <Localized id="inv-threshold-col-location">
                <label htmlFor="dialog-location">Location</label>
              </Localized>
              <select
                id="dialog-location"
                className="threshold-select"
                value={selectedLocationId}
                onChange={e => setSelectedLocationId(e.target.value)}
                disabled={editingId !== null}
              >
                <Localized id="inv-threshold-global-opt">
                  <option value="">Global (All Locations)</option>
                </Localized>
                {locations.map(loc => (
                  <option key={loc.id} value={loc.id}>
                    {loc.name}
                  </option>
                ))}
              </select>
            </div>

            <div className="form-group">
              <Localized id="inv-threshold-col-threshold">
                <label htmlFor="dialog-qty">Threshold Limit</label>
              </Localized>
              <input
                id="dialog-qty"
                type="number"
                className="threshold-input"
                value={thresholdVal}
                onChange={e => setThresholdVal(e.target.value)}
                min="0"
                required
              />
            </div>

            <label className="threshold-checkbox-label">
              <input
                type="checkbox"
                checked={enabled}
                onChange={e => setEnabled(e.target.checked)}
              />
              <span>Enabled</span>
            </label>

            <div className="dialog-actions">
              <button type="button" className="shift-btn shift-btn-danger" onClick={() => setIsDialogOpen(false)}>
                <Localized id="inv-cancel">
                  <span>Cancel</span>
                </Localized>
              </button>
              <button type="submit" className="shift-btn shift-btn-primary">
                <span>Save</span>
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
}
