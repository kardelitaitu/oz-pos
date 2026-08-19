// ── IPC contract tests for data.ts ────────────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  getBackupStatus,
  createBackup,
  exportData,
  importPreview,
  importData,
} from '@/api/data';

describe('data.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('getBackupStatus → get_backup_status (no args)', async () => {
    mockInvoke.mockResolvedValue({ lastBackup: null, backupCount: 0 });
    await getBackupStatus();
    expect(mockInvoke).toHaveBeenCalledWith('get_backup_status', undefined);
  });

  it('createBackup → create_backup (no args)', async () => {
    mockInvoke.mockResolvedValue({ success: true, path: '/backups/db.sqlite' });
    await createBackup();
    expect(mockInvoke).toHaveBeenCalledWith('create_backup', undefined);
  });

  it('exportData → export_data with args', async () => {
    mockInvoke.mockResolvedValue({ success: true, path: '/exports/data.json' });
    await exportData({ password: 'secret123' });
    expect(mockInvoke).toHaveBeenCalledWith('export_data', { args: { password: 'secret123' } });
  });

  it('importPreview → import_preview with file_path + password', async () => {
    mockInvoke.mockResolvedValue({ valid: true, recordCount: 100 });
    await importPreview('/exports/data.json', 'secret123');
    expect(mockInvoke).toHaveBeenCalledWith('import_preview', { args: { file_path: '/exports/data.json', password: 'secret123' } });
  });

  it('importData → import_data with file_path + password', async () => {
    mockInvoke.mockResolvedValue({ success: true, imported: 100 });
    await importData('/exports/data.json', 'secret123');
    expect(mockInvoke).toHaveBeenCalledWith('import_data', { args: { file_path: '/exports/data.json', password: 'secret123' } });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('backup failed'));
    await expect(createBackup()).rejects.toThrow('backup failed');
  });
});
