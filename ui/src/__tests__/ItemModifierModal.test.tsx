import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import ItemModifierModal from '@/features/sales/components/ItemModifierModal';
import type {
  ItemModifierModalProps,
  ModifierGroup,
} from '@/features/sales/components/ItemModifierModal';

// ── Fixtures ─────────────────────────────────────────────────────────

const donenessGroup: ModifierGroup = {
  id: 'grp-doneness',
  name: 'Doneness',
  minSelections: 1,
  maxSelections: 1,
  sortOrder: 1,
  modifiers: [
    { id: 'mod-rare', name: 'Rare', priceMinor: 0, sortOrder: 1, isDefault: false },
    { id: 'mod-medium', name: 'Medium', priceMinor: 0, sortOrder: 2, isDefault: true },
    { id: 'mod-well', name: 'Well Done', priceMinor: 0, sortOrder: 3, isDefault: false },
  ],
};

const sidesGroup: ModifierGroup = {
  id: 'grp-sides',
  name: 'Add Sides',
  minSelections: 0,
  maxSelections: 2,
  sortOrder: 2,
  modifiers: [
    { id: 'mod-fries', name: 'French Fries', priceMinor: 5000, sortOrder: 1, isDefault: false },
    { id: 'mod-salad', name: 'Side Salad', priceMinor: 3000, sortOrder: 2, isDefault: false },
    { id: 'mod-onion', name: 'Onion Rings', priceMinor: 7000, sortOrder: 3, isDefault: false },
  ],
};

const defaultProps: ItemModifierModalProps = {
  open: true,
  productName: 'Grilled Ribeye',
  basePriceMinor: 150000,
  currency: 'IDR',
  groups: [donenessGroup, sidesGroup],
  onConfirm: vi.fn(),
  onClose: vi.fn(),
};

const ftl = `
modifier-no-options = No customisation options available
modifier-free = Free
modifier-base-price = Base price
modifier-addons = Add-ons
modifier-total = Total
cancel = Cancel
modifier-add-to-cart = Add to Order
`;

function wrapper({ children }: { children: React.ReactNode }) {
  const bundle = new FluentBundle('en');
  bundle.addResource(new FluentResource(ftl));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
}

function renderModal(props: Partial<ItemModifierModalProps> = {}) {
  return render(<ItemModifierModal {...defaultProps} {...props} />, { wrapper });
}

// ── Tests ────────────────────────────────────────────────────────────

