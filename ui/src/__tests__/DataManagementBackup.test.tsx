import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import DataManagementScreen from '@/features/settings/DataManagementScreen';

// ── Shared mocks ─────────────────────────────────────────────────

const mockGetBackupStatus = vi.fn();
const mockCreateBackup = vi.fn();
const mockExportData = vi.fn();
const mockImportPreview = vi.fn();
const mockImportData = vi.fn();
const mockPickExportPath = vi.fn();
const mockPickImportFile = vi.fn();

vi.mock('@/api/data', () => ({
  getBackupStatus: () => mockGetBackupStatus(),
  createBackup: () => mockCreateBackup(),
  exportData: (args: unknown) => mockExportData(args),
  importPreview: (filePath: string, password: string) => mockImportPreview(filePath, password),
  importData: (filePath: string, password: string) => mockImportData(filePath, password),
  pickExportPath: () => mockPickExportPath(),
  pickImportFile: () => mockPickImportFile(),
}));

const mockAddToast = vi.fn();

vi.mock('@/frontend/shared/Toast', () => ({
  useToast: () => ({ addToast: mockAddToast }),
}));

vi.mock('@fluent/react', () => ({
  useLocalization: () => ({
    l10n: {
      getString: (id: string, args?: Record<string, unknown>) => {
        if (args) return `${id} ${JSON.stringify(args)}`;
        return id;
      },
    },
  }),
  Localized: ({ children }: { id: string; children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/components/Card', () => ({
  Card: ({ children, shadow }: { children: React.ReactNode; shadow?: string }) => (
    <div data-testid="card" data-shadow={shadow}>{children}</div>
  ),
}));

vi.mock('@/components/Button', () => ({
  Button: ({ children, onClick, variant, disabled, loading }: {
    children: React.ReactNode; onClick?: () => void; variant?: string;
    disabled?: boolean; loading?: boolean;
  }) => (
    <button onClick={onClick} className={`btn btn--${variant ?? 'primary'}`}
      disabled={disabled || loading} data-loading={loading || undefined}>
      {children}
    </button>
  ),
}));

vi.mock('@/components/Spinner', () => ({
  Spinner: ({ size }: { size?: string }) => (
    <div data-testid="spinner" data-size={size}>Loading…</div>
  ),
}));

// ── Default API responses ────────────────────────────────────────

const defaultBackupStatus = { lastBackup: null, lastBackupSize: null, dbPath: '/path/to/db.sqlite3' };

beforeEach(() => {
  mockGetBackupStatus.mockResolvedValue(defaultBackupStatus);
  mockCreateBackup.mockResolvedValue({ path: '/backups/backup_2026.db', sizeBytes: 12_582_912 });
  mockExportData.mockResolvedValue({ path: '/exports/export_2026.ozpkg', sizeBytes: 524_288, types: [] });
  mockImportPreview.mockResolvedValue({
    storeName: 'Test Store', appVersion: '0.0.4',
    createdAt: new Date('2026-01-15').toISOString(),
    types: [], productCount: 0, categoryCount: 0, saleCount: 0,
    customerCount: 0, userCount: 0, settingCount: 0,
  });
  mockImportData.mockResolvedValue({
    productsImported: 0, categoriesImported: 0, salesImported: 0,
    customersImported: 0, usersImported: 0, settingsImported: 0,
  });
  mockPickExportPath.mockResolvedValue('/exports/test.ozpkg');
  mockPickImportFile.mockResolvedValue('/imports/test.ozpkg');
  mockAddToast.mockReturnValue(undefined);
});

// ── Helpers ──────────────────────────────────────────────────────

async function clickTab(label: string) {
  const user = userEvent.setup();
  const tabs = screen.getAllByRole('tab');
  const tab = tabs.find((t) => t.textContent?.includes(label));
  if (!tab) throw new Error(`Tab "${label}" not found`);
  await user.click(tab);
}

describe('DataManagement — Backup', () => {
  it('shows "Never" when no backup has been created', async () => {
    render(<DataManagementScreen />);
    await waitFor(() => expect(screen.getByText('Backup')).toBeInTheDocument());
    await clickTab('Backup');
    await waitFor(() => {
      expect(screen.getByText('data-mgmt-backup-never')).toBeInTheDocument();
    });
  });

  it('shows last backup time when a backup exists', async () => {
    mockGetBackupStatus.mockResolvedValue({
      lastBackup: '2026-07-13 14:30:00', lastBackupSize: '12.0 MB', dbPath: '/path/to/db.sqlite3',
    });
    render(<DataManagementScreen />);
    await waitFor(() => expect(screen.getByText('Backup')).toBeInTheDocument());
    await clickTab('Backup');
    await waitFor(() => {
      expect(screen.getByText('2026-07-13 14:30:00')).toBeInTheDocument();
    });
  });

  it('shows backup size when available', async () => {
    mockGetBackupStatus.mockResolvedValue({
      lastBackup: '2026-07-13 14:30:00', lastBackupSize: '15.7 MB', dbPath: '/path/to/db.sqlite3',
    });
    render(<DataManagementScreen />);
    await waitFor(() => expect(screen.getByText('Backup')).toBeInTheDocument());
    await clickTab('Backup');
    await waitFor(() => expect(screen.getByText('15.7 MB')).toBeInTheDocument());
  });

  it('calls createBackup when "Create backup now" is clicked', async () => {
    const user = userEvent.setup();
    mockCreateBackup.mockReturnValue(new Promise(() => {}));
    render(<DataManagementScreen />);
    await waitFor(() => expect(screen.getByText('Backup')).toBeInTheDocument());
    await clickTab('Backup');
    await waitFor(() => expect(screen.getByText('Create backup now')).toBeInTheDocument());
    await user.click(screen.getByText('Create backup now'));
    expect(mockCreateBackup).toHaveBeenCalled();
  });

  it('shows "Backing up…" text while backup is in progress', async () => {
    const user = userEvent.setup();
    mockCreateBackup.mockReturnValue(new Promise(() => {}));
    render(<DataManagementScreen />);
    await waitFor(() => expect(screen.getByText('Backup')).toBeInTheDocument());
    await clickTab('Backup');
    await waitFor(() => expect(screen.getByText('Create backup now')).toBeInTheDocument());
    await user.click(screen.getByText('Create backup now'));
    await waitFor(() => expect(screen.getByText('Backing up…')).toBeInTheDocument());
  });

  it('updates last backup time on backup success', async () => {
    const user = userEvent.setup();
    mockCreateBackup.mockResolvedValue({ path: '/backups/backup_now.db', sizeBytes: 5_000_000 });
    render(<DataManagementScreen />);
    await waitFor(() => expect(screen.getByText('Backup')).toBeInTheDocument());
    await clickTab('Backup');
    await waitFor(() => expect(screen.getByText('Create backup now')).toBeInTheDocument());
    await user.click(screen.getByText('Create backup now'));
    await waitFor(() => {
      expect(screen.getByText('4.8 MB')).toBeInTheDocument();
      expect(screen.queryByText('data-mgmt-backup-never')).not.toBeInTheDocument();
    });
  });

  it('shows success toast on backup success', async () => {
    const user = userEvent.setup();
    mockCreateBackup.mockResolvedValue({ path: '/backups/backup_now.db', sizeBytes: 5_000_000 });
    render(<DataManagementScreen />);
    await waitFor(() => expect(screen.getByText('Backup')).toBeInTheDocument());
    await clickTab('Backup');
    await waitFor(() => expect(screen.getByText('Create backup now')).toBeInTheDocument());
    await user.click(screen.getByText('Create backup now'));
    await waitFor(() => {
      expect(mockAddToast).toHaveBeenCalledWith(
        expect.objectContaining({ message: 'data-mgmt-toast-backup-success', type: 'success' }),
      );
    });
  });

  it('shows error toast on backup failure', async () => {
    const user = userEvent.setup();
    mockCreateBackup.mockRejectedValue(new Error('Disk full'));
    render(<DataManagementScreen />);
    await waitFor(() => expect(screen.getByText('Backup')).toBeInTheDocument());
    await clickTab('Backup');
    await waitFor(() => expect(screen.getByText('Create backup now')).toBeInTheDocument());
    await user.click(screen.getByText('Create backup now'));
    await waitFor(() => {
      expect(mockAddToast).toHaveBeenCalledWith(
        expect.objectContaining({ message: 'data-mgmt-toast-backup-fail', type: 'error' }),
      );
    });
    expect(screen.queryByText('Backing up…')).not.toBeInTheDocument();
  });

  it('handles getBackupStatus failure gracefully', async () => {
    mockGetBackupStatus.mockRejectedValue(new Error('Network error'));
    render(<DataManagementScreen />);
    await waitFor(() => expect(screen.getByText('Backup')).toBeInTheDocument());
    await clickTab('Backup');
    await waitFor(() => {
      expect(screen.getByText('data-mgmt-backup-never')).toBeInTheDocument();
    });
  });
});
