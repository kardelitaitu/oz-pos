import type React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi } from 'vitest';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import { FluentResource, FluentBundle } from '@fluent/bundle';
import { EditProductModal } from '@/features/retail/EditProductModal';
import type { ProductDto } from '@/api/products';

const sampleProduct: ProductDto = {
  sku: 'CPU-R7-7800X3D',
  name: 'AMD Ryzen 7 7800X3D 8-Core',
  category: 'cat-cpu',
  price: { minor_units: 6250000, currency: 'IDR' },
  barcode: '730143314930',
  in_stock: true,
  stock_qty: 15,
  product_type: 'retail',
  tax_rate_ids: [],
  created_at: '',
  price_updated_at: '',
  low_stock_threshold: 5,
  high_stock_threshold: 10,
};

const ftl = `
retail-edit-product-title = Edit Product
retail-edit-field-sku = SKU / Code
retail-edit-field-name = Product Name
retail-edit-field-price = Price (IDR)
retail-edit-field-stock = Stock Quantity
retail-edit-field-low-stock = Low Stock Threshold
retail-edit-field-high-stock = High Stock Threshold
retail-edit-save = Save Changes
retail-edit-cancel = Cancel
retail-edit-btn-aria = Edit product { $name }
`;

function wrapper({ children }: { children: React.ReactNode }) {
  const resource = new FluentResource(ftl);
  const bundle = new FluentBundle('en');
  bundle.addResource(resource);
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
}

describe('EditProductModal', () => {
  it('does not render when isOpen is false', () => {
    render(
      <EditProductModal
        product={sampleProduct}
        isOpen={false}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
      { wrapper },
    );

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders modal with initial product values when open', () => {
    render(
      <EditProductModal
        product={sampleProduct}
        isOpen={true}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
      { wrapper },
    );

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByDisplayValue('CPU-R7-7800X3D')).toBeInTheDocument();
    expect(screen.getByDisplayValue('AMD Ryzen 7 7800X3D 8-Core')).toBeInTheDocument();
    expect(screen.getByDisplayValue('6250000')).toBeInTheDocument();
    expect(screen.getByDisplayValue('15')).toBeInTheDocument();
    expect(screen.getByDisplayValue('5')).toBeInTheDocument();
    expect(screen.getByDisplayValue('10')).toBeInTheDocument();
  });

  it('calls onClose when Cancel or close X is clicked', async () => {
    const user = userEvent.setup();
    const handleClose = vi.fn();

    render(
      <EditProductModal
        product={sampleProduct}
        isOpen={true}
        onClose={handleClose}
        onSave={vi.fn()}
      />,
      { wrapper },
    );

    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(handleClose).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole('button', { name: 'Close' }));
    expect(handleClose).toHaveBeenCalledTimes(2);
  });

  it('submits updated product fields when Save Changes is clicked', async () => {
    const user = userEvent.setup();
    const handleSave = vi.fn();
    const handleClose = vi.fn();

    render(
      <EditProductModal
        product={sampleProduct}
        isOpen={true}
        onClose={handleClose}
        onSave={handleSave}
      />,
      { wrapper },
    );

    const nameInput = screen.getByDisplayValue('AMD Ryzen 7 7800X3D 8-Core');
    const priceInput = screen.getByDisplayValue('6250000');
    const stockInput = screen.getByDisplayValue('15');
    const lowInput = screen.getByDisplayValue('5');
    const highInput = screen.getByDisplayValue('10');

    await user.clear(nameInput);
    await user.type(nameInput, 'AMD Ryzen 7 7800X3D (Edited)');

    await user.clear(priceInput);
    await user.type(priceInput, '6500000');

    await user.clear(stockInput);
    await user.type(stockInput, '20');

    await user.clear(lowInput);
    await user.type(lowInput, '3');

    await user.clear(highInput);
    await user.type(highInput, '12');

    await user.click(screen.getByRole('button', { name: 'Save Changes' }));

    expect(handleSave).toHaveBeenCalledWith({
      ...sampleProduct,
      name: 'AMD Ryzen 7 7800X3D (Edited)',
      price: { minor_units: 6500000, currency: 'IDR' },
      stock_qty: 20,
      in_stock: true,
      low_stock_threshold: 3,
      high_stock_threshold: 12,
      // ADR #36 attributes default in the modal.
      cost_minor: 0,
      brand: null,
      rack_location: null,
      notes: null,
      unit: null,
      is_active: true,
      default_supplier_id: null,
      popularity_score: 0,
    });
    expect(handleClose).toHaveBeenCalled();
  });
});
