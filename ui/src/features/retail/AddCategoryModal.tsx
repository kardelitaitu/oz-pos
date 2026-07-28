import React, { useState, useEffect, useCallback } from 'react';
import { Localized } from '@fluent/react';
import type { CategoryDto } from '@/api/products';

export interface AddCategoryModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (newCategory: CategoryDto) => void;
}

export const AddCategoryModal: React.FC<AddCategoryModalProps> = ({
  isOpen,
  onClose,
  onSave,
}) => {
  const [name, setName] = useState('');

  useEffect(() => {
    if (isOpen) {
      setName('');
    }
  }, [isOpen]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    },
    [onClose],
  );

  useEffect(() => {
    if (isOpen) {
      window.addEventListener('keydown', handleKeyDown);
      return () => window.removeEventListener('keydown', handleKeyDown);
    }
  }, [isOpen, handleKeyDown]);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    const newCategory: CategoryDto = {
      id: `cat-${Date.now()}`,
      name: name.trim(),
      colour: '#10b981',
      icon: '',
    };

    onSave(newCategory);
    onClose();
  };

  return (
    <div className="retail-edit-modal-backdrop" onClick={onClose} role="presentation">
      <div
        className="retail-edit-modal-dialog"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="retail-add-category-title"
      >
        <div className="retail-edit-modal-header">
          <Localized id="retail-add-category-title">
            <h3 id="retail-add-category-title" className="retail-edit-modal-title">
              Add New Category
            </h3>
          </Localized>
          <button
            type="button"
            className="retail-edit-modal-close"
            onClick={onClose}
            aria-label="Close"
          >
            &times;
          </button>
        </div>

        <form onSubmit={handleSubmit} className="retail-edit-modal-form">
          <div className="retail-edit-form-group">
            <Localized id="retail-add-category-field-name">
              <label htmlFor="add-category-name" className="retail-edit-label">
                Category Name
              </label>
            </Localized>
            <input
              id="add-category-name"
              type="text"
              className="retail-edit-input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Storage, Peripherals, Accessories"
              autoFocus
              required
            />
          </div>

          <div className="retail-edit-modal-actions">
            <Localized id="retail-edit-cancel">
              <button
                type="button"
                className="retail-edit-modal-btn retail-edit-modal-btn--secondary"
                onClick={onClose}
              >
                Cancel
              </button>
            </Localized>
            <Localized id="retail-edit-save">
              <button
                type="submit"
                className="retail-edit-modal-btn retail-edit-modal-btn--primary"
              >
                Save Category
              </button>
            </Localized>
          </div>
        </form>
      </div>
    </div>
  );
};
