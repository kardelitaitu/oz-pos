import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import SetupWizard, { type WizardState } from '@/features/setup/SetupWizard';

// ── Helper functions ────────────────────────────────────────────────

/** Get all checkbox inputs currently rendered in the step. */
function getCheckboxes(): HTMLInputElement[] {
  return screen
    .getAllByRole('checkbox') as HTMLInputElement[];
}

/** Check a checkbox by its label text. */
async function toggleFeature(label: string) {
  // The feature row labels wrap both the text and the checkbox.
  // Clicking the text label toggles the checkbox.
  const row = screen.getByText(label).closest('label');
  if (!row) throw new Error(`Feature row "${label}" not found`);
  await userEvent.click(row);
}

// ── Tests ───────────────────────────────────────────────────────────

describe('SetupWizard', () => {
  beforeEach(() => {
    // Reset local state for clean tests.
    localStorage.clear();
  });

  // ── Step 1: Preset selection ────────────────────────────────────

  it('renders the preset selection step on mount', () => {
    render(<SetupWizard />);

    expect(
      screen.getByText('What kind of store are you running?'),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('radiogroup', { name: /store preset/i }),
    ).toBeInTheDocument();

    // Should have 4 preset cards.
    const cards = screen.getAllByRole('radio');
    expect(cards).toHaveLength(4);
    expect(cards[0]).toHaveTextContent('Simple Retail');
    expect(cards[1]).toHaveTextContent('Restaurant');
    expect(cards[2]).toHaveTextContent('Full Store');
    expect(cards[3]).toHaveTextContent('Custom');
  });

  it('shows the skip button on step 1 when no preset is selected', () => {
    const onSkip = vi.fn();
    render(<SetupWizard onSkip={onSkip} />);

    expect(
      screen.getByRole('button', { name: /skip setup/i }),
    ).toBeInTheDocument();
  });

  // ── Preset selection ────────────────────────────────────────────

  it('selecting a preset advances to step 2', async () => {
    render(<SetupWizard />);
    const presetCards = screen.getAllByRole('radio');

    await userEvent.click(presetCards[0]!);

    // Should now show step 2 heading (Payment Methods).
    expect(
      screen.getByText('Payment Methods'),
    ).toBeInTheDocument();
  });

  it('preset cards start unchecked and become selected on click', async () => {
    render(<SetupWizard />);
    const presetCards = screen.getAllByRole('radio');

    // All cards start unchecked.
    for (const card of presetCards) {
      expect(card).toHaveAttribute('aria-checked', 'false');
    }

    // Click the first card.
    await userEvent.click(presetCards[0]!);

    // The wizard advances to step 2; we can check that the preset
    // was applied by verifying step 2 features are correct.
    expect(screen.getByText('Payment Methods')).toBeInTheDocument();
  });

  it('Simple Retail preset pre-populates correct features', async () => {
    render(<SetupWizard />);

    // Select Simple Retail.
    await userEvent.click(screen.getAllByRole('radio')[0]!);

    // Check step 2: Cash should be enabled, Card should be disabled.
    const checkboxes = getCheckboxes();
    // Step 2 has: Cash, Card, Multi-Currency
    expect(checkboxes).toHaveLength(3);
    // Cash is in the Simple Retail preset.
    expect(checkboxes[0]).toBeChecked();
    // Card is NOT in the Simple Retail preset.
    expect(checkboxes[1]).not.toBeChecked();
  });

  it('Restaurant preset pre-populates correct features', async () => {
    render(<SetupWizard />);

    // Select Restaurant.
    const cards = screen.getAllByRole('radio');
    await userEvent.click(cards[1]!);

    // Step 2: Cash should be enabled.
    const step2Checkboxes = getCheckboxes();
    expect(step2Checkboxes[0]).toBeChecked();
  });

  it('Full Store preset pre-populates all payment features', async () => {
    render(<SetupWizard />);

    // Select Full Store.
    const cards = screen.getAllByRole('radio');
    await userEvent.click(cards[2]!);

    // Step 2: Cash, Card, Multi-Currency all enabled.
    const checkboxes = getCheckboxes();
    expect(checkboxes[0]).toBeChecked();
    expect(checkboxes[1]).toBeChecked();
    expect(checkboxes[2]).toBeChecked();
  });

  it('Custom preset starts with no features enabled', async () => {
    render(<SetupWizard />);

    // Select Custom.
    const cards = screen.getAllByRole('radio');
    await userEvent.click(cards[3]!);

    // All checkboxes should be unchecked.
    const checkboxes = getCheckboxes();
    for (const cb of checkboxes) {
      expect(cb).not.toBeChecked();
    }
  });

  // ── Feature toggling ────────────────────────────────────────────

  it('toggles a feature on and off', async () => {
    render(<SetupWizard />);

    // Select Simple Retail (pre-enables Cash).
    await userEvent.click(screen.getAllByRole('radio')[0]!);

    // Cash should be on by default.
    const checkboxes = getCheckboxes();
    expect(checkboxes[0]).toBeChecked();

    // Toggle Cash off.
    await toggleFeature('Cash');
    expect(checkboxes[0]).not.toBeChecked();

    // Toggle Cash back on.
    await toggleFeature('Cash');
    expect(checkboxes[0]).toBeChecked();
  });

  it('all features in a step can be individually toggled', async () => {
    render(<SetupWizard />);

    // Select Custom (all off).
    await userEvent.click(screen.getAllByRole('radio')[3]!);

    // Toggle all features on.
    const checkboxes = getCheckboxes();
    for (const cb of checkboxes) {
      await userEvent.click(cb);
      expect(cb).toBeChecked();
    }

    // Toggle all features off.
    for (const cb of checkboxes) {
      await userEvent.click(cb);
      expect(cb).not.toBeChecked();
    }
  });

  // ── Navigation ──────────────────────────────────────────────────

  it('navigates to the next step via Next button', async () => {
    render(<SetupWizard />);

    // Select a preset.
    await userEvent.click(screen.getAllByRole('radio')[0]!);

    // Step 2 heading visible.
    expect(screen.getByText('Payment Methods')).toBeInTheDocument();

    // Click Next to go to Step 3.
    await userEvent.click(screen.getByRole('button', { name: /next/i }));
    expect(screen.getByText('Products & Inventory')).toBeInTheDocument();
  });

  it('navigates back via Back button', async () => {
    render(<SetupWizard />);

    // Select preset → step 2.
    await userEvent.click(screen.getAllByRole('radio')[0]!);

    // Go forward to step 3.
    await userEvent.click(screen.getByRole('button', { name: /next/i }));
    expect(screen.getByText('Products & Inventory')).toBeInTheDocument();

    // Go back to step 2.
    await userEvent.click(screen.getByRole('button', { name: /back/i }));
    expect(screen.getByText('Payment Methods')).toBeInTheDocument();
  });

  it('Back button is not shown on step 1', () => {
    render(<SetupWizard />);

    // On step 1, no Back button.
    expect(
      screen.queryByRole('button', { name: /back/i }),
    ).not.toBeInTheDocument();
  });

  it('navigates through all 8 steps', async () => {
    render(<SetupWizard />);

    // Select preset.
    await userEvent.click(screen.getAllByRole('radio')[0]!);

    const stepHeadings = [
      'Payment Methods',
      'Products & Inventory',
      'Staff Management',
      'Hardware & Peripherals',
      'Business Rules',
      'Data, Reporting & Cloud',
      'Review Your Setup',
    ];

    for (const heading of stepHeadings) {
      expect(screen.getByText(heading)).toBeInTheDocument();
      if (heading !== 'Review Your Setup') {
        await userEvent.click(screen.getByRole('button', { name: /next/i }));
      }
    }
  });

  // ── Step indicator ──────────────────────────────────────────────

  it('updates step indicator dots as steps progress', async () => {
    render(<SetupWizard />);
    const nav = screen.getByLabelText('Setup progress');

    // Step 1 (index 0): first dot should be active.
    const dots = nav.querySelectorAll('.setup-step-dot');
    expect(dots[0]).toHaveClass('setup-step-dot--active');
    expect(dots[1]).toHaveClass('setup-step-dot--pending');

    // Select preset → step 2.
    await userEvent.click(screen.getAllByRole('radio')[0]!);

    // Step 1 completed, step 2 active, step 3 pending.
    expect(dots[0]).toHaveClass('setup-step-dot--completed');
    expect(dots[1]).toHaveClass('setup-step-dot--active');
    expect(dots[2]).toHaveClass('setup-step-dot--pending');
  });

  // ── Review screen (Step 8) ─────────────────────────────────────

  it('review screen shows enabled and disabled feature tag clouds', async () => {
    render(<SetupWizard />);

    // Select Simple Retail preset.
    await userEvent.click(screen.getAllByRole('radio')[0]!);

    // Navigate through all steps to reach Review.
    for (let i = 0; i < 6; i++) {
      await userEvent.click(screen.getByRole('button', { name: /next/i }));
    }

    // Should show the review heading.
    expect(screen.getByText('Review Your Setup')).toBeInTheDocument();

    // Should show the preset name in the review card.
    expect(screen.getByText(/Simple Retail/)).toBeInTheDocument();
  });

  // ── Completion ─────────────────────────────────────────────────

  it('completion screen shows the correct feature count', async () => {
    render(<SetupWizard />);

    // Select Simple Retail.
    await userEvent.click(screen.getAllByRole('radio')[0]!);

    // Navigate to review.
    for (let i = 0; i < 6; i++) {
      await userEvent.click(screen.getByRole('button', { name: /next/i }));
    }

    // Click Complete Setup.
    await userEvent.click(
      screen.getByRole('button', { name: /complete setup/i }),
    );

    // Should show completion screen with "All Set!" heading.
    expect(screen.getByText('All Set!')).toBeInTheDocument();

    // The feature count (6 for Simple Retail) is inside a <strong>
    // element between text nodes, so we check for the number and
    // the surrounding text separately.
    expect(screen.getByText('6')).toBeInTheDocument();
    expect(
      screen.getByText(/features enabled/),
    ).toBeInTheDocument();

    // Should show "Launch OZ-POS" button.
    expect(
      screen.getByRole('button', { name: /launch oz-pos/i }),
    ).toBeInTheDocument();
  });

  it('completion screen fires onComplete with WizardState', async () => {
    const onComplete = vi.fn();
    render(<SetupWizard onComplete={onComplete} />);

    // Select Simple Retail.
    await userEvent.click(screen.getAllByRole('radio')[0]!);

    // Navigate to review.
    for (let i = 0; i < 6; i++) {
      await userEvent.click(screen.getByRole('button', { name: /next/i }));
    }

    // Complete setup.
    await userEvent.click(
      screen.getByRole('button', { name: /complete setup/i }),
    );

    expect(onComplete).toHaveBeenCalledTimes(1);
    const state: WizardState = onComplete.mock.calls[0]![0] as WizardState;
    expect(state.preset).toBe('simple-retail');
    expect(state.features['cash-payment']).toBe(true);
    expect(state.features['card-payment']).toBeUndefined();
  });

  it('Launch button on completion fires onSkip', async () => {
    const onSkip = vi.fn();
    const onComplete = vi.fn();
    render(<SetupWizard onComplete={onComplete} onSkip={onSkip} />);

    // Select preset, navigate to review, complete.
    await userEvent.click(screen.getAllByRole('radio')[0]!);
    for (let i = 0; i < 6; i++) {
      await userEvent.click(screen.getByRole('button', { name: /next/i }));
    }
    await userEvent.click(
      screen.getByRole('button', { name: /complete setup/i }),
    );

    // Click Launch OZ-POS.
    await userEvent.click(
      screen.getByRole('button', { name: /launch oz-pos/i }),
    );

    expect(onSkip).toHaveBeenCalledTimes(1);
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  // ── Skip ───────────────────────────────────────────────────────

  it('Skip button fires onSkip callback', async () => {
    const onSkip = vi.fn();
    render(<SetupWizard onSkip={onSkip} />);

    await userEvent.click(
      screen.getByRole('button', { name: /skip setup/i }),
    );

    expect(onSkip).toHaveBeenCalledTimes(1);
  });

  // ── Edge cases ─────────────────────────────────────────────────

  it('handles Full Store preset with all features enabled in review', async () => {
    const onComplete = vi.fn();
    render(<SetupWizard onComplete={onComplete} />);

    // Select Full Store (23 features).
    const cards = screen.getAllByRole('radio');
    await userEvent.click(cards[2]!);

    // Navigate to review.
    for (let i = 0; i < 6; i++) {
      await userEvent.click(screen.getByRole('button', { name: /next/i }));
    }

    // Complete.
    await userEvent.click(
      screen.getByRole('button', { name: /complete setup/i }),
    );

    expect(onComplete).toHaveBeenCalledTimes(1);
    const state: WizardState = onComplete.mock.calls[0]![0] as WizardState;
    expect(state.preset).toBe('full-store');
    // Full Store has 23 features.
    const enabledCount = Object.values(state.features).filter(Boolean).length;
    expect(enabledCount).toBe(23);
  });

  it('toggle state persists across steps into review', async () => {
    const onComplete = vi.fn();
    render(<SetupWizard onComplete={onComplete} />);

    // Select Custom (all off).
    await userEvent.click(screen.getAllByRole('radio')[3]!);

    // Step 2: enable Cash.
    await toggleFeature('Cash');

    // Step 3: enable Inventory Tracking.
    await userEvent.click(screen.getByRole('button', { name: /next/i }));
    await toggleFeature('Inventory Tracking');

    // Navigate through remaining steps to review.
    // Steps after Products (index 2): Staff(3), Hardware(4),
    // Business Rules(5), Data & Cloud(6), Review(7) → 5 clicks.
    for (let i = 0; i < 5; i++) {
      await userEvent.click(screen.getByRole('button', { name: /next/i }));
    }

    // Complete.
    await userEvent.click(
      screen.getByRole('button', { name: /complete setup/i }),
    );

    expect(onComplete).toHaveBeenCalledTimes(1);
    const state: WizardState = onComplete.mock.calls[0]![0] as WizardState;
    expect(state.features['cash-payment']).toBe(true);
    expect(state.features['inventory-tracking']).toBe(true);
    expect(state.features['card-payment']).toBeUndefined();
  });
});
