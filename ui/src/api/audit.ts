// ── Audit Log ─────────────────────────────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

/** A single audit log entry recording an action performed by a user. */
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

/** Server-filtered, keyset-paginated page of audit entries (AUD-02/AUD-03). */
export interface AuditLogPageDto {
  items: AuditEntryDto[];
  total: number;
  has_more: boolean;
}

/** Arguments for the store-scoped audit query (AUD-01/02/03). */
export interface ListAuditLogScopedArgs {
  limit?: number;
  outcome?: string;
  query?: string;
  beforeCreatedAt?: string;
  beforeId?: string;
}

/** List audit log entries with pagination (global DB, legacy). */
export const listAuditLog = (limit: number = 100, offset: number = 0): Promise<AuditEntryDto[]> =>
  loggedInvoke<AuditEntryDto[]>('list_audit_log', {
    args: { limit, offset },
  });

/**
 * Server-filtered, keyset-paginated audit log for the session's store
 * (AUD-01/02/03). The session resolves the store and user server-side and
 * `audit:view` is enforced; filtering + counts run in the store DB.
 */
export const listAuditLogScoped = (
  sessionToken: string,
  args: ListAuditLogScopedArgs,
): Promise<AuditLogPageDto> =>
  loggedInvoke<AuditLogPageDto>('list_audit_log_scoped', { sessionToken, args });

// ── Review checkpoints (AUD-04) ───────────────────────────────────

/** A persisted server-side review checkpoint (AUD-04). */
export interface ReviewCheckpointDto {
  id: string;
  store_id: string;
  reviewer_user_id: string;
  reviewed_at: string;
  reviewed_through_created_at: string;
  reviewed_through_id: string;
}

/** Latest checkpoint + server-side unreviewed count (AUD-04). */
export interface AuditReviewStatusDto {
  checkpoint: ReviewCheckpointDto | null;
  unreviewed_count: number;
}

/** Fetch the session store's latest review checkpoint + unreviewed count. */
export const getAuditReviewStatusScoped = (
  sessionToken: string,
): Promise<AuditReviewStatusDto> =>
  loggedInvoke<AuditReviewStatusDto>('get_audit_review_status_scoped', { sessionToken });

/** Mark the audit log reviewed up to the given high-water mark (AUD-04). */
export const markAuditReviewedScoped = (
  sessionToken: string,
  args: { reviewedThroughCreatedAt: string; reviewedThroughId: string },
): Promise<ReviewCheckpointDto> =>
  loggedInvoke<ReviewCheckpointDto>('mark_audit_reviewed_scoped', { sessionToken, args });

// ── Export (AUD-09) ───────────────────────────────────────────────

/** Arguments for the server-side audit export (AUD-09). */
export interface ExportAuditLogArgs {
  outcome?: string;
  query?: string;
}

/** Result of a server-side audit export (AUD-09). */
export interface AuditExportDto {
  /** RFC-4180 CSV artifact (UTF-8 BOM + header + rows, newest first). */
  csv: string;
  /** Number of rows exported. */
  row_count: number;
  /** ISO-8601 generation timestamp. */
  generated_at: string;
  /** User who requested the export. */
  requested_by: string;
}

/**
 * Export the session store's audit log to CSV (AUD-09). Server-side: the
 * session selects the store and user, `audit:export` is enforced, and an
 * `audit.export` event records the filter scope + row count.
 */
export const exportAuditLogScoped = (
  sessionToken: string,
  args: ExportAuditLogArgs,
): Promise<AuditExportDto> =>
  loggedInvoke<AuditExportDto>('export_audit_log_scoped', { sessionToken, args });
