import { useState, useCallback, useEffect, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listTaxRates,
  createTaxRate,
  updateTaxRate,
  deleteTaxRate,
  listCategoryTaxRates,
  setCategoryTaxRates,
  type TaxRateDto,

} from '@/api/tax';
import { listCategories, type CategoryDto } from '@/api/products';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Badge } from '@/components/Badge';
import './TaxConfigurationScreen.css';

interface TaxFormData {
  name: string;
  rateBps: string;
  isDefault: boolean;
  isInclusive: boolean;
}

const EMPTY_TAX_FORM: TaxFormData = {
  name: '',
  rateBps: '',
  isDefault: false,
  isInclusive: false,
};

export default function TaxConfigurationScreen() {
  const { l10n } = useLocalization();
  // ── Tax rates state ─────────────────────────────────────────────
  const [rates, setRates] = useState<TaxRateDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<TaxFormData>(EMPTY_TAX_FORM);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  // ── Category tax rates state ────────────────────────────────────
  const [categories, setCategories] = useState<CategoryDto[]>([]);
  const [catTaxRates, setCatTaxRates] = useState<Map<string, string[]>>(new Map());
  const [showCatModal, setShowCatModal] = useState(false);
  const [editingCatId, setEditingCatId] = useState<string | null>(null);
  const [editingCatName, setEditingCatName] = useState('');
  const [selectedCatRateIds, setSelectedCatRateIds] = useState<string[]>([]);
  const [savingCat, setSavingCat] = useState(false);

  // ── Data loading ────────────────────────────────────────────────

  const loadAll = useCallback(async () => {
    setLoading(true);
    try {
      const [items, cats, catTax] = await Promise.all([
        listTaxRates(),
        listCategories(),
        listCategoryTaxRates(),
      ]);
      setRates(items);
      setCategories(cats);

      const map = new Map<string, string[]>();
      for (const row of catTax) {
        map.set(row.category_id, row.tax_rate_ids);
      }
      setCatTaxRates(map);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadAll(); }, [loadAll]);

  // ── Tax rate CRUD ───────────────────────────────────────────────

  const openCreate = useCallback(() => {
    setForm(EMPTY_TAX_FORM);
    setEditingId(null);
    setShowModal(true);
  }, []);

  const openEdit = useCallback((r: TaxRateDto) => {
    setForm({
      name: r.name,
      rateBps: String(r.rate_bps),
      isDefault: r.is_default,
      isInclusive: r.is_inclusive,
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
          isInclusive: form.isInclusive,
        });
      } else {
        await createTaxRate({
          name: form.name,
          rateBps,
          isDefault: form.isDefault,
          isInclusive: form.isInclusive,
        });
      }
      setShowModal(false);
      await loadAll();
    } catch {
      // Error handling.
    } finally {
      setSaving(false);
    }
  }, [form, editingId, loadAll]);

  const confirmDelete = useCallback(async (id: string) => {
    setDeleting(id);
    try {
      await deleteTaxRate(id);
      setDeleting(null);
      await loadAll();
    } catch {
      setDeleting(null);
    }
  }, [loadAll]);

  // ── Category tax rates ──────────────────────────────────────────

  const openCatEdit = useCallback((cat: CategoryDto) => {
    setEditingCatId(cat.id);
    setEditingCatName(cat.name);
    setSelectedCatRateIds(catTaxRates.get(cat.id) ?? []);
    setShowCatModal(true);
  }, [catTaxRates]);

  const handleSaveCat = useCallback(async () => {
    if (!editingCatId) return;
    setSavingCat(true);
    try {
      await setCategoryTaxRates({
        categoryId: editingCatId,
        taxRateIds: selectedCatRateIds,
      });
      setShowCatModal(false);
      await loadAll();
    } catch {
      // Error handling.
    } finally {
      setSavingCat(false);
    }
  }, [editingCatId, selectedCatRateIds, loadAll]);

  const toggleCatRate = useCallback((rateId: string) => {
    setSelectedCatRateIds((prev) =>
      prev.includes(rateId)
        ? prev.filter((id) => id !== rateId)
        : [...prev, rateId],
    );
  }, []);

  // ── Refs for focus management ───────────────────────────────────

  const taxNameInputRef = useFocusOnShow(showModal);

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
      ) : (
        <>
          {/* ── Tax Rates Table ────────────────────────────────────── */}
          {rates.length === 0 ? (
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
              <table className="tax-config-table" aria-label={l10n.getString('tax-config-table-aria')}>
                <thead>
                  <tr>
                    <Localized id="tax-config-col-name"><th>Name</th></Localized>
                    <Localized id="tax-config-col-rate"><th>Rate (%)</th></Localized>
                    <Localized id="tax-config-col-type"><th>Type</th></Localized>
                    <Localized id="tax-config-col-default"><th>Default</th></Localized>
                    <Localized id="tax-config-col-actions" attrs={{ "aria-label": true }}>
                      <th aria-label="Actions"> </th>
                    </Localized>
                  </tr>
                </thead>
                <tbody>
                  {rates.map((r) => (
                    <tr key={r.id}>
                      <td>
                        {r.name}
                        {r.is_default && (
                          <Badge variant="info" size="sm" style={{ marginLeft: 'var(--space-2)' }}>
                            <Localized id="tax-config-default-badge">
                              <span>Default</span>
                            </Localized>
                          </Badge>
                        )}
                      </td>
                      <td>{r.display_rate}</td>
                      <td>
                        <span className={`tax-config-type-badge ${r.is_inclusive ? 'tax-config-type--inclusive' : 'tax-config-type--exclusive'}`}>
                          <Localized id={r.is_inclusive ? 'tax-config-type-inclusive' : 'tax-config-type-exclusive'}>
                            <span>{r.is_inclusive ? 'Inclusive' : 'Exclusive'}</span>
                          </Localized>
                        </span>
                      </td>
                      <td>{r.is_default ? l10n.getString('tax-config-yes') : '\u2014'}</td>
                      <td className="tax-config-cell-actions">
                        <Localized id="tax-config-edit-aria" attrs={{ "aria-label": true }} vars={{ name: r.name }}>
                        <button
                          type="button"
                          className="tax-config-action-btn"
                          onClick={() => openEdit(r)}
                        >
                          <Localized id="tax-config-edit">
                            <span>Edit</span>
                          </Localized>
                        </button>
                        </Localized>
                        <Localized id="tax-config-delete-aria" attrs={{ "aria-label": true }} vars={{ name: r.name }}>
                        <button
                          type="button"
                          className="tax-config-action-btn tax-config-action-btn--danger"
                          onClick={() => confirmDelete(r.id)}
                          disabled={deleting === r.id}
                        >
                          <Localized id="tax-config-btn-delete">
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

          {/* ── Category Tax Rates Section ──────────────────────────── */}
          <div className="tax-config-section">
            <Localized id="tax-config-cat-title">
              <h2 className="tax-config-section-title">Category Tax Rates</h2>
            </Localized>
            <Localized id="tax-config-cat-desc">
              <p className="tax-config-section-desc">
                Assign default tax rates to product categories. Products inherit their
                category&rsquo;s tax rates unless overridden at the product level.
              </p>
            </Localized>

            {categories.length === 0 ? (
              <Localized id="tax-config-no-categories">
                <p className="tax-config-loading">No categories available.</p>
              </Localized>
            ) : (
              <div className="tax-config-table-wrap">
              <table className="tax-config-table" aria-label={l10n.getString('tax-config-cat-table-aria')}>
                  <thead>
                    <tr>
                      <Localized id="tax-config-col-category"><th>Category</th></Localized>
                      <Localized id="tax-config-col-assigned"><th>Assigned Tax Rates</th></Localized>
                      <Localized id="tax-config-col-actions" attrs={{ "aria-label": true }}>
                        <th aria-label="Actions"> </th>
                      </Localized>
                    </tr>
                  </thead>
                  <tbody>
                    {categories.map((cat) => {
                      const assignedIds = catTaxRates.get(cat.id) ?? [];
                      const assignedNames = assignedIds
                        .map((id) => rates.find((r) => r.id === id))
                        .filter(Boolean)
                        .map((r) => r!.name);
                      return (
                        <tr key={cat.id}>
                          <td>
                            <span className="tax-config-cat-name">
                              <span
                                className="tax-config-cat-swatch"
                                style={{ background: cat.colour }}
                                aria-hidden="true"
                              />
                              {cat.name}
                            </span>
                          </td>
                          <td>
                            {assignedNames.length > 0 ? (
                              <span className="tax-config-cat-badges">
                                {assignedNames.map((n) => (
                                  <Badge key={n} variant="default" size="sm">{n}</Badge>
                                ))}
                              </span>
                            ) : (
                              <Localized id="tax-config-no-rates-assigned">
                                <span className="tax-config-muted">No rates assigned</span>
                              </Localized>
                            )}
                          </td>
                          <td className="tax-config-cell-actions">
                            <Localized id="tax-config-cat-edit-aria" attrs={{ "aria-label": true }} vars={{ name: cat.name }}>
                            <button
                              type="button"
                              className="tax-config-action-btn"
                              onClick={() => openCatEdit(cat)}
                            >
                              <Localized id="tax-config-edit">
                                <span>Edit</span>
                              </Localized>
                            </button>
                            </Localized>
                          </td>
                        </tr>
                      );
                    })}                </tbody>
              </table>
              </div>
            )}
          </div>
        </>
      )}

      {/* ── Tax Rate Form Modal ──────────────────────────────────── */}
      {showModal && (
        <div className="tax-config-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('tax-config-modal-aria', { editing: editingId !== null ? 'true' : 'false' })}>
          <div className="tax-config-modal">
            <div className="tax-config-modal-header">
              <Localized id="tax-config-modal-title" vars={{ editing: editingId !== null ? 'true' : 'false' }}>
                <h2>{editingId ? 'Edit Tax Rate' : 'Add Tax Rate'}</h2>
              </Localized>
                <Localized id="tax-config-modal-close" attrs={{ "aria-label": true }}>
                  <button
                    type="button"
                    className="tax-config-modal-close"
                    onClick={() => setShowModal(false)}
                    aria-label="Close"
                  >
                    &times;
                  </button>
                </Localized>
            </div>

            <div className="tax-config-modal-body">
              <label className="tax-config-field" htmlFor="tax-field-name" aria-label={l10n.getString('tax-config-field-name-aria')}>
                <Localized id="tax-config-field-name">
                  <span className="tax-config-label">Tax Name</span>
                </Localized>
                <input
                  className="tax-config-input"
                  type="text"
                  id="tax-field-name"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  placeholder={l10n.getString('tax-config-field-name-placeholder')}
                  ref={taxNameInputRef}
                />
              </label>

              <label className="tax-config-field" htmlFor="tax-field-rate">
                {l10n.getString('tax-config-field-rate')}
                <input
                  className="tax-config-input"
                  type="number"
                  id="tax-field-rate"
                  min="0"
                  value={form.rateBps}
                  onChange={(e) => setForm({ ...form, rateBps: e.target.value })}
                  placeholder={l10n.getString('tax-config-field-rate-placeholder')}
                />
                <Localized id="tax-config-rate-hint">
                  <span className="tax-config-hint">Enter rate in basis points (e.g. 825 = 8.25%)</span>
                </Localized>
              </label>

              {/* Inclusive / Exclusive toggle */}
              <div className="tax-config-field">
                <Localized id="tax-config-tax-type">
                  <span className="tax-config-label">Tax Type</span>
                </Localized>
                <div className="tax-config-toggle-group" role="radiogroup" aria-label={l10n.getString('tax-config-tax-type-aria')}>
                  <button
                    type="button"
                    role="radio"
                    aria-checked={!form.isInclusive}
                    className={`tax-config-toggle-btn ${!form.isInclusive ? 'tax-config-toggle-btn--active' : ''}`}
                    onClick={() => setForm({ ...form, isInclusive: false })}
                  >
                    <Localized id="tax-config-type-exclusive-label">
                      <span>Exclusive</span>
                    </Localized>
                    <Localized id="tax-config-type-exclusive-desc">
                      <span className="tax-config-toggle-desc">Added at checkout</span>
                    </Localized>
                  </button>
                  <button
                    type="button"
                    role="radio"
                    aria-checked={form.isInclusive}
                    className={`tax-config-toggle-btn ${form.isInclusive ? 'tax-config-toggle-btn--active' : ''}`}
                    onClick={() => setForm({ ...form, isInclusive: true })}
                  >
                    <Localized id="tax-config-type-inclusive-label">
                      <span>Inclusive</span>
                    </Localized>
                    <Localized id="tax-config-type-inclusive-desc">
                      <span className="tax-config-toggle-desc">Included in price</span>
                    </Localized>
                  </button>
                </div>
              </div>

              <label className="tax-config-checkbox">
                <input
                  type="checkbox"
                  checked={form.isDefault}
                  onChange={(e) => setForm({ ...form, isDefault: e.target.checked })}
                />
                {l10n.getString('tax-config-set-default')}
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

      {/* ── Category Tax Rates Modal ─────────────────────────────── */}
      {showCatModal && (
        <div className="tax-config-overlay" role="dialog" aria-modal="true" aria-label={l10n.getString('tax-config-cat-modal-aria', { name: editingCatName })}>
          <div className="tax-config-modal">
            <div className="tax-config-modal-header">
              <Localized id="tax-config-cat-modal-title" vars={{ name: editingCatName }}>
                <h2>Tax Rates &mdash; {editingCatName}</h2>
              </Localized>
              <Localized id="tax-config-modal-close" attrs={{ "aria-label": true }}>
                <button
                  type="button"
                  className="tax-config-modal-close"
                  onClick={() => setShowCatModal(false)}
                  aria-label="Close"
                >
                  &times;
                </button>
              </Localized>
            </div>

            <div className="tax-config-modal-body">
              <Localized id="tax-config-cat-modal-desc">
                <p className="tax-config-section-desc">
                  Select the tax rates that apply to all products in this category.
                </p>
              </Localized>

              {rates.length === 0 ? (
                <Localized id="tax-config-no-rates">
                  <p className="tax-config-loading">
                    No tax rates available. Create one first.
                  </p>
                </Localized>
              ) : (
                <div className="tax-config-cat-rate-list">
                  {rates.map((r) => {
                    const checked = selectedCatRateIds.includes(r.id);
                    return (
                      <label
                        key={r.id}
                        className={`tax-config-cat-rate-item ${checked ? 'tax-config-cat-rate-item--checked' : ''}`}
                        htmlFor={"tax-cat-rate-" + r.id}
                        aria-label={r.name}
                      >
                        <input
                          type="checkbox"
                          id={"tax-cat-rate-" + r.id}
                          checked={checked}
                          onChange={() => toggleCatRate(r.id)}
                        />
                        <div className="tax-config-cat-rate-info">
                          <span className="tax-config-cat-rate-name">{r.name}</span>
                          <span className="tax-config-cat-rate-meta">
                            {r.display_rate}
                            {' \u00b7 '}
                            {l10n.getString(r.is_inclusive ? 'tax-config-type-inclusive' : 'tax-config-type-exclusive')}
                          </span>
                        </div>
                      </label>
                    );
                  })}
                </div>
              )}
            </div>

            <div className="tax-config-modal-actions">
              <Localized id="tax-config-btn-cancel">
                <Button variant="ghost" onClick={() => setShowCatModal(false)} disabled={savingCat}>Cancel</Button>
              </Localized>
              <Button
                variant="primary"
                loading={savingCat}
                onClick={handleSaveCat}
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

// ── Focus-on-show hook ──────────────────────────────────────────────

function useFocusOnShow(show: boolean) {
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (show && ref.current) {
      ref.current.focus();
    }
  }, [show]);
  return ref;
}
