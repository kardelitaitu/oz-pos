//! Data Management screen — Settings → Data Management
//!
//! Three sections:
//! - **Export wizard**: pick data types to export, date range, password field, progress indicator
//! - **Import wizard**: pick a .ozpkg file, preview metadata, dry-run diff table, confirm
//! - **Backup status**: last backup timestamp, one-click snapshot

import { useState, useCallback, useEffect } from 'react';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './DataManagementScreen.css';

// ── Types ──────────────────────────────────────────────────────────

type DataType =
  | 'products'
  | 'categories'
  | 'sales'
  | 'customers'
  | 'users'
  | 'settings';

const DATA_TYPES: { key: DataType; label: string; description: string }[] = [
  { key: 'products', label: 'Products', description: 'SKU, name, price, barcode, stock' },
  { key: 'categories', label: 'Categories', description: 'Category id, name, colour' },
  { key: 'sales', label: 'Sales', description: 'Sale header, line items, payments' },
  { key: 'customers', label: 'Customers', description: 'Name, email, phone, loyalty points' },
  { key: 'users', label: 'Users', description: 'Usernames, display names, roles (no passwords)' },
  { key: 'settings', label: 'Settings', description: 'Store config, receipts, feature flags' },
];

interface ExportState {
  selectedTypes: Set<DataType>;
  dateFrom: string;
  dateTo: string;
  password: string;
  passwordConfirm: string;
  step: 'select' | 'encrypt' | 'exporting' | 'done';
  progress: number;
  outputFile: string | null;
  error: string | null;
}

interface ImportState {
  selectedFile: string | null;
  metadata: { name: string; version: string; types: string[]; created: string } | null;
  password: string;
  step: 'select' | 'preview' | 'importing' | 'done';
  progress: number;
  error: string | null;
  dryRun: { added: number; updated: number; skipped: number } | null;
}

interface BackupInfo {
  lastBackup: string | null;
  lastBackupSize: string | null;
  backingUp: boolean;
}

// ── Initial state ──────────────────────────────────────────────────

const INITIAL_EXPORT: ExportState = {
  selectedTypes: new Set(DATA_TYPES.map((t) => t.key)),
  dateFrom: '',
  dateTo: '',
  password: '',
  passwordConfirm: '',
  step: 'select',
  progress: 0,
  outputFile: null,
  error: null,
};

const INITIAL_IMPORT: ImportState = {
  selectedFile: null,
  metadata: null,
  password: '',
  step: 'select',
  progress: 0,
  error: null,
  dryRun: null,
};

// ── Component ──────────────────────────────────────────────────────