describe('ItemModifierModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── Render / open state ─────────────────────────────────────────

  it('renders nothing when open is false', () => {
    renderModal({ open: false });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders modal with product name and groups when open', () => {
    renderModal();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Grilled Ribeye')).toBeInTheDocument();
    expect(screen.getByText('Doneness')).toBeInTheDocument();
    expect(screen.getByText('Add Sides')).toBeInTheDocument();
  });

  it('pre-selects default modifiers', () => {
    renderModal();
    const mediumBtn = screen.getByTestId('modifier-mod-medium');
    expect(mediumBtn).toHaveAttribute('aria-selected', 'true');
  });

  it('shows price values in footer', () => {
    renderModal();
    // formatMoney with IDR/id-ID: 150000 → "IDR 1.500,00"
    const priceElements = screen.getAllByText(/1\.500/);
    expect(priceElements.length).toBeGreaterThanOrEqual(1);
  });

  // ── Selection logic ──────────────────────────────────────────────

  it('toggles a modifier on click', async () => {
    const user = userEvent.setup();
    renderModal();
    const rareBtn = screen.getByTestId('modifier-mod-rare');
    expect(rareBtn).toHaveAttribute('aria-selected', 'false');

    await user.click(rareBtn);
    await waitFor(() => {
      expect(rareBtn).toHaveAttribute('aria-selected', 'true');
    });

    // Medium should be deselected (single-select group: maxSelections=1).
    const mediumBtn = screen.getByTestId('modifier-mod-medium');
    await waitFor(() => {
      expect(mediumBtn).toHaveAttribute('aria-selected', 'false');
    });
  });

  it('selects multiple modifiers in a multi-select group', async () => {
    const user = userEvent.setup();
    renderModal();
    const friesBtn = screen.getByTestId('modifier-mod-fries');
    const saladBtn = screen.getByTestId('modifier-mod-salad');

    await user.click(friesBtn);
    await user.click(saladBtn);

    await waitFor(() => {
      expect(friesBtn).toHaveAttribute('aria-selected', 'true');
    });
    await waitFor(() => {
      expect(saladBtn).toHaveAttribute('aria-selected', 'true');
    });
  });

  it('prevents exceeding maxSelections in a group', async () => {
    const user = userEvent.setup();
    renderModal();
    const friesBtn = screen.getByTestId('modifier-mod-fries');
    const saladBtn = screen.getByTestId('modifier-mod-salad');
    const onionBtn = screen.getByTestId('modifier-mod-onion');

    await user.click(friesBtn);
    await user.click(saladBtn);

    // Third option should be disabled after 2 selections (maxSelections = 2).
    await waitFor(() => {
      expect(onionBtn).toBeDisabled();
    });
  });

  it('prevents deselecting below minSelections in a group', async () => {
    const user = userEvent.setup();
    renderModal();
    const mediumBtn = screen.getByTestId('modifier-mod-medium');

    // Try to deselect Medium (minSelections = 1 for doneness).
    await user.click(mediumBtn);
    await waitFor(() => {
      expect(mediumBtn).toHaveAttribute('aria-selected', 'true');
    });
  });

  // ── Confirm / close ──────────────────────────────────────────────

  it('calls onConfirm with selections when Add to Order is clicked', async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    renderModal({ onConfirm });
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Add to Order/ })).not.toBeDisabled();
    });

    await user.click(screen.getByRole('button', { name: /Add to Order/ }));

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(
        expect.arrayContaining([
          expect.objectContaining({ groupName: 'Doneness', modifierName: 'Medium' }),
        ]),
        150000,
      );
    });
  });

  it('calls onClose when Cancel is clicked', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderModal({ onClose });
    await user.click(screen.getByText('Cancel'));
    expect(onClose).toHaveBeenCalled();
  });

  it('calls onClose when overlay backdrop is clicked', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderModal({ onClose });
    await user.click(screen.getByRole('presentation'));
    expect(onClose).toHaveBeenCalled();
  });

  it('disables Confirm when group selections are invalid', () => {
    const invalidGroups: ModifierGroup[] = [
      {
        ...donenessGroup,
        minSelections: 1,
        maxSelections: 1,
        modifiers: donenessGroup.modifiers.map((m) => ({ ...m, isDefault: false })),
      },
    ];
    renderModal({
      groups: invalidGroups,
      onConfirm: vi.fn(),
    });

    // No default selected + minSelections = 1 → invalid.
    const confirmBtn = screen.getByRole('button', { name: /Add to Order/ });
    expect(confirmBtn).toBeDisabled();
  });

  it('shows empty state when no modifier groups are provided', () => {
    renderModal({ groups: [] });
    expect(screen.getByText('No customisation options available')).toBeInTheDocument();
  });

  // ── Selection metadata ───────────────────────────────────────────

  it('calls onConfirm with correct modifier metadata including prices', async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    renderModal({ onConfirm });

    // Select Rare (replaces Medium) and French Fries.
    await user.click(screen.getByTestId('modifier-mod-rare'));
    await user.click(screen.getByTestId('modifier-mod-fries'));

    await user.click(screen.getByRole('button', { name: /Add to Order/ }));

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(
        expect.arrayContaining([
          expect.objectContaining({
            groupName: 'Doneness',
            modifierName: 'Rare',
            priceMinor: 0,
          }),
          expect.objectContaining({
            groupName: 'Add Sides',
            modifierName: 'French Fries',
            priceMinor: 5000,
          }),
        ]),
        155000,
      );
    });
  });
});
