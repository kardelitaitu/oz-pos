import { useState, useCallback, useEffect, useRef } from 'react';
import { Localized } from '@fluent/react';
import {
  listCategories,
  createCategory,
  deleteCategory,
  type CategoryDto,
} from '@/api/products';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
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

// ── Default random colour ────────────────────────────────────────────

function randomColour(): string {
  return COLOURS[Math.floor(Math.random() * COLOURS.length)]!;
}

// ── Helpers ──────────────────────────────────────────────────────────

function colourToId(name: string): string {
  return `cat-${name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')}`;
}

// ── Component ────────────────────────────────────────────────────────

export default function CategoryManagementScreen() {
  const [categories, setCategories] = useState<CategoryDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [newName, setNewName] = useState('');
  const [newColour, setNewColour] = useState(randomColour());
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; name: string } | null>(null);
  const [error, setError] = useState<string | null>(null);

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

  const openCreate = useCallback(() => {
    setNewName('');
    setNewColour(randomColour());
    setError(null);
    setShowModal(true);
  }, []);

  const handleCreate = useCallback(async () => {
    const trimmed = newName.trim();
    if (!trimmed) return;

    setSaving(true);
    setError(null);

    try {
      const id = colourToId(trimmed);
      await createCategory({ id, name: trimmed, colour: newColour });
      setShowModal(false);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create category');
    } finally {
      setSaving(false);
    }
  }, [newName, newColour, load]);

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

  // Focus the name input when the modal opens.
  useEffect(() => {
    if (showModal && inputRef.current) {
      inputRef.current.focus();
    }
  }, [showModal]);

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
              <p>No categories yet.</p>
            </Localized>
            <Localized id="categories-empty-desc">
              <p className="cat-mgmt-empty-desc">
                Categories group your products (e.g. Drinks, Food, Merchandise).
              </p>
            </Localized>
            <Localized id="categories-add-first">
              <Button variant="secondary" onClick={openCreate}>
                Add your first category
              </Button>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="cat-mgmt-grid">
          {categories.map((cat) => (
            <Card key={cat.id} shadow="xs">
              <div className="cat-mgmt-card">
                <div
                  className="cat-mgmt-swatch"
                  style={{ background: cat.colour }}
                  aria-hidden="true"
                />
                <div className="cat-mgmt-card-info">
                  <span className="cat-mgmt-card-name">{cat.name}</span>
                  <span className="cat-mgmt-card-id">{cat.id}</span>
                  <span className="cat-mgmt-card-colour">{cat.colour}</span>
                </div>
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

      {/* ── Delete confirmation modal ──────────────────────── */}
      {deleteTarget && (
        <Localized id="category-delete-dialog-aria" attrs={{ 'aria-label': true }}>
        <div className="cat-mgmt-overlay" role="dialog" aria-modal="true" aria-label="Delete category">
          <div className="cat-mgmt-modal">
            <div className="cat-mgmt-modal-header">
              <Localized id="categories-delete-confirm" vars={{ name: deleteTarget.name }}>
                <h2 className="cat-mgmt-modal-title">Delete &quot;{deleteTarget.name}&quot;?</h2>
              </Localized>
              <Localized id="close" attrs={{ 'aria-label': true }}>
                <button
                  type="button"
                  className="cat-mgmt-modal-close"
                  onClick={() => setDeleteTarget(null)}
                  aria-label="Close"
                >
                  &times;
                </button>
              </Localized>
            </div>
            <div className="cat-mgmt-modal-body">
              <Localized id="categories-delete-warning">
                <p className="cat-mgmt-delete-warning">
                  Are you sure you want to delete this category? This action cannot be undone.
                  Products assigned to this category will lose their category association.
                </p>
              </Localized>
            </div>
            <div className="cat-mgmt-modal-actions">
              <Localized id="cancel">
                <Button variant="ghost" onClick={() => setDeleteTarget(null)} disabled={deleting !== null}>
                  Cancel
                </Button>
              </Localized>
              <Localized id="delete">
                <Button
                  variant="danger"
                  loading={deleting !== null}
                  onClick={confirmDelete}
                >
                  Delete
                </Button>
              </Localized>
            </div>
          </div>
        </div>
        </Localized>
      )}

      {/* ── Create modal ──────────────────────────────── */}
      {showModal && (
        <div className="cat-mgmt-overlay" role="dialog" aria-modal="true" aria-label="Add category">
          <div className="cat-mgmt-modal">
            <div className="cat-mgmt-modal-header">
              <Localized id="categories-add">
                <h2 className="cat-mgmt-modal-title">Add Category</h2>
              </Localized>
              <Localized id="close" attrs={{ 'aria-label': true }}>
                <button
                  type="button"
                  className="cat-mgmt-modal-close"
                  onClick={() => setShowModal(false)}
                  aria-label="Close"
                >
                  &times;
                </button>
              </Localized>
            </div>

            <div className="cat-mgmt-modal-body">
              {/* Name input */}
              <label className="cat-mgmt-field">
                <Localized id="categories-name">
                  <span className="cat-mgmt-label">Name</span>
                </Localized>
                <Localized id="categories-name-placeholder" attrs={{ placeholder: true }}>
                  <input
                    className="cat-mgmt-input"
                    type="text"
                    placeholder="e.g. Bakery, Merchandise"
                    value={newName}
                    onChange={(e) => setNewName(e.target.value)}
                    ref={inputRef}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') handleCreate();
                    }}
                  />
                </Localized>
                <span className="cat-mgmt-hint">
                  <Localized id="categories-id-preview">
                    <span>Category ID will be:</span>
                  </Localized>{' '}
                  <code>{newName.trim() ? colourToId(newName.trim()) : '…'}</code>
                </span>
              </label>

              {/* Colour swatch picker */}
              <div className="cat-mgmt-field">
                <Localized id="categories-colour">
                  <span className="cat-mgmt-label">Colour</span>
                </Localized>
                <Localized id="category-colour-picker-aria" attrs={{ 'aria-label': true }}>
                <div className="cat-mgmt-colour-picker" role="radiogroup" aria-label="Pick a colour">
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
                </Localized>
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
                  {newName.trim() || <Localized id="category-name-fallback"><span>Category Name</span></Localized>}
                </span>
              </div>

              {error && (
                <div className="cat-mgmt-error" role="alert">
                  {error}
                </div>
              )}
            </div>

            <div className="cat-mgmt-modal-actions">
              <Localized id="cancel">
                <Button variant="ghost" onClick={() => setShowModal(false)} disabled={saving}>
                  Cancel
                </Button>
              </Localized>
              <Localized id="categories-create">
                <Button
                  variant="primary"
                  loading={saving}
                  disabled={!newName.trim()}
                  onClick={handleCreate}
                >
                  Create
                </Button>
              </Localized>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
