// ── PriceOverrideModal keyboard and edge-case tests ──────────────
//
// Covers: username/PIN step navigation, hardware keyboard input,
// error handling, and modal lifecycle edge cases. Uses fireEvent.click
// for navigation buttons (faster than userEvent.click) and userEvent
// only where keyboard input simulation is needed.
// 13 tests (3 fast sync price-step tests moved to PriceOverridePriceStep.test.tsx).

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

// ── Navigation helpers (fireEvent.click ~1ms vs userEvent.click ~80ms) ─

function clickNext() {
  fireEvent.click(screen.getByText('Next'));
}

function clickBack() {
  fireEvent.click(screen.getByText('Back'));
}

function clickClose() {
  fireEvent.click(screen.getByText('\u00d7')); // × character
}

async function advanceToUsernameStep() {
  clickNext();
  await waitFor(() => {
    expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
  });
}

async function advanceToPinStep(username = 'manager') {
  await advanceToUsernameStep();
  await userEvent.type(screen.getByPlaceholderText('Username'), username);
  clickNext();
  await waitFor(() => {
    expect(screen.getByText(`Enter ${username} PIN`)).toBeInTheDocument();
  });
}

// ── Tests ─────────────────────────────────────────────────────────

describe('PriceOverrideModal — keyboard and edge cases', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockStaffLogin.mockResolvedValue({ session: { user_id: 'user-99' } });
  });

  // ── Username step: keyboard + edge cases ────────────────────

  it('disables PIN step back button and shows status during loading', async () => {
    renderModal({ onConfirm: vi.fn().mockReturnValue(new Promise<void>(() => {})) });

    await advanceToPinStep();

    for (const d of ['1', '2', '3', '4']) {
      fireEvent.click(screen.getByText(d));
    }

    await waitFor(() => {
      expect(screen.getByText('Back')).toBeDisabled();
      expect(screen.getByRole('status')).toBeInTheDocument();
    });
  });

  it('submits username form with Enter key and advances to PIN step', async () => {
    renderModal();

    await advanceToUsernameStep();

    await userEvent.type(screen.getByPlaceholderText('Username'), 'manager{Enter}');

    await waitFor(() => {
      expect(screen.getByText('Enter manager PIN')).toBeInTheDocument();
    });
  });

  // ── PIN step: hardware keyboard ────────────────────────────

  it('accepts digit keys from hardware keyboard on PIN step', async () => {
    const user = userEvent.setup();
    renderModal();

    await advanceToPinStep();
    await user.keyboard('123');

    await waitFor(() => {
      const filledDots = document.querySelectorAll('.price-override-pin-dot--filled');
      expect(filledDots.length).toBe(3);
    });
  });

  it('handles Backspace key on PIN step via keyboard', async () => {
    const user = userEvent.setup();
    renderModal();

    await advanceToPinStep();

    await user.keyboard('12');
    expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(2);

    await user.keyboard('{Backspace}');
    expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(1);
  });

  it('fills PIN on the price-override-pin-step element focus', async () => {
    const user = userEvent.setup();
    renderModal();

    await advanceToPinStep();

    const pinStep = document.querySelector('.price-override-pin-step');
    expect(pinStep).toBe(document.activeElement);

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

    await advanceToPinStep();

    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });

  // ── Error handling edge cases ───────────────────────────────

  it('shows error and clears PIN when onConfirm rejects', async () => {
    const onConfirm = vi.fn().mockRejectedValue(new Error('Server rejected override'));
    renderModal({ onConfirm });

    await advanceToPinStep();

    for (const d of ['1', '2', '3', '4']) {
      fireEvent.click(screen.getByText(d));
    }

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText('Server rejected override')).toBeInTheDocument();
      expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(0);
    });
  });

  it('displays fallback error message when onConfirm rejects with non-Error', async () => {
    const onConfirm = vi.fn().mockRejectedValue('string error');
    renderModal({ onConfirm });

    await advanceToPinStep();

    for (const d of ['1', '2', '3', '4']) {
      fireEvent.click(screen.getByText(d));
    }

    await waitFor(() => {
      expect(screen.getByText('PIN verification failed')).toBeInTheDocument();
    });
  });

  it('clears error when navigating back from PIN step to username step', async () => {
    const onConfirm = vi.fn().mockRejectedValue(new Error('Override rejected'));
    renderModal({ onConfirm });

    await advanceToPinStep();

    for (const d of ['1', '2', '3', '4']) {
      fireEvent.click(screen.getByText(d));
    }

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.getByText('Override rejected')).toBeInTheDocument();
    });

    clickBack();
    await waitFor(() => {
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });
  });

  // ── Edge cases ──────────────────────────────────────────────

  it('closes modal and fires onClose when close button is clicked', async () => {
    const onClose = vi.fn();
    renderModal({ onClose });
    clickClose();
    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('resets price input to currentPrice on reopen after cancel', async () => {
    const onClose = vi.fn();
    const { unmount } = render(
      <PriceOverrideModal {...defaultProps} open={true} onClose={onClose} />,
    );

    const input = screen.getByLabelText('Enter new price in minor units') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '75000' } });

    await waitFor(() => {
      expect(input.value).toBe('75000');
    });

    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });

    unmount();
    render(<PriceOverrideModal {...defaultProps} open={true} onClose={onClose} />);

    await waitFor(() => {
      const newInput = screen.getByLabelText('Enter new price in minor units') as HTMLInputElement;
      expect(newInput.value).toBe('50000');
    });
  });

  it('closes modal when close button clicked on username step', async () => {
    const onClose = vi.fn();
    renderModal({ onClose });

    await advanceToUsernameStep();

    clickClose();
    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('resets PIN state when going back from PIN step to username step', async () => {
    renderModal();

    await advanceToPinStep();

    fireEvent.click(screen.getByText('1'));
    fireEvent.click(screen.getByText('2'));
    expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(2);

    clickBack();
    await waitFor(() => expect(screen.getByPlaceholderText('Username')).toBeInTheDocument());

    clickNext();
    await waitFor(() => {
      expect(screen.getByText('Enter manager PIN')).toBeInTheDocument();
      expect(document.querySelectorAll('.price-override-pin-dot--filled').length).toBe(0);
    });
  });
});
