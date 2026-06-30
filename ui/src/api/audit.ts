// ── Audit Log ─────────────────────────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

export interface AuditEntryDto {
  id: string;
  user_id: string;
  action: string;
  target_type: string | null;
  target_id: string | null;
  details: string;
  outcome: string;
  created_at: string;
}

export const listAuditLog = (limit: number = 100, offset: number = 0): Promise<AuditEntryDto[]> =>
  invoke<AuditEntryDto[]>('list_audit_log', {
    args: { limit, offset },
  });
