// ── ReceiptSection tests ──────────────────────────────────────────
//
// Covers: receipt settings rendering (show currency, decimal separator,
// show tax, paper width, footer, show table number, paper margins),
// markDirty tracking on all state changes.
//
// ADR #22 Phase 1 testing gate (§9).

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { LocalizationProvider } from '@fluent/react';
import type { ReactNode } from 'react';
import ReceiptSection from '@/features/settings/sections/ReceiptSection';
import type { ReceiptSettingsDto } from '@/api/settings';

// ── Fluent test l10n ───────────────────────────────────────────────

const testL10n = {
  bundles: [],
  areBundlesEmpty: () => true,
  parseMarkup: (str: string) => [{ nodeName: '#text', textContent: str } as unknown as Node],
  getElement: (sourceElement: React.ReactElement) => sourceElement,
  getString: (id: string) => {
    const defaults: Record<string, string> = {
      'settings-section-receipt': 'Receipt',
      'settings-toggle-show-currency': 'Show currency symbol on amounts',
      'settings-toggle-show-tax': 'Show tax line on receipts',
      'settings-toggle-show-table-number': 'Show table number on cart and receipts',
      'settings-field-decimal-separator': 'Decimal separator',
      'settings-field-paper-width': 'Paper width',
      'settings-field-footer': 'Footer text',
      'settings-footer-placeholder': 'Thank you for shopping!',
      'settings-decimal-separator-dot': '1.00 (dot)',
      'settings-decimal-separator-comma': '1,00 (comma)',
      'settings-decimal-separator-none': '1 (none)',
      'settings-paper-width-standard': '80 mm (standard)',
      'settings-paper-width-narrow': '58 mm (narrow)',
      'settings-margins-heading': 'Paper Margins (mm)',
      'settings-margin-top': 'Top',
      'settings-margin-bottom': 'Bottom',
      'settings-margin-left': 'Left',
      'settings-margin-right': 'Right',
    };
    return defaults[id] ?? id;
  },
  reportError: () => {},
  getBundle: () => null,
  getChildren: (str: string) => str,
} as unknown as React.ComponentProps<typeof LocalizationProvider>['l10n'];

// ── Mocks ──────────────────────────────────────────────────────────

vi.mock('@/components/Card', () => ({
  Card: ({ children, shadow, header }: {
    children: React.ReactNode; shadow?: string; header?: React.ReactNode;
  }) => (
    <div data-testid="card" data-shadow={shadow}>
      {header}
      {children}
    </div>
  ),
}));

vi.mock('@/features/settings/SettingsSelect', () => ({
  default: ({ id, value, onChange, options }: {
    id?: string; value: string; onChange: (v: string) => void;
    options: { value: string; label: string }[];
  }) => (
    <select
      id={id}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>{opt.label}</option>
      ))}
    </select>
  ),
}));

// ── Wrapper ─────────────────────────────────────────────────────────

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <LocalizationProvider l10n={testL10n}>
      {children}
    </LocalizationProvider>
  );
}

const DEFAULT_RECEIPT: ReceiptSettingsDto = {
  showCurrency: true,
  decimalSeparator: 'dot',
  showTax: false,
  paperWidth: 'standard',
  footer: 'Thank you!',
  showTableNumber: true,
  marginTop: 5,
  marginBottom: 5,
  marginLeft: 3,
  marginRight: 3,
};

function renderSection(overrides: Record<string, unknown> = {}) {
  const defaultProps = {
    receipt: DEFAULT_RECEIPT,
    setReceipt: vi.fn(),
    setDecimalSep: vi.fn(),
    markDirty: vi.fn(),
    l10n: testL10n,
  };

  return render(
    <Wrapper>
      <ReceiptSection {...defaultProps} {...overrides} />
    </Wrapper>,
  );
}

// ── Tests ──────────────────────────────────────────────────────────

