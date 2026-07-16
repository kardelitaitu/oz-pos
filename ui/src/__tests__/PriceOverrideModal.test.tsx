import { describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { PriceOverrideModalProps } from '@/features/sales/PriceOverrideModal';

vi.mock('@/api/staff', () => ({
  staffLogin: vi.fn(),
}));

import PriceOverrideModal from '@/features/sales/PriceOverrideModal';
import { staffLogin } from '@/api/staff';

const mockStaffLogin = staffLogin as ReturnType<typeof vi.fn>;

const defaultProps: PriceOverrideModalProps = {
  open: true,
  lineDescription: 'Widget x 2',
  currentPrice: { minor_units: 50000, currency: 'IDR' },
  onConfirm: vi.fn().mockResolvedValue(undefined),
  onClose: vi.fn(),
};

function renderModal(props: Partial<PriceOverrideModalProps> = {}) {
  return render(<PriceOverrideModal {...defaultProps} {...props} />);
}

describe('PriceOverrideModal', () => {
  // ── Closed state ─────────────────────────────────────────────

  it('renders nothing when open is false', () => {
    render(<PriceOverrideModal {...defaultProps} open={false} />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  // ── Price step ───────────────────────────────────────────────

  it('renders the modal with title and current price', () => {
    renderModal();
    expect(screen.getByText('Price Override')).toBeInTheDocument();
    expect(screen.getByText('Widget x 2')).toBeInTheDocument();
    expect(screen.getByText('Current price')).toBeInTheDocument();
  });

  it('shows price input with current value pre-filled', () => {
    renderModal();
    const input = screen.getByLabelText('Enter new price in minor units') as HTMLInputElement;
    expect(input.value).toBe('50000');
  });

  it('disables Next button when price is zero or negative', () => {
    renderModal();
    const nextBtn = screen.getByText('Next');

    // Initially enabled (50000 > 0).
    expect(nextBtn).toBeEnabled();
  });

  it('closes modal when Cancel is clicked on price step', async () => {
    const user = userEvent.setup();
    renderModal();
    await user.click(screen.getByText('Cancel'));
    await vi.waitFor(() => {
      expect(defaultProps.onClose).toHaveBeenCalled();
    });
  });

  // ── Username step ────────────────────────────────────────────

  it('advances to username step when Next is clicked with valid price', async () => {
    const user = userEvent.setup();
    renderModal();
    await user.click(screen.getByText('Next'));

    await waitFor(() => {
      expect(screen.getByText('Enter manager username')).toBeInTheDocument();
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });
  });

  it('disables username Next when username is empty', async () => {
    const user = userEvent.setup();
    renderModal();
    await user.click(screen.getByText('Next'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });

    // Next on username step should be disabled when empty.
    const nextBtns = screen.getAllByText('Next');
    expect(nextBtns[nextBtns.length - 1]).toBeDisabled();
  });

  it('goes back to price step when Back is clicked', async () => {
    const user = userEvent.setup();
    renderModal();
    await user.click(screen.getByText('Next'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Back'));

    await waitFor(() => {
      expect(screen.getByText('Current price')).toBeInTheDocument();
    });
  });

  // ── PIN step ─────────────────────────────────────────────────

  it('advances to PIN step after entering username', async () => {
    const user = userEvent.setup();
    renderModal();

    // Step 1: price.
    await user.click(screen.getByText('Next'));

    // Step 2: username.
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);

    // Step 3: PIN.
    await waitFor(() => {
      expect(screen.getByText('Enter manager PIN')).toBeInTheDocument();
      // Should show 4 PIN dots.
      const dots = document.querySelectorAll('.price-override-pin-dot');
      expect(dots.length).toBe(4);
    });
  });

  it('fills PIN dots when digits are pressed', async () => {
    const user = userEvent.setup();
    renderModal();
    await user.click(screen.getByText('Next'));
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);

    await waitFor(() => {
      expect(screen.getByText('Enter manager PIN')).toBeInTheDocument();
    });

    // Press some PIN digits.
    await user.click(screen.getByText('1'));
    await user.click(screen.getByText('2'));
    await user.click(screen.getByText('3'));

    const filledDots = document.querySelectorAll('.price-override-pin-dot--filled');
    expect(filledDots.length).toBe(3);
  });

  it('calls onConfirm when PIN reaches 4 digits and login succeeds', async () => {
    const user = userEvent.setup();
    mockStaffLogin.mockResolvedValue({ session: { user_id: 'user-99' } });
    renderModal();

    await user.click(screen.getByText('Next'));
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);

    await waitFor(() => {
      expect(screen.getByText('Enter manager PIN')).toBeInTheDocument();
    });

    // Enter 4-digit PIN.
    await user.click(screen.getByText('1'));
    await user.click(screen.getByText('2'));
    await user.click(screen.getByText('3'));
    await user.click(screen.getByText('4'));

    await waitFor(() => {
      expect(mockStaffLogin).toHaveBeenCalledWith({ username: 'manager', pin: '1234' });
      expect(defaultProps.onConfirm).toHaveBeenCalledWith(50000, 'user-99');
    });
  });

  it('shows error when PIN login fails', async () => {
    const user = userEvent.setup();
    mockStaffLogin.mockRejectedValue(new Error('Invalid PIN'));
    renderModal();

    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);

    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    // Enter full 4-digit PIN.
    for (const d of ['1', '2', '3', '4']) {
      await user.click(screen.getByText(d));
    }

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText('Invalid PIN')).toBeInTheDocument();
    });
  });

  it('clears PIN with Clear button', async () => {
    const user = userEvent.setup();
    renderModal();
    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);

    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    await user.click(screen.getByText('1'));
    await user.click(screen.getByText('2'));
    expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(2);

    await user.click(screen.getByText('Clear'));
    expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(0);
  });

  it('removes last digit with backspace', async () => {
    const user = userEvent.setup();
    renderModal();
    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);

    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    await user.click(screen.getByText('1'));
    await user.click(screen.getByText('2'));
    await user.click(screen.getByText('⌫'));
    expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(1);
  });

  it('shows verifying status while login is in progress', async () => {
    const user = userEvent.setup();
    mockStaffLogin.mockReturnValue(new Promise(() => {})); // never resolves
    renderModal();

    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);

    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    for (const d of ['1', '2', '3', '4', '5', '6']) {
      await user.click(screen.getByText(d));
    }

    await waitFor(() => {
      expect(screen.getByRole('status')).toBeInTheDocument();
      expect(screen.getByText('Verifying…')).toBeInTheDocument();
    });
  });
});
