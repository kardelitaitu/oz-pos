// ── WorkspaceRestaurantPosSettings tests ───────────────────────────
//
// Covers: table management toggle, course firing toggle, kitchen
// printer section (gated on terminalId), dirty tracking, save flow,
// variant differences.
//
// ADR #22 Phase 1 testing gate (§9).

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import type { ReactNode, ReactElement } from 'react';
import { LocalizationProvider } from '@fluent/react';
import { WorkspaceRestaurantPosSettings } from '@/features/settings/workspace-cards/WorkspaceRestaurantPosSettings';

// ── Fluent test l10n ───────────────────────────────────────────────

const testL10n = {
  bundles: [], areBundlesEmpty: () => true,
  parseMarkup: (str: string) => [{ nodeName: '#text', textContent: str } as unknown as Node],
  getElement: (sourceElement: ReactElement) => sourceElement,
  getString: (id: string) => {
    const defaults: Record<string, string> = {
      'workspace-resto-table-heading': 'Table Management',
      'workspace-resto-table-enable': 'Enable Table Layout',
      'workspace-resto-table-hint': 'Tables appear on the POS screen for dine-in orders',
      'workspace-resto-courses-heading': 'Course Firing',
      'workspace-resto-courses-enable': 'Enable Course Firing',
      'workspace-resto-kitchen-printer-heading': 'Kitchen Printer',
      'workspace-resto-kp-connection': 'Connection',
      'workspace-resto-kp-ip': 'Kitchen Printer IP',
      'save': 'Save',
    };
    return defaults[id] ?? id;
  },
  reportError: () => {}, getBundle: () => null, getChildren: (str: string) => str,
};

// ── Mock state ──────────────────────────────────────────────────────

const mocks = vi.hoisted(() => ({
  receiptSettings: { showTableNumber: false, showCurrency: false, showTax: true, showTableNumber_alias: false,
    footer: '', paperWidth: 'standard' as const, decimalSeparator: 'dot' as const,
    marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0 },
  storeSettings: { name: '', address: '', taxId: '', currency: 'USD', branch: '' },
}));

// Stable profile — prevents useEffect([hw.profile]) from re-firing
const stableProfile = {
  terminalId: 'term-1', storeId: 'store-1',
  hardware: {
    printer: { connection: 'auto' as const, devicePath: '', paperSize: '80' as const, testPrintIp: '' },
    scale: { connection: 'none' as const, devicePath: '', baudRate: 9600, zeroOnBoot: false },
    scanner: { mode: 'auto' as const, deviceId: '' },
  },
  localPrefs: { soundVolume: 80, darkMode: false, scaleAutoZero: true },
  initialized: '2026-01-01T00:00:00Z', version: 1,
};

// ── Mocks ───────────────────────────────────────────────────────────

vi.mock('@/contexts/SettingsContext', () => ({
  useSettings: () => ({
    settings: { receipt: mocks.receiptSettings, store: mocks.storeSettings },
    loading: false, error: null, hasPartialError: false,
    refetch: vi.fn(), lastChangedKeys: [], markSettingsUpdated: vi.fn(),
  }),
}));

vi.mock('@/hooks/useTerminalHardware', () => ({
  useTerminalHardware: (terminalId: string) => {
    if (!terminalId) return { profile: null, isLoading: false, error: null,
      updatePrinter: vi.fn(), updateScale: vi.fn(), updateScanner: vi.fn(),
      updateLocalPrefs: vi.fn(), save: vi.fn(), reload: vi.fn() };
    return { profile: stableProfile, isLoading: false, error: null,
      updatePrinter: vi.fn(), updateScale: vi.fn(), updateScanner: vi.fn(),
      updateLocalPrefs: vi.fn(), save: vi.fn().mockResolvedValue(undefined), reload: vi.fn() };
  },
}));

vi.mock('../features/settings/SettingsSelect', () => ({
  default: ({ id, value, onChange, options }: {
    id: string; value: string; onChange: (v: string) => void;
    options: Array<{ value: string; label: string }>;
  }) => (
    <select id={id} data-testid={id} value={value}
      onChange={(e) => onChange(e.target.value)}>
      {options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
    </select>
  ),
}));

// ── Helpers ─────────────────────────────────────────────────────────

function Wrapper({ children }: { children: ReactNode }) {
  return <LocalizationProvider l10n={testL10n}>{children}</LocalizationProvider>;
}

function renderCard(overrides: Record<string, unknown> = {}) {
  return render(<Wrapper><WorkspaceRestaurantPosSettings
    terminalId="term-1" variant="full-page" onSaved={vi.fn()} {...overrides} /></Wrapper>);
}

beforeEach(() => {
  Object.assign(mocks.receiptSettings, { showTableNumber: false,
    showCurrency: false, showTax: true, footer: '', paperWidth: 'standard',
    decimalSeparator: 'dot', marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0 });
  Object.assign(mocks.storeSettings, { name: '', address: '', taxId: '', currency: 'USD', branch: '' });
});

describe('WorkspaceRestaurantPosSettings', () => {
  it('renders Table Management heading', () => {
    renderCard();
    expect(screen.getByText('Table Management')).toBeInTheDocument();
  });

  it('renders table layout toggle unchecked', () => {
    renderCard();
    const t = document.getElementById('resto-table-mgmt') as HTMLInputElement;
    expect(t.checked).toBe(false);
  });

  it('renders Course Firing heading', () => {
    renderCard();
    expect(screen.getByText('Course Firing')).toBeInTheDocument();
  });

  it('renders course firing toggle unchecked', () => {
    renderCard();
    const t = document.getElementById('resto-course-firing') as HTMLInputElement;
    expect(t.checked).toBe(false);
  });

  it('renders Kitchen Printer section when terminalId present', () => {
    renderCard({ terminalId: 'term-1' });
    expect(screen.getByText('Kitchen Printer')).toBeInTheDocument();
  });

  it('hides Kitchen Printer section when terminalId is empty', () => {
    renderCard({ terminalId: '' });
    expect(screen.queryByText('Kitchen Printer')).not.toBeInTheDocument();
  });

  // ── Dirty tracking ───────────────────────────────────────────
  // originalsRef starts with current state values, so dirty is false
  // on first render. No waitFor needed.

  it('Save button disabled when clean', () => {
    renderCard();
    expect(screen.getByRole('button', { name: /save/i })).toBeDisabled();
  });

  it('Save button enabled after toggling table layout', async () => {
    renderCard();
    const t = document.getElementById('resto-table-mgmt') as HTMLInputElement;
    fireEvent.click(t);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /save/i })).not.toBeDisabled();
    });
  });

  it('calls onSaved after successful save', async () => {
    const onSaved = vi.fn();
    renderCard({ onSaved });
    const t = document.getElementById('resto-table-mgmt') as HTMLInputElement;
    fireEvent.click(t);
    await waitFor(() => expect(screen.getByRole('button', { name: /save/i })).not.toBeDisabled());
    fireEvent.click(screen.getByRole('button', { name: /save/i }));
    await waitFor(() => expect(onSaved).toHaveBeenCalled());
  });

  it('hides Save button in inspector-drawer variant', () => {
    renderCard({ variant: 'inspector-drawer' });
    expect(screen.queryByRole('button', { name: /save/i })).not.toBeInTheDocument();
  });
});
