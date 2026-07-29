import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import RetailReminderPopup from '@/features/retail/RetailReminderPopup';

// Mock @fluent/react so useLocalization returns identity keys
vi.mock('@fluent/react', () => ({
  useLocalization: () => ({
    l10n: { getString: (id: string) => id },
  }),
  Localized: ({ children }: { id: string; children: React.ReactNode }) => <>{children}</>,
}));

// ── Empty state ───────────────────────────────────────────────

describe('RetailReminderPopup — empty state', () => {
  it('returns null when all counts are 0', () => {
    const { container } = render(
      <RetailReminderPopup lowStockCount={0} creditCount={0} heldCartCount={0} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('returns null when all counts are 0 even after a re-render', () => {
    const { container, rerender } = render(
      <RetailReminderPopup lowStockCount={0} creditCount={0} heldCartCount={0} />,
    );
    expect(container.firstChild).toBeNull();

    rerender(<RetailReminderPopup lowStockCount={0} creditCount={0} heldCartCount={0} />);
    expect(container.firstChild).toBeNull();
  });
});

// ── Single reminder types ─────────────────────────────────────

describe('RetailReminderPopup — single reminder types', () => {
  it('renders low stock row when lowStockCount > 0', () => {
    render(
      <RetailReminderPopup lowStockCount={3} creditCount={0} heldCartCount={0} />,
    );
    expect(screen.getByRole('status')).toBeInTheDocument();
    // Mock returns FTL key; test that the row renders, not the exact text
    expect(document.querySelector('.retail-reminder-row--low-stock')).toBeInTheDocument();
    expect(document.querySelector('.retail-reminder-row--credit')).toBeNull();
    expect(document.querySelector('.retail-reminder-row--held-cart')).toBeNull();
  });

  it('renders credit row when creditCount > 0', () => {
    render(
      <RetailReminderPopup lowStockCount={0} creditCount={2} heldCartCount={0} />,
    );
    expect(screen.getByRole('status')).toBeInTheDocument();
    expect(document.querySelector('.retail-reminder-row--credit')).toBeInTheDocument();
    expect(document.querySelector('.retail-reminder-row--low-stock')).toBeNull();
  });

  it('renders held cart row when heldCartCount > 0', () => {
    render(
      <RetailReminderPopup lowStockCount={0} creditCount={0} heldCartCount={1} />,
    );
    expect(screen.getByRole('status')).toBeInTheDocument();
    expect(document.querySelector('.retail-reminder-row--held-cart')).toBeInTheDocument();
  });

  it('held cart row renders for both singular and plural counts', () => {
    const { rerender } = render(
      <RetailReminderPopup lowStockCount={0} creditCount={0} heldCartCount={1} />,
    );
    expect(document.querySelector('.retail-reminder-row--held-cart')).toBeInTheDocument();

    rerender(
      <RetailReminderPopup lowStockCount={0} creditCount={0} heldCartCount={5} />,
    );
    expect(document.querySelector('.retail-reminder-row--held-cart')).toBeInTheDocument();
  });
});

// ── Unified notification center ───────────────────────────────

describe('RetailReminderPopup — all three reminder types', () => {
  it('renders all three rows simultaneously', () => {
    render(
      <RetailReminderPopup lowStockCount={3} creditCount={2} heldCartCount={1} />,
    );

    expect(document.querySelector('.retail-reminder-row--low-stock')).toBeInTheDocument();
    expect(document.querySelector('.retail-reminder-row--credit')).toBeInTheDocument();
    expect(document.querySelector('.retail-reminder-row--held-cart')).toBeInTheDocument();
  });

  it('has correct CSS classes for each row type', () => {
    render(
      <RetailReminderPopup lowStockCount={3} creditCount={2} heldCartCount={1} />,
    );

    const rows = document.querySelectorAll('.retail-reminder-row');
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveClass('retail-reminder-row--low-stock');
    expect(rows[1]).toHaveClass('retail-reminder-row--credit');
    expect(rows[2]).toHaveClass('retail-reminder-row--held-cart');
  });
});

// ── ARIA attributes ──────────────────────────────────────────

describe('RetailReminderPopup — accessibility', () => {
  it('has role="status" and aria-live="polite"', () => {
    render(
      <RetailReminderPopup lowStockCount={1} creditCount={0} heldCartCount={0} />,
    );
    const status = screen.getByRole('status');
    expect(status).toHaveAttribute('aria-live', 'polite');
  });

  it('dismiss button has an accessible label', () => {
    render(
      <RetailReminderPopup lowStockCount={1} creditCount={0} heldCartCount={0} />,
    );
    const dismissBtn = document.querySelector('.retail-reminder-dismiss')!;
    expect(dismissBtn).toHaveAttribute('aria-label');
    expect(dismissBtn.getAttribute('aria-label')).toBeTruthy();
  });

  it('all SVG icons are aria-hidden', () => {
    render(
      <RetailReminderPopup lowStockCount={1} creditCount={1} heldCartCount={1} />,
    );
    const svgs = document.querySelectorAll('svg');
    svgs.forEach((svg) => {
      expect(svg).toHaveAttribute('aria-hidden', 'true');
    });
  });
});

// ── Dismiss behavior ──────────────────────────────────────────

describe('RetailReminderPopup — dismiss behavior', () => {
  it('hides popup when dismiss button is clicked', () => {
    const { container } = render(
      <RetailReminderPopup lowStockCount={3} creditCount={0} heldCartCount={0} />,
    );
    expect(screen.getByRole('status')).toBeInTheDocument();

    const dismissBtn = document.querySelector('.retail-reminder-dismiss')!;
    fireEvent.click(dismissBtn);

    // Popup should be gone from the DOM
    expect(container.querySelector('.retail-reminder-popup')).toBeNull();
  });

  it('stays hidden after dismiss even on re-render with same props', () => {
    const { container, rerender } = render(
      <RetailReminderPopup lowStockCount={3} creditCount={0} heldCartCount={0} />,
    );
    const dismissBtn = document.querySelector('.retail-reminder-dismiss')!;
    fireEvent.click(dismissBtn);
    expect(container.querySelector('.retail-reminder-popup')).toBeNull();

    // Re-render with same props — still hidden
    rerender(
      <RetailReminderPopup lowStockCount={3} creditCount={0} heldCartCount={0} />,
    );
    expect(container.querySelector('.retail-reminder-popup')).toBeNull();
  });
});

// ── Auto-reset on count change ────────────────────────────────

describe('RetailReminderPopup — auto-reset on count change', () => {
  it('reappears when a count increases after dismissal', () => {
    const { container, rerender } = render(
      <RetailReminderPopup lowStockCount={1} creditCount={0} heldCartCount={0} />,
    );
    const dismissBtn = document.querySelector('.retail-reminder-dismiss')!;
    fireEvent.click(dismissBtn);
    expect(container.querySelector('.retail-reminder-popup')).toBeNull();

    // New low-stock item arrives — popup should reappear
    rerender(
      <RetailReminderPopup lowStockCount={2} creditCount={0} heldCartCount={0} />,
    );
    expect(container.querySelector('.retail-reminder-popup')).toBeInTheDocument();
    expect(document.querySelector('.retail-reminder-row--low-stock')).toBeInTheDocument();
  });

  it('reappears when a new reminder type appears after dismissal', () => {
    const { container, rerender } = render(
      <RetailReminderPopup lowStockCount={2} creditCount={0} heldCartCount={0} />,
    );
    const dismissBtn = document.querySelector('.retail-reminder-dismiss')!;
    fireEvent.click(dismissBtn);
    expect(container.querySelector('.retail-reminder-popup')).toBeNull();

    // Credit sales come in — popup should reappear with both rows
    rerender(
      <RetailReminderPopup lowStockCount={2} creditCount={3} heldCartCount={0} />,
    );
    expect(container.querySelector('.retail-reminder-popup')).toBeInTheDocument();
    expect(document.querySelector('.retail-reminder-row--credit')).toBeInTheDocument();
  });

  it('reappears when count drops after dismissal (any change triggers reset)', () => {
    const { container, rerender } = render(
      <RetailReminderPopup lowStockCount={3} creditCount={1} heldCartCount={2} />,
    );
    const dismissBtn = document.querySelector('.retail-reminder-dismiss')!;
    fireEvent.click(dismissBtn);
    expect(container.querySelector('.retail-reminder-popup')).toBeNull();

    // One held cart resumed, count drops — popup still reappears due to change
    rerender(
      <RetailReminderPopup lowStockCount={3} creditCount={1} heldCartCount={1} />,
    );
    expect(container.querySelector('.retail-reminder-popup')).toBeInTheDocument();
  });
});

// ── Click-to-action ──────────────────────────────────────────

describe('RetailReminderPopup — click-to-action', () => {
  const onClickLowStock = vi.fn();
  const onClickCredit = vi.fn();
  const onClickHeldCarts = vi.fn();

  beforeEach(() => {
    onClickLowStock.mockReset();
    onClickCredit.mockReset();
    onClickHeldCarts.mockReset();
  });

  it('calls onClickLowStock when low stock row is clicked', () => {
    render(
      <RetailReminderPopup
        lowStockCount={3}
        creditCount={0}
        heldCartCount={0}
        onClickLowStock={onClickLowStock}
      />,
    );
    const row = document.querySelector('.retail-reminder-row--low-stock')!;
    fireEvent.click(row);
    expect(onClickLowStock).toHaveBeenCalledTimes(1);
  });

  it('calls onClickCredit when credit row is clicked', () => {
    render(
      <RetailReminderPopup
        lowStockCount={0}
        creditCount={2}
        heldCartCount={0}
        onClickCredit={onClickCredit}
      />,
    );
    const row = document.querySelector('.retail-reminder-row--credit')!;
    fireEvent.click(row);
    expect(onClickCredit).toHaveBeenCalledTimes(1);
  });

  it('calls onClickHeldCarts when held cart row is clicked', () => {
    render(
      <RetailReminderPopup
        lowStockCount={0}
        creditCount={0}
        heldCartCount={1}
        onClickHeldCarts={onClickHeldCarts}
      />,
    );
    const row = document.querySelector('.retail-reminder-row--held-cart')!;
    fireEvent.click(row);
    expect(onClickHeldCarts).toHaveBeenCalledTimes(1);
  });

  it('does not throw when clicked without onClick handler', () => {
    render(
      <RetailReminderPopup lowStockCount={3} creditCount={2} heldCartCount={1} />,
    );
    const row = document.querySelector('.retail-reminder-row--low-stock')!;
    expect(() => fireEvent.click(row)).not.toThrow();
  });
});
