import { useState, useCallback, useEffect } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listStaff,
  listRoles,
  createStaff,
  updateStaff,
  type StaffMemberDto,
  type RoleDto,
} from '@/api/staff';
import {
  listAllWorkspaces,
  setUserWorkspaces,
  getUserWorkspaces,
  type WorkspaceTypeDto,
} from '@/api/workspaces';
import { useAuth } from '@/contexts/AuthContext';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Badge } from '@/components/Badge';
import { RoleIcon } from '@/components/RoleIcon';
import { useToast } from '@/frontend/shared/Toast';
import './StaffManagementScreen.css';

// ── SVG icon props ────────────────────────────────────────────────

const ICON_PROPS = { width: 18, height: 18, viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', strokeWidth: '1.5', strokeLinecap: 'round', strokeLinejoin: 'round' } as const;

function wsIcon(key: string): React.ReactNode {
  switch (key) {
    case 'restaurant':
      return <svg {...ICON_PROPS}><path d="M6 2v20m12-20v5.3c0 3.3-2.7 6-6 6s-6-2.7-6-6V2"/></svg>;
    case 'store':
      return <svg {...ICON_PROPS}><path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>;
    case 'inventory':
      return <svg {...ICON_PROPS}><path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/></svg>;
    case 'admin':
      return <svg {...ICON_PROPS}><circle cx="12" cy="12" r="3"/><path d="M12 1v2m0 18v2m-9.9-4.9l1.4 1.4m12.8 1.4l1.4-1.4M1 12h2m18 0h2M4.2 4.2l1.4 1.4m12.8 12.8l1.4 1.4"/></svg>;
    default:
      return <svg {...ICON_PROPS}><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>;
  }
}

// ── Form state ──────────────────────────────────────────────────────

interface FormData {
  username: string;
  displayName: string;
  pin: string;
  roleId: string;
  /** Only used when editing — workspace assignment mode */
  wsMode: 'default' | 'custom';
  /** Only used when editing — selected workspace keys */
  wsKeys: string[];
}

const EMPTY_FORM: FormData = {
  username: '',
  displayName: '',
  pin: '',
  roleId: '',
  wsMode: 'default',
  wsKeys: [],
};

// ── Component ───────────────────────────────────────────────────────

/** Staff management screen — manage user accounts, roles, PIN codes, and workspace assignments. */
export default function StaffManagementScreen() {
  const { l10n } = useLocalization();
  const { session } = useAuth();
  const { addToast } = useToast();
  const [staff, setStaff] = useState<StaffMemberDto[]>([]);
  const [roles, setRoles] = useState<RoleDto[]>([]);
  const [allWorkspaces, setAllWorkspaces] = useState<WorkspaceTypeDto[]>([]);
  const [workspaceNameMap, setWorkspaceNameMap] = useState<Map<string, string>>(new Map());
  const [staffWorkspaces, setStaffWorkspaces] = useState<Map<string, string[]>>(new Map());
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<FormData>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const callerUserId = session?.user_id ?? '';

  // ── Load data

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [staffData, rolesData] = await Promise.all([
        listStaff(),
        listRoles(),
      ]);
      setStaff(staffData);
      setRoles(rolesData);

      // Load workspace names and assignments for the table column.
      try {
        const workspaces = await listAllWorkspaces(callerUserId);
        const nameMap = new Map<string, string>();
        for (const w of workspaces) {
          nameMap.set(w.key, w.name);
        }
        setWorkspaceNameMap(nameMap);

        const wsMap = new Map<string, string[]>();
        const results = await Promise.allSettled(
          staffData.map((m) => getUserWorkspaces(m.id)),
        );
        for (let i = 0; i < results.length; i++) {
          const member = staffData[i];
          const r = results[i];
          if (member && r && r.status === 'fulfilled') {
            wsMap.set(member.id, r.value);
          } else if (member) {
            wsMap.set(member.id, []);
          }
        }
        setStaffWorkspaces(wsMap);
      } catch {
        // Workspace data unavailable — column will be empty.
      }
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, [callerUserId]);

  useEffect(() => { load(); }, [load]);

  // ── Modal handlers

  const openCreate = useCallback(() => {
    setForm(EMPTY_FORM);
    setEditingId(null);
    setError(null);
    setShowModal(true);
  }, []);

  const openEdit = useCallback(async (member: StaffMemberDto) => {
    setForm({
      username: member.username,
      displayName: member.display_name,
      pin: '',
      roleId: member.role_id,
      wsMode: 'default',
      wsKeys: [],
    });
    setEditingId(member.id);
    setError(null);
    setShowModal(true);

    // Load workspaces and user's current assignments in parallel.
    try {
      const [workspaces, userKeys] = await Promise.all([
        listAllWorkspaces(callerUserId),
        getUserWorkspaces(member.id),
      ]);
      setAllWorkspaces(workspaces);
      if (userKeys.length > 0) {
        setForm((prev) => ({ ...prev, wsMode: 'custom', wsKeys: userKeys }));
      }
    } catch {
      setAllWorkspaces([]);
    }
  }, [callerUserId]);

  const closeModal = useCallback(() => {
    setShowModal(false);
    setError(null);
  }, []);

  // ── Toggle workspace checkbox ──────────────────────────────────

  const toggleWsKey = useCallback((key: string) => {
    setForm((prev) => ({
      ...prev,
      wsKeys: prev.wsKeys.includes(key)
        ? prev.wsKeys.filter((k) => k !== key)
        : [...prev.wsKeys, key],
    }));
  }, []);

  // ── Save / Update ──────────────────────────────────────────────

  // handleSave reads form state directly on every invocation — no useCallback
  // needed since it's only used as an onClick handler on a single button.
  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      const username = form.username.trim().toLowerCase();
      const displayName = form.displayName.trim();

      if (!username) {
        setError(l10n.getString('staff-error-username-required'));
        setSaving(false);
        return;
      }
      if (!displayName) {
        setError(l10n.getString('staff-error-display-name-required'));
        setSaving(false);
        return;
      }
      if (!form.roleId) {
        setError(l10n.getString('staff-error-role-required'));
        setSaving(false);
        return;
      }

      if (editingId) {
        await updateStaff({
          id: editingId,
          username,
          display_name: displayName,
          role_id: form.roleId,
          is_active: true,
          caller_user_id: callerUserId,
        });

        // Save workspace assignments.
        await setUserWorkspaces(
          editingId,
          form.wsMode === 'custom' ? form.wsKeys : [],
          callerUserId,
        );
      } else {
        if (!form.pin || form.pin.length < 4) {
          setError(l10n.getString('staff-error-pin-length'));
          setSaving(false);
          return;
        }
        await createStaff({
          username,
          pin: form.pin,
          display_name: displayName,
          role_id: form.roleId,
          caller_user_id: callerUserId,
        });
      }

      closeModal();
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : l10n.getString('staff-error-save-failed'));
    } finally {
      setSaving(false);
    }
  };

  // ── Deactivate / Reactivate ────────────────────────────────────

  const toggleActive = useCallback(async (member: StaffMemberDto) => {
    try {
      await updateStaff({
        id: member.id,
        username: member.username,
        display_name: member.display_name,
        role_id: member.role_id,
        is_active: !member.is_active,
        caller_user_id: callerUserId,
      });
      await load();
    } catch {
      addToast({ message: l10n.getString('staff-error-save-failed'), type: 'error' });
    }
  }, [load, callerUserId, addToast, l10n]);

  // ── Role colour mapping ────────────────────────────────────────

  const roleVariant = (roleName: string): 'warning' | 'info' | 'default' | 'success' => {
    switch (roleName.toLowerCase()) {
      case 'owner':
      case 'role-owner':
      case 'admin':
      case 'role-admin':   return 'warning';
      case 'manager':
      case 'role-manager': return 'info';
      case 'kitchen':
      case 'role-kitchen': return 'success';
      case 'cashier':
      case 'role-cashier': return 'default';
      case 'staff':        return 'default';
      default:             return 'default';
    }
  };

  // ── Render ─────────────────────────────────────────────────────

  const isEditing = editingId !== null;
  const hasRoleSelected = roles.length > 0;

  return (
    <div className="staff-mgmt">
      <div className="staff-mgmt-header">
        <Localized id="staff-title">
          <h1 className="staff-mgmt-title">Staff</h1>
        </Localized>
        <Localized id="staff-add-button">
          <Button onClick={openCreate}>Add Staff</Button>
        </Localized>
      </div>

      {loading ? (
        <Localized id="staff-loading">
          <p className="staff-mgmt-loading">Loading staff…</p>
        </Localized>
      ) : staff.length === 0 ? (
        <Card shadow="sm">
          <div className="staff-mgmt-empty">
            <div className="staff-mgmt-empty-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" width="48" height="48">
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                <circle cx="9" cy="7" r="4" />
                <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                <path d="M16 3.13a4 4 0 0 1 0 7.75" />
              </svg>
            </div>
            <Localized id="staff-empty">
              <p>No staff members yet.</p>
            </Localized>
            <Localized id="staff-empty-cta">
              <Button variant="secondary" onClick={openCreate}>
                Add your first staff member
              </Button>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="staff-mgmt-table-wrap">
          <table className="staff-mgmt-table" aria-label={l10n.getString('staff-table-aria')}>
            <thead>
              <tr>
                <Localized id="staff-col-name"><th>Name</th></Localized>
                <Localized id="staff-col-username"><th>Username</th></Localized>
                <Localized id="staff-col-role"><th>Role</th></Localized>
                <Localized id="staff-col-status"><th>Status</th></Localized>
                <Localized id="staff-col-workspace"><th>Workspace</th></Localized>
                <Localized id="staff-col-actions" attrs={{ "aria-label": true }}>
                  <th aria-label="Actions"> </th>
                </Localized>
              </tr>
            </thead>
            <tbody>
              {staff.map((member) => (
                <tr key={member.id} className={!member.is_active ? 'staff-mgmt-row--inactive' : ''}>
                  <td>
                    <div className="staff-mgmt-cell-name">
                      <div className="staff-mgmt-avatar">
                        {member.display_name.charAt(0).toUpperCase()}
                      </div>
                      <span>{member.display_name}</span>
                    </div>
                  </td>
                  <td className="staff-mgmt-cell-username">{member.username}</td>
                  <td>
                    <Badge variant={roleVariant(member.role_name)}>
                      <span className="staff-mgmt-role-badge-content">
                        <RoleIcon role={member.role_name} size={12} className="staff-mgmt-role-icon" />
                        <span>{member.role_name}</span>
                      </span>
                    </Badge>
                  </td>
                  <td>
                    {member.is_active ? (
                      <Localized id="staff-status-active">
                        <span className="staff-mgmt-status-active">Active</span>
                      </Localized>
                    ) : (
                      <Localized id="staff-status-inactive">
                        <span className="staff-mgmt-status-inactive">Inactive</span>
                      </Localized>
                    )}
                  </td>
                  <td className="staff-mgmt-cell-username">
                    {(staffWorkspaces.get(member.id) ?? [])
                      .map((k) => workspaceNameMap.get(k) ?? k)
                      .join(', ') || '—'}
                  </td>
                  <td>
                    <div className="staff-mgmt-cell-actions">
                    <Localized id="staff-edit-aria" attrs={{ "aria-label": true }} vars={{ name: member.display_name }}>
                      <button
                        type="button"
                        className="staff-mgmt-action-btn"
                        onClick={() => openEdit(member)}
                        aria-label={`Edit ${member.display_name}`}
                      >
                        <Localized id="staff-edit"><span>Edit</span></Localized>
                      </button>
                    </Localized>
                    <Localized id={member.is_active ? 'staff-deactivate-aria' : 'staff-restore-aria'} attrs={{ "aria-label": true }} vars={{ name: member.display_name }}>
                      <button
                        type="button"
                        className={`staff-mgmt-action-btn ${member.is_active ? 'staff-mgmt-action-btn--warn' : 'staff-mgmt-action-btn--restore'}`}
                        onClick={() => toggleActive(member)}
                        aria-label={member.is_active ? `Deactivate ${member.display_name}` : `Reactivate ${member.display_name}`}
                      >
                        <Localized id={member.is_active ? 'staff-deactivate' : 'staff-restore'}>
                          <span>{member.is_active ? 'Deactivate' : 'Restore'}</span>
                        </Localized>
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
      {showModal && (
        <Localized id={isEditing ? 'staff-modal-edit-aria' : 'staff-modal-add-aria'} attrs={{ "aria-label": true }}>
          <div className="staff-mgmt-overlay" role="dialog" aria-modal="true" aria-label={isEditing ? 'Edit staff member' : 'Add staff member'}>
            <div className="staff-mgmt-modal">
              <div className="staff-mgmt-modal-header">
                <Localized id={isEditing ? 'staff-modal-edit-title' : 'staff-modal-add-title'}>
                  <h2>{isEditing ? 'Edit Staff Member' : 'Add Staff Member'}</h2>
                </Localized>
                <Localized id="staff-modal-close" attrs={{ "aria-label": true }}>
                  <button
                    type="button"
                    className="staff-mgmt-modal-close"
                    onClick={closeModal}
                    aria-label="Close"
                  >
                    &times;
                  </button>
                </Localized>
              </div>

              <div className="staff-mgmt-modal-body">
                {/* Username */}
                <label className="staff-mgmt-field" htmlFor="staff-field-username" aria-label={l10n.getString('staff-field-username-aria')}>
                  <Localized id="staff-field-username-label">
                    <span className="staff-mgmt-label">Username *</span>
                  </Localized>
                  <Localized id="staff-username-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="staff-mgmt-input"
                      type="text"
                      id="staff-field-username"
                      value={form.username}
                      onChange={(e) => setForm({ ...form, username: e.target.value })}
                      placeholder="e.g. jane"
                      disabled={isEditing}
                      autoComplete="off"
                    />
                  </Localized>
                </label>

                {/* Display name */}
                <label className="staff-mgmt-field" htmlFor="staff-field-name" aria-label={l10n.getString('staff-field-name-aria')}>
                  <Localized id="staff-field-name-label">
                    <span className="staff-mgmt-label">Display Name *</span>
                  </Localized>
                  <Localized id="staff-name-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="staff-mgmt-input"
                      type="text"
                      id="staff-field-name"
                      value={form.displayName}
                      onChange={(e) => setForm({ ...form, displayName: e.target.value })}
                      placeholder="e.g. Jane Smith"
                      autoComplete="off"
                    />
                  </Localized>
                </label>

                {/* PIN */}
                <label className="staff-mgmt-field" htmlFor="staff-field-pin" aria-label={l10n.getString('staff-field-pin-aria')}>
                  <Localized id={isEditing ? 'staff-field-pin-edit-label' : 'staff-field-pin-label'}>
                    <span className="staff-mgmt-label">
                      {isEditing ? 'New PIN (leave blank to keep current)' : 'PIN * (4+ characters)'}
                    </span>
                  </Localized>
                  <Localized id={isEditing ? 'staff-pin-edit-placeholder' : 'staff-pin-placeholder'} attrs={{ placeholder: true }}>
                    <input
                      className="staff-mgmt-input"
                      type="password"
                      id="staff-field-pin"
                      value={form.pin}
                      onChange={(e) => setForm({ ...form, pin: e.target.value })}
                      placeholder={isEditing ? 'Leave blank to keep current' : 'Enter PIN'}
                      autoComplete="new-password"
                    />
                  </Localized>
                </label>

                {/* Role selector */}
                {hasRoleSelected && (
                  <label className="staff-mgmt-field" htmlFor="staff-field-role">
                    <Localized id="staff-field-role-label">
                      <span className="staff-mgmt-label">Role *</span>
                    </Localized>
                    <select
                      className="staff-mgmt-input staff-mgmt-select"
                      id="staff-field-role"
                      value={form.roleId}
                      onChange={(e) => setForm({ ...form, roleId: e.target.value })}
                    >
                      <Localized id="staff-role-select-default">
                        <option value="">Select a role…</option>
                      </Localized>
                      {roles.map((role) => (
                        <option key={role.id} value={role.id}>
                          {role.name} — {role.description}
                        </option>
                      ))}
                    </select>
                  </label>
                )}

                {/* ── Workspace Access Section (edit only) ──────── */}
                {isEditing && allWorkspaces.length > 0 && (
                  <fieldset className="staff-mgmt-ws-section">
                    <Localized id="staff-ws-section-label">
                      <legend className="staff-mgmt-label">Workspace Access</legend>
                    </Localized>

                    <div className="staff-mgmt-radio">
                      <input
                        type="radio"
                        name="wsMode"
                        value="default"
                        checked={form.wsMode === 'default'}
                        onChange={() => setForm({ ...form, wsMode: 'default', wsKeys: [] })}
                        aria-label={l10n.getString('staff-ws-role-defaults')}
                      />
                      <Localized id="staff-ws-role-defaults">
                        <span>Use role defaults</span>
                      </Localized>
                    </div>

                    <div className="staff-mgmt-radio">
                      <input
                        type="radio"
                        name="wsMode"
                        value="custom"
                        checked={form.wsMode === 'custom'}
                        onChange={() => setForm({ ...form, wsMode: 'custom' })}
                        aria-label={l10n.getString('staff-ws-custom')}
                      />
                      <Localized id="staff-ws-custom">
                        <span>Custom</span>
                      </Localized>
                    </div>

                    {form.wsMode === 'custom' && (
                      <div className="staff-mgmt-ws-checkboxes">
                        {allWorkspaces.map((ws) => (
                          <label key={ws.key} className="staff-mgmt-ws-checkbox">
                            <input
                              type="checkbox"
                              checked={form.wsKeys.includes(ws.key)}
                              onChange={() => toggleWsKey(ws.key)}
                            />
                            <span className="staff-mgmt-ws-checkbox-label">
                              {ws.icon && (
                                <span className="staff-mgmt-ws-icon" aria-hidden="true">
                                  {wsIcon(ws.icon)}
                                </span>
                              )}
                              {ws.name}
                            </span>
                            <span className="staff-mgmt-ws-desc">{ws.description}</span>
                          </label>
                        ))}
                      </div>
                    )}
                  </fieldset>
                )}

                {/* Error */}
                {error && (
                  <div className="staff-mgmt-error" role="alert">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16" aria-hidden="true">
                      <circle cx="12" cy="12" r="10" />
                      <line x1="15" y1="9" x2="9" y2="15" />
                      <line x1="9" y1="9" x2="15" y2="15" />
                    </svg>
                    <Localized id="staff-error-generic" vars={{ message: error }}>
                      <span>{error}</span>
                    </Localized>
                  </div>
                )}
              </div>

              <div className="staff-mgmt-modal-actions">
                <Localized id="staff-btn-cancel">
                  <Button variant="ghost" onClick={closeModal} disabled={saving}>
                    Cancel
                  </Button>
                </Localized>
                <Button
                  variant="primary"
                  loading={saving}
                  disabled={
                    !form.username.trim() ||
                    !form.displayName.trim() ||
                    !form.roleId ||
                    (!isEditing && (!form.pin || form.pin.length < 4)) ||
                    (isEditing && form.wsMode === 'custom' && allWorkspaces.length > 0 && form.wsKeys.length === 0)
                  }
                  onClick={handleSave}
                >
                  <Localized id={isEditing ? 'staff-btn-update' : 'staff-btn-create'}>
                    <span>{isEditing ? 'Update' : 'Create'}</span>
                  </Localized>
                </Button>
              </div>
            </div>
          </div>
        </Localized>
      )}
    </div>
  );
}
