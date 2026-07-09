import { useState, useCallback, useEffect, useMemo } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listSuppliers,
  createSupplier,
  updateSupplier,
  type SupplierDto,
  type UpdateSupplierArgs,
  type CreateSupplierArgs,
} from '@/api/purchasing';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './SuppliersScreen.css';

interface FormData {
  code: string;
  name: string;
  contact_person: string;
  phone: string;
  email: string;
  address: string;
  tax_id: string;
  payment_terms: string;
  notes: string;
}

const EMPTY_FORM: FormData = {
  code: '',
  name: '',
  contact_person: '',
  phone: '',
  email: '',
  address: '',
  tax_id: '',
  payment_terms: '',
  notes: '',
};

export default function SuppliersScreen() {
  const { l10n } = useLocalization();
  const [suppliers, setSuppliers] = useState<SupplierDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [showModal, setShowModal] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<FormData>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await listSuppliers();
      setSuppliers(data);
    } catch {
      // IPC unavailable
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!searchQuery.trim()) return suppliers;
    const q = searchQuery.trim().toLowerCase();
    return suppliers.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.code.toLowerCase().includes(q) ||
        s.contact_person.toLowerCase().includes(q),
    );
  }, [suppliers, searchQuery]);

  const openCreate = useCallback(() => {
    setForm(EMPTY_FORM);
    setEditingId(null);
    setError(null);
    setShowModal(true);
  }, []);

  const openEdit = useCallback((s: SupplierDto) => {
    setForm({
      code: s.code,
      name: s.name,
      contact_person: s.contact_person,
      phone: s.phone,
      email: s.email,
      address: s.address,
      tax_id: s.tax_id,
      payment_terms: s.payment_terms,
      notes: s.notes,
    });
    setEditingId(s.id);
    setError(null);
    setShowModal(true);
  }, []);

  const closeModal = useCallback(() => {
    setShowModal(false);
    setError(null);
  }, []);

  const handleSave = useCallback(async () => {
    if (!form.name.trim()) {
      setError(l10n.getString('supplier-name-required'));
      return;
    }
    if (!form.code.trim()) {
      setError(l10n.getString('supplier-code-required'));
      return;
    }
    setSaving(true);
    setError(null);
    try {
      if (editingId) {
        const args: UpdateSupplierArgs = {
          id: editingId,
          code: form.code.trim(),
          name: form.name.trim(),
        };
        if (form.contact_person.trim()) args.contact_person = form.contact_person.trim();
        if (form.phone.trim()) args.phone = form.phone.trim();
        if (form.email.trim()) args.email = form.email.trim();
        if (form.address.trim()) args.address = form.address.trim();
        if (form.tax_id.trim()) args.tax_id = form.tax_id.trim();
        if (form.payment_terms.trim()) args.payment_terms = form.payment_terms.trim();
        if (form.notes.trim()) args.notes = form.notes.trim();
        await updateSupplier(args);
      } else {
        const args: CreateSupplierArgs = {
          code: form.code.trim(),
          name: form.name.trim(),
        };
        if (form.contact_person.trim()) args.contact_person = form.contact_person.trim();
        if (form.phone.trim()) args.phone = form.phone.trim();
        if (form.email.trim()) args.email = form.email.trim();
        if (form.address.trim()) args.address = form.address.trim();
        if (form.tax_id.trim()) args.tax_id = form.tax_id.trim();
        if (form.payment_terms.trim()) args.payment_terms = form.payment_terms.trim();
        if (form.notes.trim()) args.notes = form.notes.trim();
        await createSupplier(args);
      }
      closeModal();
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : l10n.getString('supplier-save-failed'));
    } finally {
      setSaving(false);
    }
  }, [form, editingId, closeModal, load, l10n]);

  return (
    <div className="suppliers-screen">
      <div className="suppliers-header">
        <Localized id="suppliers-title">
          <h1 className="suppliers-title">Suppliers</h1>
        </Localized>
        <Localized id="suppliers-add">
          <Button onClick={openCreate}>Add Supplier</Button>
        </Localized>
      </div>

      <div className="suppliers-search-wrap">
        <svg className="suppliers-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <Localized id="suppliers-search" attrs={{ placeholder: true, 'aria-label': true }}>
          <input
            type="search"
            className="suppliers-search"
            placeholder="Search by name, code, or contact…"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            aria-label="Search suppliers"
          />
        </Localized>
      </div>

      {loading ? (
        <Localized id="suppliers-loading">
          <p className="suppliers-loading">Loading suppliers…</p>
        </Localized>
      ) : suppliers.length === 0 ? (
        <Card shadow="sm">
          <div className="suppliers-empty">
            <p>No suppliers yet.</p>
            <Button variant="secondary" onClick={openCreate}>Add your first supplier</Button>
          </div>
        </Card>
      ) : filtered.length === 0 ? (
        <Card shadow="sm">
          <div className="suppliers-empty">
            <p>No suppliers match your search.</p>
            <Button variant="ghost" onClick={() => setSearchQuery('')}>Clear search</Button>
          </div>
        </Card>
      ) : (
        <div className="suppliers-table-wrap">
          <table className="suppliers-table" aria-label="Suppliers">
            <thead>
              <tr>
                <th>Code</th>
                <th>Name</th>
                <th>Contact</th>
                <th>Phone</th>
                <th>Email</th>
                <th>Status</th>
                <th aria-label="Actions"> </th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((s) => (
                <tr key={s.id}>
                  <td className="suppliers-cell-code">{s.code}</td>
                  <td className="suppliers-cell-name">{s.name}</td>
                  <td className="suppliers-cell-contact">{s.contact_person || '\u2014'}</td>
                  <td className="suppliers-cell-phone">{s.phone || '\u2014'}</td>
                  <td className="suppliers-cell-email">{s.email || '\u2014'}</td>
                  <td>
                    <span className={`suppliers-badge suppliers-badge--${s.status}`}>{s.status}</span>
                  </td>
                  <td className="suppliers-cell-actions">
                    <button type="button" className="suppliers-action-btn" onClick={() => openEdit(s)}>
                      Edit
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {showModal && (
        <div className="suppliers-overlay" role="dialog" aria-modal="true" aria-label={editingId ? 'Edit supplier' : 'Add supplier'}>
          <div className="suppliers-modal">
            <div className="suppliers-modal-header">
              <h2>{editingId ? 'Edit Supplier' : 'Add Supplier'}</h2>
              <button type="button" className="suppliers-modal-close" onClick={closeModal} aria-label="Close">&times;</button>
            </div>
            <div className="suppliers-modal-body">
              <label className="suppliers-field">
                <span className="suppliers-label">Code *</span>
                <input className="suppliers-input" type="text" value={form.code} onChange={(e) => setForm({ ...form, code: e.target.value })} />
              </label>
              <label className="suppliers-field">
                <span className="suppliers-label">Name *</span>
                <input className="suppliers-input" type="text" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} />
              </label>
              <div className="suppliers-row">
                <label className="suppliers-field">
                  <span className="suppliers-label">Contact Person</span>
                  <input className="suppliers-input" type="text" value={form.contact_person} onChange={(e) => setForm({ ...form, contact_person: e.target.value })} />
                </label>
                <label className="suppliers-field">
                  <span className="suppliers-label">Phone</span>
                  <input className="suppliers-input" type="tel" value={form.phone} onChange={(e) => setForm({ ...form, phone: e.target.value })} />
                </label>
              </div>
              <label className="suppliers-field">
                <span className="suppliers-label">Email</span>
                <input className="suppliers-input" type="email" value={form.email} onChange={(e) => setForm({ ...form, email: e.target.value })} />
              </label>
              <label className="suppliers-field">
                <span className="suppliers-label">Address</span>
                <input className="suppliers-input" type="text" value={form.address} onChange={(e) => setForm({ ...form, address: e.target.value })} />
              </label>
              <div className="suppliers-row">
                <label className="suppliers-field">
                  <span className="suppliers-label">Tax ID</span>
                  <input className="suppliers-input" type="text" value={form.tax_id} onChange={(e) => setForm({ ...form, tax_id: e.target.value })} />
                </label>
                <label className="suppliers-field">
                  <span className="suppliers-label">Payment Terms</span>
                  <input className="suppliers-input" type="text" value={form.payment_terms} onChange={(e) => setForm({ ...form, payment_terms: e.target.value })} />
                </label>
              </div>
              <label className="suppliers-field">
                <span className="suppliers-label">Notes</span>
                <textarea className="suppliers-input suppliers-textarea" value={form.notes} onChange={(e) => setForm({ ...form, notes: e.target.value })} rows={3} />
              </label>
              {error && <div className="suppliers-error" role="alert">{error}</div>}
            </div>
            <div className="suppliers-modal-actions">
              <Button variant="ghost" onClick={closeModal} disabled={saving}>Cancel</Button>
              <Button variant="primary" loading={saving} disabled={!form.name.trim() || !form.code.trim()} onClick={handleSave}>
                {editingId ? 'Update' : 'Create'}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