describe('ReceiptSection', () => {
  it('renders Receipt section header', () => {
    renderSection();
    expect(screen.getByText('Receipt')).toBeInTheDocument();
  });

  // ── Toggle switches ─────────────────────────────────────────

  it('renders show currency toggle checked when enabled', () => {
    renderSection({ receipt: { ...DEFAULT_RECEIPT, showCurrency: true } });
    const toggle = screen.getByRole('switch', { name: /currency/i });
    expect(toggle).toBeChecked();
  });

  it('renders show currency toggle unchecked when disabled', () => {
    renderSection({ receipt: { ...DEFAULT_RECEIPT, showCurrency: false } });
    const toggle = screen.getByRole('switch', { name: /currency/i });
    expect(toggle).not.toBeChecked();
  });

  it('calls markDirty when show currency toggle changes', () => {
    const markDirty = vi.fn();
    const setReceipt = vi.fn();
    renderSection({ markDirty, setReceipt });

    const toggle = screen.getByRole('switch', { name: /currency/i });
    fireEvent.click(toggle);
    expect(setReceipt).toHaveBeenCalled();
    expect(markDirty).toHaveBeenCalled();
  });

  it('renders show tax toggle unchecked by default', () => {
    renderSection({ receipt: { ...DEFAULT_RECEIPT, showTax: false } });
    const toggle = screen.getByRole('switch', { name: /tax/i });
    expect(toggle).not.toBeChecked();
  });

  it('calls markDirty when show tax toggle changes', () => {
    const markDirty = vi.fn();
    const setReceipt = vi.fn();
    renderSection({ markDirty, setReceipt });

    const toggle = screen.getByRole('switch', { name: /tax/i });
    fireEvent.click(toggle);
    expect(setReceipt).toHaveBeenCalled();
    expect(markDirty).toHaveBeenCalled();
  });

  it('renders show table number toggle', () => {
    renderSection();
    expect(screen.getByRole('switch', { name: /table/i })).toBeInTheDocument();
  });

  // ── Selects ─────────────────────────────────────────────────

  it('renders decimal separator select with current value', () => {
    renderSection({ receipt: { ...DEFAULT_RECEIPT, decimalSeparator: 'comma' } });
    const select = screen.getByDisplayValue('1,00 (comma)');
    expect(select).toBeInTheDocument();
  });

  it('calls markDirty + setDecimalSep when decimal separator changes', () => {
    const markDirty = vi.fn();
    const setDecimalSep = vi.fn();
    const setReceipt = vi.fn();
    renderSection({ markDirty, setDecimalSep, setReceipt });

    fireEvent.change(screen.getByDisplayValue('1.00 (dot)'), {
      target: { value: 'comma' },
    });
    expect(setDecimalSep).toHaveBeenCalledWith('comma');
    expect(markDirty).toHaveBeenCalled();
  });

  it('renders paper width select with current value', () => {
    renderSection({ receipt: { ...DEFAULT_RECEIPT, paperWidth: 'narrow' } });
    expect(screen.getByDisplayValue('58 mm (narrow)')).toBeInTheDocument();
  });

  it('calls markDirty when paper width changes', () => {
    const markDirty = vi.fn();
    const setReceipt = vi.fn();
    renderSection({ markDirty, setReceipt });

    fireEvent.change(screen.getByDisplayValue('80 mm (standard)'), {
      target: { value: 'narrow' },
    });
    expect(setReceipt).toHaveBeenCalled();
    expect(markDirty).toHaveBeenCalled();
  });

  // ── Footer textarea ─────────────────────────────────────────

  it('renders footer textarea with current value', () => {
    renderSection({ receipt: { ...DEFAULT_RECEIPT, footer: 'Thanks!' } });
    expect(screen.getByDisplayValue('Thanks!')).toBeInTheDocument();
  });

  it('shows character count for footer', () => {
    renderSection({ receipt: { ...DEFAULT_RECEIPT, footer: 'Hi' } });
    expect(screen.getByText('2/500')).toBeInTheDocument();
  });

  it('calls markDirty when footer changes', () => {
    const markDirty = vi.fn();
    const setReceipt = vi.fn();
    renderSection({ markDirty, setReceipt });

    fireEvent.change(screen.getByDisplayValue('Thank you!'), {
      target: { value: 'New footer text' },
    });
    expect(setReceipt).toHaveBeenCalled();
    expect(markDirty).toHaveBeenCalled();
  });

  // ── Paper margins ───────────────────────────────────────────

  it('renders margin heading', () => {
    renderSection();
    expect(screen.getByText('Paper Margins (mm)')).toBeInTheDocument();
  });

  it('renders margin inputs with correct default values', () => {
    renderSection({
      receipt: {
        ...DEFAULT_RECEIPT,
        marginTop: 10, marginBottom: 8, marginLeft: 5, marginRight: 5,
      },
    });
    expect(screen.getByDisplayValue('10')).toBeInTheDocument();
    expect(screen.getByDisplayValue('8')).toBeInTheDocument();
    expect(screen.getAllByDisplayValue('5')).toHaveLength(2);
  });

  it('calls markDirty when margin changes', () => {
    const markDirty = vi.fn();
    const setReceipt = vi.fn();
    renderSection({ markDirty, setReceipt });

    fireEvent.change(screen.getAllByDisplayValue('5')[0]!, {
      target: { value: '10' },
    });
    expect(setReceipt).toHaveBeenCalled();
    expect(markDirty).toHaveBeenCalled();
  });

  it('clamps margin values to 0-20 range', () => {
    const setReceipt = vi.fn();
    renderSection({ setReceipt });

    const marginInputs = screen.getAllByDisplayValue('5');
    fireEvent.change(marginInputs[0]!, { target: { value: '25' } });
    // The component clamps via Math.min(20, ...)
    const callArg = setReceipt.mock.calls[0]?.[0] as ReceiptSettingsDto;
    expect(callArg.marginTop).toBeLessThanOrEqual(20);

    fireEvent.change(marginInputs[0]!, { target: { value: '-5' } });
    const callArg2 = setReceipt.mock.calls[1]?.[0] as ReceiptSettingsDto;
    expect(callArg2.marginTop).toBeGreaterThanOrEqual(0);
  });
});
