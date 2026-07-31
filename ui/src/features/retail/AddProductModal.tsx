import type React from 'react';
import { requiredLocalized } from '@/frontend/shared';
import { useState, useEffect, useRef } from 'react';
import { useLocalization, Localized } from '@fluent/react';
import { useFocusTrap } from '@/hooks/useFocusTrap';
import type { ProductDto, CategoryDto } from '@/api/products';

export interface AddProductModalProps {
  categories: CategoryDto[];
  isOpen: boolean;
  onClose: () => void;
  onSave: (newProduct: ProductDto) => void;
}

export const AddProductModal: React.FC<AddProductModalProps> = ({
  categories,
  isOpen,
  onClose,
  onSave,
}) => {
  const { l10n } = useLocalization();
  const [sku, setSku] = useState('');
  const [name, setName] = useState('');
  const [category, setCategory] = useState('');
  const [priceMinor, setPriceMinor] = useState<number | ''>(0);
  const [stockQty, setStockQty] = useState<number | ''>(10);
  const [lowThreshold, setLowThreshold] = useState<number | ''>(5);
  const [highThreshold, setHighThreshold] = useState<number | ''>(10);

  const hasInitialized = useRef(false);

  useEffect(() => {
    if (isOpen && !hasInitialized.current) {
      const generatedSku = `PROD-${Math.floor(1000 + Math.random() * 9000)}`;
      setSku(generatedSku);
      setName('');
      setCategory(categories[0]?.name || '');
      setPriceMinor(0);
      setStockQty(10);
      setLowThreshold(5);
      setHighThreshold(10);
      hasInitialized.current = true;
    }
    if (!isOpen) hasInitialized.current = false;
  }, [isOpen, categories]);

  const panelRef = useRef<HTMLDivElement>(null);
  useFocusTrap(panelRef, isOpen, onClose);
  const nameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) nameInputRef.current?.focus();
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !sku.trim()) return;

    const newProduct: ProductDto = {
      sku: sku.trim().toUpperCase(),
      name: name.trim(),
      category: category || categories[0]?.name || 'General',
      price: {
        minor_units: Math.max(0, priceMinor === '' ? 0 : priceMinor),
        currency: 'IDR',
      },
      barcode: null,
      in_stock: (stockQty === '' ? 0 : Math.max(0, stockQty)) > 0,
      stock_qty: Math.max(0, stockQty === '' ? 0 : stockQty),
      tax_rate_ids: [],
      created_at: new Date().toISOString(),
      price_updated_at: new Date().toISOString(),
      product_type: 'retail',
      low_stock_threshold: Math.max(0, lowThreshold === '' ? 0 : lowThreshold),
      high_stock_threshold: Math.max(0, highThreshold === '' ? 0 : highThreshold),
    };

    onSave(newProduct);
    onClose();
  };

  return (
    <>
    <div className="retail-edit-modal-backdrop" onClick={(e) => { if (e.target === e.currentTarget) onClose(); }} onKeyDown={(e) => { if (e.key === 'Escape') onClose(); }} role="presentation" tabIndex={-1}>
      <div
        ref={panelRef}
        className="retail-edit-modal-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="retail-add-product-title"
      >
        <div className="retail-edit-modal-header">
          <Localized id="retail-add-product-title">
            <h3 id="retail-add-product-title" className="retail-edit-modal-title">
              Add New Product
            </h3>
          </Localized>
          <Localized id="retail-edit-modal-close-aria">
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
          <div className="retail-edit-form-row">
            <div className="retail-edit-form-group">
              <Localized id="retail-edit-field-sku">
                <label htmlFor="add-product-sku" className="retail-edit-label">
                  SKU / Code
                </label>
              </Localized>
              <input
                id="add-product-sku"
                type="text"
                className="retail-edit-input"
                value={sku}
                onChange={(e) => setSku(e.target.value)}
                required
              />
            </div>

            <div className="retail-edit-form-group">
              <Localized id="retail-add-product-category-label">
                <label htmlFor="add-product-category" className="retail-edit-label">
                  Category
                </label>
              </Localized>
              <select
                id="add-product-category"
                className="retail-edit-input"
                value={category}
                onChange={(e) => setCategory(e.target.value)}
              >
                {categories.map((cat) => (
                  <option key={cat.id} value={cat.name}>
                    {cat.name}
                  </option>
                ))}
              </select>
            </div>
          </div>

          <div className="retail-edit-form-group">
            <Localized id="retail-edit-field-name">
              <label htmlFor="add-product-name" className="retail-edit-label">
                Product Name
              </label>
            </Localized>
            <input
              id="add-product-name"
              type="text"
              className="retail-edit-input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={requiredLocalized(l10n, 'retail-add-product-name-placeholder')}
              ref={nameInputRef}
              required
            />
          </div>

          <div className="retail-edit-form-row">
            <div className="retail-edit-form-group">
              <Localized id="retail-edit-field-price">
                <label htmlFor="add-product-price" className="retail-edit-label">
                  Price (IDR)
                </label>
              </Localized>
              <input
                id="add-product-price"
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
                <label htmlFor="add-product-stock" className="retail-edit-label">
                  Stock Quantity
                </label>
              </Localized>
              <input
                id="add-product-stock"
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
                <label htmlFor="add-product-low" className="retail-edit-label">
                  Low Stock Threshold
                </label>
              </Localized>
              <input
                id="add-product-low"
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
                <label htmlFor="add-product-high" className="retail-edit-label">
                  High Stock Threshold
                </label>
              </Localized>
              <input
                id="add-product-high"
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
                Create Product
              </button>
            </Localized>
          </div>
        </form>
      </div>
    </div>
    </>
  );
};
