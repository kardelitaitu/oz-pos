import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import giftCardsFtl from '@/locales/gift-cards.ftl?raw';

// Mock the API module.
vi.mock('@/api/giftCards', () => ({
  getGiftCardBalance: vi.fn(),
  redeemGiftCard: vi.fn(),
}));

import GiftCardPayment from '@/features/gift-cards/GiftCardPayment';
import { getGiftCardBalance, redeemGiftCard } from '@/api/giftCards';

const wrap = (children: React.ReactNode) => withFluent(children, giftCardsFtl);

const defaultProps = {
  totalMinor: 100000,
  currency: 'IDR',
  saleId: 'sale-1',
  onApplied: vi.fn(),
  onError: vi.fn(),
  onCancel: vi.fn(),
  onComplete: vi.fn(),
};

describe('GiftCardPayment', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the payment form', () => {
    render(wrap(<GiftCardPayment {...defaultProps} />));
    expect(screen.getByText('Gift Card')).toBeInTheDocument();
    expect(screen.getByLabelText('Gift card number')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /check/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
  });

  it('shows total due amount', () => {
    render(wrap(<GiftCardPayment {...defaultProps} />));
    // IDR has 2 fraction digits with the default formatting: IDR 1,000.00
    expect(screen.getByText(/IDR/)).toBeInTheDocument();
  });

  it('does not call API when card number is empty', async () => {
    const mockBalance = getGiftCardBalance as ReturnType<typeof vi.fn>;
    render(wrap(<GiftCardPayment {...defaultProps} />));
    await userEvent.click(screen.getByRole('button', { name: /check/i }));
    expect(mockBalance).not.toHaveBeenCalled();
  });

  it('looks up a gift card balance on check', async () => {
    const mockBalance = getGiftCardBalance as ReturnType<typeof vi.fn>;
    mockBalance.mockResolvedValueOnce({
      balance_minor: 50000,
      currency: 'IDR',
      status: 'active',
    });

    render(wrap(<GiftCardPayment {...defaultProps} />));
    const input = screen.getByLabelText('Gift card number');
    await userEvent.type(input, 'GC-TEST');

    await userEvent.click(screen.getByRole('button', { name: /check/i }));

    await vi.waitFor(() => {
      expect(mockBalance).toHaveBeenCalledWith('GC-TEST');
      expect(screen.getByText('Available Balance')).toBeInTheDocument();
      expect(screen.getByText('To Apply')).toBeInTheDocument();
    });
  });

  it('shows error when gift card is not found', async () => {
    const mockBalance = getGiftCardBalance as ReturnType<typeof vi.fn>;
    mockBalance.mockResolvedValueOnce(null);

    render(wrap(<GiftCardPayment {...defaultProps} />));
    const input = screen.getByLabelText('Gift card number');
    await userEvent.type(input, 'GC-BAD');
    await userEvent.click(screen.getByRole('button', { name: /check/i }));

    await vi.waitFor(() => {
      expect(screen.getByText('Gift card not found')).toBeInTheDocument();
    });
  });

  it('shows error when gift card is not active', async () => {
    const mockBalance = getGiftCardBalance as ReturnType<typeof vi.fn>;
    mockBalance.mockResolvedValueOnce({
      balance_minor: 1000,
      currency: 'IDR',
      status: 'frozen',
    });

    render(wrap(<GiftCardPayment {...defaultProps} />));
    const input = screen.getByLabelText('Gift card number');
    await userEvent.type(input, 'GC-FROZEN');
    await userEvent.click(screen.getByRole('button', { name: /check/i }));

    await vi.waitFor(() => {
      expect(screen.getByText('Gift card is not active')).toBeInTheDocument();
    });
  });

  it('shows balance and enables apply button after successful lookup', async () => {
    const mockBalance = getGiftCardBalance as ReturnType<typeof vi.fn>;
    mockBalance.mockResolvedValueOnce({
      balance_minor: 50000,
      currency: 'IDR',
      status: 'active',
    });

    render(wrap(<GiftCardPayment {...defaultProps} />));
    await userEvent.type(screen.getByLabelText('Gift card number'), 'GC-TEST');
    await userEvent.click(screen.getByRole('button', { name: /check/i }));

    await vi.waitFor(() => {
      expect(screen.getByRole('button', { name: /apply gift card/i })).toBeInTheDocument();
    });
  });

  it('applies gift card and calls onApplied and onComplete', async () => {
    const mockBalance = getGiftCardBalance as ReturnType<typeof vi.fn>;
    mockBalance.mockResolvedValueOnce({
      balance_minor: 50000,
      currency: 'IDR',
      status: 'active',
    });

    const mockRedeem = redeemGiftCard as ReturnType<typeof vi.fn>;
    mockRedeem.mockResolvedValueOnce({
      card: { id: 'gc-1', status: 'active' },
      transaction: { id: 'txn-1', txn_type: 'redeem' },
    });

    const onApplied = vi.fn();
    const onComplete = vi.fn();

    render(
      wrap(
        <GiftCardPayment
          {...defaultProps}
          totalMinor={30000}
          onApplied={onApplied}
          onComplete={onComplete}
        />,
      ),
    );

    await userEvent.type(screen.getByLabelText('Gift card number'), 'GC-TEST');
    await userEvent.click(screen.getByRole('button', { name: /check/i }));

    await vi.waitFor(() => {
      expect(screen.getByRole('button', { name: /apply gift card/i })).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /apply gift card/i }));

    await vi.waitFor(() => {
      expect(mockRedeem).toHaveBeenCalledWith('GC-TEST', 30000, 'sale-1');
      expect(onApplied).toHaveBeenCalledWith(30000, 'GC-TEST');
      expect(onComplete).toHaveBeenCalledTimes(1);
    });
  });

  it('calls onCancel when cancel is clicked', async () => {
    const onCancel = vi.fn();
    render(wrap(<GiftCardPayment {...defaultProps} onCancel={onCancel} />));
    await userEvent.click(screen.getByRole('button', { name: /cancel/i }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('disables the Check button when input is empty', () => {
    render(wrap(<GiftCardPayment {...defaultProps} />));
    expect(screen.getByRole('button', { name: /check/i })).toBeDisabled();
  });

  it('calls onError when redemption fails', async () => {
    const mockBalance = getGiftCardBalance as ReturnType<typeof vi.fn>;
    mockBalance.mockResolvedValueOnce({
      balance_minor: 50000,
      currency: 'IDR',
      status: 'active',
    });

    const mockRedeem = redeemGiftCard as ReturnType<typeof vi.fn>;
    mockRedeem.mockRejectedValueOnce(new Error('Redemption failed'));

    const onError = vi.fn();
    render(wrap(<GiftCardPayment {...defaultProps} onError={onError} />));

    await userEvent.type(screen.getByLabelText('Gift card number'), 'GC-TEST');
    await userEvent.click(screen.getByRole('button', { name: /check/i }));

    await vi.waitFor(() => {
      expect(screen.getByRole('button', { name: /apply gift card/i })).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /apply gift card/i }));

    await vi.waitFor(() => {
      expect(onError).toHaveBeenCalledWith('Redemption failed');
    });
  });
});
