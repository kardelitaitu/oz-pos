import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import StaffManagementScreen from '@/features/staff/StaffManagementScreen';

const LOCALE_STRINGS = [
  'staff-title = Staff',
  'staff-add-button = Add Staff',
  'staff-loading = Loading staff…',
  'staff-empty = No staff members yet.',
  'staff-empty-cta = Add your first staff member',
  'staff-col-name = Name',
  'staff-col-username = Username',
  'staff-col-role = Role',
  'staff-col-status = Status',
  'staff-status-active = Active',
  'staff-status-inactive = Inactive',
  'staff-edit = Edit',
  'staff-deactivate = Deactivate',
  'staff-restore = Restore',
  'staff-modal-add-title = Add Staff Member',
  'staff-modal-edit-title = Edit Staff Member',
  'staff-modal-close = Close',
  'staff-field-username-label = Username *',
  'staff-field-name-label = Display Name *',
  'staff-field-pin-label = PIN * (4+ characters)',
  'staff-field-role-label = Role *',
  'staff-role-select-default = Select a role…',
  'staff-btn-cancel = Cancel',
  'staff-btn-create = Create',
  'staff-btn-update = Update',
  'staff-error-generic = { $message }',
  'staff-modal-add-aria = Add staff member',
  'staff-modal-edit-aria = Edit staff member',
  'staff-edit-aria = { $name }',
  'staff-deactivate-aria = { $name }',
  'staff-restore-aria = { $name }',
  'staff-col-actions = Actions',
  'staff-username-placeholder = e.g. jane',
  'staff-name-placeholder = e.g. Jane Smith',
  'staff-pin-placeholder = Enter PIN',
  'staff-field-pin-edit-label = New PIN (leave blank to keep current)',
  'staff-pin-edit-placeholder = Leave blank to keep current',
].join('\n');

const wrap = (children: React.ReactNode) => {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(new FluentResource(LOCALE_STRINGS));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
};

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
  invokeMock: vi.fn() as any,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockClear();
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'list_staff') return Promise.resolve(SAMPLE_STAFF);
    if (cmd === 'list_roles') return Promise.resolve(SAMPLE_ROLES);
    if (cmd === 'create_staff') return Promise.resolve({ ...SAMPLE_STAFF[0], username: 'newuser' });
    if (cmd === 'update_staff') return Promise.resolve(SAMPLE_STAFF[0]);
    return Promise.reject(new Error(`Unknown command: ${cmd}`));
  });
});

async function waitForTable() {
  await screen.findByRole('table', { name: /staff members/i });
}

describe('StaffManagementScreen', () => {
  it('renders title and add button', async () => {
    render(wrap(<StaffManagementScreen />));
    await waitForTable();
    expect(screen.getByRole('heading', { name: /staff/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add staff/i })).toBeInTheDocument();
  });

  it('renders staff table rows', async () => {
    render(wrap(<StaffManagementScreen />));
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
      return Promise.resolve([]);
    });
    render(wrap(<StaffManagementScreen />));
    await waitFor(() => {
      expect(screen.getByText(/no staff members yet/i)).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: /add your first staff member/i })).toBeInTheDocument();
  });

  it('shows loading state initially', async () => {
    invokeMock.mockImplementation(() => new Promise(() => {}));
    render(wrap(<StaffManagementScreen />));
    expect(screen.getByText(/loading staff/i)).toBeInTheDocument();
  });

  it('opens add modal', async () => {
    render(wrap(<StaffManagementScreen />));
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /add staff/i }));
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveTextContent(/add staff member/i);
  });

  it('opens edit modal pre-filled', async () => {
    render(wrap(<StaffManagementScreen />));
    await waitForTable();
    const editBtn = screen.getByRole('button', { name: /edit jane smith/i });
    await userEvent.click(editBtn);
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveTextContent(/edit staff member/i);
  });
});
