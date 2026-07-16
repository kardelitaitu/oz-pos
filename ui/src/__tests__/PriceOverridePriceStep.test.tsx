// ── PriceOverrideModal price step tests ───────────────────────────
//
// Covers: validation edge cases on the price input step.
// Fast synchronous tests extracted from PriceOverrideKeyboardEdgeCases
// to enable parallel execution. 3 tests.

import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';

vi.mock('@/api/staff', () => ({
  staffLogin: vi.fn(),
}));

import PriceOverrideModal from '@/features/sales/PriceOverrideModal';
import type { PriceOverrideModalProps } from '@/features/sales/PriceOverrideModal';

const defaultProps: PriceOverrideModalProps = {
  open: true,
  lineDescription: 'Widget x 2',
  currentPrice: { minor_units: 50000, currency: 'IDR' },
  onConfirm: vi.fn().mockResolvedValue(undefined),
  onClose: vi.fn(),
};

describe('PriceOverrideModal — price step', () => {
  it('disables Next when currentPrice is already zero', () => {
    render(
      <PriceOverrideModal
        {...defaultProps}
        currentPrice={{ minor_units: 0, currency: 'IDR' }}
      />,
    );
    expect(screen.getByText('Next')).toBeDisabled();
  });

  it('disables Next when currentPrice is negative', () => {
    render(
      <PriceOverrideModal
        {...defaultProps}
        currentPrice={{ minor_units: -1, currency: 'IDR' }}
      />,
    );
    expect(screen.getByText('Next')).toBeDisabled();
  });

  it('clamps negative typed value to minimum of 1 via onChange', () => {
    render(<PriceOverrideModal {...defaultProps} />);
    const input = screen.getByLabelText('Enter new price in minor units') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '-1' } });
    expect(screen.getByText('Next')).toBeEnabled();
  });
});
