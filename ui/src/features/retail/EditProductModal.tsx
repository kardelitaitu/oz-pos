import React, { useState, useEffect, useRef } from 'react';
import { Localized } from '@fluent/react';
import { useFocusTrap } from '@/hooks/useFocusTrap';
import type { ProductDto } from '@/api/products';

export interface EditProductModalProps {
  product: ProductDto | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (updatedProduct: ProductDto) => void;
}

export const EditProductModal: React.FC<EditProductModalProps> = ({
  product,
  isOpen,
  onClose,
  onSave,
}) => {
  const [name, setName] = useState('');
  const [priceMinor, setPriceMinor] = useState<number | ''>(0);
  const [stockQty, setStockQty] = useState<number | ''>(0);
  const [lowThreshold, setLowThreshold] = useState<number | ''>(5);
  const [highThreshold, setHighThreshold] = useState<number | ''>(10);

  useEffect(() => {
    if (isOpen && product) {
      setName(product.name || '');
      setPriceMinor(product.price?.minor_units ?? 0);
      setStockQty(product.stock_qty ?? 0);
      setLowThreshold(product.low_stock_threshold ?? 5);
      setHighThreshold(product.high_stock_threshold ?? 10);
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
    };

    onSave(updatedProduct);
    onClose();
  };

  return (
    <div className="retail-edit-modal-backdrop" onClick={onClose} role="presentation">
      <div
        ref={panelRef}
        className="retail-edit-modal-dialog"
        onClick={(e) => e.stopPropagation()}
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
          <button
            type="button"
            className="retail-edit-modal-close"
            onClick={onClose}
            aria-label="Close"
          >
            &times;
          </button>
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
  );
};
