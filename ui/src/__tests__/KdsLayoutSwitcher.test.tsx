import { describe, expect, it, vi } from 'vitest';
import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import kdsFtl from '@/locales/kds.ftl?raw';
import { KdsLayoutSwitcher } from '@/features/kds/KdsLayoutSwitcher';
import type { KdsLayout } from '@/features/kds/hooks/useKdsPreferences';

describe('KdsLayoutSwitcher', () => {
  const defaultProps = {
    currentLayout: 'kanban' as KdsLayout,
    showOrderId: true,
    showTableNumber: false,
    onSelectLayout: vi.fn(),
    onToggleOrderId: vi.fn(),
    onToggleTableNumber: vi.fn(),
  };

  // ── Initial render ──────────────────────────────────────────────

  it('renders the toggle button', () => {
    renderWithFluentSync(<KdsLayoutSwitcher {...defaultProps} />, kdsFtl);
    const btn = screen.getByRole('button', { name: /layout options/i });
    expect(btn).toBeInTheDocument();
    expect(btn).toHaveAttribute('aria-expanded', 'false');
  });

  it('does not show the popover initially', () => {
    renderWithFluentSync(<KdsLayoutSwitcher {...defaultProps} />, kdsFtl);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  // ── Popover open/close ─────────────────────────────────────────

  it('opens the popover when toggle button is clicked', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutSwitcher {...defaultProps} />, kdsFtl);

    const btn = screen.getByRole('button', { name: /layout options/i });
    await user.click(btn);

    const dialog = screen.getByRole('dialog', { name: /KDS layout/i });
    expect(dialog).toBeInTheDocument();
    expect(btn).toHaveAttribute('aria-expanded', 'true');
  });

  it('closes the popover when Escape is pressed', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutSwitcher {...defaultProps} />, kdsFtl);

    await user.click(screen.getByRole('button', { name: /layout options/i }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('closes the popover when clicking outside', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutSwitcher {...defaultProps} />, kdsFtl);

    await user.click(screen.getByRole('button', { name: /layout options/i }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    // Click on the body (outside the popover)
    await user.click(document.body);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('toggles the popover on second button click', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutSwitcher {...defaultProps} />, kdsFtl);

    const btn = screen.getByRole('button', { name: /layout options/i });
    await user.click(btn);
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    await user.click(btn);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  // ── Layout selection ────────────────────────────────────────────

  it('shows all three layout options with icons', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutSwitcher {...defaultProps} />, kdsFtl);

    await user.click(screen.getByRole('button', { name: /layout options/i }));
    const dialog = screen.getByRole('dialog');

    expect(within(dialog).getByRole('button', { name: /kanban/i })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: /focus/i })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: /metro/i })).toBeInTheDocument();
  });

  it('marks the current layout as pressed', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutSwitcher {...defaultProps} currentLayout="metro" />, kdsFtl);

    await user.click(screen.getByRole('button', { name: /layout options/i }));
    const metroBtn = screen.getByRole('button', { name: /metro/i });
    expect(metroBtn).toHaveAttribute('aria-pressed', 'true');

    const kanbanBtn = screen.getByRole('button', { name: /kanban/i });
    expect(kanbanBtn).toHaveAttribute('aria-pressed', 'false');
  });

  it('calls onSelectLayout and closes popover when a layout is chosen', async () => {
    const user = userEvent.setup();
    const onSelectLayout = vi.fn();
    renderWithFluentSync(
      <KdsLayoutSwitcher {...defaultProps} onSelectLayout={onSelectLayout} />,
      kdsFtl,
    );

    await user.click(screen.getByRole('button', { name: /layout options/i }));
    await user.click(screen.getByRole('button', { name: /focus/i }));

    expect(onSelectLayout).toHaveBeenCalledWith('focus');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  // ── Display toggles ─────────────────────────────────────────────

  it('renders Order ID toggle in the checked state', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutSwitcher {...defaultProps} showOrderId={true} />, kdsFtl);

    await user.click(screen.getByRole('button', { name: /layout options/i }));
    const orderIdToggle = screen.getByRole('switch', { name: /order id/i });
    expect(orderIdToggle).toBeChecked();
  });

  it('renders Table Number toggle in the unchecked state', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutSwitcher {...defaultProps} showTableNumber={false} />, kdsFtl);

    await user.click(screen.getByRole('button', { name: /layout options/i }));
    const tableNumToggle = screen.getByRole('switch', { name: /table number/i });
    expect(tableNumToggle).not.toBeChecked();
  });

  it('calls onToggleOrderId when Order ID switch is toggled', async () => {
    const user = userEvent.setup();
    const onToggleOrderId = vi.fn();
    renderWithFluentSync(
      <KdsLayoutSwitcher {...defaultProps} onToggleOrderId={onToggleOrderId} />,
      kdsFtl,
    );

    await user.click(screen.getByRole('button', { name: /layout options/i }));
    await user.click(screen.getByRole('switch', { name: /order id/i }));

    expect(onToggleOrderId).toHaveBeenCalledWith(false);
  });

  it('calls onToggleTableNumber when Table Number switch is toggled', async () => {
    const user = userEvent.setup();
    const onToggleTableNumber = vi.fn();
    renderWithFluentSync(
      <KdsLayoutSwitcher {...defaultProps} showTableNumber={false} onToggleTableNumber={onToggleTableNumber} />,
      kdsFtl,
    );

    await user.click(screen.getByRole('button', { name: /layout options/i }));
    await user.click(screen.getByRole('switch', { name: /table number/i }));

    expect(onToggleTableNumber).toHaveBeenCalledWith(true);
  });
});
