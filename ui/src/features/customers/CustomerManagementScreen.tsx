import { useState, useCallback, useEffect, useMemo } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useAuth } from '@/contexts/AuthContext';
import {
  listCustomers,
  createCustomer,
  updateCustomer,
  deleteCustomer,
  type CustomerDto,
  type UpdateCustomerArgs,
  type CreateCustomerArgs,
} from '@/api/customers';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Skeleton } from '@/components/Skeleton';
import { SettingsPopup } from '@/frontend/shared';
import './CustomerManagementScreen.css';

// ── Form state ──────────────────────────────────────────────────────

interface FormData {
  name: string;
  email: string;
  phone: string;
  notes: string;
}

const EMPTY_FORM: FormData = {
  name: '',
  email: '',
  phone: '',
  notes: '',
};

// ── Component ───────────────────────────────────────────────────────

/** Customer management screen — list, search, create, edit, and delete customer records. */
export default function CustomerManagementScreen() {
  const { l10n } = useLocalization();
  const { session } = useAuth();
  const userId = session?.user_id ?? '';
  const [customers, setCustomers] = useState<CustomerDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [showModal, setShowModal] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<FormData>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // ── Load data ──────────────────────────────────────────────────

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await listCustomers();
      setCustomers(data);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  // ── Search filter ──────────────────────────────────────────────

  const filteredCustomers = useMemo(() => {
    if (!searchQuery.trim()) return customers;
    const q = searchQuery.trim().toLowerCase();
    return customers.filter(
      (c) =>
        c.name.toLowerCase().includes(q) ||
        (c.email ?? '').toLowerCase().includes(q) ||
        (c.phone ?? '').toLowerCase().includes(q),
    );
  }, [customers, searchQuery]);

  // ── Modal handlers ──────────────────────────────────────────────

  const openCreate = useCallback(() => {
    setForm(EMPTY_FORM);
    setEditingId(null);
    setError(null);
    setShowModal(true);
  }, []);

  const openEdit = useCallback((customer: CustomerDto) => {
    setForm({
      name: customer.name,
      email: customer.email ?? '',
      phone: customer.phone ?? '',
      notes: customer.notes,
    });
    setEditingId(customer.id);
    setError(null);
    setShowModal(true);
  }, []);

  const closeModal = useCallback(() => {
    setShowModal(false);
    setError(null);
  }, []);

  // ── Save / Update ──────────────────────────────────────────────

  const handleSave = useCallback(async () => {
    if (!form.name.trim()) {
      setError(l10n.getString('customer-mgmt-error-name-required'));
      return;
    }

    setSaving(true);
    setError(null);
    try {
      const name = form.name.trim();

      if (editingId) {
        const args: UpdateCustomerArgs = { userId, id: editingId, name };
        if (form.email.trim()) args.email = form.email.trim();
        if (form.phone.trim()) args.phone = form.phone.trim();
        if (form.notes.trim()) args.notes = form.notes.trim();
        await updateCustomer(args);
      } else {
        const args: CreateCustomerArgs = { userId, name };
        if (form.email.trim()) args.email = form.email.trim();
        if (form.phone.trim()) args.phone = form.phone.trim();
        if (form.notes.trim()) args.notes = form.notes.trim();
        await createCustomer(args);
      }
      closeModal();
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : l10n.getString('customer-mgmt-error-save-failed'));
    } finally {
      setSaving(false);
    }
  }, [form, editingId, closeModal, load, userId, l10n]);

  // ── Delete ─────────────────────────────────────────────────────

  const confirmDelete = useCallback(async (id: string) => {
    setDeleting(id);
    try {
      await deleteCustomer({ userId, id });
      setDeleting(null);
      await load();
    } catch {
      setDeleting(null);
    }
  }, [load, userId]);

  // ── Render ─────────────────────────────────────────────────────

  return (
    <div className="customer-mgmt">
      <div className="customer-mgmt-header">
        <Localized id="customer-mgmt-title">
          <h1 className="customer-mgmt-title">Customers</h1>
        </Localized>
        <Localized id="customer-mgmt-add">
          <Button onClick={openCreate}>Add Customer</Button>
        </Localized>
      </div>

      {/* Search */}
      <div className="customer-mgmt-search-wrap">
        <svg className="customer-mgmt-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <Localized id="customer-mgmt-search" attrs={{ placeholder: true, 'aria-label': true }}>
          <input
            type="search"
            className="customer-mgmt-search"
            id="customer-mgmt-search"
            name="customer-mgmt-search"
            placeholder="Search by name, email, or phone…"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            aria-label="Search customers"
          />
        </Localized>
      </div>

      {/* Content */}
      {loading ? (
        <div className="customer-mgmt-loading-skeleton" aria-hidden="true">
          {/* Header skeleton: title + button */}
          <div className="customer-mgmt-header">
            <Skeleton variant="block" width="10rem" height="1.75rem" />
            <Skeleton variant="block" width="9rem" height="2.25rem" />
          </div>
          {/* Search bar skeleton */}
          <div className="customer-mgmt-skeleton-search">
            <Skeleton variant="circle" width="1rem" height="1rem" />
            <Skeleton variant="text" width="100%" height="1.125rem" />
          </div>
          {/* Table skeleton: header + 4 rows with 5 columns */}
          <div className="customer-mgmt-table-wrap">
            <table className="customer-mgmt-table" aria-hidden="true">
              <thead>
                <tr>
                  {['Name', 'Email', 'Phone', 'Notes', ''].map((_, i) => (
                    <th key={i}><Skeleton variant="text" width={i < 4 ? '4rem' : '3rem'} height="0.75rem" /></th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {[0, 1, 2, 3].map((r) => (
                  <tr key={r}>
                    <td>
                      <div className="customer-mgmt-cell-name">
                        <Skeleton variant="circle" width="2rem" height="2rem" />
                        <Skeleton variant="text" width="6rem" height="0.875rem" />
                      </div>
                    </td>
                    <td><Skeleton variant="text" width="8rem" height="0.75rem" /></td>
                    <td><Skeleton variant="text" width="6rem" height="0.75rem" /></td>
                    <td><Skeleton variant="text" width="5rem" height="0.75rem" /></td>
                    <td className="customer-mgmt-cell-actions">
                      <Skeleton variant="block" width="3.5rem" height="1.375rem" />
                      <Skeleton variant="block" width="3.5rem" height="1.375rem" />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : customers.length === 0 ? (
        <Card shadow="sm">
          <div className="customer-mgmt-empty">
            <div className="customer-mgmt-empty-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" width="48" height="48">
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                <circle cx="9" cy="7" r="4" />
                <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                <path d="M16 3.13a4 4 0 0 1 0 7.75" />
              </svg>
            </div>
            <Localized id="customer-mgmt-empty">
              <p>No customers yet.</p>
            </Localized>
            <Localized id="customer-mgmt-empty-cta">
              <Button variant="secondary" onClick={openCreate}>
                Add your first customer
              </Button>
            </Localized>
          </div>
        </Card>
      ) : filteredCustomers.length === 0 ? (
        <Card shadow="sm">
          <div className="customer-mgmt-empty">
            <Localized id="customer-mgmt-search-empty">
              <p>No customers match your search.</p>
            </Localized>
            <Localized id="customer-mgmt-search-clear">
              <Button variant="ghost" onClick={() => setSearchQuery('')}>
                Clear search
              </Button>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="customer-mgmt-table-wrap">
          <table className="customer-mgmt-table" aria-label={l10n.getString('customer-mgmt-table-aria')}>
            <thead>
              <tr>
                <Localized id="customer-mgmt-col-name"><th>Name</th></Localized>
                <Localized id="customer-mgmt-col-email"><th>Email</th></Localized>
                <Localized id="customer-mgmt-col-phone"><th>Phone</th></Localized>
                <Localized id="customer-mgmt-col-notes"><th>Notes</th></Localized>
                <Localized id="customer-mgmt-col-actions" attrs={{ 'aria-label': true }}>
                  <th aria-label="Actions"> </th>
                </Localized>
              </tr>
            </thead>
            <tbody>
              {filteredCustomers.map((customer) => (
                <tr key={customer.id}>
                  <td>
                    <div className="customer-mgmt-cell-name">
                      <div className="customer-mgmt-avatar">
                        {customer.name.charAt(0).toUpperCase()}
                      </div>
                      <span className="customer-mgmt-name-text">{customer.name}</span>
                    </div>
                  </td>
                  <td className="customer-mgmt-cell-email">
                    {customer.email ?? '\u2014'}
                  </td>
                  <td className="customer-mgmt-cell-phone">
                    {customer.phone ?? '\u2014'}
                  </td>
                  <td className="customer-mgmt-cell-notes">
                    {customer.notes || '\u2014'}
                  </td>
                  <td className="customer-mgmt-cell-actions">
                    <Localized id="customer-mgmt-edit-aria" attrs={{ 'aria-label': true }} vars={{ name: customer.name }}>
                      <button
                        type="button"
                        className="customer-mgmt-action-btn"
                        onClick={() => openEdit(customer)}
                        aria-label={`Edit ${customer.name}`}
                      >
                        <Localized id="customer-mgmt-edit"><span>Edit</span></Localized>
                      </button>
                    </Localized>
                    <Localized id="customer-mgmt-delete-aria" attrs={{ 'aria-label': true }} vars={{ name: customer.name }}>
                      <button
                        type="button"
                        className="customer-mgmt-action-btn customer-mgmt-action-btn--danger"
                        onClick={() => confirmDelete(customer.id)}
                        disabled={deleting === customer.id}
                        aria-label={`Delete ${customer.name}`}
                      >
                        <Localized id="customer-mgmt-delete"><span>Delete</span></Localized>
                      </button>
                    </Localized>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
              </div>
      )}

      <SettingsPopup
        open={showModal}
        onClose={closeModal}
        title={l10n.getString(editingId ? 'customer-mgmt-modal-edit-title' : 'customer-mgmt-modal-add-title')}
        saving={saving}
        error={error}
        onSave={handleSave}
        saveLabel={l10n.getString(editingId ? 'customer-mgmt-btn-update' : 'customer-mgmt-btn-create')}
        saveDisabled={!form.name.trim()}
        cancelLabel={l10n.getString('customer-mgmt-btn-cancel')}
      >
        <div className="customer-mgmt-field">
          <label htmlFor="customer-field-name" className="customer-mgmt-label">
            <Localized id="customer-mgmt-field-name">
              <span>Name *</span>
            </Localized>
          </label>
          <Localized id="customer-mgmt-name-placeholder" attrs={{ placeholder: true }}>
            <input
              className="customer-mgmt-input"
              type="text"
              id="customer-field-name"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              placeholder="e.g. Jane Smith"
              autoComplete="off"
            />
          </Localized>
        </div>

        <div className="customer-mgmt-field">
          <label htmlFor="customer-field-email" className="customer-mgmt-label">
            <Localized id="customer-mgmt-field-email">
              <span>Email</span>
            </Localized>
          </label>
          <Localized id="customer-mgmt-email-placeholder" attrs={{ placeholder: true }}>
            <input
              className="customer-mgmt-input"
              type="email"
              id="customer-field-email"
              value={form.email}
              onChange={(e) => setForm({ ...form, email: e.target.value })}
              placeholder="jane@example.com"
              autoComplete="off"
            />
          </Localized>
        </div>

        <div className="customer-mgmt-field">
          <label htmlFor="customer-field-phone" className="customer-mgmt-label">
            <Localized id="customer-mgmt-field-phone">
              <span>Phone</span>
            </Localized>
          </label>
          <Localized id="customer-mgmt-phone-placeholder" attrs={{ placeholder: true }}>
            <input
              className="customer-mgmt-input"
              type="tel"
              id="customer-field-phone"
              value={form.phone}
              onChange={(e) => setForm({ ...form, phone: e.target.value })}
              placeholder="+1-555-0100"
              autoComplete="off"
            />
          </Localized>
        </div>

        <div className="customer-mgmt-field">
          <label htmlFor="customer-field-notes" className="customer-mgmt-label">
            <Localized id="customer-mgmt-field-notes">
              <span>Notes</span>
            </Localized>
          </label>
          <Localized id="customer-mgmt-notes-placeholder" attrs={{ placeholder: true }}>
            <textarea
              className="customer-mgmt-input customer-mgmt-textarea"
              id="customer-field-notes"
              value={form.notes}
              onChange={(e) => setForm({ ...form, notes: e.target.value })}
              placeholder="Preferences, special notes…"
              rows={3}
            />
          </Localized>
        </div>
      </SettingsPopup>
    </div>
  );
}
