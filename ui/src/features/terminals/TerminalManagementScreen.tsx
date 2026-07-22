import { useState, useCallback, useEffect } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { SettingsPopup, useToast } from '@/frontend/shared';
import {
  listTerminals,
  registerTerminal,
  updateTerminal,
  deleteTerminal,
  listTerminalOverrides,
  setTerminalOverride,
  deleteTerminalOverride,
  getDeviceBinding,
  setDeviceBinding,
  clearDeviceBinding,
  type TerminalDto,
  type TerminalFeatureOverride,
  type DeviceBindingDto,
} from '@/api/terminals';
import { listStores, type StoreProfile } from '@/api/stores';
import { listWorkspaces, type WorkspaceDto } from '@/api/workspaces';
import { FEATURES } from '@/hooks/useFeatures';
import { useAuth } from '@/contexts/AuthContext';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Skeleton } from '@/components/Skeleton';
import './TerminalManagementScreen.css';

// ── Feature groups for the override toggle UI ─────────────────────

const FEATURE_GROUPS: { label: string; keys: string[] }[] = [
  {
    label: 'Sales',
    keys: [
      FEATURES.SIMPLE_RETAIL,
      FEATURES.RESTAURANT,
      FEATURES.DISCOUNT_ENGINE,
      FEATURES.TAX_ENGINE,
      FEATURES.PROMOTIONS_ENGINE,
      FEATURES.PRODUCT_BUNDLES,
      FEATURES.LOYALTY_PROGRAM,
      FEATURES.KITCHEN_DISPLAY,
      FEATURES.TABLE_MANAGEMENT,
    ],
  },
  {
    label: 'Payments',
    keys: [
      FEATURES.CASH_PAYMENT,
      FEATURES.CARD_PAYMENT,
      FEATURES.MULTI_CURRENCY,
    ],
  },
  {
    label: 'Inventory & Products',
    keys: [
      FEATURES.INVENTORY_TRACKING,
      FEATURES.PRODUCT_VARIANTS,
      FEATURES.CATEGORIES_ENABLED,
      FEATURES.BARCODE_SCANNING,
    ],
  },
  {
    label: 'Hardware',
    keys: [
      FEATURES.RECEIPT_PRINTING,
      FEATURES.CASH_DRAWER,
      FEATURES.CUSTOMER_DISPLAY,
      FEATURES.NFC_READER,
    ],
  },
  {
    label: 'Staff & Security',
    keys: [
      FEATURES.STAFF_LOGIN,
      FEATURES.STAFF_ROLES,
      FEATURES.SHIFT_MANAGEMENT,
      FEATURES.AUDIT_LOG,
    ],
  },
  {
    label: 'System',
    keys: [
      FEATURES.CLOUD_SYNC,
      FEATURES.MULTI_STORE,
      FEATURES.MULTI_TERMINAL,
      FEATURES.REPORTING,
      FEATURES.ANALYTICS,
      FEATURES.EXPORT_IMPORT,
      FEATURES.PLUGIN_SYSTEM,
      FEATURES.SELF_SERVICE_KIOSK,
    ],
  },
];

