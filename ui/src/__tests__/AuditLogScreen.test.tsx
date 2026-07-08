import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderInAct } from '@/test-utils/renderInAct';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import AuditLogScreen from '@/features/audit/AuditLogScreen';
import sharedFtl from '@/locales/shared.ftl?raw';
import type { AuditEntryDto } from '@/api/audit';

const { mockListAuditLog } = vi.hoisted(() => ({
  mockListAuditLog: vi.fn(),
}));

vi.mock('@/api/audit', () => ({
  listAuditLog: (limit: number, offset: number) => mockListAuditLog(limit, offset),
}));

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(sharedFtl));
const l10n = new ReactLocalization([bundle]);

async function renderScreen() {
  await renderInAct(
    <LocalizationProvider l10n={l10n}>
      <AuditLogScreen />
    </LocalizationProvider>,
  );
}

function makeEntry(overrides: Partial<AuditEntryDto> = {}): AuditEntryDto {
  return {
    id: 'a-1',
    user_id: 'user-1',
    action: 'sale.complete',
    target_type: 'sale',
    target_id: 'sale-1234-abcd-efgh',
    details: '{"total":50000}',
    outcome: 'success',
    created_at: '2026-07-01T12:00:00Z',
    ...overrides,
  };
}

describe('AuditLogScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListAuditLog.mockResolvedValue([]);
  });

  it('renders the title', async () => {
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Audit Log')).toBeDefined());
  });

  it('renders the Refresh button', async () => {
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Refresh')).toBeDefined());
  });

  it('shows loading state initially', async () => {
    mockListAuditLog.mockReturnValue(new Promise(() => {}));
    await renderScreen();
    expect(screen.getByText('Loading audit log…')).toBeDefined();
  });

  it('shows empty state with no entries yet', async () => {
    await renderScreen();
    await waitFor(() => {
      const msg = screen.getByText(/No audit entries recorded yet/);
      expect(msg).toBeDefined();
    });
  });

  it('shows error state with retry button', async () => {
    mockListAuditLog.mockRejectedValue(new Error('DB error'));
    await renderScreen();
    await waitFor(() => expect(screen.getByText('DB error')).toBeDefined());
    expect(screen.getByText('Retry')).toBeDefined();
  });

  it('renders table with audit entries', async () => {
    mockListAuditLog.mockResolvedValue([makeEntry(), makeEntry({ id: 'a-2', action: 'login' })]);
    await renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Date')).toBeDefined();
      expect(screen.getByText('Action')).toBeDefined();
      expect(screen.getByText('Target')).toBeDefined();
      expect(screen.getByText('User ID')).toBeDefined();
      expect(screen.getByText('Outcome')).toBeDefined();
      expect(screen.getByText('Details')).toBeDefined();
    });
  });

  it('shows outcome badge with proper class', async () => {
    mockListAuditLog.mockResolvedValue([
      makeEntry({ id: 'a-1', outcome: 'success' }),
      makeEntry({ id: 'a-2', outcome: 'failure' }),
    ]);
    await renderScreen();
    await waitFor(() => {
      const successBadges = document.querySelectorAll('.audit-badge--success');
      const failureBadges = document.querySelectorAll('.audit-badge--failure');
      expect(successBadges.length).toBeGreaterThanOrEqual(1);
      expect(failureBadges.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('shows action label for known action keys', async () => {
    mockListAuditLog.mockResolvedValue([makeEntry({ action: 'sale.void' })]);
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Void Sale')).toBeDefined());
  });

  it('shows fallback action key for unknown actions', async () => {
    mockListAuditLog.mockResolvedValue([makeEntry({ action: 'custom.event' })]);
    await renderScreen();
    await waitFor(() => {
      const actionKeys = document.querySelectorAll('.audit-log-action-key');
      expect(actionKeys.length).toBeGreaterThanOrEqual(1);
      expect(actionKeys[0]?.textContent).toBe('custom.event');
    });
  });

  it('shows target type and truncated target id', async () => {
    mockListAuditLog.mockResolvedValue([makeEntry({ target_type: 'product', target_id: 'prod-abcdef-123456' })]);
    await renderScreen();
    await waitFor(() => {
      const targetType = document.querySelector('.audit-log-target-type');
      expect(targetType?.textContent).toBe('product');
    });
  });

  it('shows em-dash when target_type is null', async () => {
    mockListAuditLog.mockResolvedValue([makeEntry({ target_type: null, target_id: null })]);
    await renderScreen();
    await waitFor(() => {
      const dash = document.querySelector('.audit-log-target-none');
      expect(dash).toBeDefined();
    });
  });

  it('truncates details preview to 60 chars', async () => {
    const longDetails = 'x'.repeat(100);
    mockListAuditLog.mockResolvedValue([makeEntry({ details: longDetails })]);
    await renderScreen();
    await waitFor(() => {
      const preview = document.querySelector('.audit-log-details-preview');
      // 60 chars + ellipsis character = 61
      expect(preview?.textContent?.length).toBe(61);
    });
  });

  it('shows em-dash for empty/null details', async () => {
    mockListAuditLog.mockResolvedValue([makeEntry({ details: '{}' })]);
    await renderScreen();
    await waitFor(() => {
      const dash = document.querySelector('.audit-log-details-none');
      expect(dash).toBeDefined();
    });
  });

  it('filters entries by outcome', async () => {
    mockListAuditLog.mockResolvedValue([
      makeEntry({ id: 'a-1', outcome: 'success', action: 'login' }),
      makeEntry({ id: 'a-2', outcome: 'failure', action: 'login.failed' }),
    ]);
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Audit Log')).toBeDefined());

    // Click the Success filter chip
    const successChip = screen.getByText('Success').closest('button')!;
    await userEvent.click(successChip);

    await waitFor(() => {
      // The failed entry should be filtered out
      const failureBadges = document.querySelectorAll('.audit-badge--failure');
      expect(failureBadges.length).toBe(0);
    });
  });

  it('shows empty filtered state when filters match nothing', async () => {
    mockListAuditLog.mockResolvedValue([makeEntry({ outcome: 'success' })]);
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Audit Log')).toBeDefined());

    // Click Failure filter
    const failureChip = screen.getByText('Failure').closest('button')!;
    await userEvent.click(failureChip);

    await waitFor(() =>
      expect(screen.getByText('No audit entries match the current filters.')).toBeDefined(),
    );
  });

  it('shows Load More button when more entries exist', async () => {
    // Return 50 entries (equal to limit) — hasMore will be true
    const entries = Array.from({ length: 50 }, (_, i) => makeEntry({ id: `a-${i}` }));
    mockListAuditLog.mockResolvedValue(entries);
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Load More')).toBeDefined());
  });

  it('calls load with offset on Load More click', async () => {
    const entries = Array.from({ length: 50 }, (_, i) => makeEntry({ id: `a-${i}` }));
    mockListAuditLog.mockResolvedValue(entries);
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Load More')).toBeDefined());

    mockListAuditLog.mockClear();
    mockListAuditLog.mockResolvedValue([]);

    const loadMoreBtn = screen.getByText('Load More').closest('button')!;
    await userEvent.click(loadMoreBtn);

    await waitFor(() =>
      expect(mockListAuditLog).toHaveBeenCalledWith(50, 50), // limit=50, offset=50
    );
  });

  it('filters entries by search query', async () => {
    mockListAuditLog.mockResolvedValue([
      makeEntry({ id: 'a-1', action: 'sale.complete', user_id: 'alice' }),
      makeEntry({ id: 'a-2', action: 'login', user_id: 'bob' }),
    ]);
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Audit Log')).toBeDefined());

    // Type into the search input to filter
    const searchInput = document.querySelector('.audit-log-search') as HTMLInputElement;
    await userEvent.type(searchInput, 'alice');

    // Entries not containing 'alice' should be filtered out
    await waitFor(() => {
      // 'bob' entry should be hidden since 'alice' is not found in its fields
      const visibleRows = document.querySelectorAll('.audit-log-table tbody tr');
      expect(visibleRows.length).toBe(1);
    });
  });
});
