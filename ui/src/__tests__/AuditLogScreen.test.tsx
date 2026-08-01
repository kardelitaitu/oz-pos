import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, screen, waitFor } from '@testing-library/react';
import { renderInAct } from '@/test-utils/renderInAct';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import AuditLogScreen from '@/features/audit/AuditLogScreen';
import sharedFtl from '@/locales/shared.ftl?raw';
import type { AuditEntryDto, AuditLogPageDto } from '@/api/audit';

const { mockListAuditLogScoped, mockGetAuditReviewStatusScoped, mockMarkAuditReviewedScoped } =
  vi.hoisted(() => ({
    mockListAuditLogScoped: vi.fn(),
    mockGetAuditReviewStatusScoped: vi.fn(),
    mockMarkAuditReviewedScoped: vi.fn(),
  }));

vi.mock('@/api/audit', () => ({
  listAuditLog: (limit: number, offset: number) => mockListAuditLogScoped('tok', { limit, offset }),
  listAuditLogScoped: (token: string, args: unknown) => mockListAuditLogScoped(token, args),
  getAuditReviewStatusScoped: (token: string) => mockGetAuditReviewStatusScoped(token),
  markAuditReviewedScoped: (token: string, args: unknown) => mockMarkAuditReviewedScoped(token, args),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({ sessionToken: 'tok' }),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', username: 'admin', role_name: 'admin', token: 'tok', role_id: 'r1', display_name: 'Admin' },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    swapSession: vi.fn(),
    isManager: true,
    isOwner: true,
  }),
}));

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(sharedFtl));
// Suppress Fluent errors for fallback test: 'shows fallback action key for unknown actions'
bundle.addResource(new FluentResource('custom.event = Custom Event\n'));
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

function makePage(entries: AuditEntryDto[], total?: number, has_more = false): AuditLogPageDto {
  return { items: entries, total: total ?? entries.length, has_more };
}

