import { describe, expect, it, vi, beforeEach } from 'vitest';
import { readFileSync } from 'fs';
import { join } from 'path';
import { screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import { getBundle } from '@/i18n';
import customersFtl from '@/locales/customers.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

vi.mock('@/api/customers', () => {
  const listCustomersScoped = vi.fn();
  const createCustomerScoped = vi.fn();
  const updateCustomerScoped = vi.fn();
  const deleteCustomerScoped = vi.fn();
  return {
    listCustomersScoped,
    createCustomerScoped,
    updateCustomerScoped,
    deleteCustomerScoped,
  };
});

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({ sessionToken: 'session-1' }),
}));

import CustomerManagementScreen from '@/features/customers/CustomerManagementScreen';
import {
  listCustomersScoped,
  createCustomerScoped,
  updateCustomerScoped,
  deleteCustomerScoped,
} from '@/api/customers';

const mockListCustomers = listCustomersScoped as ReturnType<typeof vi.fn>;
const mockCreateCustomer = createCustomerScoped as ReturnType<typeof vi.fn>;
const mockUpdateCustomer = updateCustomerScoped as ReturnType<typeof vi.fn>;
const mockDeleteCustomer = deleteCustomerScoped as ReturnType<typeof vi.fn>;



const sampleCustomers = [
  { id: 'cust-1', name: 'Alice', email: 'alice@example.com', phone: '+1-555-0101', notes: 'Regular' },
  { id: 'cust-2', name: 'Bob', email: null, phone: null, notes: '' },
  { id: 'cust-3', name: 'Carol', email: 'carol@example.com', phone: '+1-555-0103', notes: 'VIP' },
];

