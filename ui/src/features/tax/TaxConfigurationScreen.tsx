import { useState, useCallback, useEffect } from 'react';
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
import { Skeleton } from '@/components/Skeleton';
import { SettingsPopup } from '@/frontend/shared';
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

/** Tax configuration screen — CRUD for tax rates, inclusive/exclusive toggle, and per-category tax rate assignment. */
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
        <div className="tax-config-loading-skeleton" aria-hidden="true">
          {/* Header skeleton: title + button */}
          <div className="tax-config-header">
            <Skeleton variant="block" width="14rem" height="1.75rem" />
            <Skeleton variant="block" width="8rem" height="2.25rem" />
          </div>
          {/* Table skeleton: header + 4 rows with 5 columns */}
          <div className="tax-config-table-wrap">
            <table className="tax-config-table" aria-hidden="true">
              <thead>
                <tr>
                  {['Name', 'Rate (%)', 'Type', 'Default', ''].map((_, i) => (
                    <th key={i}>
                      <Skeleton variant="text" width={i < 4 ? '4rem' : '3rem'} height="0.75rem" />
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>{[0, 1, 2, 3].map((r) => (
                  <tr key={r}>
                    <td><Skeleton variant="text" width="6rem" height="0.875rem" /></td>
                    <td><Skeleton variant="text" width="3rem" height="0.875rem" /></td>
                    <td><Skeleton variant="block" width="5rem" height="1.25rem" style={{ borderRadius: 'var(--radius-sm)' }} /></td>
                    <td><Skeleton variant="text" width="2.5rem" height="0.875rem" /></td>
                    <td className="tax-config-cell-actions">
                      <Skeleton variant="block" width="3.5rem" height="1.375rem" />
                    </td>
                  </tr>
                ))}</tbody>
            </table>
          </div>
        </div>
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
                <tbody>{rates.map((r) => (
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
                      {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized attrs */}
                      <td>
                        <span className={`tax-config-type-badge ${r.is_inclusive ? 'tax-config-type--inclusive' : 'tax-config-type--exclusive'}`}>
                          <Localized id={r.is_inclusive ? 'tax-config-type-inclusive' : 'tax-config-type-exclusive'}>
                            <span>{r.is_inclusive ? 'Inclusive' : 'Exclusive'}</span>
                          </Localized>
                        </span>
                      </td>
                      <td>{r.is_default ? l10n.getString('tax-config-yes') : '\u2014'}</td>
                      {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized attrs */}
                      <td className="tax-config-cell-actions">
                        <Localized id="tax-config-edit-aria" attrs={{ "aria-label": true }} vars={{ name: r.name }}>
                        {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- visible text inside Localized */}
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
                        {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- visible text inside Localized */}
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
                  ))}</tbody>
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
                  <tbody>{categories.map((cat) => {
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
                          {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized attrs */}
                          <td className="tax-config-cell-actions">
                            <Localized id="tax-config-cat-edit-aria" attrs={{ "aria-label": true }} vars={{ name: cat.name }}>
                            {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized attrs */}
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
                    })}</tbody>
              </table>
              </div>
            )}
          </div>
        </>
      )}

      {/* ── Tax Rate Form Modal ──────────────────────────────────── */}
      <SettingsPopup
        open={showModal}
        onClose={() => setShowModal(false)}
        title={l10n.getString('tax-config-modal-title', { editing: editingId !== null ? 'true' : 'false' })}
        saving={saving}
        onSave={handleSave}
        saveLabel={l10n.getString('tax-config-btn-save')}
        saveDisabled={!form.name.trim() || !form.rateBps.trim()}
        cancelLabel={l10n.getString('tax-config-btn-cancel')}
      >
        <div className="tax-config-field tax-config-field--horizontal">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- @fluent/react Localized wrapper */}
          <label htmlFor="tax-field-name" className="tax-config-label">
            <Localized id="tax-config-field-name">
              <span>Tax Name</span>
            </Localized>
          </label>
          <input
            className="tax-config-input"
            type="text"
            id="tax-field-name"
            value={form.name}
            onChange={(e) => setForm({ ...form, name: e.target.value })}
            placeholder={l10n.getString('tax-config-field-name-placeholder')}
          />
        </div>

        <div className="tax-config-field tax-config-field--horizontal">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- @fluent/react Localized wrapper */}
          <label htmlFor="tax-field-rate" className="tax-config-label">
            <Localized id="tax-config-field-rate">
              <span>Rate (BPS)</span>
            </Localized>
          </label>
          <div className="tax-config-field-input-wrap">
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
          </div>
        </div>

        {/* Inclusive / Exclusive toggle */}
        <div className="tax-config-field tax-config-field--horizontal">
          <Localized id="tax-config-tax-type">
            <span className="tax-config-label">Tax Type</span>
          </Localized>
          <div className="tax-config-toggle-group" role="radiogroup" aria-label={l10n.getString('tax-config-tax-type-aria')}>
            <button
              type="button"
              role="radio"
              aria-checked={!form.isInclusive}
              aria-label={l10n.getString('tax-config-type-exclusive-label')}
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
              aria-label={l10n.getString('tax-config-type-inclusive-label')}
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
      </SettingsPopup>

      {/* ── Category Tax Rates Modal ─────────────────────────────── */}
      <SettingsPopup
        open={showCatModal}
        onClose={() => setShowCatModal(false)}
        title={l10n.getString('tax-config-cat-modal-title', { name: editingCatName })}
        saving={savingCat}
        onSave={handleSaveCat}
        saveLabel={l10n.getString('tax-config-btn-save')}
        cancelLabel={l10n.getString('tax-config-btn-cancel')}
        size="sm"
      >
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
      </SettingsPopup>
    </div>
  );
}

