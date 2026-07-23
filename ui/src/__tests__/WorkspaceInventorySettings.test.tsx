// ── WorkspaceInventorySettings tests ───────────────────────────────
//
// Covers: low stock threshold number input, deduction prefer warehouse
// toggle (gated on locationId), dirty tracking, save flow,
// variant differences.
//
// ADR #22 Phase 1 testing gate (§9).

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import type { ReactNode, ReactElement } from 'react';
import { LocalizationProvider } from '@fluent/react';
import { WorkspaceInventorySettings } from '@/features/settings/workspace-cards/WorkspaceInventorySettings';

const testL10n = {
  bundles: [], areBundlesEmpty: () => true,
  parseMarkup: (str: string) => [{ nodeName: '#text', textContent: str } as unknown as Node],
  getElement: (sourceElement: ReactElement) => sourceElement,
  getString: (id: string) => {
    const d: Record<string, string> = {
      'workspace-inv-threshold-heading': 'Stock Thresholds',
      'workspace-inv-low-stock': 'Low Stock Alert At',
      'workspace-inv-deduction-heading': 'Deduction Rules',
      'workspace-inv-deduction-warehouse': 'Prefer Warehouse First',
      'save': 'Save',
    };
    return d[id] ?? id;
  },
  reportError: () => {}, getBundle: () => null, getChildren: (str: string) => str,
};

function Wrapper({ children }: { children: ReactNode }) {
  return <LocalizationProvider l10n={testL10n}>{children}</LocalizationProvider>;
}
function renderCard(overrides: Record<string, unknown> = {}) {
  return render(<Wrapper><WorkspaceInventorySettings
    variant="full-page" onSaved={vi.fn()} {...overrides} /></Wrapper>);
}

describe('WorkspaceInventorySettings', () => {
  it('renders Stock Thresholds heading', () => {
    renderCard();
    expect(screen.getByText('Stock Thresholds')).toBeInTheDocument();
  });

  it('renders low stock threshold input with default value', () => {
    renderCard();
    const input = document.getElementById('inv-low-stock') as HTMLInputElement;
    expect(Number(input.value)).toBe(10);
    expect(input.type).toBe('number');
  });

  it('shows Deduction Rules card when locationId is present', () => {
    renderCard({ locationId: 'loc-1' });
    expect(screen.getByText('Deduction Rules')).toBeInTheDocument();
  });

  it('hides Deduction Rules card when locationId is absent', () => {
    renderCard();
    expect(screen.queryByText('Deduction Rules')).not.toBeInTheDocument();
  });

  it('renders prefer warehouse toggle unchecked when locationId present', () => {
    renderCard({ locationId: 'loc-1' });
    const t = document.getElementById('inv-deduction-wh') as HTMLInputElement;
    expect(t.checked).toBe(false);
  });

  // ── Dirty tracking ───────────────────────────────────────────
  // originalsRef starts with current state (10, false), dirty=false

  it('Save button disabled when clean', () => {
    renderCard();
    expect(screen.getByRole('button', { name: /save/i })).toBeDisabled();
  });

  it('Save button enabled after changing threshold', async () => {
    renderCard();
    const input = document.getElementById('inv-low-stock') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '5' } });
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /save/i })).not.toBeDisabled();
    });
  });

  it('calls onSaved after successful save', async () => {
    const onSaved = vi.fn();
    renderCard({ onSaved });
    fireEvent.change(document.getElementById('inv-low-stock')!, { target: { value: '5' } });
    await waitFor(() => expect(screen.getByRole('button', { name: /save/i })).not.toBeDisabled());
    fireEvent.click(screen.getByRole('button', { name: /save/i }));
    await waitFor(() => expect(onSaved).toHaveBeenCalled());
  });

  it('hides Save button in inspector-drawer variant', () => {
    renderCard({ variant: 'inspector-drawer' });
    expect(screen.queryByRole('button', { name: /save/i })).not.toBeInTheDocument();
  });
});
