import { describe, expect, it, vi } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import purchasingFtl from '@/locales/purchasing.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

vi.mock('@/api/purchasing', () => ({
  listSuppliers: vi.fn(),
  createSupplier: vi.fn(),
  updateSupplier: vi.fn(),
}));

import SuppliersScreen from '@/features/purchasing/SuppliersScreen';
import { listSuppliers, createSupplier } from '@/api/purchasing';

const mockListSuppliers = listSuppliers as ReturnType<typeof vi.fn>;
const mockCreateSupplier = createSupplier as ReturnType<typeof vi.fn>;

// Inline FTL for keys not in the bundle yet.
const supplierFtl = `
supplier-name-required = Name is required
supplier-code-required = Code is required
supplier-save-failed = Save failed
suppliers-add-title = Add Supplier
suppliers-edit-title = Edit Supplier
suppliers-btn-create = Create
suppliers-btn-update = Update
suppliers-btn-cancel = Cancel
`;



const sampleSuppliers = [
  {
    id: 'sup-1', code: 'SUP001', name: 'Acme Corp',
    contact_person: 'John', phone: '555-0100', email: 'john@acme.com',
    address: '123 Main St', tax_id: 'TAX-001', payment_terms: 'NET30',
    notes: '', status: 'active', created_at: '2026-01-01', updated_at: '2026-01-01',
  },
  {
    id: 'sup-2', code: 'SUP002', name: 'Global Goods',
    contact_person: 'Jane', phone: '555-0200', email: 'jane@global.com',
    address: '', tax_id: '', payment_terms: '',
    notes: 'Preferred vendor', status: 'active', created_at: '2026-02-01', updated_at: '2026-02-01',
  },
];

