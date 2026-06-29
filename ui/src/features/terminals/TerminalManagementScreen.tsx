import { useState, useCallback, useEffect } from 'react';
import { Localized } from '@fluent/react';
import {
  listTerminals,
  registerTerminal,
  updateTerminal,
  deleteTerminal,
  type TerminalDto,
} from '@/api/terminals';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './TerminalManagementScreen.css';

// ── Form state ──────────────────────────────────────────────────────

interface FormData {
  name: string;
  deviceId: string;
  terminalSecret: string;
  metadata: string;
  isActive: boolean;
}

const EMPTY_FORM: FormData = {
  name: '',
  deviceId: '',
  terminalSecret: '',
  metadata: '',
  isActive: true,
};

// ── Helpers ─────────────────────────────────────────────────────────

function formatDate(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

// ── Component ───────────────────────────────────────────────────────

export default function TerminalManagementScreen() {
  const [terminals, setTerminals] = useState<TerminalDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Add / Edit modal
  const [showModal, setShowModal] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<FormData>(EMPTY_FORM);

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<TerminalDto | null>(null);
  const [deleting, setDeleting] = useState(false);

  const [saving, setSaving] = useState(false);

  // ── Load data ──────────────────────────────────────────────────

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await listTerminals();
      setTerminals(data);
    } catch {
      setError('Failed to load terminals');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  // ── Modal handlers ──────────────────────────────────────────────

  const openCreate = useCallback(() => {
    setForm(EMPTY_FORM);
    setEditingId(null);
    setError(null);
    setShowModal(true);
  }, []);

  const openEdit = useCallback((terminal: TerminalDto) => {
    setForm({
      name: terminal.name,
      deviceId: terminal.deviceId,
      terminalSecret: '',
      metadata: terminal.metadata ?? '',
      isActive: terminal.isActive,
    });
    setEditingId(terminal.id);
    setError(null);
    setShowModal(true);
  }, []);

  const closeModal = useCallback(() => {
    setShowModal(false);
    setError(null);
  }, []);

  const openDelete = useCallback((terminal: TerminalDto) => {
    setDeleteTarget(terminal);
  }, []);

  const closeDelete = useCallback(() => {
    setDeleteTarget(null);
  }, []);

  // ── Save / Update ──────────────────────────────────────────────

  const handleSave = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      const name = form.name.trim();
      const deviceId = form.deviceId.trim();

      if (!name) {
        setError('Name is required');
        setSaving(false);
        return;
      }
      if (!deviceId) {
        setError('Device ID is required');
        setSaving(false);
        return;
      }

      if (editingId) {
        await updateTerminal({
          id: editingId,
          name,
          deviceId,
          isActive: form.isActive,
          metadata: form.metadata || null,
        });
      } else {
        await registerTerminal({
          name,
          deviceId,
          terminalSecret: form.terminalSecret || null,
          metadata: form.metadata || null,
        });
      }

      closeModal();
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save terminal');
    } finally {
      setSaving(false);
    }
  }, [form, editingId, closeModal, load]);

  // ── Delete ─────────────────────────────────────────────────────

  const handleDelete = useCallback(async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      await deleteTerminal(deleteTarget.id);
      closeDelete();
      await load();
    } catch {
      // Error handling.
    } finally {
      setDeleting(false);
    }
  }, [deleteTarget, closeDelete, load]);

  // ── Render ─────────────────────────────────────────────────────

  const isEditing = editingId !== null;

  return (
    <div className="terminal-mgmt">
      <div className="terminal-mgmt-header">
        <Localized id="terminal-management-title">
          <h1 className="terminal-mgmt-title">Terminal Management</h1>
        </Localized>
        <Localized id="terminal-register">
          <Button onClick={openCreate}>Register Terminal</Button>
        </Localized>
      </div>

      {loading ? (
        <Localized id="terminal-management-loading">
          <p className="terminal-mgmt-loading">Loading terminals…</p>
        </Localized>
      ) : error ? (
        <Card shadow="sm">
          <div className="terminal-mgmt-empty">
            <Localized id="terminal-management-error">
              <p>Failed to load terminals. Please try again.</p>
            </Localized>
            <Localized id="terminal-management-retry">
              <Button variant="secondary" onClick={load}>Retry</Button>
            </Localized>
          </div>
        </Card>
      ) : terminals.length === 0 ? (
        <Card shadow="sm">
          <div className="terminal-mgmt-empty">
            <div className="terminal-mgmt-empty-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" width="48" height="48">
                <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
                <line x1="8" y1="21" x2="16" y2="21" />
                <line x1="12" y1="17" x2="12" y2="21" />
                <path d="M7 7l3 3-3 3" />
              </svg>
            </div>
            <Localized id="terminal-management-empty">
              <p>No terminals registered yet. Register the first terminal to get started.</p>
            </Localized>
            <Localized id="terminal-register">
              <Button variant="secondary" onClick={openCreate}>
                Register Terminal
              </Button>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="terminal-mgmt-table-wrap">
          <table className="terminal-mgmt-table" aria-label="Terminals">
            <thead>
              <tr>
                <Localized id="terminal-name"><th>Name</th></Localized>
                <Localized id="terminal-device-id"><th>Device ID</th></Localized>
                <Localized id="terminal-status"><th>Status</th></Localized>
                <Localized id="terminal-last-seen"><th>Last Seen</th></Localized>
                <Localized id="terminal-created"><th>Created</th></Localized>
                <th aria-label="Actions"> </th>
              </tr>
            </thead>
            <tbody>
              {terminals.map((terminal) => (
                <tr key={terminal.id}>
                  <td className="terminal-mgmt-cell-name">{terminal.name}</td>
                  <td className="terminal-mgmt-cell-device-id">{terminal.deviceId}</td>
                  <td>
                    {terminal.isActive ? (
                      <Localized id="terminal-is-active">
                        <span className="terminal-mgmt-status-active">Active</span>
                      </Localized>
                    ) : (
                      <Localized id="terminal-status">
                        <span className="terminal-mgmt-status-inactive">Inactive</span>
                      </Localized>
                    )}
                  </td>
                  <td className="terminal-mgmt-cell-last-seen">
                    {terminal.lastSeenAt ? formatDate(terminal.lastSeenAt) : (
                      <Localized id="terminal-never">
                        <span>Never</span>
                      </Localized>
                    )}
                  </td>
                  <td className="terminal-mgmt-cell-created">{formatDate(terminal.createdAt)}</td>
                  <td className="terminal-mgmt-cell-actions">
                    <Localized id="terminal-edit-action" attrs={{ "aria-label": true }} vars={{ name: terminal.name }}>
                      <button
                        type="button"
                        className="terminal-mgmt-action-btn"
                        onClick={() => openEdit(terminal)}
                        aria-label={`Edit ${terminal.name}`}
                      >
                        <Localized id="terminal-edit-action"><span>Edit</span></Localized>
                      </button>
                    </Localized>
                    <Localized id="terminal-delete-action" attrs={{ "aria-label": true }} vars={{ name: terminal.name }}>
                      <button
                        type="button"
                        className="terminal-mgmt-action-btn terminal-mgmt-action-btn--danger"
                        onClick={() => openDelete(terminal)}
                        aria-label={`Delete ${terminal.name}`}
                      >
                        <Localized id="terminal-delete-action"><span>Delete</span></Localized>
                      </button>
                    </Localized>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* ── Add/Edit Modal ──────────────────────────────────────── */}
      {showModal && (
        <Localized id={isEditing ? 'terminal-edit-title' : 'terminal-register-title'} attrs={{ "aria-label": true }}>
          <div className="terminal-mgmt-overlay" role="dialog" aria-modal="true" aria-label={isEditing ? 'Edit Terminal' : 'Register New Terminal'}>
            <div className="terminal-mgmt-modal">
              <div className="terminal-mgmt-modal-header">
                <Localized id={isEditing ? 'terminal-edit-title' : 'terminal-register-title'}>
                  <h2>{isEditing ? 'Edit Terminal' : 'Register New Terminal'}</h2>
                </Localized>
                <button
                  type="button"
                  className="terminal-mgmt-modal-close"
                  onClick={closeModal}
                  aria-label="Close"
                >
                  &times;
                </button>
              </div>

              <div className="terminal-mgmt-modal-body">
                {/* Name */}
                <label className="terminal-mgmt-field" htmlFor="terminal-field-name" aria-label="Terminal name">
                  <Localized id="terminal-name-label">
                    <span className="terminal-mgmt-label">Terminal name</span>
                  </Localized>
                  <Localized id="terminal-name-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="terminal-mgmt-input"
                      type="text"
                      id="terminal-field-name"
                      value={form.name}
                      onChange={(e) => setForm({ ...form, name: e.target.value })}
                      placeholder="e.g. Front Counter"
                      autoComplete="off"
                    />
                  </Localized>
                </label>

                {/* Device ID */}
                <label className="terminal-mgmt-field" htmlFor="terminal-field-device-id" aria-label="Device identifier">
                  <Localized id="terminal-device-id-label">
                    <span className="terminal-mgmt-label">Device identifier</span>
                  </Localized>
                  <Localized id="terminal-device-id-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="terminal-mgmt-input"
                      type="text"
                      id="terminal-field-device-id"
                      value={form.deviceId}
                      onChange={(e) => setForm({ ...form, deviceId: e.target.value })}
                      placeholder="e.g. hostname or MAC address"
                      autoComplete="off"
                    />
                  </Localized>
                </label>

                {/* Secret — only for register */}
                {!isEditing && (
                  <label className="terminal-mgmt-field" htmlFor="terminal-field-secret" aria-label="Shared secret">
                    <Localized id="terminal-secret-label">
                      <span className="terminal-mgmt-label">Optional shared secret for sync authentication</span>
                    </Localized>
                    <input
                      className="terminal-mgmt-input"
                      type="password"
                      id="terminal-field-secret"
                      value={form.terminalSecret}
                      onChange={(e) => setForm({ ...form, terminalSecret: e.target.value })}
                      autoComplete="new-password"
                    />
                  </label>
                )}

                {/* Metadata */}
                <label className="terminal-mgmt-field" htmlFor="terminal-field-metadata" aria-label="JSON metadata">
                  <Localized id="terminal-metadata-label">
                    <span className="terminal-mgmt-label">Optional JSON metadata</span>
                  </Localized>
                  <textarea
                    className="terminal-mgmt-input terminal-mgmt-textarea"
                    id="terminal-field-metadata"
                    value={form.metadata}
                    onChange={(e) => setForm({ ...form, metadata: e.target.value })}
                    rows={3}
                  />
                </label>

                {/* Active toggle — only for edit */}
                {isEditing && (
                  <div className="terminal-mgmt-checkbox-wrap">
                    <input
                      className="terminal-mgmt-checkbox"
                      type="checkbox"
                      id="terminal-field-active"
                      checked={form.isActive}
                      onChange={(e) => setForm({ ...form, isActive: e.target.checked })}
                    />
                    <Localized id="terminal-is-active">
                      <label className="terminal-mgmt-checkbox-label" htmlFor="terminal-field-active">
                        Active
                      </label>
                    </Localized>
                  </div>
                )}

                {/* Error */}
                {error && (
                  <div className="terminal-mgmt-error" role="alert">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16" aria-hidden="true">
                      <circle cx="12" cy="12" r="10" />
                      <line x1="15" y1="9" x2="9" y2="15" />
                      <line x1="9" y1="9" x2="15" y2="15" />
                    </svg>
                    <span>{error}</span>
                  </div>
                )}
              </div>

              <div className="terminal-mgmt-modal-actions">
                <Localized id="terminal-cancel">
                  <Button variant="ghost" onClick={closeModal} disabled={saving}>
                    Cancel
                  </Button>
                </Localized>
                <Button
                  variant="primary"
                  loading={saving}
                  disabled={!form.name.trim() || !form.deviceId.trim()}
                  onClick={handleSave}
                >
                  <Localized id={isEditing ? 'terminal-save' : 'terminal-register-action'}>
                    <span>{isEditing ? 'Save' : 'Register'}</span>
                  </Localized>
                </Button>
              </div>
            </div>
          </div>
        </Localized>
      )}

      {/* ── Delete Confirmation Modal ────────────────────────────── */}
      {deleteTarget && (
        <div className="terminal-mgmt-overlay" role="dialog" aria-modal="true" aria-label="Delete terminal">
          <div className="terminal-mgmt-modal">
            <div className="terminal-mgmt-modal-header">
              <Localized id="terminal-delete-title">
                <h2>Delete Terminal</h2>
              </Localized>
              <button
                type="button"
                className="terminal-mgmt-modal-close"
                onClick={closeDelete}
                aria-label="Close"
              >
                &times;
              </button>
            </div>

            <div className="terminal-mgmt-modal-body">
              <Localized id="terminal-delete-confirm" vars={{ name: deleteTarget.name }}>
                <p>Are you sure you want to delete terminal &quot;{deleteTarget.name}&quot;? This action cannot be undone.</p>
              </Localized>
            </div>

            <div className="terminal-mgmt-modal-actions">
              <Localized id="terminal-cancel">
                <Button variant="ghost" onClick={closeDelete} disabled={deleting}>
                  Cancel
                </Button>
              </Localized>
              <Button
                variant="danger"
                loading={deleting}
                onClick={handleDelete}
              >
                <Localized id="terminal-delete">
                  <span>Delete</span>
                </Localized>
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
