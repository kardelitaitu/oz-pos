import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  createBackup,
  getBackupStatus,
  exportData,
  importPreview,
  importData,
} from '@/api/data';

describe('data.ts API contract', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('getBackupStatus calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue({ lastBackup: null });
    await getBackupStatus();
    expect(mockInvoke).toHaveBeenCalledWith('get_backup_status');
  });

  it('createBackup calls correct command', async () => {
    mockInvoke.mockResolvedValue({ path: '/backups/db.db' });
    const result = await createBackup();
    expect(mockInvoke).toHaveBeenCalledWith('create_backup');
    expect(result.path).toBe('/backups/db.db');
  });

  it('exportData calls correct command', async () => {
    const args = { types: ['products'], password: 'secret', outputPath: '/tmp/export.csv' };
    mockInvoke.mockResolvedValue({ rows: 10 });
    await exportData(args);
    expect(mockInvoke).toHaveBeenCalledWith('export_data', { args });
  });

  it('importPreview calls correct command', async () => {
    mockInvoke.mockResolvedValue({ rows: [], errors: [] });
    await importPreview('/path/to/file.csv', 'pass');
    expect(mockInvoke).toHaveBeenCalledWith('import_preview', { args: { file_path: '/path/to/file.csv', password: 'pass' } });
  });

  it('importData calls correct command', async () => {
    mockInvoke.mockResolvedValue({ imported: 5 });
    await importData('/path/to/file.csv', 'pass');
    expect(mockInvoke).toHaveBeenCalledWith('import_data', { args: { file_path: '/path/to/file.csv', password: 'pass' } });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('backup failed'));
    await expect(createBackup()).rejects.toThrow('backup failed');
  });
});
