import { useState, useCallback, useEffect } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listProductVariants,
  createProductVariant,
  updateProductVariant,
  deleteProductVariant,
  type ProductVariantDto,
} from '@/api/products';
import { Button } from '@/components/Button';

interface Props {
  productSku: string;
  productName: string;
  onClose: () => void;
}

interface VariantForm {
  name: string;
  sku: string;
  priceMinor: string;
  currency: string;
  barcode: string;
  sortOrder: string;
  isActive: boolean;
}

const EMPTY_FORM: VariantForm = {
  name: '',
  sku: '',
  priceMinor: '',
  currency: '',
  barcode: '',
  sortOrder: '0',
  isActive: true,
};

/** Variant management screen — manage product variants (size, colour, etc.) with separate SKU, pricing, and barcode. */
export default function VariantManagementScreen({ productSku, productName, onClose }: Props) {
  const [variants, setVariants] = useState<ProductVariantDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [showModal, setShowModal] = useState(false);
  const [editingSku, setEditingSku] = useState<string | null>(null);
  const [form, setForm] = useState<VariantForm>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [deletingSku, setDeletingSku] = useState<string | null>(null);
  const [confirmDeleteSku, setConfirmDeleteSku] = useState<string | null>(null);

  const { l10n } = useLocalization();

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const dtos = await listProductVariants(productSku);
      setVariants(dtos);
    } catch {
      setLoadError('variant-mgmt-error-load');
    } finally {
      setLoading(false);
    }
  }, [productSku]);

  useEffect(() => { load(); }, [load]);

  const openCreate = useCallback(() => {
    setForm(EMPTY_FORM);
    setEditingSku(null);
    setShowModal(true);
  }, []);

  const openEdit = useCallback((v: ProductVariantDto) => {
    setForm({
      name: v.name,
      sku: v.sku,
      priceMinor: v.price != null ? String(v.price.minor_units) : '',
      currency: v.price != null ? v.price.currency : '',
      barcode: v.barcode ?? '',
      sortOrder: String(v.sort_order),
      isActive: v.is_active,
    });
    setEditingSku(v.sku);
    setShowModal(true);
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const hasPrice = form.priceMinor.trim() !== '' && form.currency.trim() !== '';
      const priceMinor = hasPrice ? parseInt(form.priceMinor, 10) : null;
      if (hasPrice && (Number.isNaN(priceMinor) || priceMinor! < 0)) {
        setSaving(false);
        return;
      }

      if (editingSku) {
        await updateProductVariant({
          sku: editingSku,
          name: form.name,
          priceMinor: hasPrice ? priceMinor : null,
          currency: hasPrice ? form.currency : null,
          barcode: form.barcode || null,
          sortOrder: parseInt(form.sortOrder, 10) || 0,
          isActive: form.isActive,
        });
      } else {
        await createProductVariant({
          parentSku: productSku,
          name: form.name,
          sku: form.sku,
          priceMinor: hasPrice ? priceMinor : null,
          currency: hasPrice ? form.currency : null,
          barcode: form.barcode || null,
          sortOrder: parseInt(form.sortOrder, 10) || 0,
        });
      }
      setShowModal(false);
      await load();
    } catch {
      // Error handling.
    } finally {
      setSaving(false);
    }
  }, [form, editingSku, productSku, load]);

  const handleDelete = useCallback(async () => {
    if (!confirmDeleteSku) return;
    setDeletingSku(confirmDeleteSku);
    try {
      await deleteProductVariant(confirmDeleteSku);
      setConfirmDeleteSku(null);
      await load();
    } catch {
      // Error handling.
    } finally {
      setDeletingSku(null);
    }
  }, [confirmDeleteSku, load]);

  return (
    <div className="product-mgmt-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('variant-mgmt-overlay-aria', { name: productName })}>
      <div className="product-mgmt-modal" style={{ width: '640px' }}>
        <div className="product-mgmt-modal-header">
          <Localized id="variant-mgmt-title" vars={{ product: productName }}>
            <h2>Variants — {productName}</h2>
          </Localized>
          <Localized id="variant-mgmt-close-aria" attrs={{ 'aria-label': true }}>
            <button
              type="button"
              className="product-mgmt-modal-close"
              onClick={onClose}
              aria-label="Close"
            >
              &times;
            </button>
          </Localized>
        </div>

        <div className="product-mgmt-modal-body">
          <div style={{ display: 'flex', justifyContent: 'flex-end', marginBottom: 'var(--space-3)' }}>
            <Localized id="variant-mgmt-add">
              <Button onClick={openCreate}>Add Variant</Button>
            </Localized>
          </div>

          {loadError ? (
            <div className="product-mgmt-empty">
              <Localized id={loadError}>
                <p>Failed to load variants</p>
              </Localized>
              <Button variant="secondary" onClick={load}>Retry</Button>
            </div>
          ) : loading ? (
            <Localized id="variant-mgmt-loading">
              <p className="product-mgmt-loading">Loading variants…</p>
            </Localized>
          ) : variants.length === 0 ? (
            <div className="product-mgmt-empty">
              <Localized id="variant-mgmt-empty">
                <p>No variants yet.</p>
              </Localized>
              <Localized id="variant-mgmt-empty-cta">
                <Button variant="secondary" onClick={openCreate}>Add a variant</Button>
              </Localized>
            </div>
          ) : (
            <div className="product-mgmt-table-wrap">
              <table className="product-mgmt-table" aria-label={l10n.getString('variant-mgmt-table-aria')}>
                <thead>
                  <tr>
                    <Localized id="variant-mgmt-col-name"><th>Name</th></Localized>
                    <Localized id="variant-mgmt-col-sku"><th>SKU</th></Localized>
                    <Localized id="variant-mgmt-col-price"><th>Price</th></Localized>
                    <Localized id="variant-mgmt-col-barcode"><th>Barcode</th></Localized>
                    <Localized id="variant-mgmt-col-status"><th>Status</th></Localized>
                    <Localized id="variant-mgmt-actions-aria" attrs={{ 'aria-label': true }}>
                      <th aria-label="Actions"> </th>
                    </Localized>
                  </tr>
                </thead>
                <tbody>
                  {variants.map((v) => (
                    <tr key={v.sku}>
                      <td>{v.name}</td>
                      <td className="product-mgmt-cell-sku">{v.sku}</td>
                      <td className="product-mgmt-cell-price">
                        {v.price != null ? (
                          <span>{formatVariantPrice(v.price.minor_units, v.price.currency)}</span>
                        ) : (
                          <Localized id="variant-mgmt-price-parent">
                            <span style={{ fontStyle: 'italic', color: 'var(--color-fg-tertiary)' }}>Uses parent price</span>
                          </Localized>
                        )}
                      </td>
                      <td className="product-mgmt-cell-barcode">{v.barcode ?? '\u2014'}</td>
                      <td>
                        {v.is_active ? (
                          <span style={{ color: 'var(--color-success, #16a34a)', fontWeight: 500 }}>
                            <Localized id="variant-mgmt-status-active"><span>Active</span></Localized>
                          </span>
                        ) : (
                          <span style={{ color: 'var(--color-fg-tertiary)' }}>
                            <Localized id="variant-mgmt-status-inactive"><span>Inactive</span></Localized>
                          </span>
                        )}
                      </td>
                      <td className="product-mgmt-cell-actions">
                        <Localized id="variant-mgmt-edit-aria" attrs={{ 'aria-label': true }} vars={{ name: v.name }}>
                          <button
                            type="button"
                            className="product-mgmt-action-btn"
                            onClick={() => openEdit(v)}
                            aria-label={`Edit ${v.name}`}
                          >
                            <Localized id="variant-mgmt-edit">
                              <span>Edit</span>
                            </Localized>
                          </button>
                        </Localized>
                        <Localized id="variant-mgmt-delete-aria" attrs={{ 'aria-label': true }} vars={{ name: v.name }}>
                          <button
                            type="button"
                            className="product-mgmt-action-btn product-mgmt-action-btn--danger"
                            onClick={() => setConfirmDeleteSku(v.sku)}
                            disabled={deletingSku === v.sku}
                            aria-label={`Delete ${v.name}`}
                          >
                            <Localized id="variant-mgmt-delete">
                              <span>Delete</span>
                            </Localized>
                          </button>
                        </Localized>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {showModal && (            <div className="product-mgmt-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('variant-mgmt-dialog-aria', { mode: editingSku ? 'edit' : 'add' })} style={{ zIndex: 200 }}>
            <div className="product-mgmt-modal">
              <div className="product-mgmt-modal-header">
                <Localized id={editingSku ? 'variant-mgmt-modal-edit-title' : 'variant-mgmt-modal-add-title'}>
                  <h2>{editingSku ? 'Edit Variant' : 'Add Variant'}</h2>
                </Localized>
                <Localized id="variant-mgmt-close-aria" attrs={{ 'aria-label': true }}>
                  <button
                    type="button"
                    className="product-mgmt-modal-close"
                    onClick={() => setShowModal(false)}
                    aria-label="Close"
                  >
                    &times;
                  </button>
                </Localized>
              </div>

              <div className="product-mgmt-modal-body">
                <label className="product-mgmt-field" htmlFor="variant-field-name">
                  {l10n.getString('variant-mgmt-field-name-required')}
                  <Localized id="variant-mgmt-name-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="product-mgmt-input"
                      type="text"
                      id="variant-field-name"
                      value={form.name}
                      onChange={(e) => setForm({ ...form, name: e.target.value })}
                      placeholder="e.g. Large"
                    />
                  </Localized>
                </label>

                <label className="product-mgmt-field" htmlFor="variant-field-sku">
                  {l10n.getString('variant-mgmt-field-sku-required')}
                  <Localized id="variant-mgmt-sku-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="product-mgmt-input"
                      type="text"
                      id="variant-field-sku"
                      value={form.sku}
                      onChange={(e) => setForm({ ...form, sku: e.target.value })}
                      disabled={!!editingSku}
                      placeholder="e.g. TEA-LARGE"
                    />
                  </Localized>
                </label>

                <div className="product-mgmt-row">
                  <label className="product-mgmt-field" htmlFor="variant-field-price">
                    {l10n.getString('variant-mgmt-field-price')}
                    <Localized id="variant-mgmt-price-placeholder" attrs={{ placeholder: true }}>
                      <input
                        className="product-mgmt-input"
                        type="number"
                        id="variant-field-price"
                        min="0"
                        value={form.priceMinor}
                        onChange={(e) => setForm({ ...form, priceMinor: e.target.value })}
                        placeholder="450"
                      />
                    </Localized>
                  </label>

                  <label className="product-mgmt-field" htmlFor="variant-field-currency">
                    {l10n.getString('variant-mgmt-field-currency')}
                    <Localized id="variant-mgmt-currency-placeholder" attrs={{ placeholder: true }}>
                      <input
                        className="product-mgmt-input"
                        type="text"
                        id="variant-field-currency"
                        value={form.currency}
                        onChange={(e) => setForm({ ...form, currency: e.target.value })}
                        placeholder="USD"
                        maxLength={3}
                      />
                    </Localized>
                  </label>
                </div>

                <label className="product-mgmt-field" htmlFor="variant-field-barcode">
                  {l10n.getString('variant-mgmt-field-barcode')}
                  <Localized id="variant-mgmt-barcode-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="product-mgmt-input"
                      type="text"
                      id="variant-field-barcode"
                      value={form.barcode}
                      onChange={(e) => setForm({ ...form, barcode: e.target.value })}
                      placeholder="4901234567890"
                    />
                  </Localized>
                </label>

                <div className="product-mgmt-row">
                  <label className="product-mgmt-field" htmlFor="variant-field-sort">
                    {l10n.getString('variant-mgmt-field-sort-order')}
                    <Localized id="variant-mgmt-sort-placeholder" attrs={{ placeholder: true }}>
                      <input
                        className="product-mgmt-input"
                        type="number"
                        id="variant-field-sort"
                        min="0"
                        value={form.sortOrder}
                        onChange={(e) => setForm({ ...form, sortOrder: e.target.value })}
                        placeholder="0"
                      />
                    </Localized>
                  </label>

                  <label className="product-mgmt-field" htmlFor="variant-field-active" style={{ justifyContent: 'flex-end', paddingBottom: 'var(--space-1)' }}>
                    <span style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
                      <input
                        type="checkbox"
                        id="variant-field-active"
                        checked={form.isActive}
                        onChange={(e) => setForm({ ...form, isActive: e.target.checked })}
                      />
                      {l10n.getString('variant-mgmt-field-active')}
                    </span>
                  </label>
                </div>
              </div>

              <div className="product-mgmt-modal-actions">
                <Localized id="variant-mgmt-btn-cancel">
                  <Button variant="ghost" onClick={() => setShowModal(false)} disabled={saving}>Cancel</Button>
                </Localized>
                <Button
                  variant="primary"
                  loading={saving}
                  disabled={!form.name.trim() || !form.sku.trim()}
                  onClick={handleSave}
                >
                  <Localized id={editingSku ? 'variant-mgmt-btn-update' : 'variant-mgmt-btn-create'}>
                    <span>{editingSku ? 'Update' : 'Create'}</span>
                  </Localized>
                </Button>
              </div>
            </div>
          </div>        )}

        {confirmDeleteSku && (
          <div className="product-mgmt-overlay" role="alertdialog" aria-modal="true" aria-label={l10n.getString('variant-mgmt-delete-confirm-aria')} style={{ zIndex: 200 }}>
            <div className="product-mgmt-modal" style={{ width: '380px' }}>
              <div className="product-mgmt-modal-header">
                <Localized id="variant-mgmt-delete-confirm-title">
                  <h2>Delete Variant</h2>
                </Localized>
              </div>
              <div className="product-mgmt-modal-body">
                {(() => {
                  const v = variants.find((x) => x.sku === confirmDeleteSku);
                  return v ? (
                    <Localized id="variant-mgmt-delete-confirm-body" vars={{ name: v.name, sku: v.sku }}>
                      <p>Are you sure you want to delete variant &quot;{v.name}&quot; ({v.sku})? This action cannot be undone.</p>
                    </Localized>
                  ) : null;
                })()}
              </div>
              <div className="product-mgmt-modal-actions">
                <Localized id="variant-mgmt-delete-confirm-cancel">
                  <Button variant="ghost" onClick={() => setConfirmDeleteSku(null)} disabled={!!deletingSku}>Cancel</Button>
                </Localized>
                <Button
                  variant="danger"
                  loading={!!deletingSku}
                  onClick={handleDelete}
                >
                  <Localized id="variant-mgmt-delete-confirm-confirm">
                    <span>Delete</span>
                  </Localized>
                </Button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function formatVariantPrice(minorUnits: number, currency: string): string {
  const known: Record<string, number> = {
    JPY: 0, KRW: 0, VND: 0, CLP: 0, ISK: 0, HUF: 0,
    KWD: 3, OMR: 3, BHD: 3, JOD: 3, TND: 3,
  };
  const exp = known[currency] ?? 2;
  const major = minorUnits / 10 ** exp;
  const fmt = new Intl.NumberFormat('en-US', { style: 'currency', currency });
  return fmt.format(major);
}
