import type React from 'react';
import { useState, useEffect, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { requiredLocalized } from '@/frontend/shared';
import { useFocusTrap } from '@/hooks/useFocusTrap';
import type { ProductDto } from '@/api/products';

export interface EditProductModalProps {
  product: ProductDto | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (updatedProduct: ProductDto) => void;
  /** ADR #36 D7: false hides the Cost field + override hint (manager+ only). */
  canEditCost?: boolean;
}

export const EditProductModal: React.FC<EditProductModalProps> = ({
  product,
  isOpen,
  onClose,
  onSave,
  canEditCost = true,
}) => {
  const { l10n } = useLocalization();
  const [name, setName] = useState('');
  const [priceMinor, setPriceMinor] = useState<number | ''>(0);
  const [stockQty, setStockQty] = useState<number | ''>(0);
  const [lowThreshold, setLowThreshold] = useState<number | ''>(5);
  const [highThreshold, setHighThreshold] = useState<number | ''>(10);
  // ADR #36: cost (edit/override), brand, rack, notes, unit, status.
  const [costMinor, setCostMinor] = useState<number | ''>(0);
  const [brand, setBrand] = useState('');
  const [rackLocation, setRackLocation] = useState('');
  const [notes, setNotes] = useState('');
  const [unit, setUnit] = useState('');
  const [isActive, setIsActive] = useState(true);

  useEffect(() => {
    if (isOpen && product) {
      setName(product.name || '');
      setPriceMinor(product.price?.minor_units ?? 0);
      setStockQty(product.stock_qty ?? 0);
      setLowThreshold(product.low_stock_threshold ?? 5);
      setHighThreshold(product.high_stock_threshold ?? 10);
      setCostMinor(product.cost_minor ?? 0);
      setBrand(product.brand ?? '');
      setRackLocation(product.rack_location ?? '');
      setNotes(product.notes ?? '');
      setUnit(product.unit ?? '');
      setIsActive(product.is_active !== false);
    }
  }, [isOpen, product]);

  const panelRef = useRef<HTMLDivElement>(null);
  useFocusTrap(panelRef, isOpen, onClose);


  if (!isOpen || !product) {
    return null;
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    const updatedProduct: ProductDto = {
      ...product,
      name: name.trim(),
      price: {
        ...product.price,
        minor_units: Math.max(0, priceMinor === '' ? 0 : priceMinor),
      },
      stock_qty: Math.max(0, stockQty === '' ? 0 : stockQty),
      in_stock: (stockQty === '' ? 0 : Math.max(0, stockQty)) > 0,
      low_stock_threshold: Math.max(0, lowThreshold === '' ? 0 : lowThreshold),
      high_stock_threshold: Math.max(0, highThreshold === '' ? 0 : highThreshold),
      cost_minor: Math.max(0, costMinor === '' ? 0 : costMinor),
      brand: brand.trim() || null,
      rack_location: rackLocation.trim() || null,
      notes: notes.trim() || null,
      unit: unit.trim() || null,
      is_active: isActive,
      default_supplier_id: product.default_supplier_id ?? null,
      popularity_score: product.popularity_score ?? 0,
    };

    onSave(updatedProduct);
    onClose();
  };

  // ADR #36 D5: restocking (stock qty increased) shows the cost-override
  // hint — the Cost field doubles as the override for the newly received
  // stock.
  const originalStock = product.stock_qty ?? 0;
  const restocking = Math.max(0, stockQty === '' ? 0 : stockQty) > originalStock;

  return (
    <>
    <div className="retail-edit-modal-backdrop" onClick={(e) => { if (e.target === e.currentTarget) onClose(); }} onKeyDown={(e) => { if (e.key === 'Escape') onClose(); }} role="presentation" tabIndex={-1}>
      <div
        ref={panelRef}
        className="retail-edit-modal-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="retail-edit-modal-title"
      >
        <div className="retail-edit-modal-header">
          <Localized id="retail-edit-product-title">
            <h3 id="retail-edit-modal-title" className="retail-edit-modal-title">
              Edit Product
            </h3>
          </Localized>
          <Localized id="retail-edit-modal-close-aria" attrs={{ 'aria-label': true }}>
            <button
              type="button"
              className="retail-edit-modal-close"
              onClick={onClose}
              aria-label="Close"
            >
              &times;
            </button>
          </Localized>
        </div>

        <form onSubmit={handleSubmit} className="retail-edit-modal-form">
          <div className="retail-edit-form-group">
            <Localized id="retail-edit-field-sku">
              <label htmlFor="edit-product-sku" className="retail-edit-label">
                SKU / Code
              </label>
            </Localized>
            <input
              id="edit-product-sku"
              type="text"
              className="retail-edit-input retail-edit-input--readonly"
              value={product.sku}
              disabled
              readOnly
            />
          </div>

          <div className="retail-edit-form-group">
            <Localized id="retail-edit-field-name">
              <label htmlFor="edit-product-name" className="retail-edit-label">
                Product Name
              </label>
            </Localized>
            <input
              id="edit-product-name"
              type="text"
              className="retail-edit-input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
          </div>

          <div className="retail-edit-form-row">
            <div className="retail-edit-form-group">
              <Localized id="retail-edit-field-price">
                <label htmlFor="edit-product-price" className="retail-edit-label">
                  Price (IDR)
                </label>
              </Localized>
              <input
                id="edit-product-price"
                type="number"
                min="0"
                step="1"
                className="retail-edit-input"
                value={priceMinor}
              onChange={(e) => {
                  const v = e.target.value;
                  setPriceMinor(v === '' ? '' : Math.max(0, parseInt(v, 10) || 0));
                }}
                required
              />
            </div>

            <div className="retail-edit-form-group">
              <Localized id="retail-edit-field-stock">
                <label htmlFor="edit-product-stock" className="retail-edit-label">
                  Stock Quantity
                </label>
              </Localized>
              <input
                id="edit-product-stock"
                type="number"
                min="0"
                step="1"
                className="retail-edit-input"
                value={stockQty}
                onChange={(e) => {
                  const v = e.target.value;
                  setStockQty(v === '' ? '' : Math.max(0, parseInt(v, 10) || 0));
                }}
                required
              />
            </div>
          </div>

          <div className="retail-edit-form-row">
            <div className="retail-edit-form-group">
              <Localized id="retail-edit-field-low-stock">
                <label htmlFor="edit-product-low-threshold" className="retail-edit-label">
                  Low Stock Threshold
                </label>
              </Localized>
              <input
                id="edit-product-low-threshold"
                type="number"
                min="0"
                step="1"
                className="retail-edit-input"
                value={lowThreshold}
                onChange={(e) => {
                  const v = e.target.value;
                  setLowThreshold(v === '' ? '' : Math.max(0, parseInt(v, 10) || 0));
                }}
                required
              />
            </div>

            <div className="retail-edit-form-group">
              <Localized id="retail-edit-field-high-stock">
                <label htmlFor="edit-product-high-threshold" className="retail-edit-label">
                  High Stock Threshold
                </label>
              </Localized>
              <input
                id="edit-product-high-threshold"
                type="number"
                min="0"
                step="1"
                className="retail-edit-input"
                value={highThreshold}
                onChange={(e) => {
                  const v = e.target.value;
                  setHighThreshold(v === '' ? '' : Math.max(0, parseInt(v, 10) || 0));
                }}
                required
              />
            </div>
          </div>

          {/* ── ADR #36 attributes ── */}
          <div className="retail-edit-form-row">
            {canEditCost && (
            <div className="retail-edit-form-group">
              <Localized id="retail-edit-field-cost">
                <label htmlFor="edit-product-cost" className="retail-edit-label">
                  Cost (IDR)
                </label>
              </Localized>
              <input
                id="edit-product-cost"
                type="number"
                min="0"
                step="1"
                className="retail-edit-input"
                value={costMinor}
                onChange={(e) => {
                  const v = e.target.value;
                  setCostMinor(v === '' ? '' : Math.max(0, parseInt(v, 10) || 0));
                }}
              />
              {restocking && (
                <div className="retail-edit-cost-override-hint" role="note">
                  {requiredLocalized(l10n, 'retail-edit-cost-override-hint')}
                </div>
              )}
            </div>
            )}

            <div className="retail-edit-form-group">
              <Localized id="retail-edit-field-unit">
                <label htmlFor="edit-product-unit" className="retail-edit-label">
                  Unit
                </label>
              </Localized>
              <input
                id="edit-product-unit"
                type="text"
                className="retail-edit-input"
                value={unit}
                onChange={(e) => setUnit(e.target.value)}
                placeholder="pcs / kg / box"
              />
            </div>
          </div>

          <div className="retail-edit-form-row">
            <div className="retail-edit-form-group">
              <Localized id="retail-edit-field-brand">
                <label htmlFor="edit-product-brand" className="retail-edit-label">
                  Brand
                </label>
              </Localized>
              <input
                id="edit-product-brand"
                type="text"
                className="retail-edit-input"
                value={brand}
                onChange={(e) => setBrand(e.target.value)}
              />
            </div>

            <div className="retail-edit-form-group">
              <Localized id="retail-edit-field-rack">
                <label htmlFor="edit-product-rack" className="retail-edit-label">
                  Rack
                </label>
              </Localized>
              <input
                id="edit-product-rack"
                type="text"
                className="retail-edit-input"
                value={rackLocation}
                onChange={(e) => setRackLocation(e.target.value)}
                placeholder="A-01"
              />
            </div>
          </div>

          <div className="retail-edit-form-group">
            <Localized id="retail-edit-field-notes">
              <label htmlFor="edit-product-notes" className="retail-edit-label">
                Notes
              </label>
            </Localized>
            <textarea
              id="edit-product-notes"
              rows={2}
              className="retail-edit-input"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
            />
          </div>

          <label
            className="retail-edit-checkbox"
            htmlFor="edit-product-active"
            aria-label={requiredLocalized(l10n, 'retail-edit-field-active')}
          >
            <input
              id="edit-product-active"
              type="checkbox"
              checked={isActive}
              onChange={(e) => setIsActive(e.target.checked)}
            />
            <Localized id="retail-edit-field-active">
              <span>Active (sellable)</span>
            </Localized>
          </label>

          <div className="retail-edit-modal-actions">
            <Localized id="retail-edit-cancel">
              <button
                type="button"
                className="retail-edit-modal-btn retail-edit-modal-btn--secondary"
                onClick={onClose}
              >
                Cancel
              </button>
            </Localized>
            <Localized id="retail-edit-save">
              <button
                type="submit"
                className="retail-edit-modal-btn retail-edit-modal-btn--primary"
              >
                Save Changes
              </button>
            </Localized>
          </div>
        </form>
      </div>
    </div>
    </>
  );
};