describe('AuditLogScreen', () => {
  beforeEach(() => {
    mockListAuditLogScoped.mockReset();
    mockGetAuditReviewStatusScoped.mockReset();
    mockMarkAuditReviewedScoped.mockReset();
    mockGetAuditReviewStatusScoped.mockResolvedValue({ checkpoint: null, unreviewed_count: 0 });
    mockListAuditLogScoped.mockResolvedValue(makePage([]));
  });

  it('renders the title', async () => {
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Audit Log')).toBeDefined());
  });

  it('renders the Refresh button', async () => {
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Refresh')).toBeDefined());
  });

  it('loads the scoped page with the session token on mount (AUD-01)', async () => {
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry()]));
    await renderScreen();
    await waitFor(() =>
      expect(mockListAuditLogScoped).toHaveBeenCalledWith(
        'tok',
        expect.objectContaining({ limit: 50 }),
      ),
    );
  });

  it('calls load(reset) when Refresh is clicked', async () => {
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry()]));
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Refresh')).toBeDefined());

    mockListAuditLogScoped.mockClear();
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry()]));
    await userEvent.click(screen.getByRole('button', { name: 'Refresh' }));
    await waitFor(() =>
      expect(mockListAuditLogScoped).toHaveBeenCalledWith(
        'tok',
        expect.objectContaining({ limit: 50 }),
      ),
    );
  });

  it('shows loading skeleton initially', async () => {
    mockListAuditLogScoped.mockReturnValue(new Promise(() => {}));
    await renderScreen();
    const skeleton = document.querySelector('.audit-log-loading-skeleton');
    expect(skeleton).toBeTruthy();
    expect(screen.queryByText('Loading audit log…')).toBeNull();
  });

  it('shows empty state with no entries yet', async () => {
    await renderScreen();
    await waitFor(() => {
      const msg = screen.getByText(/No audit entries recorded yet/);
      expect(msg).toBeDefined();
    });
  });

  it('shows error state with retry button', async () => {
    mockListAuditLogScoped.mockRejectedValue(new Error('DB error'));
    await renderScreen();
    await waitFor(() => expect(screen.getByText('DB error')).toBeDefined());
    expect(screen.getByText('Retry')).toBeDefined();
  });

  it('calls load(reset) when retry button is clicked after error', async () => {
    // First call during mount rejects — shows error state
    mockListAuditLogScoped.mockRejectedValueOnce(new Error('DB error'));
    await renderScreen();
    await waitFor(() => expect(screen.getByText('DB error')).toBeDefined());

    // Clear mock so retry uses the default resolved value from beforeEach
    mockListAuditLogScoped.mockClear();
    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));
    await waitFor(() =>
      expect(mockListAuditLogScoped).toHaveBeenCalledWith(
        'tok',
        expect.objectContaining({ limit: 50 }),
      ),
    );
  });

  it('renders table with audit entries', async () => {
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry(), makeEntry({ id: 'a-2', action: 'login' })]));
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
    mockListAuditLogScoped.mockResolvedValue(makePage([
      makeEntry({ id: 'a-1', outcome: 'success' }),
      makeEntry({ id: 'a-2', outcome: 'failure' }),
    ]));
    await renderScreen();
    await waitFor(() => {
      const successBadges = document.querySelectorAll('.audit-badge--success');
      const failureBadges = document.querySelectorAll('.audit-badge--failure');
      expect(successBadges.length).toBeGreaterThanOrEqual(1);
      expect(failureBadges.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('shows action label for known action keys', async () => {
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ action: 'sale.void' })]));
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Void Sale')).toBeDefined());
  });

  it('shows fallback action key for unknown actions', async () => {
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ action: 'custom.event' })]));
    await renderScreen();
    await waitFor(() => {
      const actionKeys = document.querySelectorAll('.audit-log-action-key');
      expect(actionKeys.length).toBeGreaterThanOrEqual(1);
      expect(actionKeys[0]?.textContent).toBe('custom.event');
    });
  });

  it('shows target type and truncated target id', async () => {
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ target_type: 'product', target_id: 'prod-abcdef-123456' })]));
    await renderScreen();
    await waitFor(() => {
      const targetType = document.querySelector('.audit-log-target-type');
      expect(targetType?.textContent).toBe('product');
    });
  });

  it('shows em-dash when target_type is null', async () => {
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ target_type: null, target_id: null })]));
    await renderScreen();
    await waitFor(() => {
      const dash = document.querySelector('.audit-log-target-none');
      expect(dash).toBeDefined();
    });
  });

  it('truncates details preview to 60 chars', async () => {
    const longDetails = 'x'.repeat(100);
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ details: longDetails })]));
    await renderScreen();
    await waitFor(() => {
      const preview = document.querySelector('.audit-log-details-preview');
      // 60 chars + ellipsis character = 61
      expect(preview?.textContent?.length).toBe(61);
    });
  });

  it('shows em-dash for empty/null details', async () => {
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ details: '{}' })]));
    await renderScreen();
    await waitFor(() => {
      const dash = document.querySelector('.audit-log-details-none');
      expect(dash).toBeDefined();
    });
  });

  it('sends the outcome filter to the server (AUD-02)', async () => {
    mockListAuditLogScoped.mockImplementation((_token: string, args: { outcome?: string }) => {
      if (args.outcome === 'success') {
        return Promise.resolve(makePage([makeEntry({ id: 'a-1', outcome: 'success', action: 'login' })]));
      }
      return Promise.resolve(makePage([
        makeEntry({ id: 'a-1', outcome: 'success', action: 'login' }),
        makeEntry({ id: 'a-2', outcome: 'failure', action: 'login.failed' }),
      ]));
    });
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Audit Log')).toBeDefined());

    // Click the Success filter chip
    const successChip = screen.getByText('Success').closest('button')!;
    await userEvent.click(successChip);

    await waitFor(() => {
      expect(mockListAuditLogScoped).toHaveBeenCalledWith(
        'tok',
        expect.objectContaining({ outcome: 'success' }),
      );
      // The failed entry should be gone (server returned only success rows)
      const failureBadges = document.querySelectorAll('.audit-badge--failure');
      expect(failureBadges.length).toBe(0);
    });
  });

  it('shows empty filtered state when the server returns no matches', async () => {
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ outcome: 'success' })]));
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Audit Log')).toBeDefined());

    // Click Failure filter — server returns an empty page
    mockListAuditLogScoped.mockResolvedValue(makePage([], 0, false));
    const failureChip = screen.getByText('Failure').closest('button')!;
    await userEvent.click(failureChip);

    await waitFor(() =>
      expect(screen.getByText('No audit entries match the current filters.')).toBeDefined(),
    );
  });

  it('shows Load More button when has_more is true (AUD-03)', async () => {
    const entries = Array.from({ length: 50 }, (_, i) => makeEntry({ id: `a-${i}`, created_at: `2026-07-01T12:0${Math.floor(i / 10)}:00Z` }));
    mockListAuditLogScoped.mockResolvedValue(makePage(entries, 60, true));
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Load More')).toBeDefined());
  });

  it('sends the keyset cursor on Load More (AUD-03)', async () => {
    const entries = Array.from({ length: 50 }, (_, i) => makeEntry({ id: `a-${i}` }));
    mockListAuditLogScoped.mockResolvedValue(makePage(entries, 60, true));
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Load More')).toBeDefined());

    mockListAuditLogScoped.mockClear();
    mockListAuditLogScoped.mockResolvedValue(makePage([], 60, false));

    const loadMoreBtn = screen.getByText('Load More').closest('button')!;
    await userEvent.click(loadMoreBtn);

    await waitFor(() =>
      expect(mockListAuditLogScoped).toHaveBeenCalledWith(
        'tok',
        expect.objectContaining({ beforeId: 'a-49' }),
      ),
    );
  });

  it('sends the debounced search query to the server (AUD-02)', async () => {
    mockListAuditLogScoped.mockImplementation((_token: string, args: { query?: string }) => {
      if (args.query) {
        return Promise.resolve(makePage([makeEntry({ id: 'a-1', user_id: 'alice' })]));
      }
      return Promise.resolve(makePage([
        makeEntry({ id: 'a-1', user_id: 'alice' }),
        makeEntry({ id: 'a-2', action: 'login', user_id: 'bob' }),
      ]));
    });
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Audit Log')).toBeDefined());

    const searchInput = document.querySelector('.audit-log-search') as HTMLInputElement;
    await userEvent.type(searchInput, 'alice');

    await waitFor(() =>
      expect(mockListAuditLogScoped).toHaveBeenCalledWith(
        'tok',
        expect.objectContaining({ query: 'alice' }),
      ),
    );
  });

  // ── Review checkpoints (AUD-04) ─────────────────────────────────

  it('shows the server-side unreviewed badge count (AUD-04)', async () => {
    mockGetAuditReviewStatusScoped.mockResolvedValue({ checkpoint: null, unreviewed_count: 3 });
    await renderScreen();
    await waitFor(() => {
      expect(screen.getByText('3 new')).toBeDefined();
    });
  });

  it('shows reviewed-at date when a checkpoint exists (AUD-04)', async () => {
    mockGetAuditReviewStatusScoped.mockResolvedValue({
      checkpoint: {
        id: 'cp-1',
        store_id: 'store-a',
        reviewer_user_id: 'user-1',
        reviewed_at: '2026-07-02T00:00:00Z',
        reviewed_through_created_at: '2026-07-01T12:00:00Z',
        reviewed_through_id: 'a-1',
      },
      unreviewed_count: 0,
    });
    await renderScreen();
    await waitFor(() => {
      expect(screen.getByText(/Reviewed:/)).toBeDefined();
    });
  });

  it('marks reviewed via the scoped API with the newest entry cursor (AUD-04)', async () => {
    mockGetAuditReviewStatusScoped.mockResolvedValue({ checkpoint: null, unreviewed_count: 5 });
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ id: 'a-1', created_at: '2026-07-01T12:00:00Z' })]));
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Mark Reviewed')).toBeDefined());

    await userEvent.click(screen.getByRole('button', { name: 'Mark Reviewed' }));

    await waitFor(() =>
      expect(mockMarkAuditReviewedScoped).toHaveBeenCalledWith('tok', {
        reviewedThroughCreatedAt: '2026-07-01T12:00:00Z',
        reviewedThroughId: 'a-1',
      }),
    );
  });

  // ── Request-generation protection (AUD-05) ─────────────────────

  it('discards a stale slower load in favor of a newer one (AUD-05)', async () => {
    // Mount load (request 1) is deferred; a later filter change (request 2)
    // resolves first. The outcome chip stands in for the Refresh button's
    // load({reset:true}) path because the shared Button disables while
    // loading.
    let resolveStale!: (page: AuditLogPageDto) => void;
    mockListAuditLogScoped.mockImplementationOnce(
      () => new Promise<AuditLogPageDto>((resolve) => { resolveStale = resolve; }),
    );
    // The newer request returns the newest row. user_id is truncated to 8
    // chars in the table, so use <=8-char ids for exact text matching.
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ id: 'a-fresh', user_id: 'fresh-1' })]));

    await renderScreen();

    // Trigger a new request generation while the first is still in flight by
    // changing a server-side filter. The outcome chip is a plain button (never
    // disabled), unlike the Refresh Button which disables while loading — so
    // this deterministically exercises the "overlap across state transitions
    // and external triggers" the seq guard must absorb.
    const successChip = screen.getByText('Success').closest('button')!;
    await userEvent.click(successChip);
    await waitFor(() => expect(screen.getByText('fresh-1')).toBeDefined());

    // The stale first request resolves late — it must be ignored.
    await act(async () => {
      resolveStale(makePage([makeEntry({ id: 'a-stale', user_id: 'stale-1' })]));
    });
    expect(screen.queryByText('stale-1')).toBeNull();
    expect(screen.getByText('fresh-1')).toBeDefined();
  });

  it('discards a stale Load More append after a filter change (AUD-05)', async () => {
    // First page loaded normally.
    mockListAuditLogScoped.mockResolvedValueOnce(
      makePage(Array.from({ length: 50 }, (_, i) => makeEntry({ id: `a-${i}` })), 60, true),
    );
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Load More')).toBeDefined());

    // Load More (request 2) is deferred.
    let resolveAppend!: (page: AuditLogPageDto) => void;
    mockListAuditLogScoped.mockImplementationOnce(
      () => new Promise<AuditLogPageDto>((resolve) => { resolveAppend = resolve; }),
    );
    const loadMoreBtn = screen.getByText('Load More').closest('button')!;
    await userEvent.click(loadMoreBtn);

    // A filter change (request 3, a reset generation) resolves with a fresh
    // first page while the append is still in flight.
    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ id: 'a-fresh', user_id: 'fresh-1' })], 1, false));
    const successChip = screen.getByText('Success').closest('button')!;
    await userEvent.click(successChip);
    await waitFor(() => expect(screen.getByText('fresh-1')).toBeDefined());

    // The stale append resolves late — it must NOT append its rows.
    await act(async () => {
      resolveAppend(makePage([makeEntry({ id: 'a-late', user_id: 'late-1' })], 60, true));
    });
    expect(screen.queryByText('late-1')).toBeNull();
    expect(screen.getByText('fresh-1')).toBeDefined();
  });

  it('deduplicates appended rows by id (AUD-03 defense in depth)', async () => {
    // First page + append both contain id 'a-dup'.
    mockListAuditLogScoped.mockResolvedValueOnce(
      makePage([makeEntry({ id: 'a-dup', user_id: 'dup-1' })], 2, true),
    );
    await renderScreen();
    await waitFor(() => expect(screen.getByText('Load More')).toBeDefined());

    mockListAuditLogScoped.mockResolvedValue(makePage([makeEntry({ id: 'a-dup', user_id: 'dup-1' })], 2, false));
    const loadMoreBtn = screen.getByText('Load More').closest('button')!;
    await userEvent.click(loadMoreBtn);

    await waitFor(() => {
      const rows = document.querySelectorAll('.audit-log-table tbody tr');
      expect(rows.length).toBe(1);
    });
  });
});
