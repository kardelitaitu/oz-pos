import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  listAuditLogScoped,
  markAuditReviewedScoped,
  exportAuditLogScoped,
  listAuditLog,
  getAuditReviewStatusScoped,
} from '@/api/audit';

describe('audit.ts API contract', () => {
  const TOKEN = 'tok_audit';

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('listAuditLogScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ entries: [], total: 0 });
    await listAuditLogScoped(TOKEN, { limit: 50 });
    expect(mockInvoke).toHaveBeenCalledWith('list_audit_log_scoped', {
      sessionToken: TOKEN,
      args: { limit: 50 },
    });
  });

  it('markAuditReviewedScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ reviewedAt: '2026-01-01' });
    await markAuditReviewedScoped(TOKEN, {
      reviewedThroughCreatedAt: '2026-01-01T00:00:00Z',
      reviewedThroughId: 'entry-1',
    });
    expect(mockInvoke).toHaveBeenCalledWith('mark_audit_reviewed_scoped', {
      sessionToken: TOKEN,
      args: {
        reviewedThroughCreatedAt: '2026-01-01T00:00:00Z',
        reviewedThroughId: 'entry-1',
      },
    });
  });

  it('exportAuditLogScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ csv: 'col1,col2', row_count: 10 });
    const result = await exportAuditLogScoped(TOKEN, { outcome: 'success' });
    expect(mockInvoke).toHaveBeenCalledWith('export_audit_log_scoped', {
      sessionToken: TOKEN,
      args: { outcome: 'success' },
    });
    expect(result.row_count).toBe(10);
  });

  it('listAuditLog calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listAuditLog(100, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_audit_log', { args: { limit: 100, offset: 0 } });
  });

  it('getAuditReviewStatusScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ total: 100, reviewed: 50 });
    await getAuditReviewStatusScoped(TOKEN);
    expect(mockInvoke).toHaveBeenCalledWith('get_audit_review_status_scoped', {
      sessionToken: TOKEN,
    });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('db error'));
    await expect(listAuditLogScoped(TOKEN, {})).rejects.toThrow('db error');
  });
});
