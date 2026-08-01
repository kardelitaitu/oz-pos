//! Audit Log — append-only immutable entries.

use crate::AuditEntry;
use crate::error::CoreError;

use super::Store;

impl Store<'_> {
    /// Insert a new audit log entry (append-only).
    pub fn log_audit(&self, entry: &AuditEntry) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO audit_log (id, user_id, action, target_type, target_id, details, outcome, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry.id, entry.user_id, entry.action,
                entry.target_type, entry.target_id,
                entry.details, entry.outcome, entry.created_at,
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
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn seed_audit_entries(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO audit_log (id, user_id, action, target_type, target_id, details, outcome, created_at) VALUES
                ('aud-1', 'user-1', 'sale.create',  'sale', 'sale-1', '{\"total\":1000}', 'success', '2025-01-01T12:00:00.000Z'),
                ('aud-2', 'user-2', 'sale.void',    'sale', 'sale-2', '{\"reason\":\"test\"}', 'success', '2025-01-01T12:05:00.000Z'),
                ('aud-3', 'user-1', 'product.create','product','prod-1','{}','success','2025-01-01T13:00:00.000Z'),
                ('aud-4', 'system', 'user.login',   'user',  'user-1', '{}', 'failure', '2025-01-01T14:00:00.000Z');"
        ).unwrap();
    }

    // ── log_audit ───────────────────────────────────────────────────

    #[test]
    fn log_audit_persists_entry() {
        let conn = fresh();
        let s = store(&conn);
        let entry = AuditEntry::new(
            "user-1",
            "sale.create",
            Some("sale".to_string()),
            Some("sale-99".to_string()),
            Some("{\"total\":500}".to_string()),
            "success",
        );
        s.log_audit(&entry).unwrap();

        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "sale.create");
        assert_eq!(entries[0].user_id, "user-1");
        assert_eq!(entries[0].target_id.as_deref(), Some("sale-99"));
        assert_eq!(entries[0].outcome, "success");
    }

    #[test]
    fn log_audit_nullable_types() {
        let conn = fresh();
        let s = store(&conn);
        let entry = AuditEntry::new(
            "user-1",
            "test.event",
            None::<String>,
            None::<String>,
            None::<String>,
            "info",
        );
        s.log_audit(&entry).unwrap();

        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].target_type.is_none());
        assert!(entries[0].target_id.is_none());
        assert_eq!(entries[0].details, "{}");
    }

    #[test]
    fn log_audit_multiple_entries() {
        let conn = fresh();
        let s = store(&conn);
        for i in 0..5 {
            let entry = AuditEntry::new(
                "user-1",
                format!("event.{i}"),
                None::<String>,
                None::<String>,
                None::<String>,
                "ok",
            );
            s.log_audit(&entry).unwrap();
        }
        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 5);
    }

    // ── list_audit_entries ──────────────────────────────────────────

    #[test]
    fn list_audit_entries_empty_db() {
        let conn = fresh();
        let entries = store(&conn).list_audit_entries(10, 0).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn list_audit_entries_returns_all() {
        let conn = fresh();
        seed_audit_entries(&conn);
        let entries = store(&conn).list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 4);
    }

    #[test]
    fn list_audit_entries_ordered_desc() {
        let conn = fresh();
        seed_audit_entries(&conn);
        let entries = store(&conn).list_audit_entries(10, 0).unwrap();
        // Most recent first.
        assert_eq!(entries[0].id, "aud-4");
        assert_eq!(entries[1].id, "aud-3");
        assert_eq!(entries[2].id, "aud-2");
        assert_eq!(entries[3].id, "aud-1");
    }

    #[test]
    fn list_audit_entries_respects_limit() {
        let conn = fresh();
        seed_audit_entries(&conn);
        let entries = store(&conn).list_audit_entries(2, 0).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn list_audit_entries_pagination() {
        let conn = fresh();
        seed_audit_entries(&conn);
        let page1 = store(&conn).list_audit_entries(2, 0).unwrap();
        let page2 = store(&conn).list_audit_entries(2, 2).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_ne!(page1[0].id, page2[0].id);
        // Combined should cover all 4.
    }

    #[test]
    fn list_audit_entries_large_offset() {
        let conn = fresh();
        seed_audit_entries(&conn);
        let entries = store(&conn).list_audit_entries(10, 100).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn list_audit_entries_includes_null_details() {
        let conn = fresh();
        seed_audit_entries(&conn);
        let entries = store(&conn).list_audit_entries(10, 0).unwrap();
        let login_entry = entries.iter().find(|e| e.action == "user.login").unwrap();
        assert_eq!(login_entry.outcome, "failure");
        assert_eq!(login_entry.details, "{}");
    }

    #[test]
    fn audit_log_with_large_details() {
        let conn = fresh();
        let s = store(&conn);

        let large_details = format!(
            "{{\"payload\":\"{}\",\"metadata\":{{\"count\":{}}}}}",
            "x".repeat(2000),
            42
        );
        assert!(large_details.len() > 2000);

        let entry = AuditEntry::new(
            "user-1",
            "bulk.import",
            Some("product".to_string()),
            Some("batch-99".to_string()),
            Some(large_details.clone()),
            "success",
        );
        s.log_audit(&entry).unwrap();

        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].details, large_details);
    }

    #[test]
    fn audit_log_multiple_same_action() {
        let conn = fresh();
        let s = store(&conn);

        for i in 0..3 {
            let entry = AuditEntry::new(
                "user-1",
                "inventory.sync",
                Some("inventory".to_string()),
                Some(format!("item-{i}")),
                Some(format!("{{\"qty\":{i}}}")),
                "ok",
            );
            s.log_audit(&entry).unwrap();
        }

        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 3);
        // All should have the same action.
        assert!(entries.iter().all(|e| e.action == "inventory.sync"));
        // Should be in reverse chronological order (most recent first).
        assert_eq!(entries[0].target_id.as_deref(), Some("item-2"));
        assert_eq!(entries[1].target_id.as_deref(), Some("item-1"));
        assert_eq!(entries[2].target_id.as_deref(), Some("item-0"));
    }

    #[test]
    fn audit_log_limit_zero_returns_empty() {
        let conn = fresh();
        seed_audit_entries(&conn);
        let entries = store(&conn).list_audit_entries(0, 0).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn audit_log_exact_limit_matches_total() {
        let conn = fresh();
        seed_audit_entries(&conn);
        let entries = store(&conn).list_audit_entries(4, 0).unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].id, "aud-4");
    }

    #[test]
    fn audit_log_very_long_action_name() {
        let conn = fresh();
        let s = store(&conn);

        let long_action = "custom.event.".to_owned() + &"x".repeat(180);
        // "custom.event." = 13 chars, + 180 = 193
        assert_eq!(long_action.len(), 193);

        let entry = AuditEntry::new(
            "admin",
            &long_action,
            Some("test".to_string()),
            None::<String>,
            None::<String>,
            "info",
        );
        s.log_audit(&entry).unwrap();

        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action.len(), 193);
    }

    /// ── Additional edge cases (ADR-8 PCI-DSS §10) ───────────────────

    #[test]
    fn audit_log_duplicate_id_rejected() {
        let conn = fresh();
        let s = store(&conn);

        let entry = AuditEntry::new(
            "user-1",
            "test.dup",
            Some("x".to_string()),
            None::<String>,
            None::<String>,
            "success",
        );
        s.log_audit(&entry).unwrap();

        // Attempt to insert a second entry with the same ID via raw SQL
        let result = conn.execute(
            "INSERT INTO audit_log (id, user_id, action, target_type, target_id, details, outcome, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &entry.id, "user-1", "test.dup", "x", std::option::Option::<&str>::None,
                "{}", "success", &entry.created_at,
            ],
        );
        assert!(
            result.is_err(),
            "duplicate PK should produce SQLITE_CONSTRAINT"
        );
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("UNIQUE") || err.to_string().contains("constraint"),
            "expected constraint error, got: {err}"
        );

        // Verify only one entry exists
        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn audit_log_html_in_details_preserved() {
        let conn = fresh();
        let s = store(&conn);

        // HTML strings should be stored as-is (no sanitization at the DB layer)
        let html_details = r#"{"message":"<script>alert('xss')</script>","input":"<b>bold</b>"}"#;
        let entry = AuditEntry::new(
            "admin",
            "form.submit",
            Some("form".to_string()),
            Some("form-1".to_string()),
            Some(html_details.to_string()),
            "failure",
        );
        s.log_audit(&entry).unwrap();

        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(
            entries[0].details.contains("<script>"),
            "HTML in details should be stored as-is"
        );
        assert!(entries[0].details.contains("<b>bold</b>"));
    }

    #[test]
    fn audit_log_unicode_in_details() {
        let conn = fresh();
        let s = store(&conn);

        let unicode_details = r#"{"message":"Selamat pagi 🌏","emoji":"✅🚀","cjk":"你好世界"}"#;
        let entry = AuditEntry::new(
            "user-1",
            "i18n.test",
            None::<String>,
            None::<String>,
            Some(unicode_details.to_string()),
            "success",
        );
        s.log_audit(&entry).unwrap();

        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(
            entries[0].details.contains("🌏"),
            "emoji should survive round-trip"
        );
        assert!(
            entries[0].details.contains("你好世界"),
            "CJK should survive round-trip"
        );
    }

    #[test]
    fn audit_log_long_user_id() {
        let conn = fresh();
        let s = store(&conn);

        // 200-char user ID
        let long_user = "user-".to_owned() + &"a".repeat(195);
        assert_eq!(long_user.len(), 200);

        let entry = AuditEntry::new(
            &long_user,
            "bulk.import",
            None::<String>,
            None::<String>,
            None::<String>,
            "success",
        );
        s.log_audit(&entry).unwrap();

        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].user_id.len(), 200);
    }

    #[test]
    fn audit_log_special_chars_target_type() {
        let conn = fresh();
        let s = store(&conn);

        // Dotted namespace path, hyphenated, with numbers
        let entry = AuditEntry::new(
            "system",
            "setting.update",
            Some("oz-pos.settings.v3".to_string()),
            Some("workspace.123".to_string()),
            None::<String>,
            "success",
        );
        s.log_audit(&entry).unwrap();

        let entries = s.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].target_type.as_deref(),
            Some("oz-pos.settings.v3")
        );
        assert_eq!(entries[0].target_id.as_deref(), Some("workspace.123"));
    }

    // ── Filtered + keyset pagination (AUD-02/AUD-03) ──────────────

    #[test]
    fn filtered_entries_outcome_filter_matches_db_rows() {
        let conn = fresh();
        seed_audit_entries(&conn);
        let (items, total, has_more) = store(&conn)
            .list_audit_entries_filtered(Some("failure"), None, None, None, 50)
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, "user.login");
        assert!(!has_more);
    }

    #[test]
    fn filtered_entries_free_text_query_matches_action_and_target() {
        let conn = fresh();
        seed_audit_entries(&conn);
        // Query matches action (sale) — 2 rows.
        let (items, total, _) = store(&conn)
            .list_audit_entries_filtered(None, Some("sale"), None, None, 50)
            .unwrap();
        assert_eq!(total, 2);
        assert_eq!(items.len(), 2);
        // Query matches target_id (prod-1) — 1 row.
        let (items, total, _) = store(&conn)
            .list_audit_entries_filtered(None, Some("prod-1"), None, None, 50)
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(items[0].id, "aud-3");
    }

    #[test]
    fn filtered_entries_wildcard_query_is_escaped() {
        let conn = fresh();
        let s = store(&conn);
        s.log_audit(&AuditEntry::new(
            "user-1",
            "bulk.import",
            Some("product".to_string()),
            Some("batch-50%".to_string()),
            None::<String>,
            "success",
        ))
        .unwrap();
        // A bare '%' must not match every row — only literal-% rows.
        let (items, total, _) = store(&conn)
            .list_audit_entries_filtered(None, Some("%"), None, None, 50)
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(items[0].target_id.as_deref(), Some("batch-50%"));
    }

    #[test]
    fn filtered_entries_keyset_cursor_is_stable_and_excludes_prior_rows() {
        let conn = fresh();
        seed_audit_entries(&conn);
        // Page 1: most recent 2 (aud-4, aud-3).
        let (page1, total1, has_more1) = store(&conn)
            .list_audit_entries_filtered(None, None, None, None, 2)
            .unwrap();
        assert_eq!(total1, 4);
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].id, "aud-4");
        assert_eq!(page1[1].id, "aud-3");
        assert!(has_more1);

        // Page 2: continue strictly before (created_at, id) of last row.
        let last = &page1[1];
        let (page2, _, has_more2) = store(&conn)
            .list_audit_entries_filtered(None, None, Some(&last.created_at), Some(&last.id), 2)
            .unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].id, "aud-2");
        assert_eq!(page2[1].id, "aud-1");
        assert!(!has_more2);
    }

    #[test]
    fn filtered_entries_limit_clamped_to_200() {
        let conn = fresh();
        seed_audit_entries(&conn);
        let (items, total, _) = store(&conn)
            .list_audit_entries_filtered(None, None, None, None, 10_000)
            .unwrap();
        assert_eq!(items.len(), 4);
        assert_eq!(total, 4);
        // 0 clamps to 1 and still returns a row.
        let (items, _, _) = store(&conn)
            .list_audit_entries_filtered(None, None, None, None, 0)
            .unwrap();
        assert_eq!(items.len(), 1);
    }
}
