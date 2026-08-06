import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import offlineFtl from '@/locales/offline.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import OfflineQueueScreen from '@/features/offline/OfflineQueueScreen';

// ── Mocks ────────────────────────────────────────────────────────────

const mockListAllOffline = vi.fn();
const mockPendingOfflineCount = vi.fn();
const mockRetryOfflineSync = vi.fn();
const mockDeleteOfflineItem = vi.fn();
const mockOfflineQueueStatusSummary = vi.fn();

vi.mock('@/api/offline', () => ({
  listAllOffline: (...args: unknown[]) => mockListAllOffline(...args),
  pendingOfflineCount: (...args: unknown[]) => mockPendingOfflineCount(...args),
  retryOfflineSync: (...args: unknown[]) => mockRetryOfflineSync(...args),
  deleteOfflineItem: (...args: unknown[]) => mockDeleteOfflineItem(...args),
  getOfflineQueueStatusSummary: (...args: unknown[]) => mockOfflineQueueStatusSummary(...args),
}));

// ── Helpers ───────────────────────────────────────────────────────────

function makeQueueItem(overrides: Record<string, unknown> = {}) {
  return {
    id: 'oq-1',
    action: 'sale.create',
    status: 'pending',
    retryCount: 0,
    lastError: null,
    createdAt: '2025-07-07T12:00:00.000Z',
    syncedAt: null,
    ...overrides,
  };
}

function renderScreen() {
  return renderWithFluentSync(<OfflineQueueScreen />, offlineFtl, sharedFtl);
}

// ── Tests ─────────────────────────────────────────────────────────────

