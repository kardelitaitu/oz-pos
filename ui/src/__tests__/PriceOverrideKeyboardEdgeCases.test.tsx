import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
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

describe('PriceOverrideModal — keyboard and edge cases', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default: login succeeds
    mockStaffLogin.mockResolvedValue({ session: { user_id: 'user-99' } });
  });

  // ── Price step: validation edge cases ────────────────────────

  it('disables Next when currentPrice is already zero', () => {
    render(
      <PriceOverrideModal
        {...defaultProps}
        currentPrice={{ minor_units: 0, currency: 'IDR' }}
      />
    );
    expect(screen.getByText('Next')).toBeDisabled();
  });

  it('disables Next when currentPrice is negative', () => {
    render(
      <PriceOverrideModal
        {...defaultProps}
        currentPrice={{ minor_units: -1, currency: 'IDR' }}
      />
    );
    expect(screen.getByText('Next')).toBeDisabled();
  });

  it('clamps negative typed value to minimum of 1 via onChange', () => {
    renderModal();
    const input = screen.getByLabelText('Enter new price in minor units') as HTMLInputElement;

    // Use fireEvent.change for deterministic controlled input behavior
    fireEvent.change(input, { target: { value: '-1' } });
    expect(screen.getByText('Next')).toBeEnabled();
  });

  // ── Username step: keyboard + edge cases ────────────────────

  it('disables PIN step back button and shows status during loading', async () => {
    const user = userEvent.setup();
    // onConfirm never resolves to simulate loading
    renderModal({ onConfirm: vi.fn().mockReturnValue(new Promise<void>(() => {})) });

    // Price step → next
    await user.click(screen.getByText('Next'));
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });

    // Type username and click Next
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);

    // Now on PIN step
    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    // Enter 4 digits to trigger auto-submit and loading state
    for (const d of ['1', '2', '3', '4']) {
      await user.click(screen.getByText(d));
    }

    await waitFor(() => {
      // Back button should be disabled during loading
      expect(screen.getByText('Back')).toBeDisabled();
      // Verify status indicator
      expect(screen.getByRole('status')).toBeInTheDocument();
    });
  });

  it('submits username form with Enter key and advances to PIN step', async () => {
    const user = userEvent.setup();
    renderModal();

    // Price step → next
    await user.click(screen.getByText('Next'));
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });

    // Type username and press Enter (inside <form> so it triggers onSubmit)
    await user.type(screen.getByPlaceholderText('Username'), 'manager{Enter}');

    await waitFor(() => {
      expect(screen.getByText('Enter manager PIN')).toBeInTheDocument();
    });
  });

  // ── PIN step: hardware keyboard ────────────────────────────

  it('accepts digit keys from hardware keyboard on PIN step', async () => {
    const user = userEvent.setup();
    renderModal();

    // Advance to PIN step
    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);
    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    // Type digits via hardware keyboard
    await user.keyboard('123');

    await waitFor(() => {
      const filledDots = document.querySelectorAll('.price-override-pin-dot--filled');
      expect(filledDots.length).toBe(3);
    });
  });

  it('handles Backspace key on PIN step via keyboard', async () => {
    const user = userEvent.setup();
    renderModal();

    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);
    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    // Type two digits via keyboard
    await user.keyboard('12');
    expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(2);

    // Backspace
    await user.keyboard('{Backspace}');
    expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(1);
  });

  it('fills PIN on the price-override-pin-step element focus', async () => {
    const user = userEvent.setup();
    renderModal();

    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);
    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    // The pin step should have focus (auto-focused via useEffect)
    const pinStep = document.querySelector('.price-override-pin-step');
    expect(pinStep).toBe(document.activeElement);

    // Type digits directly on the focused element
    await user.keyboard('789');

    await waitFor(() => {
      const filledDots = document.querySelectorAll('.price-override-pin-dot--filled');
      expect(filledDots.length).toBe(3);
    });
  });

  // ── PIN step: Escape key (handled by focus trap) ────────────

  it('closes modal when Escape is pressed on PIN step', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderModal({ onClose });

    // Advance to PIN step
    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);
    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    // Escape closes modal via global keydown listener in useFocusTrap
    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });

  // ── Error handling edge cases ───────────────────────────────

  it('shows error and clears PIN when onConfirm rejects', async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn().mockRejectedValue(new Error('Server rejected override'));
    renderModal({ onConfirm });

    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);
    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    // Enter 4-digit PIN (auto-submits at 4 digits)
    for (const d of ['1', '2', '3', '4']) {
      await user.click(screen.getByText(d));
    }

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText('Server rejected override')).toBeInTheDocument();
      // PIN should be cleared so user can retry
      expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(0);
    });
  });

  it('displays fallback error message when onConfirm rejects with non-Error', async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn().mockRejectedValue('string error');
    renderModal({ onConfirm });

    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);
    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    for (const d of ['1', '2', '3', '4']) {
      await user.click(screen.getByText(d));
    }

    await waitFor(() => {
      expect(screen.getByText('PIN verification failed')).toBeInTheDocument();
    });
  });

  it('clears error when navigating back from PIN step to username step', async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn().mockRejectedValue(new Error('Override rejected'));
    renderModal({ onConfirm });

    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);
    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    // Trigger error
    for (const d of ['1', '2', '3', '4']) {
      await user.click(screen.getByText(d));
    }

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText('Override rejected')).toBeInTheDocument();
    });

    // Going back should clear the error (fixed: handleGoBack now calls setError(null))
    await user.click(screen.getByText('Back'));
    await waitFor(() => {
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });
  });

  // ── Edge cases ──────────────────────────────────────────────

  it('closes modal and fires onClose when close button is clicked', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderModal({ onClose });
    await user.click(screen.getByText('×'));
    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('resets price input to currentPrice on reopen after cancel', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const { unmount } = render(
      <PriceOverrideModal {...defaultProps} open={true} onClose={onClose} />
    );

    // Use fireEvent.change for reliable controlled input updates
    const input = screen.getByLabelText('Enter new price in minor units') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '75000' } });

    await waitFor(() => {
      expect(input.value).toBe('75000');
    });

    // Cancel to close
    await user.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });

    // Unmount + remount to simulate real usage (parent conditional render)
    unmount();
    render(<PriceOverrideModal {...defaultProps} open={true} onClose={onClose} />);

    await waitFor(() => {
      const newInput = screen.getByLabelText('Enter new price in minor units') as HTMLInputElement;
      // Fresh mount means useState(currentPrice.minor_units) reinitializes to 50000
      expect(newInput.value).toBe('50000');
    });
  });

  it('closes modal when close button clicked on username step', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderModal({ onClose });

    // Advance to username step
    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());

    // Close button should still work
    await user.click(screen.getByText('×'));
    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('resets PIN state when going back from PIN step to username step', async () => {
    const user = userEvent.setup();
    renderModal();

    // Advance to PIN step
    await user.click(screen.getByText('Next'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());
    await user.type(screen.getByPlaceholderText('Username'), 'manager');
    await user.click(screen.getAllByText('Next').at(-1)!);
    await waitFor(() => expect(screen.getByText('Enter manager PIN')).toBeInTheDocument());

    // Enter some PIN digits
    await user.click(screen.getByText('1'));
    await user.click(screen.getByText('2'));
    expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(2);

    // Go back to username
    await user.click(screen.getByText('Back'));
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());

    // Advance back to PIN — should be reset
    await user.click(screen.getAllByText('Next').at(-1)!);
    await waitFor(() => {
      expect(screen.getByText('Enter manager PIN')).toBeInTheDocument();
      expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(0);
    });
  });
});
