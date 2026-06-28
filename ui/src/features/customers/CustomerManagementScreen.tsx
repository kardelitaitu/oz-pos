import { useState, useCallback, useEffect, useMemo } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listCustomers,
  createCustomer,
  updateCustomer,
  deleteCustomer,
  type CustomerDto,
} from '@/api/pos';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
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

export default function CustomerManagementScreen() {
  const { l10n } = useLocalization();
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
        const args: import('@/api/pos').UpdateCustomerArgs = { id: editingId, name };
        if (form.email.trim()) args.email = form.email.trim();
        if (form.phone.trim()) args.phone = form.phone.trim();
        if (form.notes.trim()) args.notes = form.notes.trim();
        await updateCustomer(args);
      } else {
        const args: import('@/api/pos').CreateCustomerArgs = { name };
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
  }, [form, editingId, closeModal, load]);

  // ── Delete ─────────────────────────────────────────────────────

  const confirmDelete = useCallback(async (id: string) => {
    setDeleting(id);
    try {
      await deleteCustomer(id);
      setDeleting(null);
      await load();
    } catch {
      setDeleting(null);
    }
  }, [load]);

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
            placeholder="Search by name, email, or phone…"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            aria-label="Search customers"
          />
        </Localized>
      </div>

      {/* Content */}
      {loading ? (
        <Localized id="customer-mgmt-loading">
          <p className="customer-mgmt-loading">Loading customers…</p>
        </Localized>
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
          <table className="customer-mgmt-table" aria-label="Customers">
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

      {/* ── Add/Edit Modal ──────────────────────────────────────── */}
      {showModal && (
        <Localized id={editingId ? 'customer-mgmt-modal-edit-aria' : 'customer-mgmt-modal-add-aria'} attrs={{ 'aria-label': true }}>
          <div className="customer-mgmt-overlay" role="dialog" aria-modal="true" aria-label={editingId ? 'Edit customer' : 'Add customer'}>
          <div className="customer-mgmt-modal">
            <div className="customer-mgmt-modal-header">
              <Localized id={editingId ? 'customer-mgmt-modal-edit-title' : 'customer-mgmt-modal-add-title'}>
                <h2>{editingId ? 'Edit Customer' : 'Add Customer'}</h2>
              </Localized>
              <Localized id="customer-mgmt-modal-close" attrs={{ 'aria-label': true }}>
                <button
                  type="button"
                  className="customer-mgmt-modal-close"
                  onClick={closeModal}
                  aria-label="Close"
                >
                  &times;
                </button>
              </Localized>
            </div>

            <div className="customer-mgmt-modal-body">
              <label className="customer-mgmt-field" htmlFor="customer-field-name">
                <Localized id="customer-mgmt-field-name">
                  <span className="customer-mgmt-label">Name *</span>
                </Localized>
                <Localized id="customer-mgmt-name-placeholder" attrs={{ placeholder: true }}>
                  <input
                    className="customer-mgmt-input"
                    type="text"
                    id="customer-field-name"
                    value={form.name}
                    onChange={(e) => setForm({ ...form, name: e.target.value })}
                    placeholder="e.g. Jane Smith"
                    autoComplete="name"
                  />
                </Localized>
              </label>

              <label className="customer-mgmt-field" htmlFor="customer-field-email">
                <Localized id="customer-mgmt-field-email">
                  <span className="customer-mgmt-label">Email</span>
                </Localized>
                <Localized id="customer-mgmt-email-placeholder" attrs={{ placeholder: true }}>
                  <input
                    className="customer-mgmt-input"
                    type="email"
                    id="customer-field-email"
                    value={form.email}
                    onChange={(e) => setForm({ ...form, email: e.target.value })}
                    placeholder="jane@example.com"
                    autoComplete="email"
                  />
                </Localized>
              </label>

              <label className="customer-mgmt-field" htmlFor="customer-field-phone">
                <Localized id="customer-mgmt-field-phone">
                  <span className="customer-mgmt-label">Phone</span>
                </Localized>
                <Localized id="customer-mgmt-phone-placeholder" attrs={{ placeholder: true }}>
                  <input
                    className="customer-mgmt-input"
                    type="tel"
                    id="customer-field-phone"
                    value={form.phone}
                    onChange={(e) => setForm({ ...form, phone: e.target.value })}
                    placeholder="+1-555-0100"
                    autoComplete="tel"
                  />
                </Localized>
              </label>

              <label className="customer-mgmt-field" htmlFor="customer-field-notes">
                <Localized id="customer-mgmt-field-notes">
                  <span className="customer-mgmt-label">Notes</span>
                </Localized>
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
              </label>

              {error && (
                <div className="customer-mgmt-error" role="alert">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16" aria-hidden="true">
                    <circle cx="12" cy="12" r="10" />
                    <line x1="15" y1="9" x2="9" y2="15" />
                    <line x1="9" y1="9" x2="15" y2="15" />
                  </svg>
                  {error}
                </div>
              )}
            </div>

            <div className="customer-mgmt-modal-actions">
              <Localized id="customer-mgmt-btn-cancel">
                <Button variant="ghost" onClick={closeModal} disabled={saving}>
                  Cancel
                </Button>
              </Localized>
              <Button
                variant="primary"
                loading={saving}
                disabled={!form.name.trim()}
                onClick={handleSave}
              >
                <Localized id={editingId ? 'customer-mgmt-btn-update' : 'customer-mgmt-btn-create'}>
                  <span>{editingId ? 'Update' : 'Create'}</span>
                </Localized>
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