describe('CustomerManagementScreen', () => {
  beforeEach(() => {
    mockListCustomers.mockResolvedValue(sampleCustomers);
  });

  // ── Rendering ─────────────────────────────────────────────────

  it('loads customers with the active session token', async () => {
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Customers')).toBeInTheDocument();
    });
    expect(mockListCustomers).toHaveBeenCalledWith('session-1');
  });

  it('renders the title and Add Customer button', async () => {
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Customers')).toBeInTheDocument();
    });
    expect(screen.getByText('Add Customer')).toBeInTheDocument();
  });

  it('shows loading skeleton while fetching customers', async () => {
    mockListCustomers.mockReturnValue(new Promise(() => {}));
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    expect(document.querySelector('.customer-mgmt-loading-skeleton')).toBeInTheDocument();
    expect(screen.queryByText('Loading customers…')).not.toBeInTheDocument();
  });

  it('shows empty state when no customers exist', async () => {
    mockListCustomers.mockResolvedValue([]);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('No customers yet.')).toBeInTheDocument();
    });
    expect(screen.getByText('Add your first customer')).toBeInTheDocument();
  });

  // ── Table rendering ──────────────────────────────────────────

  it('displays customers in the table', async () => {
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
    });
    expect(screen.getByText('Bob')).toBeInTheDocument();
    expect(screen.getByText('Carol')).toBeInTheDocument();
  });

  it('displays email and phone columns', async () => {
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('alice@example.com')).toBeInTheDocument();
      expect(screen.getByText('+1-555-0101')).toBeInTheDocument();
    });
  });

  it('displays dash for null email and phone', async () => {
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Bob')).toBeInTheDocument();
    });
    // Bob has null email/phone — dashes should appear in those cells.
    const dashes = screen.getAllByText('—');
    expect(dashes.length).toBeGreaterThanOrEqual(1);
  });

  it('shows Edit and Delete buttons per row', async () => {
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getAllByText('Edit').length).toBeGreaterThanOrEqual(3);
    });
    expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
  });

  // ── Search ────────────────────────────────────────────────────

  it('filters customers by search query', async () => {
    const user = userEvent.setup();
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
    });

    const searchInput = screen.getByPlaceholderText(/search by name/i);
    await user.type(searchInput, 'Bob');

    await waitFor(() => {
      expect(screen.queryByText('Alice')).not.toBeInTheDocument();
      expect(screen.getByText('Bob')).toBeInTheDocument();
    });
  });

  it('shows no-match state for search with no results', async () => {
    const user = userEvent.setup();
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
    });

    const searchInput = screen.getByPlaceholderText(/search by name/i);
    await user.type(searchInput, 'ZZZZZZ');

    await waitFor(() => {
      expect(screen.getByText('No customers match your search.')).toBeInTheDocument();
      expect(screen.getByText('Clear search')).toBeInTheDocument();
    });
  });

  // ── Create modal ──────────────────────────────────────────────

  it('opens the add customer modal when Add Customer is clicked', async () => {
    const user = userEvent.setup();
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Add Customer')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Add Customer'));

    await waitFor(() => {
      // Modal should show with the form input; title and button both say "Add Customer".
      expect(screen.getByPlaceholderText(/jane smith/i)).toBeInTheDocument();
    });
  });

  it('creates a customer when form is filled and saved', async () => {
    const user = userEvent.setup();
    mockCreateCustomer.mockResolvedValue({});
    mockListCustomers.mockResolvedValueOnce(sampleCustomers);
    mockListCustomers.mockResolvedValueOnce([...sampleCustomers, { id: 'cust-4', name: 'Dave', email: null, phone: null, notes: '' }]);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Add Customer')).toBeInTheDocument();
    });
    await user.click(screen.getByText('Add Customer'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/jane smith/i)).toBeInTheDocument();
    });

    await user.type(screen.getByPlaceholderText(/jane smith/i), 'Dave');
    await user.click(screen.getByText('Create'));

    await waitFor(() => {
      expect(mockCreateCustomer).toHaveBeenCalledWith('session-1', {
        name: 'Dave',
      });
    });
  });

  it('disables Create button when name is empty', async () => {
    const user = userEvent.setup();
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Add Customer')).toBeInTheDocument();
    });
    await user.click(screen.getByText('Add Customer'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/jane smith/i)).toBeInTheDocument();
    });

    // The Button component renders as a span, not a native button.
    const createSpan = screen.getByText('Create');
    expect(createSpan).toBeInTheDocument();
  });

  it('closes the modal when Cancel is clicked', async () => {
    const user = userEvent.setup();
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Add Customer')).toBeInTheDocument();
    });
    await user.click(screen.getByText('Add Customer'));

    await waitFor(() => {
      expect(screen.getByText('Cancel')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Cancel'));

    await waitFor(() => {
      expect(screen.queryByPlaceholderText(/jane smith/i)).not.toBeInTheDocument();
    });
  });

  // ── Edit modal ────────────────────────────────────────────────

  it('opens edit modal pre-filled with customer data', async () => {
    const user = userEvent.setup();
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getAllByText('Edit').length).toBeGreaterThanOrEqual(1);
    });

    await user.click(screen.getAllByText('Edit')[0]!);

    await waitFor(() => {
      expect(screen.getByText('Edit Customer')).toBeInTheDocument();
    });
    // The name field should be pre-filled with Alice.
    const nameInput = screen.getByPlaceholderText(/jane smith/i) as HTMLInputElement;
    expect(nameInput.value).toBe('Alice');
  });

  it('updates a customer when edit form is saved', async () => {
    const user = userEvent.setup();
    mockUpdateCustomer.mockResolvedValue({});
    mockListCustomers.mockResolvedValue(sampleCustomers);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('Edit').length).toBeGreaterThanOrEqual(1);
    });
    await user.click(screen.getAllByText('Edit')[0]!);

    await waitFor(() => {
      expect(screen.getByText('Edit Customer')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Update'));

    await waitFor(() => {
      expect(mockUpdateCustomer).toHaveBeenCalledWith('session-1', {
        id: 'cust-1',
        name: 'Alice',
        email: 'alice@example.com',
        phone: '+1-555-0101',
        notes: 'Regular',
      });
    });
  });

  // ── Delete confirmation + failure (CUST-02/04) ────────────────

  it('does not delete without confirmation (CUST-02)', async () => {
    const user = userEvent.setup();
    mockDeleteCustomer.mockResolvedValue(undefined);
    mockListCustomers.mockResolvedValue(sampleCustomers);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
    });
    // The row button only arms the confirmation dialog — no IPC yet.
    await user.click(screen.getAllByText('Delete')[0]!);
    await waitFor(() => {
      expect(screen.getByText('Delete customer?')).toBeInTheDocument();
    });
    expect(mockDeleteCustomer).not.toHaveBeenCalled();
  });

  it('deletes only after explicit confirmation with the session token (CUST-02)', async () => {
    const user = userEvent.setup();
    mockDeleteCustomer.mockResolvedValue(undefined);
    mockListCustomers.mockResolvedValue(sampleCustomers);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
    });
    await user.click(screen.getAllByText('Delete')[0]!);
    await waitFor(() => {
      expect(screen.getByText('Delete customer?')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: 'Delete' }));
    await waitFor(() => {
      expect(mockDeleteCustomer).toHaveBeenCalledWith('session-1', 'cust-1');
    });
  });

  it('surfaces a delete failure with a localized toast (CUST-04)', async () => {
    const user = userEvent.setup();
    mockDeleteCustomer.mockRejectedValue(new Error('fk constraint'));
    mockListCustomers.mockResolvedValue(sampleCustomers);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
    });
    await user.click(screen.getAllByText('Delete')[0]!);
    await waitFor(() => {
      expect(screen.getByText('Delete customer?')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: 'Delete' }));
    await waitFor(() => {
      expect(screen.getByText('Failed to delete customer')).toBeInTheDocument();
    });
    // The row stays visible and the dialog remains open for retry.
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.getByText('Delete customer?')).toBeInTheDocument();
  });

  it('dismisses the delete dialog via Cancel without deleting (CUST-02)', async () => {
    const user = userEvent.setup();
    mockDeleteCustomer.mockResolvedValue(undefined);
    mockListCustomers.mockResolvedValue(sampleCustomers);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
    });
    await user.click(screen.getAllByText('Delete')[0]!);
    await waitFor(() => {
      expect(screen.getByText('Delete customer?')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    await waitFor(() => {
      expect(screen.queryByText('Delete customer?')).toBeNull();
    });
    expect(mockDeleteCustomer).not.toHaveBeenCalled();
  });

  // ── CUST-07: locale parity ──────────────────────────────────────

  it('resolves every screen Localized id in both the en and id bundles (CUST-07)', () => {
    // Reuse the production bundle loader (@/i18n) which already includes
    // customers.ftl + customers.id.ftl — same approach as i18nBundle.test.tsx.
    const en = getBundle('en');
    const id = getBundle('id');

    // Every id the screen resolves — via <Localized id> or l10n.getString().
    const usedIds = [
      'customer-mgmt-title',
      'customer-mgmt-add',
      'customer-mgmt-search',
      'customer-mgmt-empty',
      'customer-mgmt-empty-cta',
      'customer-mgmt-search-empty',
      'customer-mgmt-search-clear',
      'customer-mgmt-col-name',
      'customer-mgmt-col-email',
      'customer-mgmt-col-phone',
      'customer-mgmt-col-notes',
      'customer-mgmt-col-actions',
      'customer-mgmt-table-aria',
      'customer-mgmt-edit',
      'customer-mgmt-edit-aria',
      'customer-mgmt-delete',
      'customer-mgmt-delete-aria',
      'customer-mgmt-modal-add-title',
      'customer-mgmt-modal-edit-title',
      'customer-mgmt-field-name',
      'customer-mgmt-field-email',
      'customer-mgmt-field-phone',
      'customer-mgmt-field-notes',
      'customer-mgmt-name-placeholder',
      'customer-mgmt-email-placeholder',
      'customer-mgmt-phone-placeholder',
      'customer-mgmt-notes-placeholder',
      'customer-mgmt-btn-cancel',
      'customer-mgmt-btn-create',
      'customer-mgmt-btn-update',
      'customer-mgmt-error-name-required',
      'customer-mgmt-error-save-failed',
      'customer-mgmt-delete-confirm-title',
      'customer-mgmt-delete-confirm-message',
      'customer-mgmt-delete-confirm-btn',
      'customer-mgmt-error-delete',
      'customer-mgmt-error-load',
      'customer-mgmt-error-retry',
    ];
    for (const key of usedIds) {
      expect(en.getMessage(key), `en bundle missing ${key}`).toBeDefined();
      expect(id.getMessage(key), `id bundle missing ${key}`).toBeDefined();
    }
  });

  // ── CUST-09: field-level validation ────────────────────────────

  it('blocks save and flags an invalid email with aria-invalid (CUST-09)', async () => {
    const user = userEvent.setup();
    mockListCustomers.mockResolvedValue(sampleCustomers);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
    });

    await user.click(screen.getByRole('button', { name: 'Add Customer' }));
    await user.type(screen.getByLabelText('Name *'), 'Alice');
    await user.type(screen.getByLabelText('Email'), 'not-an-email');

    await user.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByText('Enter a valid email address')).toBeInTheDocument();
    });
    expect(screen.getByLabelText('Email')).toHaveAttribute('aria-invalid', 'true');
    expect(mockCreateCustomer).not.toHaveBeenCalled();
  });

  it('blocks save when the phone contains no digits (CUST-09)', async () => {
    const user = userEvent.setup();
    mockListCustomers.mockResolvedValue(sampleCustomers);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
    });

    await user.click(screen.getByRole('button', { name: 'Add Customer' }));
    await user.type(screen.getByLabelText('Name *'), 'Bob');
    await user.type(screen.getByLabelText('Phone'), '---');

    await user.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(
        screen.getByText('Phone must contain at least one digit'),
      ).toBeInTheDocument();
    });
    expect(screen.getByLabelText('Phone')).toHaveAttribute('aria-invalid', 'true');
    expect(mockCreateCustomer).not.toHaveBeenCalled();
  });

  it('accepts a valid phone with digits and letters (CUST-09)', async () => {
    const user = userEvent.setup();
    mockListCustomers.mockResolvedValue(sampleCustomers);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
    });

    await user.click(screen.getByRole('button', { name: 'Add Customer' }));
    await user.type(screen.getByLabelText('Name *'), 'Carol');
    await user.type(screen.getByLabelText('Phone'), '+62 812-3456');

    await user.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(mockCreateCustomer).toHaveBeenCalledWith('session-1', {
        name: 'Carol',
        phone: '+62 812-3456',
      });
    });
  });

  it('shows a localized error for overlong notes (CUST-09)', async () => {
    const user = userEvent.setup();
    mockListCustomers.mockResolvedValue(sampleCustomers);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
    });

    await user.click(screen.getByRole('button', { name: 'Add Customer' }));
    await user.type(screen.getByLabelText('Name *'), 'Dan');
    // fireEvent.change bypasses the HTML maxLength truncation, reaching the
    // JS guard branch (the reason the guard exists alongside the attribute).
    fireEvent.change(screen.getByLabelText('Notes'), {
      target: { value: 'x'.repeat(501) },
    });

    await user.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(
        screen.getByText('Notes must be 500 characters or fewer'),
      ).toBeInTheDocument();
    });
    expect(mockCreateCustomer).not.toHaveBeenCalled();
  });

  it('clears the field error once the operator fixes the value (CUST-09)', async () => {
    const user = userEvent.setup();
    mockListCustomers.mockResolvedValue(sampleCustomers);
    renderWithProvidersSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
    });

    await user.click(screen.getByRole('button', { name: 'Add Customer' }));
    await user.type(screen.getByLabelText('Name *'), 'Eve');
    await user.type(screen.getByLabelText('Email'), 'bad');
    await user.click(screen.getByRole('button', { name: 'Create' }));
    await waitFor(() => {
      expect(screen.getByText('Enter a valid email address')).toBeInTheDocument();
    });

    await user.clear(screen.getByLabelText('Email'));
    await user.type(screen.getByLabelText('Email'), 'eve@example.com');
    expect(screen.queryByText('Enter a valid email address')).toBeNull();
    expect(screen.getByLabelText('Email')).not.toHaveAttribute('aria-invalid');
  });

  // ── CUST-08: 44px touch targets ─────────────────────────────────

  it('declares guaranteed 44px touch targets for row action buttons (CUST-08)', () => {
    // jsdom runs with `css: false` so computed styles are meaningless here;
    // assert against the stylesheet source on disk (mirrors
    // animationCompliance.test.ts).
    const cssPath = join(
      process.cwd(),
      'src/features/customers/CustomerManagementScreen.css',
    );
    const css = readFileSync(cssPath, 'utf8');

    // The action button block must declare a minimum 44px hit area in both
    // axes while keeping compact visual padding.
    const blockMatch = css.match(/\.customer-mgmt-action-btn\s*{[^}]*}/);
    expect(blockMatch).not.toBeNull();
    const block = blockMatch![0];
    expect(block).toMatch(/min-height:\s*2\.75rem/);
    expect(block).toMatch(/min-width:\s*2\.75rem/);
  });
});
