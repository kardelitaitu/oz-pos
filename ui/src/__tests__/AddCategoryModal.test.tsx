import type React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi } from 'vitest';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import { FluentResource, FluentBundle } from '@fluent/bundle';
import { AddCategoryModal } from '@/features/retail/AddCategoryModal';

const ftl = `
retail-add-category-title = Add New Category
retail-add-category-field-name = Category Name
retail-edit-save = Save Category
retail-edit-cancel = Cancel
`;

function wrapper({ children }: { children: React.ReactNode }) {
  const resource = new FluentResource(ftl);
  const bundle = new FluentBundle('en');
  bundle.addResource(resource);
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
}

describe('AddCategoryModal', () => {
  it('does not render when isOpen is false', () => {
    render(
      <AddCategoryModal isOpen={false} onClose={vi.fn()} onSave={vi.fn()} />,
      { wrapper },
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders modal inputs when open', () => {
    render(
      <AddCategoryModal isOpen={true} onClose={vi.fn()} onSave={vi.fn()} />,
      { wrapper },
    );
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('e.g. Storage, Peripherals, Accessories')).toBeInTheDocument();
  });

  it('submits new category name on form submission', async () => {
    const user = userEvent.setup();
    const handleSave = vi.fn();
    const handleClose = vi.fn();

    render(
      <AddCategoryModal isOpen={true} onClose={handleClose} onSave={handleSave} />,
      { wrapper },
    );

    const input = screen.getByPlaceholderText('e.g. Storage, Peripherals, Accessories');
    await user.type(input, 'Cooling Solutions');

    await user.click(screen.getByRole('button', { name: 'Save Category' }));

    expect(handleSave).toHaveBeenCalledWith({
      id: expect.stringMatching(/^cat-\d+$/),
      name: 'Cooling Solutions',
      colour: '#10b981',
      icon: '',
    });
    expect(handleClose).toHaveBeenCalled();
  });
});
