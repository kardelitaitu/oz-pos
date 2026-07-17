import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import { ToastProvider } from '@/frontend/shared/Toast';
import TerminalManagementScreen from '@/features/terminals/TerminalManagementScreen';
import terminalsFtl from '@/locales/terminals.ftl?raw';
import type { TerminalDto } from '@/api/terminals';

const { mockListTerminals, mockRegisterTerminal, mockDeleteTerminal,
  mockListTerminalOverrides } =
  vi.hoisted(() => ({
    mockListTerminals: vi.fn(),
    mockRegisterTerminal: vi.fn(),
    mockDeleteTerminal: vi.fn(),
    mockListTerminalOverrides: vi.fn(),
  }));

vi.mock('@/api/terminals', () => ({
  listTerminals: () => mockListTerminals(),
  registerTerminal: (userId: string, args: unknown) => mockRegisterTerminal(userId, args),
  updateTerminal: () => Promise.resolve({ id: 't-1' }),
  deleteTerminal: (userId: string, id: string) => mockDeleteTerminal(userId, id),
  listTerminalOverrides: (id: string) => mockListTerminalOverrides(id),
  setTerminalOverride: () => Promise.resolve(),
  deleteTerminalOverride: () => Promise.resolve(),
}));

vi.mock('@/hooks/useFeatures', () => ({
  FEATURES: {
    SIMPLE_RETAIL: 'simple-retail', RESTAURANT: 'restaurant',
    DISCOUNT_ENGINE: 'discount-engine', TAX_ENGINE: 'tax-engine',
    PROMOTIONS_ENGINE: 'promotions-engine', PRODUCT_BUNDLES: 'product-bundles',
    LOYALTY_PROGRAM: 'loyalty-program', KITCHEN_DISPLAY: 'kitchen-display',
    TABLE_MANAGEMENT: 'table-management', CASH_PAYMENT: 'cash-payment',
    CARD_PAYMENT: 'card-payment', MULTI_CURRENCY: 'multi-currency',
    INVENTORY_TRACKING: 'inventory-tracking', PRODUCT_VARIANTS: 'product-variants',
    CATEGORIES_ENABLED: 'categories-enabled', BARCODE_SCANNING: 'barcode-scanning',
    RECEIPT_PRINTING: 'receipt-printing', CASH_DRAWER: 'cash-drawer',
    CUSTOMER_DISPLAY: 'customer-display', NFC_READER: 'nfc-reader',
    STAFF_LOGIN: 'staff-login', STAFF_ROLES: 'staff-roles',
    SHIFT_MANAGEMENT: 'shift-management', AUDIT_LOG: 'audit-log',
    CLOUD_SYNC: 'cloud-sync', MULTI_STORE: 'multi-store',
    MULTI_TERMINAL: 'multi-terminal', REPORTING: 'reporting',
    ANALYTICS: 'analytics', EXPORT_IMPORT: 'export-import',
    PLUGIN_SYSTEM: 'plugin-system', SELF_SERVICE_KIOSK: 'self-service-kiosk',
  },
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({ session: { user_id: 'user-1' } }),
}));

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(terminalsFtl));
const l10n = new ReactLocalization([bundle]);

function renderScreen() {
  return render(
    <LocalizationProvider l10n={l10n}>
      <ToastProvider>
        <TerminalManagementScreen />
      </ToastProvider>
    </LocalizationProvider>,
  );
}

function makeTerminal(overrides: Partial<TerminalDto> = {}): TerminalDto {
  return {
    id: 't-1', name: 'Front Counter', deviceId: 'dev-001', isActive: true,
    lastSeenAt: '2026-07-01T12:00:00Z', metadata: null,
    createdAt: '2026-01-01T00:00:00Z', updatedAt: '2026-07-01T12:00:00Z',
    ...overrides,
  };
}