export default function DataManagementScreen() {
  const [exportState, setExportState] = useState<ExportState>(INITIAL_EXPORT);
  const [importState, setImportState] = useState<ImportState>(INITIAL_IMPORT);
  const [backup, setBackup] = useState<BackupInfo>({
    lastBackup: null,
    lastBackupSize: null,
    backingUp: false,
  });
  const [activeTab, setActiveTab] = useState<'export' | 'import' | 'backup'>('export');
  const [toast, setToast] = useState<{ message: string; variant: 'success' | 'error' } | null>(null);

  // ── Load backup status on mount ─────────────────────────────────

  useEffect(() => {
    // TODO: Call IPC to get last backup status
    setBackup((prev) => ({
      ...prev,
      lastBackup: null,
    }));
  }, []);

  // ── Backup handlers ─────────────────────────────────────────────

  const handleBackup = useCallback(async () => {
    setBackup((prev) => ({ ...prev, backingUp: true }));
    try {
      // TODO: Call IPC to create backup
      await new Promise((resolve) => setTimeout(resolve, 1000));
      setBackup({
        lastBackup: new Date().toLocaleString(),
        lastBackupSize: '1.2 MB',
        backingUp: false,
      });
      setToast({ message: 'Backup created successfully', variant: 'success' });
    } catch {
      setBackup((prev) => ({ ...prev, backingUp: false }));
      setToast({ message: 'Backup failed', variant: 'error' });
    }
  }, []);

  // ── Toggle data type selection ──────────────────────────────────

  const toggleType = useCallback((type: DataType) => {
    setExportState((prev) => {
      const next = new Set(prev.selectedTypes);
      if (next.has(type)) next.delete(type);
      else next.add(type);
      return { ...prev, selectedTypes: next };
    });
  }, []);

  const toggleAll = useCallback(() => {
    setExportState((prev) => {
      const allSelected = prev.selectedTypes.size === DATA_TYPES.length;
      if (allSelected) {
        return { ...prev, selectedTypes: new Set() };
      }
      return { ...prev, selectedTypes: new Set(DATA_TYPES.map((t) => t.key)) };
    });
  }, []);

  // ── Export flow ─────────────────────────────────────────────────

  const startExport = useCallback(() => {
    if (exportState.selectedTypes.size === 0) {
      setToast({ message: 'Select at least one data type to export', variant: 'error' });
      return;
    }
    setExportState((prev) => ({ ...prev, step: 'encrypt', error: null }));
  }, [exportState.selectedTypes]);

  const confirmExport = useCallback(async () => {
    if (exportState.password.length < 8) {
      setToast({ message: 'Password must be at least 8 characters', variant: 'error' });
      return;
    }
    if (exportState.password !== exportState.passwordConfirm) {
      setToast({ message: 'Passwords do not match', variant: 'error' });
      return;
    }

    setExportState((prev) => ({ ...prev, step: 'exporting', progress: 0, error: null }));

    // Simulate export progress
    for (let i = 0; i <= 100; i += 10) {
      await new Promise((r) => setTimeout(r, 200));
      setExportState((prev) => ({ ...prev, progress: i }));
    }

    setExportState((prev) => ({
      ...prev,
      step: 'done',
      progress: 100,
      outputFile: `${new Date().toISOString().slice(0, 10)}_ozpos_export.ozpkg`,
    }));
    setToast({ message: 'Export complete', variant: 'success' });
  }, [exportState.password, exportState.passwordConfirm]);

  const resetExport = useCallback(() => {
    setExportState(INITIAL_EXPORT);
  }, []);

  // ── Import flow ─────────────────────────────────────────────────

  const handleFileSelect = useCallback(async () => {
    // TODO: Use Tauri dialog to pick a .ozpkg file
    // For now, simulate selecting a file
    setImportState((prev) => ({
      ...prev,
      selectedFile: 'ozpos_export.ozpkg',
      metadata: {
        name: 'My Store Backup',
        version: '0.0.1',
        types: ['products', 'categories', 'settings'],
        created: '2026-06-28T10:30:00Z',
      },
      step: 'preview',
    }));
  }, []);

  const startImport = useCallback(async () => {
    if (!importState.password) {
      setToast({ message: 'Enter the export password', variant: 'error' });
      return;
    }

    setImportState((prev) => ({ ...prev, step: 'importing', progress: 0, error: null }));

    // Simulate dry-run
    await new Promise((r) => setTimeout(r, 500));
    setImportState((prev) => ({
      ...prev,
      progress: 30,
      dryRun: { added: 15, updated: 3, skipped: 0 },
    }));

    // Simulate import
    for (let i = 30; i <= 100; i += 10) {
      await new Promise((r) => setTimeout(r, 200));
      setImportState((prev) => ({ ...prev, progress: i }));
    }

    setImportState((prev) => ({ ...prev, step: 'done' }));
    setToast({ message: 'Import complete', variant: 'success' });
  }, [importState.password]);

  const resetImport = useCallback(() => {
    setImportState(INITIAL_IMPORT);
  }, []);

  // ── Toast auto-dismiss ──────────────────────────────────────────

  useEffect(() => {
    if (toast) {
      const timer = setTimeout(() => setToast(null), 4000);
      return () => clearTimeout(timer);
    }
  }, [toast]);

  // ── Render ──────────────────────────────────────────────────────

  return (
    <div className="data-mgmt">
      <div className="data-mgmt-header">
        <h1 className="data-mgmt-title">Data Management</h1>
      </div>

      {/* ── Tab bar ────────────────────────────────── */}
      <div className="data-mgmt-tabs" role="tablist" aria-label="Data management actions">
        {(['export', 'import', 'backup'] as const).map((tab) => (
          <button
            key={tab}
            type="button"
            role="tab"
            aria-selected={activeTab === tab}
            className={`data-mgmt-tab ${activeTab === tab ? 'data-mgmt-tab--active' : ''}`}
            onClick={() => setActiveTab(tab)}
          >
            {tab === 'export' && '📤'} {tab === 'import' && '📥'} {tab === 'backup' && '💾'}
            {' '}
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      {/* ── Export tab ─────────────────────────────── */}
      {activeTab === 'export' && (
        <div role="tabpanel" aria-label="Export wizard">
          {exportState.step === 'select' && (
            <Card shadow="sm">
              <div className="data-mgmt-section">
                <h2 className="data-mgmt-section-title">Select data to export</h2>

                <div className="data-mgmt-types" role="group" aria-label="Data types to export">
                  <div className="data-mgmt-type-checkbox data-mgmt-type-checkbox--all">
                    <input
                      id="type-select-all"
                      type="checkbox"
                      checked={exportState.selectedTypes.size === DATA_TYPES.length}
                      onChange={toggleAll}
                    />
                    <label className="data-mgmt-type-label" htmlFor="type-select-all">Select all / none</label>
                  </div>

                  {DATA_TYPES.map((dt) => (
                    <div key={dt.key} className="data-mgmt-type-checkbox">
                      <input
                        id={`type-${dt.key}`}
                        type="checkbox"
                        checked={exportState.selectedTypes.has(dt.key)}
                        onChange={() => toggleType(dt.key)}
                      />
                      <label className="data-mgmt-type-info" htmlFor={`type-${dt.key}`}>
                        <span className="data-mgmt-type-label">{dt.label}</span>
                        <span className="data-mgmt-type-desc">{dt.description}</span>
                      </label>
                    </div>
                  ))}
                </div>

                <div className="data-mgmt-date-range">
                  <div className="data-mgmt-field">
                    <label className="data-mgmt-label" htmlFor="export-date-from">From</label>
                    <input
                      id="export-date-from"
                      className="data-mgmt-input"
                      type="date"
                      value={exportState.dateFrom}
                      onChange={(e) => setExportState((prev) => ({ ...prev, dateFrom: e.target.value }))}
                    />
                  </div>
                  <div className="data-mgmt-field">
                    <label className="data-mgmt-label" htmlFor="export-date-to">To</label>
                    <input
                      id="export-date-to"
                      className="data-mgmt-input"
                      type="date"
                      value={exportState.dateTo}
                      onChange={(e) => setExportState((prev) => ({ ...prev, dateTo: e.target.value }))}
                    />
                  </div>
                </div>

                <div className="data-mgmt-actions">
                  <Button variant="primary" onClick={startExport}>
                    Next: Encryption
                  </Button>
                </div>
              </div>
            </Card>
          )}

          {exportState.step === 'encrypt' && (
            <Card shadow="sm">
              <div className="data-mgmt-section">
                <h2 className="data-mgmt-section-title">Set encryption password</h2>
                <p className="data-mgmt-section-desc">
                  The export file will be encrypted with AES-256-GCM. Choose a strong
                  password — you will need it to import the data later.
                </p>

                <div className="data-mgmt-form">
                  <div className="data-mgmt-field">
                    <label className="data-mgmt-label" htmlFor="export-password">Password</label>
                    <input
                      id="export-password"
                      className="data-mgmt-input"
                      type="password"
                      placeholder="At least 8 characters"
                      value={exportState.password}
                      onChange={(e) => setExportState((prev) => ({ ...prev, password: e.target.value }))}
                    />
                  </div>
                  <div className="data-mgmt-field">
                    <label className="data-mgmt-label" htmlFor="export-password-confirm">Confirm password</label>
                    <input
                      id="export-password-confirm"
                      className="data-mgmt-input"
                      type="password"
                      placeholder="Re-enter password"
                      value={exportState.passwordConfirm}
                      onChange={(e) => setExportState((prev) => ({ ...prev, passwordConfirm: e.target.value }))}
                    />
                  </div>
                </div>

                <div className="data-mgmt-actions">
                  <Button variant="ghost" onClick={() => setExportState((prev) => ({ ...prev, step: 'select' }))}>
                    Back
                  </Button>
                  <Button variant="primary" onClick={confirmExport}>
                    Export
                  </Button>
                </div>

                {exportState.error && (
                  <div className="data-mgmt-error" role="alert">{exportState.error}</div>
                )}
              </div>
            </Card>
          )}

          {(exportState.step === 'exporting' || exportState.step === 'done') && (
            <Card shadow="sm">
              <div className="data-mgmt-section">
                <h2 className="data-mgmt-section-title">
                  {exportState.step === 'exporting' ? 'Exporting…' : 'Export complete'}
                </h2>

                <div className="data-mgmt-progress-bar">
                  <div
                    className="data-mgmt-progress-fill"
                    style={{ width: `${exportState.progress}%` }}
                  />
                </div>
                <span className="data-mgmt-progress-text">{exportState.progress}%</span>

                {exportState.step === 'done' && (
                  <>
                    <p className="data-mgmt-done-text">
                      Data exported to: <code>{exportState.outputFile}</code>
                    </p>
                    <p className="data-mgmt-done-text">
                      Selected types:{' '}
                      {Array.from(exportState.selectedTypes).join(', ')}
                    </p>
                    <div className="data-mgmt-actions">
                      <Button variant="primary" onClick={resetExport}>
                        New export
                      </Button>
                    </div>
                  </>
                )}
              </div>
            </Card>
          )}
        </div>
      )}

      {/* ── Import tab ─────────────────────────────── */}
      {activeTab === 'import' && (
        <div role="tabpanel" aria-label="Import wizard">
          {importState.step === 'select' && (
            <Card shadow="sm">
              <div className="data-mgmt-section">
                <h2 className="data-mgmt-section-title">Select a backup file</h2>
                <p className="data-mgmt-section-desc">
                  Choose an encrypted .ozpkg file to import. The file must have been
                  created by OZ-POS export.
                </p>

                <div className="data-mgmt-file-picker">
                  <div className="data-mgmt-file-dropzone">
                    <span className="data-mgmt-file-icon">📂</span>
                    <p>Drag & drop a .ozpkg file here, or</p>
                    <Button variant="secondary" onClick={handleFileSelect}>
                      Browse files…
                    </Button>
                  </div>
                </div>
              </div>
            </Card>
          )}

          {importState.step === 'preview' && importState.metadata && (
            <Card shadow="sm">
              <div className="data-mgmt-section">
                <h2 className="data-mgmt-section-title">Preview import</h2>

                <div className="data-mgmt-meta">
                  <div className="data-mgmt-meta-row">
                    <span className="data-mgmt-meta-label">File</span>
                    <span>{importState.selectedFile}</span>
                  </div>
                  <div className="data-mgmt-meta-row">
                    <span className="data-mgmt-meta-label">Store</span>
                    <span>{importState.metadata.name}</span>
                  </div>
                  <div className="data-mgmt-meta-row">
                    <span className="data-mgmt-meta-label">Version</span>
                    <span>{importState.metadata.version}</span>
                  </div>
                  <div className="data-mgmt-meta-row">
                    <span className="data-mgmt-meta-label">Created</span>
                    <span>{new Date(importState.metadata.created).toLocaleString()}</span>
                  </div>
                  <div className="data-mgmt-meta-row">
                    <span className="data-mgmt-meta-label">Contains</span>
                    <span>{importState.metadata.types.join(', ')}</span>
                  </div>
                </div>

                <div className="data-mgmt-field">
                  <label className="data-mgmt-label" htmlFor="import-password">Decryption password</label>
                  <input
                    id="import-password"
                    className="data-mgmt-input"
                    type="password"
                    placeholder="Enter the export password"
                    value={importState.password}
                    onChange={(e) => setImportState((prev) => ({ ...prev, password: e.target.value }))}
                  />
                </div>

                <div className="data-mgmt-actions">
                  <Button variant="ghost" onClick={resetImport}>
                    Cancel
                  </Button>
                  <Button variant="primary" onClick={startImport}>
                    Start import
                  </Button>
                </div>
              </div>
            </Card>
          )}

          {importState.step === 'importing' && (
            <Card shadow="sm">
              <div className="data-mgmt-section">
                <h2 className="data-mgmt-section-title">
                  {importState.dryRun ? 'Dry-run complete — importing…' : 'Analysing file…'}
                </h2>

                <div className="data-mgmt-progress-bar">
                  <div
                    className="data-mgmt-progress-fill"
                    style={{ width: `${importState.progress}%` }}
                  />
                </div>
                <span className="data-mgmt-progress-text">{importState.progress}%</span>

                {importState.dryRun && (
                  <div className="data-mgmt-dry-run">
                    <h3 className="data-mgmt-dry-run-title">Changes to be applied</h3>
                    <div className="data-mgmt-dry-run-grid">
                      <div className="data-mgmt-dry-run-item">
                        <span className="data-mgmt-dry-run-count">{importState.dryRun.added}</span>
                        <span>New items</span>
                      </div>
                      <div className="data-mgmt-dry-run-item">
                        <span className="data-mgmt-dry-run-count">{importState.dryRun.updated}</span>
                        <span>Updated</span>
                      </div>
                      <div className="data-mgmt-dry-run-item">
                        <span className="data-mgmt-dry-run-count">{importState.dryRun.skipped}</span>
                        <span>Skipped</span>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </Card>
          )}

          {importState.step === 'done' && (
            <Card shadow="sm">
              <div className="data-mgmt-section">
                <h2 className="data-mgmt-section-title">Import complete</h2>
                <p className="data-mgmt-done-text">
                  All data has been imported successfully.
                </p>
                {importState.dryRun && (
                  <p className="data-mgmt-done-text">
                    {importState.dryRun.added} items added, {importState.dryRun.updated} updated,
                    {importState.dryRun.skipped} skipped.
                  </p>
                )}
                <div className="data-mgmt-actions">
                  <Button variant="primary" onClick={resetImport}>
                    New import
                  </Button>
                </div>
              </div>
            </Card>
          )}
        </div>
      )}

      {/* ── Backup tab ─────────────────────────────── */}
      {activeTab === 'backup' && (
        <div role="tabpanel" aria-label="Backup status">
          <Card shadow="sm">
            <div className="data-mgmt-section">
              <h2 className="data-mgmt-section-title">Database backup</h2>
              <p className="data-mgmt-section-desc">
                Create an online snapshot of the current database. The backup runs
                in the background and does not interrupt POS operations.
              </p>

              <div className="data-mgmt-backup-status">
                <div className="data-mgmt-backup-row">
                  <span className="data-mgmt-label">Last backup</span>
                  <span className="data-mgmt-value">
                    {backup.lastBackup ?? 'Never'}
                  </span>
                </div>
                {backup.lastBackupSize && (
                  <div className="data-mgmt-backup-row">
                    <span className="data-mgmt-label">Size</span>
                    <span className="data-mgmt-value">{backup.lastBackupSize}</span>
                  </div>
                )}
              </div>

              <div className="data-mgmt-actions">
                <Button
                  variant="primary"
                  loading={backup.backingUp}
                  onClick={handleBackup}
                >
                  {backup.backingUp ? 'Backing up…' : 'Create backup now'}
                </Button>
              </div>
            </div>
          </Card>
        </div>
      )}

      {/* ── Toast ──────────────────────────────────── */}
      {toast && (
        <button
          type="button"
          className={`data-mgmt-toast data-mgmt-toast--${toast.variant}`}
          onClick={() => setToast(null)}
          aria-label="Dismiss notification"
        >
          {toast.message}
        </button>
      )}
    </div>
  );
}
