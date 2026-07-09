import { useState, useCallback, useEffect } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listPromotions,
  createPromotion,
  updatePromotion,
  deletePromotion,
  type Promotion,
  type CreatePromotionArgs,
} from '@/api/promotions';
import { useAuth } from '@/contexts/AuthContext';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './PromotionManagementScreen.css';

type ModalMode = 'add' | 'edit' | null;

const PROMO_TYPES = ['percentage', 'fixed_amount', 'buy_x_get_y'] as const;

const PROMO_TYPE_LABELS: Record<string, string> = {
  percentage: 'promotions-percentage',
  fixed_amount: 'promotions-fixed-amount',
  buy_x_get_y: 'promotions-buy-x-get-y',
};

const emptyForm = (): Promotion => ({
  id: '',
  name: '',
  description: '',
  promo_type: 'percentage',
  value_minor: 0,
  min_qty: null,
  trigger_sku: null,
  reward_sku: null,
  reward_qty: null,
  starts_at: null,
  ends_at: null,
  min_order_minor: 0,
  category_id: null,
  active: true,
  created_at: '',
  updated_at: '',
});

export default function PromotionManagementScreen() {
  const { l10n } = useLocalization();
  const { session } = useAuth();
  const [promotions, setPromotions] = useState<Promotion[]>([]);
  const [loading, setLoading] = useState(true);
  const [modalMode, setModalMode] = useState<ModalMode>(null);
  const [form, setForm] = useState<Promotion>(emptyForm());
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; name: string } | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const items = await listPromotions();
      setPromotions(items);
    } catch {
      // IPC unavailable
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const openAdd = useCallback(() => {
    setForm(emptyForm());
    setModalMode('add');
  }, []);

  const openEdit = useCallback((p: Promotion) => {
    setForm({ ...p });
    setModalMode('edit');
  }, []);

  const closeModal = useCallback(() => {
    setModalMode(null);
  }, []);

  const handleSave = useCallback(async () => {
    if (!form.name.trim()) return;
    setSaving(true);
    try {
      if (modalMode === 'add') {
        const args: CreatePromotionArgs = {
          name: form.name,
          description: form.description,
          promo_type: form.promo_type,
          value_minor: form.value_minor,
          min_qty: form.min_qty,
          trigger_sku: form.trigger_sku,
          reward_sku: form.reward_sku,
          reward_qty: form.reward_qty,
          starts_at: form.starts_at,
          ends_at: form.ends_at,
          min_order_minor: form.min_order_minor,
          category_id: form.category_id,
        };
        await createPromotion(session?.user_id ?? '', args);
      } else {
        await updatePromotion(session?.user_id ?? '', form);
      }
      closeModal();
      await load();
    } catch (err) {
      console.error('Failed to save promotion:', err);
    } finally {
      setSaving(false);
    }
  }, [form, modalMode, load, closeModal, session?.user_id]);

  const confirmDelete = useCallback(async () => {
    if (!deleteTarget) return;
    setDeleting(deleteTarget.id);
    setDeleteTarget(null);
    try {
      await deletePromotion(session?.user_id ?? '', deleteTarget.id);
      await load();
    } catch (err) {
      console.error('Failed to delete promotion:', err);
    } finally {
      setDeleting(null);
    }
  }, [deleteTarget, load, session?.user_id]);

  const toggleActive = useCallback(async (p: Promotion) => {
    try {
      await updatePromotion(session?.user_id ?? '', { ...p, active: !p.active });
      await load();
    } catch (err) {
      console.error('Failed to toggle promotion:', err);
    }
  }, [load, session?.user_id]);

  return (
    <div className="promo-mgmt">
      <div className="promo-mgmt-header">
        <Localized id="promotions-title">
          <h1 className="promo-mgmt-title">Promotions</h1>
        </Localized>
        <Localized id="promotions-add">
          <Button onClick={openAdd}>Add Promotion</Button>
        </Localized>
      </div>

      {loading ? (
        <p className="promo-mgmt-loading"><Localized id="loading"><span>Loading…</span></Localized></p>
      ) : promotions.length === 0 ? (
        <Card shadow="sm">
          <div className="promo-mgmt-empty">
            <Localized id="promotions-no-promotions">
              <p>No promotions yet.</p>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="promo-mgmt-table-wrap">
          <table className="promo-mgmt-table" role="grid" aria-label={l10n.getString('promotions-table-label')}>
            <thead>
              <tr>
                <Localized id="promotions-name"><th>Name</th></Localized>
                <Localized id="promotions-type"><th>Type</th></Localized>
                <Localized id="promotions-value"><th>Value</th></Localized>
                <Localized id="promotions-active"><th>Active</th></Localized>
                <Localized id="promotions-starts-at"><th>Starts</th></Localized>
                <Localized id="promotions-ends-at"><th>Ends</th></Localized>
                <th aria-label={l10n.getString('promotions-table-actions')}> </th>
              </tr>
            </thead>
            <tbody>
              {promotions.map((p) => (
                <tr key={p.id}>
                  <td>{p.name}</td>
                  <td>
                    <Localized id={PROMO_TYPE_LABELS[p.promo_type]!}>
                      <span>{p.promo_type}</span>
                    </Localized>
                  </td>
                  <td>{p.promo_type === 'percentage' ? `${p.value_minor}%` : p.value_minor}</td>
                  <td>
                    <label className="promo-mgmt-toggle" aria-label={l10n.getString('promotions-toggle-active', { name: p.name })}>
                      <input
                        type="checkbox"
                        checked={p.active}
                        onChange={() => toggleActive(p)}
                      />
                      <span className="promo-mgmt-toggle-slider" />
                    </label>
                  </td>
                  <td>{p.starts_at ? new Date(p.starts_at).toLocaleDateString() : '—'}</td>
                  <td>{p.ends_at ? new Date(p.ends_at).toLocaleDateString() : '—'}</td>
                  <td className="promo-mgmt-actions">
                    <Localized id="promotions-edit">
                      <button type="button" className="promo-mgmt-btn" onClick={() => openEdit(p)} aria-label={l10n.getString('promotions-edit-label', { name: p.name })}>Edit</button>
                    </Localized>
                    <Localized id="promotions-delete">
                      <button type="button" className="promo-mgmt-btn promo-mgmt-btn--danger" onClick={() => setDeleteTarget({ id: p.id, name: p.name })} aria-label={l10n.getString('promotions-delete-label', { name: p.name })}>Delete</button>
                    </Localized>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* ── Delete confirmation modal ── */}
      {deleteTarget && (
        <div className="promo-mgmt-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('promotions-modal-delete-label')}>
          <div className="promo-mgmt-modal">
            <div className="promo-mgmt-modal-header">
              <Localized id="promotions-delete-confirm-title">
                <h2 className="promo-mgmt-modal-title">Delete Promotion</h2>
              </Localized>
              <button type="button" className="promo-mgmt-modal-close" onClick={() => setDeleteTarget(null)} aria-label={l10n.getString('close')}>&times;</button>
            </div>
            <div className="promo-mgmt-modal-body">
              <Localized id="promotions-delete-confirm" vars={{ name: deleteTarget.name }}>
                <p>Are you sure you want to delete &quot;{deleteTarget.name}&quot;?</p>
              </Localized>
            </div>
            <div className="promo-mgmt-modal-actions">
              <Localized id="cancel">
                <Button variant="ghost" onClick={() => setDeleteTarget(null)} disabled={deleting !== null}>Cancel</Button>
              </Localized>
              <Localized id="delete">
                <Button variant="danger" loading={deleting !== null} onClick={confirmDelete}>Delete</Button>
              </Localized>
            </div>
          </div>
        </div>
      )}

      {/* ── Add / Edit modal ── */}
      {modalMode && (
        <div className="promo-mgmt-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString(modalMode === 'add' ? 'promotions-modal-add-label' : 'promotions-modal-edit-label')}>
          <div className="promo-mgmt-modal promo-mgmt-modal--wide">
            <div className="promo-mgmt-modal-header">
              <Localized id={modalMode === 'add' ? 'promotions-add' : 'promotions-edit'}>
                <h2 className="promo-mgmt-modal-title">{modalMode === 'add' ? 'Add Promotion' : 'Edit Promotion'}</h2>
              </Localized>
              <button type="button" className="promo-mgmt-modal-close" onClick={closeModal} aria-label={l10n.getString('close')}>&times;</button>
            </div>

            <div className="promo-mgmt-modal-body">
              <div className="promo-mgmt-form">
                <label className="promo-mgmt-field">
                  <Localized id="promotions-name"><span>Name</span></Localized>
                  <input type="text" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} required aria-label={l10n.getString('promotions-field-name')} />
                </label>

                <label className="promo-mgmt-field">
                  <Localized id="promotions-type"><span>Type</span></Localized>
                  <select value={form.promo_type} onChange={(e) => setForm({ ...form, promo_type: e.target.value })} aria-label={l10n.getString('promotions-field-type')}>
                    {PROMO_TYPES.map((t) => (
                      <option key={t} value={t}>
                        <Localized id={PROMO_TYPE_LABELS[t]!}><span>{t}</span></Localized>
                      </option>
                    ))}
                  </select>
                </label>

                <label className="promo-mgmt-field">
                  <Localized id="promotions-value"><span>Value</span></Localized>
                  <input type="number" value={form.value_minor} onChange={(e) => setForm({ ...form, value_minor: parseInt(e.target.value) || 0 })} aria-label={l10n.getString('promotions-field-value')} />
                </label>

                {form.promo_type === 'buy_x_get_y' && (
                  <>
                    <label className="promo-mgmt-field">
                      <Localized id="promotions-min-qty"><span>Min Qty</span></Localized>
                      <input type="number" value={form.min_qty ?? ''} onChange={(e) => setForm({ ...form, min_qty: e.target.value ? parseInt(e.target.value) : null })} aria-label={l10n.getString('promotions-field-min-qty')} />
                    </label>
                    <label className="promo-mgmt-field">
                      <Localized id="promotions-trigger-sku"><span>Trigger SKU</span></Localized>
                      <input type="text" value={form.trigger_sku ?? ''} onChange={(e) => setForm({ ...form, trigger_sku: e.target.value || null })} aria-label={l10n.getString('promotions-field-trigger-sku')} />
                    </label>
                    <label className="promo-mgmt-field">
                      <Localized id="promotions-reward-sku"><span>Reward SKU</span></Localized>
                      <input type="text" value={form.reward_sku ?? ''} onChange={(e) => setForm({ ...form, reward_sku: e.target.value || null })} aria-label={l10n.getString('promotions-field-reward-sku')} />
                    </label>
                    <label className="promo-mgmt-field">
                      <Localized id="promotions-reward-qty"><span>Reward Qty</span></Localized>
                      <input type="number" value={form.reward_qty ?? ''} onChange={(e) => setForm({ ...form, reward_qty: e.target.value ? parseInt(e.target.value) : null })} aria-label={l10n.getString('promotions-field-reward-qty')} />
                    </label>
                  </>
                )}

                <label className="promo-mgmt-field">
                  <Localized id="promotions-field-starts-at"><span>Starts At</span></Localized>
                  <input type="datetime-local" value={form.starts_at ? form.starts_at.substring(0, 16) : ''} onChange={(e) => setForm({ ...form, starts_at: e.target.value ? new Date(e.target.value).toISOString() : null })} aria-label={l10n.getString('promotions-field-starts-at')} />
                </label>
                <label className="promo-mgmt-field">
                  <Localized id="promotions-field-ends-at"><span>Ends At</span></Localized>
                  <input type="datetime-local" value={form.ends_at ? form.ends_at.substring(0, 16) : ''} onChange={(e) => setForm({ ...form, ends_at: e.target.value ? new Date(e.target.value).toISOString() : null })} aria-label={l10n.getString('promotions-field-ends-at')} />
                </label>

                <label className="promo-mgmt-field">
                  <Localized id="promotions-min-order"><span>Min Order</span></Localized>
                  <input type="number" value={form.min_order_minor} onChange={(e) => setForm({ ...form, min_order_minor: parseInt(e.target.value) || 0 })} aria-label={l10n.getString('promotions-field-min-order')} />
                </label>

                <label className="promo-mgmt-field">
                  <Localized id="promotions-category"><span>Category</span></Localized>
                  <input type="text" value={form.category_id ?? ''} onChange={(e) => setForm({ ...form, category_id: e.target.value || null })} aria-label={l10n.getString('promotions-field-category')} />
                </label>
              </div>
            </div>

            <div className="promo-mgmt-modal-actions">
              <Localized id="cancel">
                <Button variant="ghost" onClick={closeModal} disabled={saving}>Cancel</Button>
              </Localized>
              <Localized id="save">
                <Button variant="primary" loading={saving} disabled={!form.name.trim()} onClick={handleSave}>Save</Button>
              </Localized>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
