import { useState, useCallback, useEffect, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listCategories,
  createCategory,
  updateCategory,
  deleteCategory,
  type CategoryDto,
} from '@/api/products';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { SettingsPopup } from '@/frontend/shared';
import './CategoryManagementScreen.css';

// ── Predefined colour palette for the colour picker ──────────────────

const COLOURS = [
  '#06b6d4', // cyan
  '#f97316', // orange
  '#10b981', // emerald
  '#6366f1', // indigo
  '#ec4899', // pink
  '#f59e0b', // amber
  '#8b5cf6', // violet
  '#14b8a6', // teal
  '#ef4444', // red
  '#84cc16', // lime
  '#3b82f6', // blue
  '#a855f7', // purple
  '#e11d48', // rose
  '#0ea5e9', // sky
  '#22c55e', // green
  '#d946ef', // fuchsia
];

// ── Icon set ─────────────────────────────────────────────

interface IconOption {
  id: string;
  label: string;
}

const ICON_OPTIONS: IconOption[] = [
  { id: 'food',       label: 'Food'       },
  { id: 'snack',      label: 'Snack'      },
  { id: 'hot-drink',  label: 'Hot drink'  },
  { id: 'cold-drink', label: 'Cold drink' },
  { id: 'dots-1',     label: 'Generic ·'  },
  { id: 'dots-2',     label: 'Generic ··' },
  { id: 'dots-3',     label: 'Generic ···'},
];

/** Render the SVG for a given icon id. Returns null for no-icon. */
function CategoryIconSvg({ icon, size = 18 }: { icon: string; size?: number }) {
  const strokeProps = {
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: 2,
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const,
    width: size,
    height: size,
    'aria-hidden': true,
  };

  if (icon === 'food') {
    return (
      <svg viewBox="0 0 24 24" {...strokeProps}>
        {/* Fork */}
        <path d="M3 2v7c0 1.1.9 2 2 2h4a2 2 0 0 0 2-2V2" />
        <line x1="7" y1="11" x2="7" y2="22" />
        {/* Knife */}
        <path d="M21 15V2a5 5 0 0 0-5 5v6c0 1.1.9 2 2 2h3z" />
        <line x1="21" y1="15" x2="21" y2="22" />
      </svg>
    );
  }
  if (icon === 'snack') {
    return (
      <svg viewBox="0 0 24 24" {...strokeProps}>
        {/* Bowl */}
        <path d="M4 12h16" />
        <path d="M4 12c0 5.5 3.6 9 8 9s8-3.5 8-9" />
        {/* Snack items */}
        <circle cx="9" cy="9" r="2" fill="currentColor" stroke="none" />
        <circle cx="13" cy="8" r="2" fill="currentColor" stroke="none" />
        <circle cx="17" cy="9" r="2" fill="currentColor" stroke="none" />
      </svg>
    );
  }
  if (icon === 'hot-drink') {
    return (
      <svg viewBox="0 0 24 24" {...strokeProps}>
        {/* Cup */}
        <path d="M6 8h12l-1.5 12h-9L6 8z" />
        {/* Handle */}
        <path d="M17 11h2a2 2 0 0 1 0 4h-2" />
        {/* Steam */}
        <path d="M8 8C8.8 6.5 7.2 5.5 8 4" />
        <path d="M13 8C13.8 6.5 12.2 5.5 13 4" />
      </svg>
    );
  }
  if (icon === 'cold-drink') {
    return (
      <svg viewBox="0 0 24 24" {...strokeProps}>
        {/* Cup body */}
        <path d="M5 7h14l-2 15H7L5 7z" />
        {/* Rim */}
        <line x1="3" y1="7" x2="21" y2="7" />
        {/* Straw */}
        <line x1="16" y1="2" x2="12" y2="22" />
      </svg>
    );
  }
  if (icon === 'dots-1') {
    return (
      <svg viewBox="0 0 16 16" fill="currentColor" width={size} height={size} aria-hidden="true">
        <circle cx="8" cy="8" r="3.5" />
      </svg>
    );
  }
  if (icon === 'dots-2') {
    return (
      <svg viewBox="0 0 16 16" fill="currentColor" width={size} height={size} aria-hidden="true">
        <circle cx="4.5" cy="8" r="3" />
        <circle cx="11.5" cy="8" r="3" />
      </svg>
    );
  }
  if (icon === 'dots-3') {
    return (
      <svg viewBox="0 0 16 16" fill="currentColor" width={size} height={size} aria-hidden="true">
        <circle cx="2.5" cy="8" r="2.5" />
        <circle cx="8" cy="8" r="2.5" />
        <circle cx="13.5" cy="8" r="2.5" />
      </svg>
    );
  }
  return null;
}

