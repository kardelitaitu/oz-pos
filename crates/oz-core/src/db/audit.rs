//! Audit Log — append-only immutable entries.

use crate::AuditEntry;
use crate::error::CoreError;

use super::Store;

/// Keys whose values are considered secrets and are redacted before an audit
/// `details` payload is persisted (AUD-06). Match is case-insensitive.
const SENSITIVE_DETAIL_KEYS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "auth_token",
    "access_token",
    "refresh_token",
    "api_key",
    "apikey",
    "client_secret",
    "pin",
    "cvv",
    "cvc",
    "card_number",
    "cardnumber",
    "pan",
    "session_token",
    "authorization",
    "private_key",
];

/// Marker substituted for redacted secret values (AUD-06).
const REDACTED_MARKER: &str = "[REDACTED]";

/// Maximum persisted length (in chars) for an audit `details` payload
/// (AUD-06). Oversized payloads are truncated with an explicit marker.
const MAX_DETAIL_LEN: usize = 4000;

/// Maximum number of rows returned by a server-side audit export (AUD-09).
/// Guards memory/response size while still covering full incident and
/// retention windows.
pub const MAX_AUDIT_EXPORT_ROWS: u64 = 100_000;

/// True when any key in the JSON tree matches a sensitive key name
/// (case-insensitive). Used to decide whether re-serialisation is needed.
fn has_sensitive_key(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(k, v)| {
            SENSITIVE_DETAIL_KEYS
                .iter()
                .any(|s| k.eq_ignore_ascii_case(s))
                || has_sensitive_key(v)
        }),
        serde_json::Value::Array(items) => items.iter().any(has_sensitive_key),
        _ => false,
    }
}

/// Redact sensitive keys (case-insensitive) from a JSON value tree.
fn redact_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                if SENSITIVE_DETAIL_KEYS
                    .iter()
                    .any(|s| k.eq_ignore_ascii_case(s))
                {
                    out.insert(k.clone(), serde_json::Value::String(REDACTED_MARKER.into()));
                } else {
                    out.insert(k.clone(), redact_value(v));
                }
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(redact_value).collect())
        }
        other => other.clone(),
    }
}

/// Truncate an oversized details payload, appending an explicit marker so a
/// reader knows the record was summarised (AUD-06).
fn truncate_details(details: &str) -> String {
    if details.chars().count() <= MAX_DETAIL_LEN {
        details.to_string()
    } else {
        let mut truncated: String = details.chars().take(MAX_DETAIL_LEN).collect();
        truncated.push_str("…[truncated]");
        truncated
    }
}

/// Apply the AUD-06 policy: redact secret keys in JSON details, then cap the
/// payload size.
///
/// When no sensitive key is present the original string is returned verbatim
/// (preserving exact bytes — serde only re-serialises when a redaction is
/// actually needed). Non-JSON strings are only truncated.
fn sanitize_details(details: &str) -> String {
    let redacted = match serde_json::from_str::<serde_json::Value>(details) {
        Ok(value) if has_sensitive_key(&value) => {
            serde_json::to_string(&redact_value(&value)).unwrap_or_else(|_| details.to_string())
        }
        _ => details.to_string(),
    };
    truncate_details(&redacted)
}

