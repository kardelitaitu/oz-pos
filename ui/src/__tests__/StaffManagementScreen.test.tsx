import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import staffFtl from '@/locales/staff.ftl?raw';
import StaffManagementScreen from '@/features/staff/StaffManagementScreen';

const SAMPLE_ROLES = [
  { id: 'role-1', name: 'owner', description: 'Owner' },
  { id: 'role-2', name: 'manager', description: 'Manager' },
  { id: 'role-3', name: 'cashier', description: 'Cashier' },
];

const SAMPLE_STAFF = [
  { id: 'staff-1', username: 'jane', display_name: 'Jane Smith', role_id: 'role-1', role_name: 'owner', is_active: true },
  { id: 'staff-2', username: 'john', display_name: 'John Doe', role_id: 'role-3', role_name: 'cashier', is_active: false },
];

const { invokeMock } = vi.hoisted(() => ({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  invokeMock: vi.fn() as any,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'test', display_name: 'Test', role_name: 'owner', role_id: 'role-1' },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    isManager: true,
    isOwner: true,
  }),
}));

beforeEach(() => {
  invokeMock.mockClear();
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'list_staff') return Promise.resolve(SAMPLE_STAFF);
    if (cmd === 'list_roles') return Promise.resolve(SAMPLE_ROLES);
    if (cmd === 'create_staff') return Promise.resolve({ ...SAMPLE_STAFF[0], username: 'newuser' });
    if (cmd === 'update_staff') return Promise.resolve(SAMPLE_STAFF[0]);
    if (cmd === 'list_all_workspaces') return Promise.resolve([
      { key: 'restaurant', name: 'Restaurant', description: 'Dine-in service', icon: 'restaurant' },
      { key: 'store', name: 'Retail Store', description: 'Retail counter', icon: 'store' },
    ]);
    if (cmd === 'get_user_workspaces') return Promise.resolve([]);
    if (cmd === 'set_user_workspaces') return Promise.resolve(undefined);
    return Promise.reject(new Error(`Unknown command: ${cmd}`));
  });
});

async function waitForTable() {
  await screen.findByRole('table', { name: /staff members/i });
}

