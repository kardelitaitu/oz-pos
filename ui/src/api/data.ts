// ── Data Management IPC: backup, export/import .ozpkg ──────────

import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';

// ── Types ─────────────────────────────────────────────────────

export interface BackupStatus {
  lastBackup: string | null;
  lastBackupSize: string | null;
  dbPath: string;
}

export interface BackupResult {
  path: string;
  sizeBytes: number;
}

export interface ExportDataArgs {
  types: string[];
  password: string;
  outputPath: string;
  dateFrom?: string;
  dateTo?: string;
}

export interface ExportDataResult {
  path: string;
  sizeBytes: number;
  types: string[];
}

export interface ImportPreviewResult {
  storeName: string;
  appVersion: string;
  createdAt: string;
  types: string[];
  productCount: number;
  categoryCount: number;
  saleCount: number | null;
  customerCount: number | null;
  userCount: number | null;
  settingCount: number | null;
}

export interface ImportDataResult {
  productsImported: number;
  categoriesImported: number;
  salesImported: number;
  customersImported: number;
  usersImported: number;
  settingsImported: number;
}

// ── File dialog helpers ───────────────────────────────────────

export const pickExportPath = async (): Promise<string | null> => {
  const path = await save({
    defaultPath: `ozpos_export_${new Date().toISOString().slice(0, 10)}.ozpkg`,
    filters: [{ name: 'OZ-POS Export', extensions: ['ozpkg'] }],
  });
  return path;
};

export const pickImportFile = async (): Promise<string | null> => {
  const path = await open({
    filters: [{ name: 'OZ-POS Export', extensions: ['ozpkg'] }],
    multiple: false,
  });
  return path;
};

// ── IPC calls ─────────────────────────────────────────────────

export const getBackupStatus = (): Promise<BackupStatus> =>
  invoke<BackupStatus>('get_backup_status');

export const createBackup = (): Promise<BackupResult> =>
  invoke<BackupResult>('create_backup');

export const exportData = (args: ExportDataArgs): Promise<ExportDataResult> =>
  invoke<ExportDataResult>('export_data', { args });

export const importPreview = (filePath: string, password: string): Promise<ImportPreviewResult> =>
  invoke<ImportPreviewResult>('import_preview', { args: { file_path: filePath, password } });

export const importData = (filePath: string, password: string): Promise<ImportDataResult> =>
  invoke<ImportDataResult>('import_data', { args: { file_path: filePath, password } });
