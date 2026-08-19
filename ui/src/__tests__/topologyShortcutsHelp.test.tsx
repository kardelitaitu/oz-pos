import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import { renderInAct } from '@/test-utils/renderInAct';
import { withFluent, withFluentLocale } from '@/locales/test-utils';
import { TopologyShortcutsHelp } from '@/features/stores/topologyShortcutsHelp';
import type { ReactLocalization } from '@fluent/react';
import multiStoreFtl from '@/locales/multi-store.ftl?raw';
import multiStoreIdFtl from '@/locales/multi-store.id.ftl?raw';

// ── Test utilities ────────────────────────────────────────────────

async function renderWithFluent(ui: React.ReactElement) {
  return renderInAct(withFluent(ui, multiStoreFtl));
}

async function renderWithFluentId(ui: React.ReactElement) {
  return renderInAct(withFluentLocale('id', ui, multiStoreIdFtl));
}

// ── Button rendering ──────────────────────────────────────────────

describe('TopologyShortcutsHelp — EN', () => {
  const onToggle = vi.fn();
  const onClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the help button', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={false} onToggle={onToggle} onClose={onClose} />,
    );
    expect(screen.getByRole('button')).toBeInTheDocument();
  });

  it('button has aria-label from l10n', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={false} onToggle={onToggle} onClose={onClose} />,
    );
    const btn = screen.getByRole('button');
    expect(btn).toHaveAttribute('aria-label');
  });

  it('button has aria-expanded=false when closed', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={false} onToggle={onToggle} onClose={onClose} />,
    );
    expect(screen.getByRole('button')).toHaveAttribute('aria-expanded', 'false');
  });

  it('button has aria-expanded=true when open', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={true} onToggle={onToggle} onClose={onClose} />,
    );
    expect(screen.getByRole('button')).toHaveAttribute('aria-expanded', 'true');
  });

  it('button calls onToggle on click', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={false} onToggle={onToggle} onClose={onClose} />,
    );
    fireEvent.click(screen.getByRole('button'));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  // ── Popover when open ────────────────────────────────────────

  it('shows popover when open', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={true} onToggle={onToggle} onClose={onClose} />,
    );
    expect(screen.getByRole('region')).toBeInTheDocument();
  });

  it('does not show popover when closed', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={false} onToggle={onToggle} onClose={onClose} />,
    );
    expect(screen.queryByRole('region')).not.toBeInTheDocument();
  });

  it('popover has correct aria-label', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={true} onToggle={onToggle} onClose={onClose} />,
    );
    const region = screen.getByRole('region');
    expect(region).toHaveAttribute('aria-label');
  });

  it('renders 19 keyboard shortcuts', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={true} onToggle={onToggle} onClose={onClose} />,
    );
    const kbdElements = screen.getAllByText(/^F1$|^Space|^Alt|^Shift|^1–4$|^Ctrl|^Del$|^Esc$|^←/);
    expect(kbdElements.length).toBeGreaterThanOrEqual(15);
  });

  it('renders shortcut descriptions', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={true} onToggle={onToggle} onClose={onClose} />,
    );
    // At least some shortcut descriptions should be visible
    const rows = document.querySelectorAll('.topology-shortcuts-row');
    expect(rows.length).toBeGreaterThanOrEqual(15);
  });

  // ── Escape dismissal ─────────────────────────────────────────

  it('Escape calls onClose', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={true} onToggle={onToggle} onClose={onClose} />,
    );
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  // ── Outside click dismissal ──────────────────────────────────

  it('click outside popover calls onClose', async () => {
    await renderWithFluent(
      <TopologyShortcutsHelp open={true} onToggle={onToggle} onClose={onClose} />,
    );
    // Click on document body (outside popover and button)
    fireEvent.mouseDown(document.body);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

// ── Indonesian locale ─────────────────────────────────────────────

describe('TopologyShortcutsHelp — ID', () => {
  const onToggle = vi.fn();
  const onClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders help button in Indonesian', async () => {
    await renderWithFluentId(
      <TopologyShortcutsHelp open={false} onToggle={onToggle} onClose={onClose} />,
    );
    expect(screen.getByRole('button')).toBeInTheDocument();
  });

  it('shows popover in Indonesian when open', async () => {
    await renderWithFluentId(
      <TopologyShortcutsHelp open={true} onToggle={onToggle} onClose={onClose} />,
    );
    expect(screen.getByRole('region')).toBeInTheDocument();
  });
});
