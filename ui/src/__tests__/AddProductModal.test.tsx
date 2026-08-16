import type React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi } from 'vitest';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import { FluentResource, FluentBundle } from '@fluent/bundle';
import { AddProductModal } from '@/features/retail/AddProductModal';
import type { CategoryDto } from '@/api/products';

const sampleCategories: CategoryDto[] = [
  { id: 'cat-1', name: 'PC Components', colour: '#10b981', icon: '' },
  { id: 'cat-2', name: 'Monitors', colour: '#3b82f6', icon: '' },
];

const ftl = `
retail-add-product-title = Add New Product
retail-add-product-category-label = Category
retail-add-product-name-placeholder = e.g. Logitech G Pro X Wireless Mouse
retail-edit-modal-close-aria = Close
retail-edit-field-sku = SKU / Code
retail-edit-field-name = Product Name
retail-edit-field-price = Price (IDR)
retail-edit-field-stock = Stock Quantity
retail-edit-field-low-stock = Low Stock Threshold
retail-edit-field-high-stock = High Stock Threshold
retail-edit-save = Create Product
retail-edit-cancel = Cancel
`;

function wrapper({ children }: { children: React.ReactNode }) {
  const resource = new FluentResource(ftl);
  const bundle = new FluentBundle('en');
  bundle.addResource(resource);
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
}

describe('AddProductModal', () => {
  it('does not render when isOpen is false', () => {
    render(
      <AddProductModal
        categories={sampleCategories}
        isOpen={false}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
      { wrapper },
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders input fields when open', () => {
    render(
      <AddProductModal
        categories={sampleCategories}
        isOpen={true}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
      { wrapper },
    );
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('e.g. Logitech G Pro X Wireless Mouse')).toBeInTheDocument();
  });

  it('rejects fractional price input instead of truncating it', async () => {
    const user = userEvent.setup();

    render(
      <AddProductModal
        categories={sampleCategories}
        isOpen={true}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
      { wrapper },
    );

    // Price and cost both default to 0 (ADR #36), so disambiguate by order.
    const priceInput = screen.getAllByDisplayValue('0')[0] as HTMLInputElement;
    await user.clear(priceInput);
    await user.type(priceInput, '1850.5');

    // Fractional keystrokes are ignored — the field must not silently
    // truncate 1850.5 to 1850 via parseInt.
    expect(priceInput.value).toBe('1850');
  });

  it('creates new product when form is submitted', async () => {
    const user = userEvent.setup();
    const handleSave = vi.fn();
    const handleClose = vi.fn();

    render(
      <AddProductModal
        categories={sampleCategories}
        isOpen={true}
        onClose={handleClose}
        onSave={handleSave}
      />,
      { wrapper },
    );

    const nameInput = screen.getByPlaceholderText('e.g. Logitech G Pro X Wireless Mouse');
    await user.type(nameInput, 'Corsair Vengeance DDR5 32GB');

    // Price and cost both default to 0 (ADR #36), so disambiguate by order.
    const priceInput = screen.getAllByDisplayValue('0')[0] as HTMLElement;
    await user.clear(priceInput);
    await user.type(priceInput, '1850000');

    await user.click(screen.getByRole('button', { name: 'Create Product' }));

    expect(handleSave).toHaveBeenCalledWith(
      expect.objectContaining({
        sku: expect.stringMatching(/^PROD-\d{4}$/),
        name: 'Corsair Vengeance DDR5 32GB',
        category: 'PC Components',
        price: { minor_units: 1850000, currency: 'IDR' },
        stock_qty: 10,
        in_stock: true,
      }),
    );
    expect(handleClose).toHaveBeenCalled();
  });

  it('hides the cost field and saves cost 0 when canEditCost is false (ADR #36 D7)', async () => {
    const user = userEvent.setup();
    const handleSave = vi.fn();
    const handleClose = vi.fn();

    render(
      <AddProductModal
        categories={sampleCategories}
        isOpen={true}
        onClose={handleClose}
        onSave={handleSave}
        canEditCost={false}
      />,
      { wrapper },
    );

    // Cost label falls back to the JSX default (no FTL key in this test
    // bundle) — it must not be rendered for non-permitted sessions.
    expect(screen.queryByText('Cost (IDR)')).not.toBeInTheDocument();
    // The other ADR #36 fields still render.
    expect(screen.getByLabelText('Unit')).toBeInTheDocument();

    const nameInput = screen.getByPlaceholderText('e.g. Logitech G Pro X Wireless Mouse');
    await user.type(nameInput, 'Staff-Created Product');
    await user.click(screen.getByRole('button', { name: 'Create Product' }));

    expect(handleSave).toHaveBeenCalledWith(
      expect.objectContaining({ cost_minor: 0 }),
    );
    expect(handleClose).toHaveBeenCalled();
  });

  it('shows the cost field by default (manager session)', () => {
    render(
      <AddProductModal
        categories={sampleCategories}
        isOpen={true}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
      { wrapper },
    );
    expect(screen.getByText('Cost (IDR)')).toBeInTheDocument();
  });
});