describe('OfflineQueueScreen', () => {
  beforeEach(() => {
    mockListAllOffline.mockReset();
    mockPendingOfflineCount.mockReset();
    mockRetryOfflineSync.mockReset();
    mockDeleteOfflineItem.mockReset();
    mockOfflineQueueStatusSummary.mockReset();
    mockListAllOffline.mockResolvedValue([]);
    mockPendingOfflineCount.mockResolvedValue(0);
    mockOfflineQueueStatusSummary.mockResolvedValue({
      pendingCount: 0, syncedCount: 0, failedCount: 0, conflictCount: 0,
      lastSyncedAt: null, oldestPendingAt: null,
    });
  });

  it('renders the title', async () => {
    mockListAllOffline.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Offline Queue')).toBeTruthy();
    });
  });

  it('shows loading skeleton initially', () => {
    mockListAllOffline.mockImplementation(() => new Promise(() => {}));
    renderScreen();

    expect(document.querySelector('.offline-queue-loading-skeleton')).toBeTruthy();
  });

  it('shows empty state when no items', async () => {
    mockListAllOffline.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('All transactions synced. No pending items.')).toBeTruthy();
    });
  });

  it('shows error state with retry', async () => {
    mockListAllOffline.mockRejectedValue(new Error('Failed'));
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Retry')).toBeTruthy();
    });
  });

  it('calls load when retry button is clicked after error', async () => {
    // First call during mount rejects — shows error state
    mockListAllOffline.mockRejectedValueOnce(new Error('Failed'));
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Retry')).toBeTruthy();
    });

    // Clear mock so retry uses the default resolved value from beforeEach
    mockListAllOffline.mockClear();
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Retry' }));

    await waitFor(() => {
      expect(mockListAllOffline).toHaveBeenCalledTimes(1);
    });
  });

  it('renders queue items in a table', async () => {
    mockListAllOffline.mockResolvedValue([
      makeQueueItem(),
      makeQueueItem({ id: 'oq-2', action: 'product.update', status: 'failed', retryCount: 3, lastError: 'timeout' }),
    ]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Action')).toBeTruthy();
      expect(screen.getByText('Status')).toBeTruthy();
      expect(screen.getByText('Retries')).toBeTruthy();
      expect(screen.getByText('sale.create')).toBeTruthy();
      expect(screen.getByText('product.update')).toBeTruthy();
    });
  });

  it('shows status badges with correct classes', async () => {
    mockListAllOffline.mockResolvedValue([
      makeQueueItem({ id: 'oq-1', status: 'pending' }),
      makeQueueItem({ id: 'oq-2', status: 'synced' }),
      makeQueueItem({ id: 'oq-3', status: 'failed', lastError: 'error', retryCount: 2 }),
    ]);
    renderScreen();

    await waitFor(() => {
      expect(document.querySelector('.status-pending')).toBeTruthy();
      expect(document.querySelector('.status-synced')).toBeTruthy();
      expect(document.querySelector('.status-failed')).toBeTruthy();
    });
  });

  it('shows retry counts', async () => {
    mockListAllOffline.mockResolvedValue([makeQueueItem({ id: 'oq-1', retryCount: 5 })]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('5')).toBeTruthy();
    });
  });

  it('shows last error text', async () => {
    mockListAllOffline.mockResolvedValue([makeQueueItem({ lastError: 'Connection refused' })]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Connection refused')).toBeTruthy();
    });
  });

  it('each row has a Delete button', async () => {
    mockListAllOffline.mockResolvedValue([makeQueueItem()]);
    renderScreen();

    await waitFor(() => {
      const deleteBtns = screen.getAllByText('Delete');
      expect(deleteBtns.length).toBe(1);
    });
  });

  it('calls deleteOfflineItem on Delete click', async () => {
    mockListAllOffline.mockResolvedValueOnce([makeQueueItem()]);
    mockListAllOffline.mockResolvedValueOnce([]);
    mockDeleteOfflineItem.mockResolvedValue(undefined);

    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Delete')).toBeTruthy();
    });

    const user = userEvent.setup();
    await user.click(screen.getByText('Delete'));

    await waitFor(() => {
      expect(mockDeleteOfflineItem).toHaveBeenCalledWith('oq-1');
    });
  });

  it('shows pending count badge', async () => {
    mockListAllOffline.mockResolvedValue([makeQueueItem()]);
    mockPendingOfflineCount.mockResolvedValue(3);
    renderScreen();

    await waitFor(() => {
      const badge = document.querySelector('.offline-queue-badge');
      expect(badge).toBeTruthy();
      expect(badge!.textContent).toContain('pending');
    });
  });

  it('shows Sync All button', async () => {
    mockListAllOffline.mockResolvedValue([makeQueueItem()]);
    mockPendingOfflineCount.mockResolvedValue(3);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Sync All')).toBeTruthy();
    });
  });

  it('shows a stale notice after repeated poll failures (ERR-07)', async () => {
    vi.useFakeTimers();
    try {
      // Initial list load succeeds (empty queue); the 10s status poll fails.
      mockListAllOffline.mockResolvedValue([]);
      mockPendingOfflineCount.mockRejectedValue(new Error('poll fail'));
      mockOfflineQueueStatusSummary.mockResolvedValue({
        pendingCount: 0, syncedCount: 0, failedCount: 0, conflictCount: 0,
        lastSyncedAt: null, oldestPendingAt: null,
      });

      renderScreen();

      // Flush the initial load + the first poll attempt (fails once).
      await act(async () => { await Promise.resolve(); });
      expect(screen.queryByText('Queue status may be out of date.')).toBeNull();

      // Two more 10s cycles → three consecutive failures → stale notice.
      // Assert synchronously after each advance (waitFor does not advance
      // fake timers on its own).
      await act(async () => {
        vi.advanceTimersByTime(10_000);
        await Promise.resolve();
      });
      expect(screen.queryByText('Queue status may be out of date.')).toBeNull();

      await act(async () => {
        vi.advanceTimersByTime(10_000);
        await Promise.resolve();
      });
      expect(screen.getByText('Queue status may be out of date.')).toBeTruthy();
    } finally {
      vi.useRealTimers();
    }
  });

  it('keeps rows visible and announces refreshing during a reload (ERR-09)', async () => {
    // First load resolves with rows.
    mockListAllOffline.mockResolvedValueOnce([makeQueueItem()]);
    mockPendingOfflineCount.mockResolvedValue(1);
    // The reload (via Sync All → load) stays pending so the `refreshing`
    // phase is observable.
    mockListAllOffline.mockImplementationOnce(() => new Promise(() => {}));
    mockRetryOfflineSync.mockResolvedValue({ syncedCount: 0, failedCount: 0, totalCount: 1 });

    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('sale.create')).toBeTruthy();
    });

    // Trigger a reload while rows are on screen.
    const user = userEvent.setup();
    await user.click(screen.getByText('Sync All'));

    // Rows stay visible (no skeleton), and an accessible refreshing status
    // announces the retry intent.
    await waitFor(() => {
      expect(screen.getByText('sale.create')).toBeTruthy();
      const refreshing = document.querySelector('.offline-queue-refreshing');
      expect(refreshing).toBeTruthy();
      expect(refreshing?.getAttribute('role')).toBe('status');
      expect(refreshing?.getAttribute('aria-live')).toBe('polite');
    });
    expect(document.querySelector('.offline-queue-loading-skeleton')).toBeNull();
  });

  it('shows sync result after Sync All succeeds', async () => {
    mockListAllOffline.mockResolvedValue([makeQueueItem()]);
    mockPendingOfflineCount.mockResolvedValue(1);
    mockRetryOfflineSync.mockResolvedValue({ syncedCount: 1, failedCount: 0, totalCount: 1 });

    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Sync All')).not.toBeDisabled();
    });

    const user = userEvent.setup();
    await user.click(screen.getByText('Sync All'));

    await waitFor(() => {
      expect(mockRetryOfflineSync).toHaveBeenCalled();
    });
    // During load() the skeleton table re-renders with <th>Synced At</th>,
    // which also matches /Synced/i — use a more specific check.
    await waitFor(() => {
      const syncMessages = screen.getAllByText(/Synced/i);
      expect(syncMessages.some((el) => el.textContent?.includes('items'))).toBe(true);
    });
  });

  it('renders the exact synced/failed counts from the Rust camelCase DTO (OFF-02)', async () => {
    // OFF-02: the UI must render the counts exactly as the Rust SyncResult
    // serializes them (syncedCount/failedCount/totalCount). A mismatched
    // contract would render undefined counts and mislead the operator.
    mockListAllOffline.mockResolvedValue([makeQueueItem()]);
    mockPendingOfflineCount.mockResolvedValue(1);
    mockRetryOfflineSync.mockResolvedValue({ syncedCount: 3, failedCount: 2, totalCount: 5 });

    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Sync All')).not.toBeDisabled();
    });

    const user = userEvent.setup();
    await user.click(screen.getByText('Sync All'));

    // The localized message is "Synced { $synced } items, { $failed } failed."
    await waitFor(() => {
      const msg = screen.getByText(/Synced 3 items, 2 failed/);
      expect(msg).toBeTruthy();
    });
    // No undefined counts leak into the message.
    expect(screen.queryByText(/Synced undefined/)).toBeNull();
  });
});
