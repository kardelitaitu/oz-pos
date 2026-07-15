import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import giftCardsFtl from '@/locales/gift-cards.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

vi.mock('@/api/giftCards', () => ({
  listGiftCards: vi.fn(),
  freezeGiftCard: vi.fn(),
  unfreezeGiftCard: vi.fn(),
  topUpGiftCard: vi.fn(),
}));

// Mock the IssueGiftCardModal child component.
vi.mock('@/features/gift-cards/IssueGiftCardModal', () => ({
  default: ({ onClose, onIssued }: { onClose: () => void; onIssued: () => void }) => (
    <div role="dialog" aria-label="issue-modal">
      <button onClick={onClose}>Close Issue</button>
      <button onClick={onIssued}>Card Issued</button>
    </div>
  ),
}));

import GiftCardsScreen from '@/features/gift-cards/GiftCardsScreen';
import { listGiftCards, freezeGiftCard, unfreezeGiftCard, topUpGiftCard } from '@/api/giftCards';

const mockListGiftCards = listGiftCards as ReturnType<typeof vi.fn>;
const mockFreezeGiftCard = freezeGiftCard as ReturnType<typeof vi.fn>;
const mockUnfreezeGiftCard = unfreezeGiftCard as ReturnType<typeof vi.fn>;
const mockTopUpGiftCard = topUpGiftCard as ReturnType<typeof vi.fn>;



const sampleCards = [
  {
    card: {
      id: 'gc-1', card_number: 'GC-001', pin: '1234',
      initial_balance_minor: 100000, current_balance_minor: 75000,
      currency: 'IDR', status: 'active', issued_to: 'Alice',
      issue_date: '2026-06-01', expiry_date: '2027-06-01',
      created_by: 'user-1', updated_at: '2026-06-01',
    },
    transactions: [
      { id: 'tx1', gift_card_id: 'gc-1', sale_id: 'sale-1', txn_type: 'redeem',
        amount_minor: -25000, balance_after_minor: 75000, notes: 'POS sale', created_at: '2026-06-15' },
    ],
  },
  {
    card: {
      id: 'gc-2', card_number: 'GC-002', pin: '5678',
      initial_balance_minor: 50000, current_balance_minor: 0,
      currency: 'IDR', status: 'redeemed', issued_to: null,
      issue_date: '2026-05-01', expiry_date: null,
      created_by: 'user-1', updated_at: '2026-05-20',
    },
    transactions: [
      { id: 'tx2', gift_card_id: 'gc-2', sale_id: 'sale-2', txn_type: 'redeem',
        amount_minor: -50000, balance_after_minor: 0, notes: '', created_at: '2026-05-15' },
    ],
  },
  {
    card: {
      id: 'gc-3', card_number: 'GC-003', pin: '9012',
      initial_balance_minor: 200000, current_balance_minor: 200000,
      currency: 'IDR', status: 'frozen', issued_to: 'Bob',
      issue_date: '2026-04-01', expiry_date: '2027-04-01',
      created_by: 'user-1', updated_at: '2026-06-10',
    },
    transactions: [],
  },
];

