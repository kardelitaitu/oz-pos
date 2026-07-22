// ── Data Management IPC: backup, export/import .ozpkg ──────────

import { loggedInvoke } from '@/utils/logged-invoke';
import { open, save } from '@tauri-apps/plugin-dialog';

// ── Types ─────────────────────────────────────────────────────

/** Current backup status information. */
export interface BackupStatus {
  lastBackup: string | null;
  lastBackupSize: string | null;
  dbPath: string;
}

/** Result of a backup operation. */
export interface BackupResult {
  path: string;
  sizeBytes: number;
}

/** Arguments for exporting store data to an .ozpkg file. */
export interface ExportDataArgs {
  types: string[];
  password: string;
  outputPath: string;
  dateFrom?: string;
  dateTo?: string;
}

/** Result of a data export operation. */
export interface ExportDataResult {
  path: string;
  sizeBytes: number;
  types: string[];
}

/** Preview of an .ozpkg import file before actually importing. */
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

/** Result of an import operation with per-type counts. */
export interface ImportDataResult {
  productsImported: number;
  categoriesImported: number;
  salesImported: number;
  customersImported: number;
  usersImported: number;
  settingsImported: number;
}

// ── File dialog helpers ───────────────────────────────────────

/** Open a save dialog to choose an export file path. Returns the chosen path or null. */
export const pickExportPath = async (): Promise<string | null> => {
  const path = await save({
    defaultPath: `ozpos_export_${new Date().toISOString().slice(0, 10)}.ozpkg`,
    filters: [{ name: 'OZ-POS Export', extensions: ['ozpkg'] }],
  });
  return path;
};

/** Open a file picker dialog to select an .ozpkg import file. Returns the chosen path or null. */
export const pickImportFile = async (): Promise<string | null> => {
  const path = await open({
    filters: [{ name: 'OZ-POS Export', extensions: ['ozpkg'] }],
    multiple: false,
  });
  return path;
};

// ── IPC calls ─────────────────────────────────────────────────

/** Get the current backup status. */
export const getBackupStatus = (): Promise<BackupStatus> =>
  loggedInvoke<BackupStatus>('get_backup_status');

/** Create a new database backup. */
export const createBackup = (): Promise<BackupResult> =>
  loggedInvoke<BackupResult>('create_backup');

/** Export store data to an encrypted .ozpkg file. */
export const exportData = (args: ExportDataArgs): Promise<ExportDataResult> =>
  loggedInvoke<ExportDataResult>('export_data', { args });

/** Preview an .ozpkg import file to see its contents before importing. */
export const importPreview = (filePath: string, password: string): Promise<ImportPreviewResult> =>
  loggedInvoke<ImportPreviewResult>('import_preview', { args: { file_path: filePath, password } });

/** Import data from an encrypted .ozpkg file. */
export const importData = (filePath: string, password: string): Promise<ImportDataResult> =>
  loggedInvoke<ImportDataResult>('import_data', { args: { file_path: filePath, password } });