describe('TerminalManagementScreen', () => {
  beforeEach(() => {
    mockListTerminals.mockResolvedValue([]);
    mockListTerminalOverrides.mockResolvedValue([]);
    mockRegisterTerminal.mockResolvedValue({ id: 'new-t' });
    mockDeleteTerminal.mockResolvedValue(undefined);
  });

  it('renders the title', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getByText('Terminal Management')).toBeDefined());
  });

  it('renders the Register Terminal button', async () => {
    renderScreen();
    await waitFor(() => {
      const btns = screen.getAllByText('Register Terminal');
      expect(btns.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('shows loading skeleton while fetching terminals', () => {
    mockListTerminals.mockReturnValue(new Promise(() => {}));
    renderScreen();
    expect(document.querySelector('.terminal-mgmt-loading-skeleton')).toBeDefined();
    expect(screen.queryByText('Loading terminals…')).toBeNull();
  });

  it('shows empty state', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getByText(/No terminals registered yet/)).toBeDefined());
  });

  it('shows error state with retry', async () => {
    mockListTerminals.mockRejectedValue(new Error('Failed'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Retry')).toBeDefined());
  });

  it('renders table with terminal data', async () => {
    mockListTerminals.mockResolvedValue([
      makeTerminal(),
      makeTerminal({ id: 't-2', name: 'Bar', deviceId: 'dev-002' }),
    ]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Front Counter')).toBeDefined();
      expect(screen.getByText('Bar')).toBeDefined();
      expect(screen.getByText('Name')).toBeDefined();
      expect(screen.getByText('Device ID')).toBeDefined();
      expect(screen.getByText('Status')).toBeDefined();
    });
  });

  it('shows Active status badge for active terminal', async () => {
    mockListTerminals.mockResolvedValue([makeTerminal({ isActive: true })]);
    renderScreen();
    await waitFor(() => {
      expect(document.querySelector('.terminal-mgmt-status-active')).toBeDefined();
    });
  });

  it('shows Inactive status badge for inactive terminal', async () => {
    mockListTerminals.mockResolvedValue([makeTerminal({ isActive: false })]);
    renderScreen();
    await waitFor(() => {
      expect(document.querySelector('.terminal-mgmt-status-inactive')).toBeDefined();
    });
  });

  it('shows device ID in table', async () => {
    mockListTerminals.mockResolvedValue([makeTerminal({ deviceId: 'dev-abc' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('dev-abc')).toBeDefined());
  });

  it('shows Never for terminal with null lastSeenAt', async () => {
    mockListTerminals.mockResolvedValue([makeTerminal({ lastSeenAt: null })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Never')).toBeDefined());
  });

  it('has Edit and Delete buttons per row', async () => {
    mockListTerminals.mockResolvedValue([makeTerminal()]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getAllByText('Edit').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('Delete').length).toBeGreaterThanOrEqual(1);
    });
  });

  it('opens delete confirmation modal on Delete click', async () => {
    mockListTerminals.mockResolvedValue([makeTerminal()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Front Counter')).toBeDefined());

    await userEvent.click(screen.getAllByText('Delete')[0]!.closest('button')!);
    await waitFor(() =>
      expect(screen.getByText(/Are you sure you want to delete terminal/)).toBeDefined(),
    );
  });

  it('calls deleteTerminal on confirm delete', async () => {
    mockListTerminals.mockResolvedValue([makeTerminal()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Front Counter')).toBeDefined());

    await userEvent.click(screen.getAllByText('Delete')[0]!.closest('button')!);
    await waitFor(() => expect(screen.getByText('Delete Terminal')).toBeDefined());

    const confirmBtn = screen.getAllByText('Delete').slice(-1)[0]!.closest('button')!;
    await userEvent.click(confirmBtn);

    await waitFor(() => expect(mockDeleteTerminal).toHaveBeenCalledWith('user-1', 't-1'));
  });

  it('opens register modal when Register Terminal is clicked', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getByText('Register Terminal')).toBeDefined());

    await userEvent.click(screen.getAllByText('Register Terminal')[0]!.closest('button')!);
    await waitFor(() => expect(screen.getByText('Register New Terminal')).toBeDefined());
  });
});