/** Convert a kebab-case feature key to a human-readable label. */
function featureLabel(key: string): string {
  return key
    .split('-')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

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

/** Terminal management screen — register, configure, and manage POS terminals, feature overrides, and device bindings for multi-store deployments. */
export default function TerminalManagementScreen() {
  const { l10n } = useLocalization();
  const { session } = useAuth();
  const { addToast } = useToast();
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

  // Feature overrides
  const [overrides, setOverrides] = useState<TerminalFeatureOverride[]>([]);
  const [overridesLoading, setOverridesLoading] = useState(false);
  const [overridesError, setOverridesError] = useState<string | null>(null);

  // Device binding (ADR #4 Phase 3)
  const [binding, setBinding] = useState<DeviceBindingDto | null>(null);
  const [bindingLoading, setBindingLoading] = useState(false);
  const [bindingStores, setBindingStores] = useState<StoreProfile[]>([]);
  const [bindingInstances, setBindingInstances] = useState<WorkspaceDto[]>([]);
  const [selectedStoreId, setSelectedStoreId] = useState('');
  const [selectedInstanceId, setSelectedInstanceId] = useState('');
  const [bindingSaving, setBindingSaving] = useState(false);
  const [bindingError, setBindingError] = useState<string | null>(null);

  // ── Load data ──────────────────────────────────────────────────

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await listTerminals();
      setTerminals(data);
    } catch {
      setError(l10n.getString('terminal-error-load'));
    } finally {
      setLoading(false);
    }
  }, [l10n]);

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
    setOverrides([]);
    setOverridesError(null);
    setShowModal(true);
  }, []);

  // Load overrides when the edit modal opens for a terminal.
  useEffect(() => {
    if (!editingId) return;
    let cancelled = false;
    (async () => {
      setOverridesLoading(true);
      setOverridesError(null);
      try {
        const data = await listTerminalOverrides(editingId);
        if (!cancelled) setOverrides(data);
      } catch {
        if (!cancelled) setOverridesError(l10n.getString('terminal-error-overrides-load'));
      } finally {
        if (!cancelled) setOverridesLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [editingId, l10n]);

  // Load device binding + stores when the edit modal opens for a terminal.
  useEffect(() => {
    if (!editingId) {
      setBinding(null);
      setBindingStores([]);
      setBindingInstances([]);
      setSelectedStoreId('');
      setSelectedInstanceId('');
      return;
    }
    let cancelled = false;
    (async () => {
      setBindingLoading(true);
      setBindingError(null);
      try {
        const [b, stores] = await Promise.all([
          getDeviceBinding(editingId),
          listStores(),
        ]);
        if (!cancelled) {
          setBinding(b);
          setBindingStores(stores);
          if (b.boundStoreId) {
            setSelectedStoreId(b.boundStoreId);
            // Load instances for the bound store.
            try {
              const instances = await listWorkspaces('role-owner', b.boundStoreId);
              if (!cancelled) {
                setBindingInstances(instances);
                if (b.boundInstanceId) setSelectedInstanceId(b.boundInstanceId);
              }
            } catch {
              if (!cancelled) setBindingInstances([]);
            }
          }
        }
      } catch {
        if (!cancelled) setBindingError(l10n.getString('terminal-error-binding-load'));
      } finally {
        if (!cancelled) setBindingLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [editingId, l10n]);

  // Load instances when the selected store changes.
  useEffect(() => {
    if (!selectedStoreId || !editingId) {
      setBindingInstances([]);
      setSelectedInstanceId('');
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const instances = await listWorkspaces('role-owner', selectedStoreId);
        if (!cancelled) {
          setBindingInstances(instances);
          setSelectedInstanceId((prev) =>
            instances.some((i) => i.instance_id === prev) ? prev : '',
          );
        }
      } catch {
        if (!cancelled) setBindingInstances([]);
      }
    })();
    return () => { cancelled = true; };
  }, [selectedStoreId, editingId]);

  const closeModal = useCallback(() => {
    setShowModal(false);
    setError(null);
    setOverrides([]);
    setOverridesError(null);
    setBinding(null);
    setBindingError(null);
  }, []);

  // ── Device binding handlers (ADR #4 Phase 3) ────────────────────

  const handleBind = async () => {
    if (!editingId || !selectedStoreId || !selectedInstanceId) return;
    setBindingSaving(true);
    setBindingError(null);
    try {
      await setDeviceBinding(session?.user_id ?? '', editingId, selectedStoreId, selectedInstanceId);
      const b = await getDeviceBinding(editingId);
      setBinding(b);
    } catch {
      setBindingError(l10n.getString('terminal-error-binding-save'));
    } finally {
      setBindingSaving(false);
    }
  };

  const handleClearBinding = async () => {
    if (!editingId) return;
    setBindingSaving(true);
    setBindingError(null);
    try {
      await clearDeviceBinding(session?.user_id ?? '', editingId);
      setBinding({ bounded: false, boundStoreId: null, boundInstanceId: null, signatureValid: false });
      setSelectedStoreId('');
      setSelectedInstanceId('');
    } catch {
      setBindingError(l10n.getString('terminal-error-binding-clear'));
    } finally {
      setBindingSaving(false);
    }
  };

  const openDelete = useCallback((terminal: TerminalDto) => {
    setDeleteTarget(terminal);
  }, []);

  const closeDelete = useCallback(() => {
    setDeleteTarget(null);
  }, []);

  // ── Save / Update ──────────────────────────────────────────────

  // ── Feature override handlers ─────────────────────────────────

  const overrideEnabled = (featureKey: string): boolean | undefined => {
    const ov = overrides.find((o) => o.feature === featureKey);
    return ov?.enabled;
  };

  const handleToggleOverride = async (featureKey: string, currentEnabled: boolean) => {
    if (!editingId) return;
    try {
      await setTerminalOverride(session?.user_id ?? '', editingId, featureKey, !currentEnabled);
      const data = await listTerminalOverrides(editingId);
      setOverrides(data);
    } catch {
      setOverridesError(l10n.getString('terminal-error-override-update'));
    }
  };

  const handleResetOverrides = async () => {
    if (!editingId) return;
    try {
      const promises = overrides.map((o) =>
        deleteTerminalOverride(session?.user_id ?? '', editingId, o.feature),
      );
      await Promise.all(promises);
      setOverrides([]);
    } catch {
      setOverridesError(l10n.getString('terminal-error-override-reset'));
    }
  };

  // Validation runs BEFORE setSaving(true) to avoid calling setSaving(false)
  // twice (once in try, once in finally) and a visible loading flicker.
  // No useCallback needed — only used as onClick on a single button.
  const handleSave = async () => {
    const name = form.name.trim();
    const deviceId = form.deviceId.trim();

    if (!name) {
      setError(l10n.getString('terminal-error-name-required'));
      return;
    }
    if (!deviceId) {
      setError(l10n.getString('terminal-error-device-id-required'));
      return;
    }

    setSaving(true);
    setError(null);
    try {
      if (editingId) {
        await updateTerminal(session?.user_id ?? '', {
          id: editingId,
          name,
          deviceId,
          isActive: form.isActive,
          metadata: form.metadata || null,
        });
      } else {
        await registerTerminal(session?.user_id ?? '', {
          name,
          deviceId,
          terminalSecret: form.terminalSecret || null,
          metadata: form.metadata || null,
        });
      }

      closeModal();
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : l10n.getString('terminal-error-save'));
    } finally {
      setSaving(false);
    }
  };

  // ── Delete ─────────────────────────────────────────────────────

  const handleDelete = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      await deleteTerminal(session?.user_id ?? '', deleteTarget.id);
      closeDelete();
      await load();
    } catch {
      addToast({ message: l10n.getString('terminal-error-save'), type: 'error' });
    } finally {
      setDeleting(false);
    }
  };

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
        <div className="terminal-mgmt-loading-skeleton" aria-hidden="true">
          <div className="terminal-mgmt-header">
            <Skeleton variant="block" width="14rem" height="1.75rem" />
            <Skeleton variant="block" width="9rem" height="2.25rem" />
          </div>
          <div className="terminal-mgmt-table-wrap">
            <table className="terminal-mgmt-table" aria-hidden="true">
              <thead>
                <tr>
                  {['Name', 'Device ID', 'Status', 'Last Seen', 'Created', ''].map((_, i) => (
                    <th key={i}><Skeleton variant="text" width={i < 5 ? '5rem' : '3rem'} height="0.75rem" /></th>
                  ))}
                </tr>
              </thead>
              <tbody>{[0, 1, 2, 3].map((r) => (
                  <tr key={r}>
                    <td><Skeleton variant="text" width="7rem" height="0.875rem" /></td>
                    <td><Skeleton variant="text" width="6rem" height="0.75rem" /></td>
                    <td><Skeleton variant="block" width="4rem" height="1.125rem" style={{ borderRadius: 'var(--radius-full)' }} /></td>
                    <td><Skeleton variant="text" width="7rem" height="0.75rem" /></td>
                    <td><Skeleton variant="text" width="7rem" height="0.75rem" /></td>
                    <td>
                      <div className="terminal-mgmt-cell-actions">
                        <Skeleton variant="block" width="3rem" height="1.375rem" />
                        <Skeleton variant="block" width="3rem" height="1.375rem" />
                      </div>
                    </td>
                  </tr>
                ))}
</tbody>
            </table>
          </div>
        </div>
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
          <table className="terminal-mgmt-table" aria-label={l10n.getString('terminal-table-label')}>
            <thead>
              <tr>
                <Localized id="terminal-name"><th>Name</th></Localized>
                <Localized id="terminal-device-id"><th>Device ID</th></Localized>
                <Localized id="terminal-status"><th>Status</th></Localized>
                <Localized id="terminal-last-seen"><th>Last Seen</th></Localized>
                <Localized id="terminal-created"><th>Created</th></Localized>
                <Localized id="terminal-col-actions" attrs={{ "aria-label": true }}>
                  <th aria-label="Actions"> </th>
                </Localized>
              </tr>
            </thead>
            <tbody>{terminals.map((terminal) => (
                <tr key={terminal.id}>
                  <td className="terminal-mgmt-cell-name">{terminal.name}</td>
                  <td className="terminal-mgmt-cell-device-id">{terminal.deviceId}</td>
                  <td>
                    {terminal.isActive ? (
                      <Localized id="terminal-is-active">
                        <span className="terminal-mgmt-status-active">Active</span>
                      </Localized>
                    ) : (
                      <Localized id="terminal-is-inactive">
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
                  {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized attrs */}
                  <td>
                    <div className="terminal-mgmt-cell-actions">
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
                    </div>
                  </td>
                </tr>
              ))}
</tbody>
          </table>
        </div>
      )}

      {/* ── Add/Edit Modal ──────────────────────────────────────── */}
      <SettingsPopup
        open={showModal}
        onClose={closeModal}
        title={l10n.getString(isEditing ? 'terminal-edit-title' : 'terminal-register-title')}
        error={error}
        saving={saving}
        onSave={handleSave}
        saveLabel={l10n.getString(isEditing ? 'terminal-save' : 'terminal-register-action')}
        saveDisabled={!form.name.trim() || !form.deviceId.trim()}
        cancelLabel={l10n.getString('terminal-cancel')}
        size="lg"
      >
        {/* Name */}
        <label className="terminal-mgmt-field terminal-mgmt-field--horizontal" htmlFor="terminal-field-name" aria-label={l10n.getString('terminal-field-name-aria')}>
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
        <label className="terminal-mgmt-field terminal-mgmt-field--horizontal" htmlFor="terminal-field-device-id" aria-label={l10n.getString('terminal-field-device-id-aria')}>
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
          <label className="terminal-mgmt-field terminal-mgmt-field--horizontal" htmlFor="terminal-field-secret" aria-label={l10n.getString('terminal-field-secret-aria')}>
            <Localized id="terminal-secret-label">
              <span className="terminal-mgmt-label">Optional shared secret for sync authentication</span>
            </Localized>
            <input
              className="terminal-mgmt-input"
              type="password"
              id="terminal-field-secret"
              value={form.terminalSecret}
              onChange={(e) => setForm({ ...form, terminalSecret: e.target.value })}
              autoComplete="off"
            />
          </label>
        )}

        {/* Metadata */}
        <label className="terminal-mgmt-field terminal-mgmt-field--horizontal" htmlFor="terminal-field-metadata" aria-label={l10n.getString('terminal-field-metadata-aria')}>
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

        {/* Feature Overrides — edit mode only */}
        {isEditing && (
          <div className="terminal-mgmt-feature-overrides">
            <Localized id="terminal-feature-overrides">
              <h3 className="terminal-mgmt-feature-overrides-title">
                Feature Overrides
              </h3>
            </Localized>
            {overridesLoading ? (
              <div className="terminal-mgmt-overrides-skeleton" aria-hidden="true">
                {[0, 1, 2].map((groupIdx) => (
                  <div key={groupIdx} className="terminal-mgmt-skeleton-group">
                    <div className="terminal-mgmt-feature-group-header">
                      <Skeleton width="4rem" height="0.75rem" />
                      <Skeleton width="3rem" height="0.75rem" />
                    </div>
                    <div className="terminal-mgmt-feature-group-items">
                      {[0, 1].map((rowIdx) => (
                        <div key={rowIdx} className="terminal-mgmt-skeleton-toggle-row">
                          <Skeleton width="60%" height="0.875rem" />
                          <Skeleton variant="block" width="2.25rem" height="1.375rem" style={{ borderRadius: '0.6875rem' }} />
                        </div>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="terminal-mgmt-feature-groups">
                {FEATURE_GROUPS.map((group) => {
                  const groupOverrides = group.keys.filter((k) =>
                    overrideEnabled(k) !== undefined,
                  );
                  return (
                    <div key={group.label} className="terminal-mgmt-feature-group">
                      <div className="terminal-mgmt-feature-group-header">
                        <span className="terminal-mgmt-feature-group-label">
                          {group.label}
                        </span>
                        {groupOverrides.length > 0 && (
                          <span className="terminal-mgmt-feature-group-count">
                            {groupOverrides.length} override
                            {groupOverrides.length !== 1 ? 's' : ''}
                          </span>
                        )}
                      </div>
                      <div className="terminal-mgmt-feature-group-items">
                        {group.keys.map((featureKey) => {
                          const ov = overrideEnabled(featureKey);
                          const isOverridden = ov !== undefined;
                          const checked = ov ?? false;
                          const toggleId = `toggle-${featureKey}`;
                          return (
                            <label
                              key={featureKey}
                              htmlFor={toggleId}
                              className={
                                'terminal-mgmt-toggle-row' +
                                (isOverridden
                                  ? ' terminal-mgmt-toggle-row--overridden'
                                  : '')
                              }
                            >
                              <span className="terminal-mgmt-toggle-label">
                                <span className="terminal-mgmt-toggle-name">
                                  {featureLabel(featureKey)}
                                </span>
                                {isOverridden && (
                                  <span className="terminal-mgmt-toggle-badge">
                                    overridden
                                  </span>
                                )}
                              </span>
                              <span className="terminal-mgmt-toggle-switch">
                                <input
                                  type="checkbox"
                                  id={toggleId}
                                  className="terminal-mgmt-toggle-input"
                                  checked={checked}
                                  onChange={() =>
                                    handleToggleOverride(
                                      featureKey,
                                      checked,
                                    )
                                  }
                                  aria-label={l10n.getString(
                                    'terminal-override-aria',
                                    {
                                      feature:
                                        featureLabel(featureKey),
                                    },
                                  )}
                                />
                                <span className="terminal-mgmt-toggle-track">
                                  <span className="terminal-mgmt-toggle-thumb" />
                                </span>
                              </span>
                            </label>
                          );
                        })}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
            {overridesError && (
              <div className="terminal-mgmt-error" role="alert">
                <span>{overridesError}</span>
              </div>
            )}
            {overrides.length > 0 && (
              <div className="terminal-mgmt-reset-overrides">
                <Localized id="terminal-reset-overrides">
                  <Button variant="ghost" size="sm" onClick={handleResetOverrides}>
                    Reset all overrides
                  </Button>
                </Localized>
              </div>
            )}
          </div>
        )}

        {/* Device Binding (ADR #4 Phase 3) — edit mode only */}
        {isEditing && (
          <div className="terminal-mgmt-feature-overrides">
            <h3 className="terminal-mgmt-feature-overrides-title">
              Device Binding
            </h3>
            {bindingLoading ? (
              <div className="terminal-mgmt-binding-skeleton" aria-hidden="true">
                <div className="terminal-mgmt-skeleton-binding-info">
                  <Skeleton width="70%" height="0.875rem" />
                  <Skeleton width="50%" height="0.75rem" style={{ marginTop: '0.25rem' }} />
                </div>
                <div className="terminal-mgmt-binding-fields">
                  <div className="terminal-mgmt-skeleton-field">
                    <Skeleton width="3rem" height="0.875rem" />
                    <Skeleton variant="block" width="100%" height="2.375rem" style={{ borderRadius: 'var(--radius-lg)' }} />
                  </div>
                  <div className="terminal-mgmt-skeleton-field">
                    <Skeleton width="5rem" height="0.875rem" />
                    <Skeleton variant="block" width="100%" height="2.375rem" style={{ borderRadius: 'var(--radius-lg)' }} />
                  </div>
                </div>
                <div className="terminal-mgmt-binding-actions">
                  <Skeleton variant="block" width="7rem" height="2rem" style={{ borderRadius: 'var(--radius-lg)' }} />
                </div>
              </div>
            ) : (
              <>
                {binding?.bounded && (
                  <div className="terminal-mgmt-binding-info">
                    <p>
                      Bound to store: <strong>{binding.boundStoreId}</strong>
                      {binding.boundInstanceId && (<> &middot; instance: <strong>{binding.boundInstanceId}</strong></>)}
                    </p>
                    <p className={binding.signatureValid ? 'terminal-mgmt-status-active' : 'terminal-mgmt-status-inactive'}>
                      Signature: {binding.signatureValid ? 'Valid' : 'Invalid / Tampered'}
                    </p>
                  </div>
                )}
                <div className="terminal-mgmt-binding-fields">
                  <label className="terminal-mgmt-field terminal-mgmt-field--horizontal" htmlFor="bind-store">
                    <span className="terminal-mgmt-label">Store</span>
                    <select
                      id="bind-store"
                      className="terminal-mgmt-input"
                      value={selectedStoreId}
                      onChange={(e) => setSelectedStoreId(e.target.value)}
                    >
                      <option value="">-- Select store --</option>
                      {bindingStores.map((s) => (
                        <option key={s.id} value={s.id}>{s.name}{s.is_primary ? ' (Primary)' : ''}</option>
                      ))}
                    </select>
                  </label>
                  <label className="terminal-mgmt-field terminal-mgmt-field--horizontal" htmlFor="bind-instance">
                    <span className="terminal-mgmt-label">Workspace Instance</span>
                    <select
                      id="bind-instance"
                      className="terminal-mgmt-input"
                      value={selectedInstanceId}
                      onChange={(e) => setSelectedInstanceId(e.target.value)}
                      disabled={!selectedStoreId}
                    >
                      <option value="">-- Select instance --</option>
                      {bindingInstances.map((i) => (
                        <option key={i.instance_id} value={i.instance_id}>{i.name} ({i.type_key})</option>
                      ))}
                    </select>
                  </label>
                </div>
                <div className="terminal-mgmt-binding-actions">
                  <Button
                    variant="primary"
                    size="sm"
                    loading={bindingSaving}
                    disabled={!selectedStoreId || !selectedInstanceId}
                    onClick={handleBind}
                  >
                    {binding?.bounded ? 'Update Binding' : 'Bind Terminal'}
                  </Button>
                  {binding?.bounded && (
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={bindingSaving}
                      onClick={handleClearBinding}
                    >
                      Clear Binding
                    </Button>
                  )}
                </div>
              </>
            )}
            {bindingError && (
              <div className="terminal-mgmt-error" role="alert">
                <span>{bindingError}</span>
              </div>
            )}
          </div>
        )}
      </SettingsPopup>

      {/* ── Delete Confirmation Modal ────────────────────────────── */}
      <SettingsPopup
        open={deleteTarget !== null}
        onClose={closeDelete}
        title={l10n.getString('terminal-delete-title')}
        onSave={handleDelete}
        saveLabel={l10n.getString('terminal-delete')}
        cancelLabel={l10n.getString('terminal-cancel')}
        saving={deleting}
        size="sm"
        footer={
          <>
            <Localized id="terminal-cancel">
              <Button variant="ghost" onClick={closeDelete} disabled={deleting}>
                Cancel
              </Button>
            </Localized>
            <Button variant="danger" loading={deleting} onClick={handleDelete}>
              <Localized id="terminal-delete"><span>Delete</span></Localized>
            </Button>
          </>
        }
      >
        <Localized id="terminal-delete-confirm" vars={{ name: deleteTarget?.name ?? '' }}>
          <p>Are you sure you want to delete terminal &quot;{deleteTarget?.name ?? ''}&quot;? This action cannot be undone.</p>
        </Localized>
      </SettingsPopup>
    </div>
  );
}
