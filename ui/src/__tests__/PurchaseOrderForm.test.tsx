import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// Mock the purchasing API — use vi.fn() inline to avoid hoisting issues.
vi.mock('@/api/purchasing', () => ({
  createPurchaseOrder: vi.fn(),
  listSuppliers: vi.fn(),
}));

import PurchaseOrderForm from '@/features/purchasing/PurchaseOrderForm';
import { createPurchaseOrder, listSuppliers } from '@/api/purchasing';

const mockCreatePO = createPurchaseOrder as ReturnType<typeof vi.fn>;
const mockListSuppliers = listSuppliers as ReturnType<typeof vi.fn>;

// Sample supplier data for the dropdown.
const sampleSuppliers = [
  { id: 'sup-1', code: 'SUP001', name: 'Acme Corp', status: 'active' },
  { id: 'sup-2', code: 'SUP002', name: 'Global Goods', status: 'active' },
];

describe('PurchaseOrderForm', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
  });

  it('renders the form with title and fields', async () => {
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);

    expect(screen.getByText('New Purchase Order')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('PO-001')).toBeInTheDocument();
    expect(screen.getByText('-- Select --')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '+ Add Line' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /create po/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
  });

  it('loads suppliers into the dropdown', async () => {
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);

    // Suppliers are loaded async.
    expect(mockListSuppliers).toHaveBeenCalledTimes(1);
    await vi.waitFor(() => {
      expect(screen.getByText('Acme Corp (SUP001)')).toBeInTheDocument();
      expect(screen.getByText('Global Goods (SUP002)')).toBeInTheDocument();
    });
  });

  it('shows empty supplier dropdown without error when listSuppliers fails', async () => {
    mockListSuppliers.mockRejectedValueOnce(new Error('Network down'));

    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);

    // Should not crash; dropdown stays with just "-- Select --".
    await vi.waitFor(() => {
      expect(screen.getByText('-- Select --')).toBeInTheDocument();
    });
    // No alert or crash.
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('shows Edit title when editingId is set', () => {
    render(<PurchaseOrderForm editingId="po-existing" onClose={vi.fn()} onSaved={vi.fn()} />);
    expect(screen.getByText('Edit Purchase Order')).toBeInTheDocument();
  });

  it('calls onClose when cancel is clicked', async () => {
    const onClose = vi.fn();
    render(<PurchaseOrderForm editingId={null} onClose={onClose} onSaved={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: /cancel/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when X button is clicked', async () => {
    const onClose = vi.fn();
    render(<PurchaseOrderForm editingId={null} onClose={onClose} onSaved={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: /close/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('button is disabled when PO number or supplier is missing', () => {
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);
    const btn = screen.getByRole('button', { name: /create po/i });
    expect(btn).toBeDisabled();
  });

  it('button enables when both PO number and supplier are filled', async () => {
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);
    await userEvent.type(screen.getByPlaceholderText('PO-001'), 'PO-001');
    const select = screen.getByRole('combobox');
    await userEvent.selectOptions(select, 'sup-1');
    expect(screen.getByRole('button', { name: /create po/i })).toBeEnabled();
  });

  it('validates SKU is required on each line', async () => {
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);

    await userEvent.type(screen.getByPlaceholderText('PO-001'), 'PO-001');
    // Select first supplier.
    const select = screen.getByRole('combobox');
    await userEvent.selectOptions(select, 'sup-1');
    // Leave the default empty SKU line.
    await userEvent.click(screen.getByRole('button', { name: /create po/i }));

    expect(screen.getByRole('alert')).toHaveTextContent('Each line must have a SKU');
  });

  it('adds a new line row when + Add Line is clicked', async () => {
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);
    // Start with 1 SKU input.
    const initialSkuInputs = screen.getAllByPlaceholderText('SKU');
    expect(initialSkuInputs).toHaveLength(1);

    await userEvent.click(screen.getByRole('button', { name: '+ Add Line' }));

    const afterSkuInputs = screen.getAllByPlaceholderText('SKU');
    expect(afterSkuInputs).toHaveLength(2);
  });

  it('removes a line row when remove button is clicked', async () => {
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: '+ Add Line' }));
    expect(screen.getAllByPlaceholderText('SKU')).toHaveLength(2);

    const removeButtons = screen.getAllByRole('button', { name: /remove line/i });
    await userEvent.click(removeButtons[0]);

    expect(screen.getAllByPlaceholderText('SKU')).toHaveLength(1);
  });

  it('does not show remove button when only one line remains', () => {
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);
    expect(screen.queryByRole('button', { name: /remove line/i })).not.toBeInTheDocument();
  });

  it('updates line fields — SKU, product name, qty, unit cost', async () => {
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);

    const skuInput = screen.getByPlaceholderText('SKU');
    const nameInput = screen.getByPlaceholderText('Product name');
    const qtyInput = screen.getByDisplayValue('1');
    const costInput = screen.getByPlaceholderText('in cents');

    await userEvent.clear(skuInput);
    await userEvent.type(skuInput, 'SKU-123');
    await userEvent.type(nameInput, 'Widget');
    await userEvent.clear(qtyInput);
    await userEvent.type(qtyInput, '5');
    await userEvent.clear(costInput);
    await userEvent.type(costInput, '2000');

    expect(skuInput).toHaveValue('SKU-123');
    expect(nameInput).toHaveValue('Widget');
    expect(qtyInput).toHaveValue(5);
    expect(costInput).toHaveValue(2000);
  });

  it('shows computed line total and subtotal', async () => {
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);

    const qtyInput = screen.getByDisplayValue('1');
    const costInput = screen.getByPlaceholderText('in cents');

    await userEvent.clear(qtyInput);
    await userEvent.type(qtyInput, '3');
    await userEvent.clear(costInput);
    await userEvent.type(costInput, '1500');

    // Line total: 3 × 1500 = 4500 cents → $45.00
    // The line total cell + subtotal value should both show 45.00.
    const totalCells = screen.getAllByText('45.00');
    expect(totalCells.length).toBeGreaterThanOrEqual(2);
  });

  it('allows entering expected date and notes', async () => {
    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);

    const dateInput = screen.getByLabelText('Expected Date');
    await userEvent.type(dateInput, '2026-08-01');
    expect(dateInput).toHaveValue('2026-08-01');

    const notesInput = screen.getByPlaceholderText('Optional notes');
    await userEvent.type(notesInput, 'Rush order');
    expect(notesInput).toHaveValue('Rush order');
  });

  it('successfully submits and calls onSaved', async () => {
    mockCreatePO.mockResolvedValueOnce({ id: 'po-new' });
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
    const onSaved = vi.fn();

    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={onSaved} />);

    await userEvent.type(screen.getByPlaceholderText('PO-001'), 'PO-001');
    const select = screen.getByRole('combobox');
    await userEvent.selectOptions(select, 'sup-1');
    await userEvent.type(screen.getByPlaceholderText('SKU'), 'SKU-001');
    await userEvent.type(screen.getByPlaceholderText('Product name'), 'Test Product');

    await userEvent.click(screen.getByRole('button', { name: /create po/i }));

    expect(mockCreatePO).toHaveBeenCalledTimes(1);
    expect(mockCreatePO).toHaveBeenCalledWith(
      expect.objectContaining({
        po_number: 'PO-001',
        supplier_id: 'sup-1',
        lines: expect.arrayContaining([
          expect.objectContaining({ sku: 'SKU-001', product_name: 'Test Product' }),
        ]),
      }),
    );

    await vi.waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
  });

  it('shows error when createPurchaseOrder fails', async () => {
    mockCreatePO.mockRejectedValueOnce(new Error('Network failure'));
    mockListSuppliers.mockResolvedValue(sampleSuppliers);

    render(<PurchaseOrderForm editingId={null} onClose={vi.fn()} onSaved={vi.fn()} />);

    await userEvent.type(screen.getByPlaceholderText('PO-001'), 'PO-FAIL');
    const select = screen.getByRole('combobox');
    await userEvent.selectOptions(select, 'sup-1');
    await userEvent.type(screen.getByPlaceholderText('SKU'), 'SKU-X');

    await userEvent.click(screen.getByRole('button', { name: /create po/i }));

    await vi.waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Network failure');
    });
  });


});
