// ── IPC contract tests for audit.ts ───────────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  listAuditLog,
  listAuditLogScoped,
  getAuditReviewStatusScoped,
  markAuditReviewedScoped,
  exportAuditLogScoped,
} from '@/api/audit';

describe('audit.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('listAuditLog → list_audit_log with { args: { limit, offset } }', async () => {
    mockInvoke.mockResolvedValue([]);
    await listAuditLog(50, 10);
    expect(mockInvoke).toHaveBeenCalledWith('list_audit_log', { args: { limit: 50, offset: 10 } });
  });

  it('listAuditLogScoped → list_audit_log_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ entries: [], total: 0 });
    await listAuditLogScoped('tok', { limit: 20, offset: 0, action: 'sale.completed' });
    expect(mockInvoke).toHaveBeenCalledWith('list_audit_log_scoped', { sessionToken: 'tok', args: expect.objectContaining({ limit: 20 }) });
  });

  it('getAuditReviewStatusScoped → get_audit_review_status_scoped', async () => {
    mockInvoke.mockResolvedValue({ lastReviewedAt: null, pendingCount: 5 });
    await getAuditReviewStatusScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('get_audit_review_status_scoped', { sessionToken: 'tok' });
  });

  it('markAuditReviewedScoped → mark_audit_reviewed_scoped', async () => {
    mockInvoke.mockResolvedValue({ lastReviewedAt: '2026-08-19T00:00:00Z', pendingCount: 0 });
    await markAuditReviewedScoped('tok', { upToEntryId: 'e1' });
    expect(mockInvoke).toHaveBeenCalledWith('mark_audit_reviewed_scoped', { sessionToken: 'tok', args: { upToEntryId: 'e1' } });
  });

  it('exportAuditLogScoped → export_audit_log_scoped', async () => {
    mockInvoke.mockResolvedValue({ csv: 'action,timestamp\nsale.completed,2026-08-19' });
    await exportAuditLogScoped('tok', { from: '2026-08-01', to: '2026-08-19' });
    expect(mockInvoke).toHaveBeenCalledWith('export_audit_log_scoped', { sessionToken: 'tok', args: { from: '2026-08-01', to: '2026-08-19' } });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('permission denied'));
    await expect(listAuditLog()).rejects.toThrow('permission denied');
  });
});