describe('GiftCardsScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── List rendering ───────────────────────────────────────────
  it('renders the title and Issue New Card button', async () => {
    mockListGiftCards.mockResolvedValue([]);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);
    expect(screen.getByText('Gift Cards')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /issue new card/i })).toBeInTheDocument();
  });

  it('loads and displays gift cards in the list', async () => {
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });
    expect(screen.getByText('GC-002')).toBeInTheDocument();
    expect(screen.getByText('GC-003')).toBeInTheDocument();
    expect(screen.getByText('Alice')).toBeInTheDocument();
  });

  it('shows loading state initially', async () => {
    mockListGiftCards.mockReturnValue(new Promise(() => {}));
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it('shows empty state when no cards exist', async () => {
    mockListGiftCards.mockResolvedValue([]);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/no gift cards found/i)).toBeInTheDocument();
    });
  });

  it('renders status badges for each card', async () => {
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('active')).toBeInTheDocument();
    });
    expect(screen.getByText('redeemed')).toBeInTheDocument();
    expect(screen.getByText('frozen')).toBeInTheDocument();
  });

  it('renders the search input and status filter dropdown', async () => {
    mockListGiftCards.mockResolvedValue([]);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    expect(screen.getByRole('textbox')).toBeInTheDocument();
    expect(screen.getByRole('combobox')).toBeInTheDocument();
    // Status filter options rendered from the <select>.
    expect(screen.getByText('All Statuses')).toBeInTheDocument();
  });

  // ── Expand/collapse ──────────────────────────────────────────
  it('expands card details when summary is clicked', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));

    await waitFor(() => {
      expect(screen.getByText('Initial Balance')).toBeInTheDocument();
    });
    expect(screen.getByText('Issued')).toBeInTheDocument();
    expect(screen.getByText('Expires')).toBeInTheDocument();
  });

  it('shows transaction table in expanded detail', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));

    await waitFor(() => {
      expect(screen.getByText('Recent Transactions')).toBeInTheDocument();
      expect(screen.getByText('redeem')).toBeInTheDocument();
    });
  });

  it('collapses detail when summary is clicked again', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));
    await waitFor(() => {
      expect(screen.getByText('Initial Balance')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));
    await waitFor(() => {
      expect(screen.queryByText('Initial Balance')).not.toBeInTheDocument();
    });
  });

  it('does not show transaction section for cards with no transactions', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue([sampleCards[2]!]);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-003')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-003'));

    await waitFor(() => {
      expect(screen.queryByText('Recent Transactions')).not.toBeInTheDocument();
    });
  });

  // ── Freeze/Unfreeze ──────────────────────────────────────────
  it('shows Freeze button for active cards in expanded view', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /freeze/i })).toBeInTheDocument();
    });
  });

  it('calls freezeGiftCard when Freeze is clicked', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    mockFreezeGiftCard.mockResolvedValue({});
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /freeze/i })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /freeze/i }));

    await waitFor(() => {
      expect(mockFreezeGiftCard).toHaveBeenCalledWith('GC-001');
    });
  });

  it('shows Unfreeze button for frozen cards', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-003')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-003'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /unfreeze/i })).toBeInTheDocument();
    });
  });

  it('calls unfreezeGiftCard when Unfreeze is clicked', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    mockUnfreezeGiftCard.mockResolvedValue({});
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-003')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-003'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /unfreeze/i })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /unfreeze/i }));

    await waitFor(() => {
      expect(mockUnfreezeGiftCard).toHaveBeenCalledWith('GC-003');
    });
  });

  it('does not show Freeze/Unfreeze for redeemed cards', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue([sampleCards[1]!]);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-002')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-002'));

    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /freeze/i })).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /unfreeze/i })).not.toBeInTheDocument();
    });
  });

  // ── Top-up ───────────────────────────────────────────────────
  it('shows Top Up button for active cards', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /top up/i })).toBeInTheDocument();
    });
  });

  it('opens top-up form when Top Up is clicked', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /top up/i })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /top up/i }));

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/amount/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /confirm top-up/i })).toBeInTheDocument();
    });
  });

  it('submits top-up and refreshes the list', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    mockTopUpGiftCard.mockResolvedValue({});
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /top up/i })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /top up/i }));

    const amountInput = screen.getByPlaceholderText(/amount/i);
    await user.type(amountInput, '50000');

    await waitFor(() => {
      expect(mockListGiftCards).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: /confirm top-up/i }));

    await waitFor(() => {
      expect(mockTopUpGiftCard).toHaveBeenCalledWith('GC-001', 50000);
    });
  });

  it('shows error when top-up amount is invalid', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /top up/i })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /top up/i }));

    await user.click(screen.getByRole('button', { name: /confirm top-up/i }));

    await waitFor(() => {
      // The Fluent key is now resolved from the bundle.
      expect(screen.getByText(/top-up amount must be positive/i)).toBeInTheDocument();
    });
    expect(mockTopUpGiftCard).not.toHaveBeenCalled();
  });

  it('closes top-up form when Cancel is clicked', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue(sampleCards);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('GC-001')).toBeInTheDocument();
    });

    await user.click(screen.getByText('GC-001'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /top up/i })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /top up/i }));

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/amount/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /cancel/i }));

    await waitFor(() => {
      expect(screen.queryByPlaceholderText(/amount/i)).not.toBeInTheDocument();
    });
  });

  // ── Issue modal ──────────────────────────────────────────────
  it('opens IssueGiftCardModal when Issue New Card is clicked', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue([]);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await user.click(screen.getByRole('button', { name: /issue new card/i }));

    await waitFor(() => {
      expect(screen.getByRole('dialog', { name: /issue-modal/i })).toBeInTheDocument();
    });
  });

  it('refreshes list when issue modal reports onIssued', async () => {
    const user = userEvent.setup();
    mockListGiftCards.mockResolvedValue([]);
    renderWithFluentSync(<GiftCardsScreen />, giftCardsFtl, sharedFtl);

    await user.click(screen.getByRole('button', { name: /issue new card/i }));

    await waitFor(() => {
      expect(screen.getByRole('dialog', { name: /issue-modal/i })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /card issued/i }));

    await waitFor(() => {
      expect(mockListGiftCards).toHaveBeenCalledTimes(2);
    });
  });
});