describe('SuppliersScreen', () => {
  // ── List rendering ───────────────────────────────────────────
  it('renders the title and Add Supplier button', async () => {
    mockListSuppliers.mockResolvedValue([]);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);
    expect(screen.getByText('Suppliers')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add supplier/i })).toBeInTheDocument();
  });

  it('loads and displays suppliers in the table', async () => {
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await waitFor(() => {
      expect(screen.getByText('SUP001')).toBeInTheDocument();
    });
    expect(screen.getByText('Acme Corp')).toBeInTheDocument();
    expect(screen.getByText('Global Goods')).toBeInTheDocument();
    expect(screen.getByText('John')).toBeInTheDocument();
    expect(screen.getByText('555-0100')).toBeInTheDocument();
    expect(screen.getByText('john@acme.com')).toBeInTheDocument();
  });

  it('shows loading skeleton while fetching suppliers', async () => {
    mockListSuppliers.mockReturnValue(new Promise(() => {}));
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);
    expect(document.querySelector('.suppliers-loading-skeleton')).toBeInTheDocument();
    expect(screen.queryByText(/loading suppliers/i)).not.toBeInTheDocument();
  });

  it('shows empty state when no suppliers exist', async () => {
    mockListSuppliers.mockResolvedValue([]);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);
    await waitFor(() => {
      expect(screen.getByText(/no suppliers yet/i)).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: /add your first supplier/i })).toBeInTheDocument();
  });

  it('filters suppliers by search query', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await waitFor(() => {
      expect(screen.getByText('Acme Corp')).toBeInTheDocument();
    });

    const searchInput = screen.getByRole('searchbox');
    await user.type(searchInput, 'global');

    await waitFor(() => {
      expect(screen.getByText('Global Goods')).toBeInTheDocument();
      expect(screen.queryByText('Acme Corp')).not.toBeInTheDocument();
    });
  });

  it('shows no-match message when search filters everything out', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await waitFor(() => {
      expect(screen.getByText('Acme Corp')).toBeInTheDocument();
    });

    await user.type(screen.getByRole('searchbox'), 'zzz404');

    await waitFor(() => {
      expect(screen.getByText(/no suppliers match your search/i)).toBeInTheDocument();
    });
  });

  it('renders status badges with correct class', async () => {
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await waitFor(() => {
      const badges = screen.getAllByText('active');
      expect(badges.length).toBeGreaterThanOrEqual(2);
      if (badges[0]) {
        expect(badges[0].className).toContain('suppliers-badge--active');
      }
    });
  });

  // ── Create/Edit modal ────────────────────────────────────────
  it('opens create modal when Add Supplier is clicked', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue([]);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await user.click(screen.getByRole('button', { name: /add supplier/i }));

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    // "Add Supplier" appears in both the dialog heading and the Fluent Localized text.
    const addSupplierTexts = screen.getAllByText('Add Supplier');
    expect(addSupplierTexts.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Code *')).toBeInTheDocument();
    expect(screen.getByText('Name *')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /create/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
  });

  it('opens edit modal when Edit button is clicked on a row', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await waitFor(() => {
      expect(screen.getByText('SUP001')).toBeInTheDocument();
    });

    const editButtons = screen.getAllByRole('button', { name: /edit/i });
    await user.click(editButtons[0]!);

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Edit Supplier')).toBeInTheDocument();
    // Form pre-filled with existing code.
    expect(screen.getByDisplayValue('SUP001')).toBeInTheDocument();
  });

  it('enables save button when both code and name are filled', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue([]);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await user.click(screen.getByRole('button', { name: /add supplier/i }));

    // Initially disabled.
    expect(screen.getByRole('button', { name: /create/i })).toBeDisabled();

    // Fill both required fields.
    const inputs = screen.getAllByRole('textbox');
    await user.type(inputs[0]!, 'SUP-001');
    await user.type(inputs[1]!, 'Test Co');

    // Now enabled.
    expect(screen.getByRole('button', { name: /create/i })).toBeEnabled();
  });

  it('creates a supplier and refreshes the list', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
    mockCreateSupplier.mockResolvedValue({ id: 'sup-3', code: 'SUP-NEW', name: 'New Co', status: 'active' });
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await user.click(screen.getByRole('button', { name: /add supplier/i }));

    const inputs = screen.getAllByRole('textbox');
    await user.type(inputs[0]!, 'SUP-NEW');
    await user.type(inputs[1]!, 'New Co');

    await user.click(screen.getByRole('button', { name: /create/i }));

    await waitFor(() => {
      expect(mockCreateSupplier).toHaveBeenCalledTimes(1);
      expect(mockCreateSupplier).toHaveBeenCalledWith(
        expect.objectContaining({ code: 'SUP-NEW', name: 'New Co' }),
      );
    });
  });

  it('shows error when createSupplier fails', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue(sampleSuppliers);
    mockCreateSupplier.mockRejectedValue(new Error('Duplicate code'));
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await user.click(screen.getByRole('button', { name: /add supplier/i }));

    const inputs = screen.getAllByRole('textbox');
    await user.type(inputs[0]!, 'SUP-DUP');
    await user.type(inputs[1]!, 'Duplicate Co');

    await user.click(screen.getByRole('button', { name: /create/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Duplicate code');
    });
  });

  it('closes modal when Cancel is clicked', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue([]);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await user.click(screen.getByRole('button', { name: /add supplier/i }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /cancel/i }));
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  it('closes modal when X button is clicked', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue([]);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await user.click(screen.getByRole('button', { name: /add supplier/i }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /close/i }));
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  it('renders all optional fields in the form', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue([]);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await user.click(screen.getByRole('button', { name: /add supplier/i }));

    expect(screen.getByText('Contact Person')).toBeInTheDocument();
    expect(screen.getByText('Phone')).toBeInTheDocument();
    expect(screen.getByText('Email')).toBeInTheDocument();
    expect(screen.getByText('Address')).toBeInTheDocument();
    expect(screen.getByText('Tax ID')).toBeInTheDocument();
    expect(screen.getByText('Payment Terms')).toBeInTheDocument();
    expect(screen.getByText('Notes')).toBeInTheDocument();
  });

  it('disables save button when code or name is empty', async () => {
    const user = userEvent.setup();
    mockListSuppliers.mockResolvedValue([]);
    renderWithFluentSync(<SuppliersScreen />, purchasingFtl, sharedFtl, supplierFtl);

    await user.click(screen.getByRole('button', { name: /add supplier/i }));

    expect(screen.getByRole('button', { name: /create/i })).toBeDisabled();
  });
});
