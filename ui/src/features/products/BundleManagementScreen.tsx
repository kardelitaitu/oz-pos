import { useState, useCallback, useEffect } from 'react';
import { Localized } from '@fluent/react';
import {
  listBundles,
  createBundle,
  updateBundle,
  deleteBundle,
  type BundleWithItems,
  type CreateBundleArgs,
} from '@/api/bundles';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './BundleManagementScreen.css';

interface BundleItemForm {
  sku: string;
  qty: string;
  unitPriceMinor: string;
}

interface FormData {
  bundle_sku: string;
  name: string;
  description: string;
  bundle_price_minor: string;
  items: BundleItemForm[];
}

const EMPTY_FORM: FormData = {
  bundle_sku: '',
  name: '',
  description: '',
  bundle_price_minor: '',
  items: [{ sku: '', qty: '1', unitPriceMinor: '' }],
};

const EMPTY_ITEM: BundleItemForm = { sku: '', qty: '1', unitPriceMinor: '' };

export default function BundleManagementScreen() {
  const [bundles, setBundles] = useState<BundleWithItems[]>([]);
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<FormData>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const result = await listBundles();
      setBundles(result);
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

  const openEdit = useCallback((b: BundleWithItems) => {
    setForm({
      bundle_sku: b.bundle.bundle_sku,
      name: b.bundle.name,
      description: b.bundle.description,
      bundle_price_minor: b.bundle.bundle_price_minor != null ? String(b.bundle.bundle_price_minor) : '',
      items: b.items.map((i) => ({
        sku: i.sku,
        qty: String(i.qty),
        unitPriceMinor: i.unit_price_minor != null ? String(i.unit_price_minor) : '',
      })),
    });
    setEditingId(b.bundle.id);
    setShowModal(true);
  }, []);

  const addItemRow = useCallback(() => {
    setForm((prev) => ({ ...prev, items: [...prev.items, { ...EMPTY_ITEM }] }));
  }, []);

  const removeItemRow = useCallback((idx: number) => {
    setForm((prev) => ({
      ...prev,
      items: prev.items.filter((_, i) => i !== idx),
    }));
  }, []);

  const updateItem = useCallback((idx: number, field: keyof BundleItemForm, value: string) => {
    setForm((prev) => {
      const items = [...prev.items];
      items[idx] = { ...items[idx], [field]: value } as BundleItemForm;
      return { ...prev, items };
    });
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const priceMinor = form.bundle_price_minor
        ? parseInt(form.bundle_price_minor, 10)
        : null;
      if (priceMinor !== null && (Number.isNaN(priceMinor) || priceMinor < 0)) {
        setSaving(false);
        return;
      }

      const items = form.items
        .filter((i) => i.sku.trim().length > 0)
        .map((i) => ({
          sku: i.sku.trim(),
          qty: parseInt(i.qty, 10) || 1,
          unit_price_minor: i.unitPriceMinor ? parseInt(i.unitPriceMinor, 10) || null : null,
        }));

      if (editingId) {
        const existing = bundles.find((b) => b.bundle.id === editingId);
        if (!existing) return;
        await updateBundle({
          bundle: {
            ...existing.bundle,
            bundle_sku: form.bundle_sku,
            name: form.name,
            description: form.description,
            bundle_price_minor: priceMinor,
          },
          items: items.map((i, idx) => ({
            id: existing.items[idx]?.id ?? crypto.randomUUID(),
            bundle_id: editingId,
            sku: i.sku,
            qty: i.qty,
            unit_price_minor: i.unit_price_minor,
          })),
        });
      } else {
        await createBundle({
          bundle_sku: form.bundle_sku,
          name: form.name,
          description: form.description || undefined,
          bundle_price_minor: priceMinor,
          items,
        } as CreateBundleArgs);
      }
      setShowModal(false);
      await load();
    } catch {
      // Error handling.
    } finally {
      setSaving(false);
    }
  }, [form, editingId, bundles, load]);

  const confirmDelete = useCallback(async (id: string) => {
    setDeleting(id);
    try {
      await deleteBundle(id);
      setDeleting(null);
      await load();
    } catch {
      setDeleting(null);
    }
  }, [load]);

  const toggleActive = useCallback(async (b: BundleWithItems) => {
    await updateBundle({
      bundle: { ...b.bundle, active: !b.bundle.active },
      items: b.items,
    });
    await load();
  }, [load]);

  return (
    <div className="bundle-mgmt">
      <div className="bundle-mgmt-header">
        <Localized id="bundles-title">
          <h1 className="bundle-mgmt-title">Product Bundles</h1>
        </Localized>
        <Localized id="bundles-add">
          <Button onClick={openCreate}>Add Bundle</Button>
        </Localized>
      </div>

      {loading ? (
        <Localized id="bundles-loading">
          <p className="bundle-mgmt-loading">Loading bundles...</p>
        </Localized>
      ) : bundles.length === 0 ? (
        <Card shadow="sm">
          <div className="bundle-mgmt-empty">
            <Localized id="bundles-no-bundles">
              <p>No bundles yet.</p>
            </Localized>
            <Localized id="bundles-add">
              <Button variant="secondary" onClick={openCreate}>Add your first bundle</Button>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="bundle-mgmt-table-wrap">
          <table className="bundle-mgmt-table" aria-label="Product bundles">
            <thead>
              <tr>
                <Localized id="bundles-name"><th>Name</th></Localized>
                <Localized id="bundles-sku"><th>SKU</th></Localized>
                <Localized id="bundles-price"><th>Price</th></Localized>
                <Localized id="bundles-items"><th>Items</th></Localized>
                <Localized id="bundles-active"><th>Active</th></Localized>
                <th aria-label="Actions"> </th>
              </tr>
            </thead>
            <tbody>
              {bundles.map((b) => (
                <tr key={b.bundle.id}>
                  <td>{b.bundle.name}</td>
                  <td className="bundle-mgmt-cell-sku">{b.bundle.bundle_sku}</td>
                  <td className="bundle-mgmt-cell-price">
                    {b.bundle.bundle_price_minor != null
                      ? (b.bundle.bundle_price_minor / 100).toFixed(2)
                      : '\u2014'}
                  </td>
                  <td>{b.items.length}</td>
                  <td>
                    <button
                      type="button"
                      className={`bundle-mgmt-toggle ${b.bundle.active ? 'bundle-mgmt-toggle--on' : 'bundle-mgmt-toggle--off'}`}
                      onClick={() => toggleActive(b)}
                      aria-label={b.bundle.active ? 'Deactivate bundle' : 'Activate bundle'}
                    >
                      {b.bundle.active ? 'Active' : 'Inactive'}
                    </button>
                  </td>
                  <td className="bundle-mgmt-cell-actions">
                    <Localized id="bundles-edit">
                      <button
                        type="button"
                        className="bundle-mgmt-action-btn"
                        onClick={() => openEdit(b)}
                        aria-label={`Edit ${b.bundle.name}`}
                      >
                        Edit
                      </button>
                    </Localized>
                    <Localized id="bundles-delete">
                      <button
                        type="button"
                        className="bundle-mgmt-action-btn bundle-mgmt-action-btn--danger"
                        onClick={() => confirmDelete(b.bundle.id)}
                        disabled={deleting === b.bundle.id}
                        aria-label={`Delete ${b.bundle.name}`}
                      >
                        Delete
                      </button>
                    </Localized>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {showModal && (
        <div className="bundle-mgmt-overlay" role="dialog" aria-modal="true" aria-label={editingId ? 'Edit bundle' : 'Add bundle'}>
          <div className="bundle-mgmt-modal">
            <div className="bundle-mgmt-modal-header">
              <Localized id={editingId ? 'bundles-edit' : 'bundles-add'}>
                <h2>{editingId ? 'Edit Bundle' : 'Add Bundle'}</h2>
              </Localized>
              <button
                type="button"
                className="bundle-mgmt-modal-close"
                onClick={() => setShowModal(false)}
                aria-label="Close"
              >
                &times;
              </button>
            </div>

            <div className="bundle-mgmt-modal-body">
              <label className="bundle-mgmt-field" htmlFor="bundle-field-sku" aria-label="Bundle SKU *">
                <Localized id="bundles-sku">
                  <span className="bundle-mgmt-label">Bundle SKU *</span>
                </Localized>
                <input
                  className="bundle-mgmt-input"
                  type="text"
                  id="bundle-field-sku"
                  value={form.bundle_sku}
                  onChange={(e) => setForm({ ...form, bundle_sku: e.target.value })}
                  disabled={!!editingId}
                  placeholder="e.g. GIFT-BOX"
                />
              </label>

              <label className="bundle-mgmt-field" htmlFor="bundle-field-name" aria-label="Name *">
                <Localized id="bundles-name">
                  <span className="bundle-mgmt-label">Name *</span>
                </Localized>
                <input
                  className="bundle-mgmt-input"
                  type="text"
                  id="bundle-field-name"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  placeholder="e.g. Gift Box"
                />
              </label>

              <label className="bundle-mgmt-field" htmlFor="bundle-field-description" aria-label="Description">
                <Localized id="bundles-description">
                  <span className="bundle-mgmt-label">Description</span>
                </Localized>
                <input
                  className="bundle-mgmt-input"
                  type="text"
                  id="bundle-field-description"
                  value={form.description}
                  onChange={(e) => setForm({ ...form, description: e.target.value })}
                  placeholder="Optional description"
                />
              </label>

              <label className="bundle-mgmt-field" htmlFor="bundle-field-price" aria-label="Bundle price (minor units)">
                <Localized id="bundles-price">
                  <span className="bundle-mgmt-label">Bundle price (minor units)</span>
                </Localized>
                <input
                  className="bundle-mgmt-input"
                  type="number"
                  id="bundle-field-price"
                  min="0"
                  value={form.bundle_price_minor}
                  onChange={(e) => setForm({ ...form, bundle_price_minor: e.target.value })}
                  placeholder="Leave empty to use sum of items"
                />
              </label>

              <fieldset className="bundle-mgmt-field">
                <Localized id="bundles-items">
                  <legend className="bundle-mgmt-label">Items</legend>
                </Localized>
                <div className="bundle-mgmt-items-list">
                  {form.items.map((item, idx) => (
                    <div key={idx} className="bundle-mgmt-item-row">
                      <input
                        className="bundle-mgmt-input bundle-mgmt-item-sku"
                        type="text"
                        value={item.sku}
                        onChange={(e) => updateItem(idx, 'sku', e.target.value)}
                        placeholder="SKU"
                        aria-label={`Item ${idx + 1} SKU`}
                      />
                      <input
                        className="bundle-mgmt-input bundle-mgmt-item-qty"
                        type="number"
                        min="1"
                        value={item.qty}
                        onChange={(e) => updateItem(idx, 'qty', e.target.value)}
                        placeholder="Qty"
                        aria-label={`Item ${idx + 1} quantity`}
                      />
                      <input
                        className="bundle-mgmt-input bundle-mgmt-item-price"
                        type="number"
                        min="0"
                        value={item.unitPriceMinor}
                        onChange={(e) => updateItem(idx, 'unitPriceMinor', e.target.value)}
                        placeholder="Price override"
                        aria-label={`Item ${idx + 1} unit price override`}
                      />
                      {form.items.length > 1 && (
                        <button
                          type="button"
                          className="bundle-mgmt-item-remove"
                          onClick={() => removeItemRow(idx)}
                          aria-label={`Remove item ${idx + 1}`}
                        >
                          &times;
                        </button>
                      )}
                    </div>
                  ))}
                </div>
                <Localized id="bundles-add-item">
                  <Button variant="ghost" size="sm" onClick={addItemRow}>+ Add Item</Button>
                </Localized>
              </fieldset>
            </div>

            <div className="bundle-mgmt-modal-actions">
              <Localized id="bundles-cancel">
                <Button variant="ghost" onClick={() => setShowModal(false)} disabled={saving}>Cancel</Button>
              </Localized>
              <Button
                variant="primary"
                loading={saving}
                disabled={!form.bundle_sku.trim() || !form.name.trim() || form.items.every((i) => !i.sku.trim())}
                onClick={handleSave}
              >
                <Localized id={editingId ? 'bundles-save' : 'bundles-create'}>
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
