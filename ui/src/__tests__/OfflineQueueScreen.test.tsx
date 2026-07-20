import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
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

  it('shows sync result after Sync All succeeds', async () => {
    mockListAllOffline.mockResolvedValue([makeQueueItem()]);
    mockPendingOfflineCount.mockResolvedValue(1);
    mockRetryOfflineSync.mockResolvedValue({ synced: 1, failed: 0 });

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
});
