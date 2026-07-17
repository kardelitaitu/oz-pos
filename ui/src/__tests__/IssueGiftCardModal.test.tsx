import { describe, expect, it, vi } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import giftCardsFtl from '@/locales/gift-cards.ftl?raw';


// Mock the API module before importing the component.
vi.mock('@/api/giftCards', () => ({
  issueGiftCard: vi.fn(),
}));

// Mock the barcode generator for deterministic card numbers.
vi.mock('@/utils/giftCardBarcode', () => ({
  generateGiftCardNumber: vi.fn(() => 'GC-TEST12345678'),
  isGiftCardBarcode: vi.fn(() => true),
}));

import IssueGiftCardModal from '@/features/gift-cards/IssueGiftCardModal';
import { issueGiftCard } from '@/api/giftCards';



describe('IssueGiftCardModal', () => {
  it('renders the form with default generated card number', () => {
    renderWithFluentSync(<IssueGiftCardModal onClose={vi.fn()} onIssued={vi.fn()} />, giftCardsFtl);
    expect(screen.getByText('Issue Gift Card')).toBeInTheDocument();
    expect(screen.getByLabelText('Card number')).toHaveValue('GC-TEST12345678');
    expect(screen.getByLabelText('Initial amount')).toBeInTheDocument();
    expect(screen.getByLabelText('Issued to')).toBeInTheDocument();
    expect(screen.getByLabelText('PIN')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /issue card/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
  });

  it('shows validation error when amount is empty', async () => {
    renderWithFluentSync(<IssueGiftCardModal onClose={vi.fn()} onIssued={vi.fn()} />, giftCardsFtl);
    await userEvent.click(screen.getByRole('button', { name: /issue card/i }));
    expect(screen.getByRole('alert')).toHaveTextContent('Amount must be positive');
  });

  it('shows validation error when amount is zero or negative', async () => {
    renderWithFluentSync(<IssueGiftCardModal onClose={vi.fn()} onIssued={vi.fn()} />, giftCardsFtl);
    const amountInput = screen.getByLabelText('Initial amount');
    await userEvent.type(amountInput, '0');
    await userEvent.click(screen.getByRole('button', { name: /issue card/i }));
    expect(screen.getByRole('alert')).toHaveTextContent('Amount must be positive');
  });

  it('shows validation error when card number is empty', async () => {
    renderWithFluentSync(<IssueGiftCardModal onClose={vi.fn()} onIssued={vi.fn()} />, giftCardsFtl);
    const cardInput = screen.getByLabelText('Card number');
    await userEvent.clear(cardInput);
    const amountInput = screen.getByLabelText('Initial amount');
    await userEvent.type(amountInput, '50000');
    await userEvent.click(screen.getByRole('button', { name: /issue card/i }));
    expect(screen.getByRole('alert')).toHaveTextContent('Invalid card number format');
  });

  it('calls onClose when cancel is clicked', async () => {
    const onClose = vi.fn();
    renderWithFluentSync(<IssueGiftCardModal onClose={onClose} onIssued={vi.fn()} />, giftCardsFtl);
    await userEvent.click(screen.getByRole('button', { name: /cancel/i }));
    await vi.waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  it('calls issueGiftCard and onIssued on successful submit', async () => {
    const mockIssue = issueGiftCard as ReturnType<typeof vi.fn>;
    mockIssue.mockResolvedValueOnce(undefined);
    const onIssued = vi.fn();

    renderWithFluentSync(<IssueGiftCardModal onClose={vi.fn()} onIssued={onIssued} />, giftCardsFtl);
    const amountInput = screen.getByLabelText('Initial amount');
    await userEvent.type(amountInput, '50000');
    await userEvent.click(screen.getByRole('button', { name: /issue card/i }));

    expect(mockIssue).toHaveBeenCalledTimes(1);
    expect(mockIssue).toHaveBeenCalledWith(
      expect.objectContaining({
        card_number: 'GC-TEST12345678',
        initial_amount_minor: 50000,
        currency: 'IDR',
      }),
    );

    // Wait for the async handler to resolve.
    await vi.waitFor(() => {
      expect(onIssued).toHaveBeenCalledTimes(1);
    });
  });

  it('shows error when issueGiftCard fails', async () => {
    const mockIssue = issueGiftCard as ReturnType<typeof vi.fn>;
    mockIssue.mockRejectedValueOnce(new Error('Network error'));

    renderWithFluentSync(<IssueGiftCardModal onClose={vi.fn()} onIssued={vi.fn()} />, giftCardsFtl);
    const amountInput = screen.getByLabelText('Initial amount');
    await userEvent.type(amountInput, '50000');
    await userEvent.click(screen.getByRole('button', { name: /issue card/i }));

    await vi.waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Network error');
    });
  });

  it('allows entering an optional customer name', async () => {
    renderWithFluentSync(<IssueGiftCardModal onClose={vi.fn()} onIssued={vi.fn()} />, giftCardsFtl);
    const nameInput = screen.getByLabelText('Issued to');
    await userEvent.type(nameInput, 'Alice');
    expect(nameInput).toHaveValue('Alice');
  });

  it('allows editing the card number manually', async () => {
    renderWithFluentSync(<IssueGiftCardModal onClose={vi.fn()} onIssued={vi.fn()} />, giftCardsFtl);
    const cardInput = screen.getByLabelText('Card number');
    await userEvent.clear(cardInput);
    await userEvent.type(cardInput, 'GC-MYCARD');
    expect(cardInput).toHaveValue('GC-MYCARD');
  });
});
