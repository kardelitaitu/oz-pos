import { useState, useCallback, useEffect, useMemo } from 'react';
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
      setError('Customer name is required');
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
      setError(err instanceof Error ? err.message : 'Failed to save customer');
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
        <h1 className="customer-mgmt-title">Customers</h1>
        <Button onClick={openCreate}>Add Customer</Button>
      </div>

      {/* Search */}
      <div className="customer-mgmt-search-wrap">
        <svg className="customer-mgmt-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <input
          type="search"
          className="customer-mgmt-search"
          placeholder="Search by name, email, or phone…"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          aria-label="Search customers"
        />
      </div>

      {/* Content */}
      {loading ? (
        <p className="customer-mgmt-loading">Loading customers…</p>
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
            <p>No customers yet.</p>
            <Button variant="secondary" onClick={openCreate}>
              Add your first customer
            </Button>
          </div>
        </Card>
      ) : filteredCustomers.length === 0 ? (
        <Card shadow="sm">
          <div className="customer-mgmt-empty">
            <p>No customers match your search.</p>
            <Button variant="ghost" onClick={() => setSearchQuery('')}>
              Clear search
            </Button>
          </div>
        </Card>
      ) : (
        <div className="customer-mgmt-table-wrap">
          <table className="customer-mgmt-table" aria-label="Customers">
            <thead>
              <tr>
                <th>Name</th>
                <th>Email</th>
                <th>Phone</th>
                <th>Notes</th>
                <th aria-label="Actions"> </th>
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
                    <button
                      type="button"
                      className="customer-mgmt-action-btn"
                      onClick={() => openEdit(customer)}
                      aria-label={`Edit ${customer.name}`}
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      className="customer-mgmt-action-btn customer-mgmt-action-btn--danger"
                      onClick={() => confirmDelete(customer.id)}
                      disabled={deleting === customer.id}
                      aria-label={`Delete ${customer.name}`}
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* ── Add/Edit Modal ──────────────────────────────────────── */}
      {showModal && (
        <div className="customer-mgmt-overlay" role="dialog" aria-modal="true" aria-label={editingId ? 'Edit customer' : 'Add customer'}>
          <div className="customer-mgmt-modal">
            <div className="customer-mgmt-modal-header">
              <h2>{editingId ? 'Edit Customer' : 'Add Customer'}</h2>
              <button
                type="button"
                className="customer-mgmt-modal-close"
                onClick={closeModal}
                aria-label="Close"
              >
                &times;
              </button>
            </div>

            <div className="customer-mgmt-modal-body">
              <label className="customer-mgmt-field" htmlFor="customer-field-name">
                <span className="customer-mgmt-label">Name *</span>
                <input
                  className="customer-mgmt-input"
                  type="text"
                  id="customer-field-name"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  placeholder="e.g. Jane Smith"
                  autoComplete="name"
                />
              </label>

              <label className="customer-mgmt-field" htmlFor="customer-field-email">
                <span className="customer-mgmt-label">Email</span>
                <input
                  className="customer-mgmt-input"
                  type="email"
                  id="customer-field-email"
                  value={form.email}
                  onChange={(e) => setForm({ ...form, email: e.target.value })}
                  placeholder="jane@example.com"
                  autoComplete="email"
                />
              </label>

              <label className="customer-mgmt-field" htmlFor="customer-field-phone">
                <span className="customer-mgmt-label">Phone</span>
                <input
                  className="customer-mgmt-input"
                  type="tel"
                  id="customer-field-phone"
                  value={form.phone}
                  onChange={(e) => setForm({ ...form, phone: e.target.value })}
                  placeholder="+1-555-0100"
                  autoComplete="tel"
                />
              </label>

              <label className="customer-mgmt-field" htmlFor="customer-field-notes">
                <span className="customer-mgmt-label">Notes</span>
                <textarea
                  className="customer-mgmt-input customer-mgmt-textarea"
                  id="customer-field-notes"
                  value={form.notes}
                  onChange={(e) => setForm({ ...form, notes: e.target.value })}
                  placeholder="Preferences, special notes…"
                  rows={3}
                />
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
              <Button variant="ghost" onClick={closeModal} disabled={saving}>
                Cancel
              </Button>
              <Button
                variant="primary"
                loading={saving}
                disabled={!form.name.trim()}
                onClick={handleSave}
              >
                {editingId ? 'Update' : 'Create'}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