// ── Default random colour ────────────────────────────────────────────

function randomColour(): string {
  return COLOURS[Math.floor(Math.random() * COLOURS.length)]!;
}

function randomIcon(): string {
  return ICON_OPTIONS[Math.floor(Math.random() * ICON_OPTIONS.length)]!.id;
}

// ── Helpers ──────────────────────────────────────────────────────────

function colourToId(name: string): string {
  return `cat-${name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')}`;
}

// ── Component ────────────────────────────────────────────────────────

/** Category management screen — create, edit, and delete product categories with colour and icon selection. */
export default function CategoryManagementScreen() {
  const { l10n } = useLocalization();
  const [categories, setCategories] = useState<CategoryDto[]>([]);
  const [loading, setLoading] = useState(true);

  // ── Create modal state ──────────────────────────────────────────
  const [showModal, setShowModal] = useState(false);
  const [newName, setNewName] = useState('');
  const [newColour, setNewColour] = useState(randomColour());
  const [newIcon, setNewIcon] = useState(randomIcon());
  const [saving, setSaving] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // ── Edit modal state ────────────────────────────────────────────
  const [editTarget, setEditTarget] = useState<CategoryDto | null>(null);
  const [editName, setEditName] = useState('');
  const [editColour, setEditColour] = useState('');
  const [editIcon, setEditIcon] = useState('');
  const [editSaving, setEditSaving] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);

  // ── Delete modal state ──────────────────────────────────────────
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; name: string } | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);

  const closeCreate = useCallback(() => setShowModal(false), []);
  const closeEdit = useCallback(() => setEditTarget(null), []);
  const closeDelete = useCallback(() => setDeleteTarget(null), []);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const cats = await listCategories();
      setCategories(cats);
    } catch {
      // IPC unavailable
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  // ── Create handlers ──────────────────────────────────────────────

  const openCreate = useCallback(() => {
    setNewName('');
    setNewColour(randomColour());
    setNewIcon(randomIcon());
    setCreateError(null);
    setShowModal(true);
  }, []);

  const handleCreate = useCallback(async () => {
    const trimmed = newName.trim();
    if (!trimmed) return;

    setSaving(true);
    setCreateError(null);

    try {
      const id = colourToId(trimmed);
      await createCategory({ id, name: trimmed, colour: newColour, icon: newIcon });
      setShowModal(false);
      await load();
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : 'Failed to create category');
    } finally {
      setSaving(false);
    }
  }, [newName, newColour, newIcon, load]);

  // ── Edit handlers ────────────────────────────────────────────────

  const openEdit = useCallback((cat: CategoryDto) => {
    setEditTarget(cat);
    setEditName(cat.name);
    setEditColour(cat.colour);
    setEditIcon(cat.icon);
    setEditError(null);
  }, []);

  const handleEdit = useCallback(async () => {
    if (!editTarget) return;
    const trimmed = editName.trim();
    if (!trimmed) return;

    setEditSaving(true);
    setEditError(null);

    try {
      await updateCategory({ id: editTarget.id, name: trimmed, colour: editColour, icon: editIcon });
      setEditTarget(null);
      await load();
    } catch (err) {
      setEditError(err instanceof Error ? err.message : 'Failed to update category');
    } finally {
      setEditSaving(false);
    }
  }, [editTarget, editName, editColour, editIcon, load]);

  // ── Delete handlers ──────────────────────────────────────────────

  const confirmDelete = useCallback(async () => {
    if (!deleteTarget) return;
    setDeleting(deleteTarget.id);
    setDeleteTarget(null);
    try {
      await deleteCategory(deleteTarget.id);
      await load();
    } catch (err) {
      console.error('Failed to delete category:', err);
    } finally {
      setDeleting(null);
    }
  }, [deleteTarget, load]);

  const inputRef = useRef<HTMLInputElement>(null);
  const editInputRef = useRef<HTMLInputElement>(null);

  // Focus name inputs when modals open.
  useEffect(() => {
    if (showModal && inputRef.current) {
      inputRef.current.focus();
    }
  }, [showModal]);

  useEffect(() => {
    if (editTarget && editInputRef.current) {
      editInputRef.current.focus();
    }
  }, [editTarget]);

  return (
    <div className="cat-mgmt">
      <div className="cat-mgmt-header">
        <Localized id="categories-title">
          <h1 className="cat-mgmt-title">Categories</h1>
        </Localized>
        <Localized id="categories-add">
          <Button onClick={openCreate}>Add Category</Button>
        </Localized>
      </div>

      {loading ? (
        <Localized id="categories-loading">
          <p className="cat-mgmt-loading">Loading categories…</p>
        </Localized>
      ) : categories.length === 0 ? (
        <Card shadow="sm">
          <div className="cat-mgmt-empty">
            <Localized id="categories-no-categories">
              <p>No categories yet</p>
            </Localized>
            <Localized id="categories-empty-desc">
              <p className="cat-mgmt-empty-desc">Categories group your products.</p>
            </Localized>
            <Localized id="categories-add-first">
              <Button variant="secondary" onClick={openCreate}>Add your first category</Button>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="cat-mgmt-grid">
          {categories.map((cat) => (
            <Card key={cat.id} shadow="xs">
              <div className="cat-mgmt-card">
                {/* Icon badge — coloured circle with icon SVG */}
                <div
                  className="cat-mgmt-icon-badge"
                  style={{ background: cat.colour }}
                  aria-hidden="true"
                >
                  {cat.icon ? (
                    <CategoryIconSvg icon={cat.icon} size={20} />
                  ) : (
                    <span className="cat-mgmt-icon-badge-empty" />
                  )}
                </div>
                <div className="cat-mgmt-card-info">
                  <span className="cat-mgmt-card-name">{cat.name}</span>
                  <span className="cat-mgmt-card-id">{cat.id}</span>
                  <span className="cat-mgmt-card-colour">{cat.colour}</span>
                </div>
                {/* Edit button */}
                <button
                  type="button"
                  className="cat-mgmt-edit-btn"
                  onClick={() => openEdit(cat)}
                  aria-label={`Edit category ${cat.name}`}
                >
                  ✎
                </button>
                {/* Delete button */}
                <Localized id="category-delete-aria" attrs={{ 'aria-label': true }} vars={{ name: cat.name }}>
                  <button
                    type="button"
                    className="cat-mgmt-delete-btn"
                    onClick={() => setDeleteTarget({ id: cat.id, name: cat.name })}
                    disabled={deleting === cat.id}
                    aria-label={`Delete category ${cat.name}`}
                  >
                    &times;
                  </button>
                </Localized>
              </div>
            </Card>
          ))}
        </div>
      )}

      <SettingsPopup
        open={!!deleteTarget}
        onClose={closeDelete}
        title={l10n.getString('categories-delete-confirm', { name: deleteTarget?.name ?? '' })}
        size="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setDeleteTarget(null)} disabled={deleting !== null}>
              <Localized id="cancel"><span>Cancel</span></Localized>
            </Button>
            <Button variant="danger" loading={deleting !== null} onClick={confirmDelete}>
              <Localized id="delete"><span>Delete</span></Localized>
            </Button>
          </>
        }
      >
        <Localized id="categories-delete-warning">
          <p className="cat-mgmt-delete-warning">Are you sure you want to delete this category?</p>
        </Localized>
      </SettingsPopup>

      <SettingsPopup
        open={showModal}
        onClose={closeCreate}
        title={l10n.getString('categories-add')}
        saving={saving}
        error={createError}
        onSave={handleCreate}
        saveLabel={l10n.getString('categories-create')}
        saveDisabled={!newName.trim()}
        cancelLabel={l10n.getString('cancel')}
      >
        {/* Name input */}
        <div className="cat-mgmt-field">
          <label htmlFor="cat-new-name" className="cat-mgmt-label">
            <Localized id="categories-name">
              <span>Name</span>
            </Localized>
          </label>
          <Localized id="categories-name-placeholder" attrs={{ placeholder: true }}>
            <input
              className="cat-mgmt-input"
              type="text"
              id="cat-new-name"
              name="cat-new-name"
              placeholder="e.g. Bakery, Merchandise"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              ref={inputRef}
              aria-label="Category Name"
            />
          </Localized>
          <span className="cat-mgmt-hint">
            <Localized id="categories-id-preview">
              <span>Category ID will be:</span>
            </Localized>{' '}
            <code>{newName.trim() ? colourToId(newName.trim()) : '…'}</code>
          </span>
        </div>

        {/* Icon picker */}
        <div className="cat-mgmt-field">
          <Localized id="categories-icon">
            <span className="cat-mgmt-label">Icon</span>
          </Localized>
          <div className="cat-mgmt-icon-picker" role="radiogroup" aria-label="Pick an icon">
            {ICON_OPTIONS.map((opt) => (
              <button
                key={opt.id}
                type="button"
                role="radio"
                aria-checked={newIcon === opt.id}
                aria-label={opt.label}
                className={
                  newIcon === opt.id
                    ? 'cat-mgmt-icon-btn cat-mgmt-icon-btn--selected'
                    : 'cat-mgmt-icon-btn'
                }
                style={newIcon === opt.id ? { background: newColour, color: '#fff' } : undefined}
                onClick={() => setNewIcon(opt.id)}
              >
                <CategoryIconSvg icon={opt.id} size={20} />
              </button>
            ))}
          </div>
        </div>

        {/* Colour swatch picker */}
        <div className="cat-mgmt-field">
          <Localized id="categories-colour">
            <span className="cat-mgmt-label">Colour</span>
          </Localized>
          <div className="cat-mgmt-colour-picker" role="radiogroup" aria-label={l10n.getString('category-colour-picker-aria')}>
              {COLOURS.map((colour) => (
                <Localized key={colour} id="category-colour-swatch-aria" attrs={{ 'aria-label': true }} vars={{ colour }}>
                  <button
                    type="button"
                    role="radio"
                    aria-checked={newColour === colour}
                    className={
                      newColour === colour
                        ? 'cat-mgmt-colour-swatch cat-mgmt-colour-swatch--selected'
                        : 'cat-mgmt-colour-swatch'
                    }
                    style={{ background: colour }}
                    onClick={() => setNewColour(colour)}
                    aria-label={`Select colour ${colour}`}
                  />
                </Localized>
              ))}
            </div>
        </div>

        {/* Preview */}
        <div className="cat-mgmt-preview">
          <Localized id="categories-preview">
            <span className="cat-mgmt-label">Preview</span>
          </Localized>
          <span
            className="cat-mgmt-preview-chip"
            style={{
              background: newColour,
              color: '#fff',
            }}
          >
            <CategoryIconSvg icon={newIcon} size={14} />
            {newName.trim() || <Localized id="category-name-fallback"><span>Category Name</span></Localized>}
          </span>
        </div>
      </SettingsPopup>

      <SettingsPopup
        open={!!editTarget}
        onClose={closeEdit}
        title={l10n.getString('categories-edit')}
        saving={editSaving}
        error={editError}
        onSave={handleEdit}
        saveLabel={l10n.getString('categories-save')}
        saveDisabled={!editName.trim()}
        cancelLabel={l10n.getString('cancel')}
      >
        {/* Name input */}
        <div className="cat-mgmt-field">
          <label htmlFor="cat-edit-name" className="cat-mgmt-label">
            <Localized id="categories-name">
              <span>Name</span>
            </Localized>
          </label>
          <input
            className="cat-mgmt-input"
            type="text"
            id="cat-edit-name"
            name="cat-edit-name"
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            ref={editInputRef}
            aria-label="Category Name"
          />
        </div>

        {/* Icon picker */}
        <div className="cat-mgmt-field">
          <Localized id="categories-icon">
            <span className="cat-mgmt-label">Icon</span>
          </Localized>
          <div className="cat-mgmt-icon-picker" role="radiogroup" aria-label="Pick an icon">
            {ICON_OPTIONS.map((opt) => (
              <button
                key={opt.id}
                type="button"
                role="radio"
                aria-checked={editIcon === opt.id}
                aria-label={opt.label}
                className={
                  editIcon === opt.id
                    ? 'cat-mgmt-icon-btn cat-mgmt-icon-btn--selected'
                    : 'cat-mgmt-icon-btn'
                }
                style={editIcon === opt.id ? { background: editColour, color: '#fff' } : undefined}
                onClick={() => setEditIcon(opt.id)}
              >
                <CategoryIconSvg icon={opt.id} size={20} />
              </button>
            ))}
          </div>
        </div>

        {/* Colour swatch picker */}
        <div className="cat-mgmt-field">
          <Localized id="categories-colour">
            <span className="cat-mgmt-label">Colour</span>
          </Localized>
          <div className="cat-mgmt-colour-picker" role="radiogroup" aria-label="Pick a colour">
            {COLOURS.map((colour) => (
              <button
                key={colour}
                type="button"
                role="radio"
                aria-checked={editColour === colour}
                className={
                  editColour === colour
                    ? 'cat-mgmt-colour-swatch cat-mgmt-colour-swatch--selected'
                    : 'cat-mgmt-colour-swatch'
                }
                style={{ background: colour }}
                onClick={() => setEditColour(colour)}
                aria-label={`Select colour ${colour}`}
              />
            ))}
          </div>
        </div>

        {/* Preview */}
        <div className="cat-mgmt-preview">
          <Localized id="categories-preview">
            <span className="cat-mgmt-label">Preview</span>
          </Localized>
          <span
            className="cat-mgmt-preview-chip"
            style={{ background: editColour, color: '#fff' }}
          >
            <CategoryIconSvg icon={editIcon} size={14} />
            {editName.trim() || editTarget?.name || ''}
          </span>
        </div>
      </SettingsPopup>
    </div>
  );
}