impl Store<'_> {
    /// Insert a new audit log entry (append-only).
    ///
    /// AUD-06: the `details` payload is sanitised before persistence — secret
    /// keys are redacted and oversized payloads are truncated — so tokens,
    /// PINs, and customer data written by upstream callers never reach the
    /// audit table verbatim.
    pub fn log_audit(&self, entry: &AuditEntry) -> Result<(), CoreError> {
        let details = sanitize_details(&entry.details);
        self.conn.execute(
            "INSERT INTO audit_log (id, user_id, action, target_type, target_id, details, outcome, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry.id, entry.user_id, entry.action,
                entry.target_type, entry.target_id,
                details, entry.outcome, entry.created_at,
            ],
        )?;
        Ok(())
    }

    /// List audit log entries in reverse chronological order.
    pub fn list_audit_entries(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditEntry>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, action, target_type, target_id, details, outcome, created_at
             FROM audit_log ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![limit, offset], |row| {
            Ok(AuditEntry {
                id: row.get("id")?,
                user_id: row.get("user_id")?,
                action: row.get("action")?,
                target_type: row.get("target_type")?,
                target_id: row.get("target_id")?,
                details: row.get("details")?,
                outcome: row.get("outcome")?,
                created_at: row.get("created_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List audit log entries with server-side filters and keyset pagination.
    ///
    /// AUD-02/AUD-03: filtering and review counts are computed in the database
    /// (not over a loaded page), and paging uses a stable `(created_at, id)`
    /// cursor so new rows inserted between requests cannot shift the page
    /// boundary. Returns `(items, total_matching, has_more)`. The page size is
    /// clamped to `[1, 200]` and one extra row is fetched to compute `has_more`
    /// without an offset race.
    pub fn list_audit_entries_filtered(
        &self,
        outcome: Option<&str>,
        query: Option<&str>,
        before_created_at: Option<&str>,
        before_id: Option<&str>,
        limit: u64,
    ) -> Result<(Vec<AuditEntry>, u64, bool), CoreError> {
        let bounded = limit.clamp(1, 200);

        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1usize;

        if let Some(outcome) = outcome {
            let trimmed = outcome.trim();
            if !trimmed.is_empty() {
                where_clauses.push(format!("outcome = ?{idx}"));
                params.push(Box::new(trimmed.to_string()));
                idx += 1;
            }
        }

        if let Some(query) = query {
            let trimmed = query.trim();
            if !trimmed.is_empty() {
                // Escape LIKE wildcards so literal % or _ in the query does not
                // broaden the match (mirrors `search_customers`).
                let escaped = trimmed
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_");
                let pattern = format!("%{escaped}%");
                where_clauses.push(format!(
                    "(action LIKE ?{idx} ESCAPE '\\' OR COALESCE(target_type, '') LIKE ?{idx} ESCAPE '\\' \
                     OR COALESCE(target_id, '') LIKE ?{idx} ESCAPE '\\' OR user_id LIKE ?{idx} ESCAPE '\\')"
                ));
                params.push(Box::new(pattern));
                idx += 1;
            }
        }

        if let (Some(ct), Some(id)) = (before_created_at, before_id) {
            where_clauses.push(format!(
                "(created_at < ?{idx} OR (created_at = ?{idx} AND id < ?{}))",
                idx + 1
            ));
            params.push(Box::new(ct.to_string()));
            params.push(Box::new(id.to_string()));
            idx += 2;
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        // Total matching rows (before the cursor) — powers the server-side
        // "X of Y" count and the unreviewed badge.
        let total: u64 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM audit_log{where_sql}"),
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| row.get(0),
        )?;

        // Fetch one extra row to determine whether another page exists.
        params.push(Box::new(bounded + 1));
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, user_id, action, target_type, target_id, details, outcome, created_at
             FROM audit_log{where_sql} ORDER BY created_at DESC, id DESC LIMIT ?{idx}"
        ))?;
        let mut rows = stmt.query(rusqlite::params_from_iter(
            params.iter().map(|p| p.as_ref()),
        ))?;
        let mut items: Vec<AuditEntry> = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(AuditEntry {
                id: row.get("id")?,
                user_id: row.get("user_id")?,
                action: row.get("action")?,
                target_type: row.get("target_type")?,
                target_id: row.get("target_id")?,
                details: row.get("details")?,
                outcome: row.get("outcome")?,
                created_at: row.get("created_at")?,
            });
            if items.len() as u64 > bounded {
                break;
            }
        }
        let has_more = items.len() as u64 > bounded;
        if has_more {
            items.truncate(bounded as usize);
        }
        Ok((items, total, has_more))
    }

    /// Return ALL audit entries matching the optional filters (AUD-09).
    ///
    /// Unlike [`Self::list_audit_entries_filtered`] (which clamps pages to
    /// 200 rows), this returns every matching row in deterministic
    /// newest-first `(created_at, id)` order for a server-side export
    /// snapshot, bounded by [`MAX_AUDIT_EXPORT_ROWS`] so a runaway table
    /// cannot exhaust memory. Shares the exact outcome/query WHERE
    /// construction of the filtered listing (no keyset cursor — an export
    /// is a full snapshot, not a paged continuation).
    pub fn list_audit_entries_export(
        &self,
        outcome: Option<&str>,
        query: Option<&str>,
    ) -> Result<Vec<AuditEntry>, CoreError> {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1usize;

        if let Some(outcome) = outcome {
            let trimmed = outcome.trim();
            if !trimmed.is_empty() {
                where_clauses.push(format!("outcome = ?{idx}"));
                params.push(Box::new(trimmed.to_string()));
                idx += 1;
            }
        }

        if let Some(query) = query {
            let trimmed = query.trim();
            if !trimmed.is_empty() {
                let escaped = trimmed
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_");
                let pattern = format!("%{escaped}%");
                where_clauses.push(format!(
                    "(action LIKE ?{idx} ESCAPE '\\' OR COALESCE(target_type, '') LIKE ?{idx} ESCAPE '\\' \
                     OR COALESCE(target_id, '') LIKE ?{idx} ESCAPE '\\' OR user_id LIKE ?{idx} ESCAPE '\\')"
                ));
                params.push(Box::new(pattern));
                idx += 1;
            }
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        params.push(Box::new(MAX_AUDIT_EXPORT_ROWS));
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, user_id, action, target_type, target_id, details, outcome, created_at
             FROM audit_log{where_sql} ORDER BY created_at DESC, id DESC LIMIT ?{idx}"
        ))?;
        let mut rows = stmt.query(rusqlite::params_from_iter(
            params.iter().map(|p| p.as_ref()),
        ))?;
        let mut items: Vec<AuditEntry> = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(AuditEntry {
                id: row.get("id")?,
                user_id: row.get("user_id")?,
                action: row.get("action")?,
                target_type: row.get("target_type")?,
                target_id: row.get("target_id")?,
                details: row.get("details")?,
                outcome: row.get("outcome")?,
                created_at: row.get("created_at")?,
            });
        }
        Ok(items)
    }

    // ── Review checkpoints (AUD-04) ────────────────────────────────

    /// Persist a server-side review checkpoint and emit the matching
    /// `audit.review` audit event in one transaction (AUD-04).
    pub fn save_review_checkpoint(
        &self,
        cp: &crate::AuditReviewCheckpoint,
    ) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO audit_review_checkpoints
             (id, store_id, reviewer_user_id, reviewed_at,
              reviewed_through_created_at, reviewed_through_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                cp.id,
                cp.store_id,
                cp.reviewer_user_id,
                cp.reviewed_at,
                cp.reviewed_through_created_at,
                cp.reviewed_through_id,
            ],
        )?;
        // The review action itself is an audit event (append-only).
        let details = serde_json::json!({
            "reviewed_through_created_at": cp.reviewed_through_created_at,
            "reviewed_through_id": cp.reviewed_through_id,
        })
        .to_string();
        let event = crate::AuditEntry::new(
            &cp.reviewer_user_id,
            "audit.review",
            Some("audit_review_checkpoint"),
            Some(&cp.id),
            Some(details),
            "success",
        );
        tx.execute(
            "INSERT INTO audit_log (id, user_id, action, target_type, target_id, details, outcome, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                event.id, event.user_id, event.action,
                event.target_type, event.target_id,
                event.details, event.outcome, event.created_at,
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Most recent review checkpoint for this store (newest first).
    pub fn latest_review_checkpoint(
        &self,
    ) -> Result<Option<crate::AuditReviewCheckpoint>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, store_id, reviewer_user_id, reviewed_at,
                    reviewed_through_created_at, reviewed_through_id
             FROM audit_review_checkpoints
             ORDER BY reviewed_at DESC, id DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], |row| {
            Ok(crate::AuditReviewCheckpoint {
                id: row.get(0)?,
                store_id: row.get(1)?,
                reviewer_user_id: row.get(2)?,
                reviewed_at: row.get(3)?,
                reviewed_through_created_at: row.get(4)?,
                reviewed_through_id: row.get(5)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    /// Count audit entries created strictly after a timestamp. Powers the
    /// server-side unreviewed badge (AUD-02/AUD-04) over the full table,
    /// independent of the currently loaded page.
    pub fn count_audit_entries_after(&self, created_at: &str) -> Result<u64, CoreError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE created_at > ?1",
            rusqlite::params![created_at],
            |row| row.get(0),
        )?;
        Ok(n.max(0) as u64)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "audit_tests.rs"]
mod tests;