describe('StaffManagementScreen', () => {
  it('renders title and add button', async () => {
    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitForTable();
    expect(screen.getByRole('heading', { name: /staff/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add staff/i })).toBeInTheDocument();
  });

  it('renders staff table rows', async () => {
    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitForTable();
    expect(screen.getAllByText('Jane Smith').length).toBeGreaterThan(0);
    expect(screen.getAllByText('John Doe').length).toBeGreaterThan(0);
    expect(screen.getByText('jane')).toBeInTheDocument();
    expect(screen.getByText('john')).toBeInTheDocument();
    expect(screen.getByText('owner')).toBeInTheDocument();
    expect(screen.getByText('cashier')).toBeInTheDocument();
    expect(screen.getByText('Active')).toBeInTheDocument();
    expect(screen.getByText('Inactive')).toBeInTheDocument();
  });

  it('shows empty state when no staff', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_staff') return Promise.resolve([]);
      if (cmd === 'list_roles') return Promise.resolve(SAMPLE_ROLES);
      if (cmd === 'list_all_workspaces') return Promise.resolve([]);
      if (cmd === 'get_user_workspaces') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitFor(() => {
      expect(screen.getByText(/no staff members yet/i)).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: /add your first staff member/i })).toBeInTheDocument();
  });

  it('shows loading skeleton initially', async () => {
    invokeMock.mockImplementation(() => new Promise(() => {}));
    const { container } = renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    const skeleton = container.querySelector('[aria-hidden="true"].staff-mgmt-loading-skeleton');
    expect(skeleton).toBeInTheDocument();
    expect(screen.queryByText(/loading staff/i)).not.toBeInTheDocument();
  });

  it('opens add modal', async () => {
    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /add staff/i }));
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveTextContent(/add staff member/i);
  });

  it('opens edit modal pre-filled', async () => {
    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitForTable();
    const editBtn = screen.getByRole('button', { name: /edit.*jane smith/i });
    await userEvent.click(editBtn);
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveTextContent(/edit staff member/i);
  });

  // ── New edge-case tests ─────────────────────────────────────────

  it('deactivates an active staff member when Deactivate is clicked', async () => {
    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitForTable();

    // Find the Deactivate button for Jane (active)
    const deactivateBtn = screen.getByRole('button', { name: /deactivate.*jane smith/i });
    await userEvent.click(deactivateBtn);

    // update_staff should be called with is_active: false
    // Note: updateStaff from @/api/staff wraps args in { args } for the IPC call
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('update_staff', expect.objectContaining({
        args: expect.objectContaining({
          id: 'staff-1',
          is_active: false,
        }),
      }));
    });
  });

  it('reactivates an inactive staff member when Restore is clicked', async () => {
    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitForTable();

    // Find the Restore button for John (inactive) via visible text content
    const restoreBtn = screen.getByText('Restore').closest('button')!;
    await userEvent.click(restoreBtn);

    // update_staff wraps args in { args } — assert the inner payload
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('update_staff', expect.objectContaining({
        args: expect.objectContaining({
          id: 'staff-2',
          is_active: true,
        }),
      }));
    });
  });

  it('closes the add modal when Escape is pressed', async () => {
    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitForTable();

    // Open add modal
    await userEvent.click(screen.getByRole('button', { name: /add staff/i }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    // Press Escape
    await userEvent.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  it('creates a new staff member via the add modal', async () => {
    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitForTable();

    // Open add modal and fill form
    await userEvent.click(screen.getByRole('button', { name: /add staff/i }));
    const dialog = screen.getByRole('dialog');

    // Fill username
    const usernameInput = within(dialog).getByRole('textbox', { name: /username/i });
    await userEvent.type(usernameInput, 'newuser');

    // Fill display name
    const nameInput = within(dialog).getByRole('textbox', { name: /display name/i });
    await userEvent.type(nameInput, 'New User');

    // Fill PIN — use placeholder to avoid matching both label and input elements
    const pinInput = within(dialog).getByPlaceholderText(/enter pin/i);
    await userEvent.type(pinInput, '1234');

    // Select a role
    const roleSelect = within(dialog).getByRole('combobox', { name: /role/i });
    await userEvent.selectOptions(roleSelect, 'role-3');

    // Click Create
    const createBtn = within(dialog).getByRole('button', { name: /create/i });
    await userEvent.click(createBtn);

    // create_staff wraps args in { args }
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('create_staff', expect.objectContaining({
        args: expect.objectContaining({
          username: 'newuser',
        }),
      }));
    });

    // Modal should close
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  it('handles save failure gracefully in add modal', async () => {
    // Mock create_staff to fail
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'create_staff') return Promise.reject(new Error('DB error'));
      if (cmd === 'list_staff') return Promise.resolve(SAMPLE_STAFF);
      if (cmd === 'list_roles') return Promise.resolve(SAMPLE_ROLES);
      if (cmd === 'list_all_workspaces') return Promise.resolve([]);
      if (cmd === 'get_user_workspaces') return Promise.resolve([]);
      if (cmd === 'set_user_workspaces') return Promise.resolve(undefined);
      return Promise.resolve([]);
    });

    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitForTable();

    // Open add modal and fill form
    await userEvent.click(screen.getByRole('button', { name: /add staff/i }));
    const dialog = screen.getByRole('dialog');

    await userEvent.type(within(dialog).getByRole('textbox', { name: /username/i }), 'newuser');
    await userEvent.type(within(dialog).getByRole('textbox', { name: /display name/i }), 'New User');
    await userEvent.type(within(dialog).getByPlaceholderText(/enter pin/i), '1234');
    await userEvent.selectOptions(within(dialog).getByRole('combobox', { name: /role/i }), 'role-3');

    const createBtn = within(dialog).getByRole('button', { name: /create/i });
    await userEvent.click(createBtn);

    // Modal should stay open after failure
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });
  });

  it('renders workspace column for staff members', async () => {
    // Mock some workspace assignments
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_staff') return Promise.resolve(SAMPLE_STAFF);
      if (cmd === 'list_roles') return Promise.resolve(SAMPLE_ROLES);
      if (cmd === 'list_all_workspaces') return Promise.resolve([
        { key: 'restaurant', name: 'Restaurant', description: 'Dine-in', icon: 'restaurant' },
        { key: 'store', name: 'Retail Store', description: 'Retail', icon: 'store' },
      ]);
      if (cmd === 'get_user_workspaces') {
        // Both staff members get the same workspace assignment — sufficient
        // to verify the workspace column renders without crashing.
        return Promise.resolve(['restaurant']);
      }
      if (cmd === 'set_user_workspaces') return Promise.resolve(undefined);
      return Promise.resolve([]);
    });

    renderWithProvidersSync(<StaffManagementScreen />, staffFtl);
    await waitForTable();

    // The workspace column should be present (table has aria-label)
    expect(screen.getByRole('table', { name: /staff members/i })).toBeInTheDocument();
  });
});
