import { useState, useCallback, useEffect } from 'react';
import { Localized } from '@fluent/react';
import {
  listTaxRates,
  createTaxRate,
  updateTaxRate,
  deleteTaxRate,
  type TaxRateDto,
} from '@/api/pos';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Badge } from '@/components/Badge';
import './TaxConfigurationScreen.css';

interface FormData {
  name: string;
  rateBps: string;
  isDefault: boolean;
}

const EMPTY_FORM: FormData = { name: '', rateBps: '', isDefault: false };

export default function TaxConfigurationScreen() {
  const [rates, setRates] = useState<TaxRateDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<FormData>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const items = await listTaxRates();
      setRates(items);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const openCreate = useCallback(() => {
    setForm(EMPTY_FORM);
    setEditingId(null);
    setShowModal(true);
  }, []);

  const openEdit = useCallback((r: TaxRateDto) => {
    setForm({
      name: r.name,
      rateBps: String(r.rate_bps),
      isDefault: r.is_default,
    });
    setEditingId(r.id);
    setShowModal(true);
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const rateBps = parseInt(form.rateBps, 10);
      if (Number.isNaN(rateBps) || rateBps < 0) return;

      if (editingId) {
        await updateTaxRate({
          id: editingId,
          name: form.name,
          rateBps,
          isDefault: form.isDefault,
        });
      } else {
        await createTaxRate({
          name: form.name,
          rateBps,
          isDefault: form.isDefault,
        });
      }
      setShowModal(false);
      await load();
    } catch {
      // Error handling.
    } finally {
      setSaving(false);
    }
  }, [form, editingId, load]);

  const confirmDelete = useCallback(async (id: string) => {
    setDeleting(id);
    try {
      await deleteTaxRate(id);
      setDeleting(null);
      await load();
    } catch {
      setDeleting(null);
    }
  }, [load]);

  return (
    <div className="tax-config">
      <div className="tax-config-header">
        <Localized id="tax-config-title">
          <h1 className="tax-config-title">Tax Configuration</h1>
        </Localized>
        <Localized id="tax-config-add">
          <Button onClick={openCreate}>Add Tax Rate</Button>
        </Localized>
      </div>

      {loading ? (
        <Localized id="tax-config-loading">
          <p className="tax-config-loading">Loading tax rates&hellip;</p>
        </Localized>
      ) : rates.length === 0 ? (
        <Card shadow="sm">
          <div className="tax-config-empty">
            <Localized id="tax-config-empty">
              <p>No tax rates configured</p>
            </Localized>
            <Localized id="tax-config-add">
              <Button variant="secondary" onClick={openCreate}>Add Tax Rate</Button>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="tax-config-table-wrap">
          <table className="tax-config-table" aria-label="Tax rates">
            <thead>
              <tr>
                <Localized id="tax-config-col-name"><th>Name</th></Localized>
                <Localized id="tax-config-col-rate"><th>Rate (%)</th></Localized>
                <th>Default</th>
                <th aria-label="Actions"> </th>
              </tr>
            </thead>
            <tbody>
              {rates.map((r) => (
                <tr key={r.id}>
                  <td>
                    {r.name}
                    {r.is_default && (
                      <Badge variant="info" size="sm" style={{ marginLeft: 'var(--space-2)' }}>
                        Default
                      </Badge>
                    )}
                  </td>
                  <td>{r.display_rate}</td>
                  <td>{r.is_default ? 'Yes' : '\u2014'}</td>
                  <td className="tax-config-cell-actions">
                    <button
                      type="button"
                      className="tax-config-action-btn"
                      onClick={() => openEdit(r)}
                      aria-label={`Edit ${r.name}`}
                    >
                      <Localized id="tax-config-edit">
                        <span>Edit</span>
                      </Localized>
                    </button>
                    <button
                      type="button"
                      className="tax-config-action-btn tax-config-action-btn--danger"
                      onClick={() => confirmDelete(r.id)}
                      disabled={deleting === r.id}
                      aria-label={`Delete ${r.name}`}
                    >
                      <Localized id="tax-config-btn-delete">
                        <span>Delete</span>
                      </Localized>
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {showModal && (
        <div className="tax-config-overlay" role="dialog" aria-modal="true" aria-label={editingId ? 'Edit tax rate' : 'Add tax rate'}>
          <div className="tax-config-modal">
            <div className="tax-config-modal-header">
              <Localized id="tax-config-modal-title" vars={{ editing: editingId !== null ? 'true' : 'false' }}>
                <h2>{editingId ? 'Edit Tax Rate' : 'Add Tax Rate'}</h2>
              </Localized>
              <button
                type="button"
                className="tax-config-modal-close"
                onClick={() => setShowModal(false)}
                aria-label="Close"
              >
                &times;
              </button>
            </div>

            <div className="tax-config-modal-body">
              <label className="tax-config-field" htmlFor="tax-field-name" aria-label="Tax name">
                <Localized id="tax-config-field-name">
                  <span className="tax-config-label">Tax Name</span>
                </Localized>
                <input
                  className="tax-config-input"
                  type="text"
                  id="tax-field-name"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  placeholder="e.g. Sales Tax"
                />
              </label>

              <label className="tax-config-field" htmlFor="tax-field-rate" aria-label="Rate">
                <Localized id="tax-config-field-rate">
                  <span className="tax-config-label">Rate (%)</span>
                </Localized>
                <input
                  className="tax-config-input"
                  type="number"
                  id="tax-field-rate"
                  min="0"
                  value={form.rateBps}
                  onChange={(e) => setForm({ ...form, rateBps: e.target.value })}
                  placeholder="825"
                />
                <span className="tax-config-hint">Enter rate in basis points (e.g. 825 = 8.25%)</span>
              </label>

              <label className="tax-config-checkbox">
                <input
                  type="checkbox"
                  checked={form.isDefault}
                  onChange={(e) => setForm({ ...form, isDefault: e.target.checked })}
                />
                Set as default tax rate
              </label>
            </div>

            <div className="tax-config-modal-actions">
              <Localized id="tax-config-btn-cancel">
                <Button variant="ghost" onClick={() => setShowModal(false)} disabled={saving}>Cancel</Button>
              </Localized>
              <Button
                variant="primary"
                loading={saving}
                disabled={!form.name.trim() || !form.rateBps.trim()}
                onClick={handleSave}
              >
                <Localized id="tax-config-btn-save">
                  <span>Save</span>
                </Localized>
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
