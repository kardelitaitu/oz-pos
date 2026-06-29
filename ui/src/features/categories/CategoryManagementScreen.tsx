import { useState, useCallback, useEffect, useRef } from 'react';
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
        <h1 className="cat-mgmt-title">Categories</h1>
        <Button onClick={openCreate}>Add Category</Button>
      </div>

      {loading ? (
        <p className="cat-mgmt-loading">Loading categories…</p>
      ) : categories.length === 0 ? (
        <Card shadow="sm">
          <div className="cat-mgmt-empty">
            <p>No categories yet.</p>
            <p className="cat-mgmt-empty-desc">
              Categories group your products (e.g. Drinks, Food, Merchandise).
            </p>
            <Button variant="secondary" onClick={openCreate}>
              Add your first category
            </Button>
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
                <button
                  type="button"
                  className="cat-mgmt-delete-btn"
                  onClick={() => setDeleteTarget({ id: cat.id, name: cat.name })}
                  disabled={deleting === cat.id}
                  aria-label={`Delete category ${cat.name}`}
                >
                  &times;
                </button>
              </div>
            </Card>
          ))}
        </div>
      )}

      {/* ── Delete confirmation modal ──────────────────────── */}
      {deleteTarget && (
        <div className="cat-mgmt-overlay" role="dialog" aria-modal="true" aria-label="Delete category">
          <div className="cat-mgmt-modal">
            <div className="cat-mgmt-modal-header">
              <h2 className="cat-mgmt-modal-title">Delete &quot;{deleteTarget.name}&quot;?</h2>
              <button
                type="button"
                className="cat-mgmt-modal-close"
                onClick={() => setDeleteTarget(null)}
                aria-label="Close"
              >
                &times;
              </button>
            </div>
            <div className="cat-mgmt-modal-body">
              <p className="cat-mgmt-delete-warning">
                Are you sure you want to delete this category? This action cannot be undone.
                Products assigned to this category will lose their category association.
              </p>
            </div>
            <div className="cat-mgmt-modal-actions">
              <Button variant="ghost" onClick={() => setDeleteTarget(null)} disabled={deleting !== null}>
                Cancel
              </Button>
              <Button
                variant="danger"
                loading={deleting !== null}
                onClick={confirmDelete}
              >
                Delete
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* ── Create modal ──────────────────────────────── */}
      {showModal && (
        <div className="cat-mgmt-overlay" role="dialog" aria-modal="true" aria-label="Add category">
          <div className="cat-mgmt-modal">
            <div className="cat-mgmt-modal-header">
              <h2 className="cat-mgmt-modal-title">Add Category</h2>
              <button
                type="button"
                className="cat-mgmt-modal-close"
                onClick={() => setShowModal(false)}
                aria-label="Close"
              >
                &times;
              </button>
            </div>

            <div className="cat-mgmt-modal-body">
              {/* Name input */}
              <label className="cat-mgmt-field">
                <span className="cat-mgmt-label">Name</span>
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
                <span className="cat-mgmt-hint">
                  Category ID will be: <code>{newName.trim() ? colourToId(newName.trim()) : '…'}</code>
                </span>
              </label>

              {/* Colour swatch picker */}
              <div className="cat-mgmt-field">
                <span className="cat-mgmt-label">Colour</span>
                <div className="cat-mgmt-colour-picker" role="radiogroup" aria-label="Pick a colour">
                  {COLOURS.map((colour) => (
                    <button
                      key={colour}
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
                  ))}
                </div>
              </div>

              {/* Preview */}
              <div className="cat-mgmt-preview">
                <span className="cat-mgmt-label">Preview</span>
                <span
                  className="cat-mgmt-preview-chip"
                  style={{
                    background: newColour,
                    color: '#fff',
                  }}
                >
                  {newName.trim() || 'Category Name'}
                </span>
              </div>

              {error && (
                <div className="cat-mgmt-error" role="alert">
                  {error}
                </div>
              )}
            </div>

            <div className="cat-mgmt-modal-actions">
              <Button variant="ghost" onClick={() => setShowModal(false)} disabled={saving}>
                Cancel
              </Button>
              <Button
                variant="primary"
                loading={saving}
                disabled={!newName.trim()}
                onClick={handleCreate}
              >
                Create
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
