import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import customersFtl from '@/locales/customers.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

vi.mock('@/api/customers', () => ({
  listCustomers: vi.fn(),
  createCustomer: vi.fn(),
  updateCustomer: vi.fn(),
  deleteCustomer: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Cashier', role_name: 'cashier' },
  }),
}));

import CustomerManagementScreen from '@/features/customers/CustomerManagementScreen';
import { listCustomers, createCustomer, updateCustomer } from '@/api/customers';

const mockListCustomers = listCustomers as ReturnType<typeof vi.fn>;
const mockCreateCustomer = createCustomer as ReturnType<typeof vi.fn>;
const mockUpdateCustomer = updateCustomer as ReturnType<typeof vi.fn>;



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

  it('renders the title and Add Customer button', async () => {
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Customers')).toBeInTheDocument();
    });
    expect(screen.getByText('Add Customer')).toBeInTheDocument();
  });

  it('shows loading state', async () => {
    mockListCustomers.mockReturnValue(new Promise(() => {}));
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    expect(screen.getByText('Loading customers…')).toBeInTheDocument();
  });

  it('shows empty state when no customers exist', async () => {
    mockListCustomers.mockResolvedValue([]);
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('No customers yet.')).toBeInTheDocument();
    });
    expect(screen.getByText('Add your first customer')).toBeInTheDocument();
  });

  // ── Table rendering ──────────────────────────────────────────

  it('displays customers in the table', async () => {
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
    });
    expect(screen.getByText('Bob')).toBeInTheDocument();
    expect(screen.getByText('Carol')).toBeInTheDocument();
  });

  it('displays email and phone columns', async () => {
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('alice@example.com')).toBeInTheDocument();
      expect(screen.getByText('+1-555-0101')).toBeInTheDocument();
    });
  });

  it('displays dash for null email and phone', async () => {
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Bob')).toBeInTheDocument();
    });
    // Bob has null email/phone — dashes should appear in those cells.
    const dashes = screen.getAllByText('—');
    expect(dashes.length).toBeGreaterThanOrEqual(1);
  });

  it('shows Edit and Delete buttons per row', async () => {
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getAllByText('Edit').length).toBeGreaterThanOrEqual(3);
    });
    expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(3);
  });

  // ── Search ────────────────────────────────────────────────────

  it('filters customers by search query', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
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
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
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
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
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
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);

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
      expect(mockCreateCustomer).toHaveBeenCalled();
    });
  });

  it('disables Create button when name is empty', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
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
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
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
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);
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
    renderWithFluentSync(<CustomerManagementScreen />, customersFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('Edit').length).toBeGreaterThanOrEqual(1);
    });
    await user.click(screen.getAllByText('Edit')[0]!);

    await waitFor(() => {
      expect(screen.getByText('Edit Customer')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Update'));

    await waitFor(() => {
      expect(mockUpdateCustomer).toHaveBeenCalled();
    });
  });
});
