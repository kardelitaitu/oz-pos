//! Gift cards CRUD — issue, redeem, top-up, freeze, balance checks.

use rusqlite::params;

use crate::error::CoreError;
use crate::gift_card::{
    GiftCard, GiftCardFilter, GiftCardTransaction, GiftCardWithTransactions, IssueGiftCardInput,
    RedeemGiftCardResult,
};

use super::Store;

impl Store<'_> {
    /// Issue a new gift card and record the initial issue transaction.
    pub fn issue_gift_card(
        &self,
        input: IssueGiftCardInput,
    ) -> Result<GiftCardWithTransactions, CoreError> {
        if input.initial_amount_minor <= 0 {
            return Err(CoreError::Validation {
                field: "initial_amount_minor",
                message: "initial amount must be positive".into(),
            });
        }

        if input.card_number.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "card_number",
                message: "card number is required".into(),
            });
        }

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let pin = input.pin.unwrap_or_default();
        let issued_to = input.issued_to.unwrap_or_default();
        let amount = input.initial_amount_minor;

        let tx = self.conn.unchecked_transaction()?;

        // Create the gift card.
        tx.execute(
            "INSERT INTO gift_cards (id, card_number, pin, initial_balance_minor, current_balance_minor,
             currency, status, issued_to, issue_date, expiry_date, created_by, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                input.card_number.trim(),
                pin,
                amount,
                amount,
                input.currency,
                issued_to,
                now,
                input.expiry_date,
                input.created_by,
                now,
            ],
        )?;

        // Record the issue transaction.
        let txn_id = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO gift_card_transactions (id, gift_card_id, sale_id, txn_type, amount_minor,
             balance_after_minor, notes, created_at)
             VALUES (?1, ?2, NULL, 'issue', ?3, ?4, ?5, ?6)",
            params![
                txn_id,
                id,
                amount,
                amount,
                format!(
                    "Issued gift card {} with {}",
                    input.card_number.trim(),
                    amount
                ),
                now,
            ],
        )?;

        tx.commit()?;

        let card = self.get_gift_card_by_raw_id(&id)?.unwrap();
        Ok(GiftCardWithTransactions {
            transactions: vec![GiftCardTransaction {
                id: txn_id,
                gift_card_id: id,
                sale_id: None,
                txn_type: "issue".into(),
                amount_minor: amount,
                balance_after_minor: amount,
                notes: format!("Issued gift card {}", input.card_number.trim()),
                created_at: now,
            }],
            card,
        })
    }

    /// Look up a gift card by card number or id.
    pub fn get_gift_card(&self, card_number_or_id: &str) -> Result<Option<GiftCard>, CoreError> {
        // Try id first, then card_number.
        let mut stmt = self.conn.prepare(
            "SELECT id, card_number, pin, initial_balance_minor, current_balance_minor,
             currency, status, issued_to, issue_date, expiry_date, created_by, updated_at
             FROM gift_cards WHERE id = ?1 OR card_number = ?1",
        )?;

        let result = stmt.query_row(params![card_number_or_id], |row| {
            Ok(GiftCard {
                id: row.get("id")?,
                card_number: row.get("card_number")?,
                pin: row.get("pin")?,
                initial_balance_minor: row.get("initial_balance_minor")?,
                current_balance_minor: row.get("current_balance_minor")?,
                currency: row.get("currency")?,
                status: row.get("status")?,
                issued_to: row.get("issued_to")?,
                issue_date: row.get("issue_date")?,
                expiry_date: row.get("expiry_date")?,
                created_by: row.get("created_by")?,
                updated_at: row.get("updated_at")?,
            })
        });

        match result {
            Ok(card) => Ok(Some(card)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_gift_card_by_raw_id(&self, id: &str) -> Result<Option<GiftCard>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, card_number, pin, initial_balance_minor, current_balance_minor,
             currency, status, issued_to, issue_date, expiry_date, created_by, updated_at
             FROM gift_cards WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(GiftCard {
                id: row.get("id")?,
                card_number: row.get("card_number")?,
                pin: row.get("pin")?,
                initial_balance_minor: row.get("initial_balance_minor")?,
                current_balance_minor: row.get("current_balance_minor")?,
                currency: row.get("currency")?,
                status: row.get("status")?,
                issued_to: row.get("issued_to")?,
                issue_date: row.get("issue_date")?,
                expiry_date: row.get("expiry_date")?,
                created_by: row.get("created_by")?,
                updated_at: row.get("updated_at")?,
            })
        });

        match result {
            Ok(card) => Ok(Some(card)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List gift cards with optional filters.
    pub fn list_gift_cards(
        &self,
        filter: GiftCardFilter,
    ) -> Result<Vec<GiftCardWithTransactions>, CoreError> {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(ref search) = filter.search {
            let pattern = format!("%{}%", search);
            where_clauses.push(format!(
                "(g.card_number LIKE ?{param_idx} OR g.issued_to LIKE ?{param_idx})"
            ));
            param_values.push(Box::new(pattern));
            param_idx += 1;
        }

        if let Some(ref status) = filter.status {
            where_clauses.push(format!("g.status = ?{param_idx}"));
            param_values.push(Box::new(status.clone()));
            param_idx += 1;
        }

        if let Some(ref issued_to) = filter.issued_to {
            let pattern = format!("%{}%", issued_to);
            where_clauses.push(format!("g.issued_to LIKE ?{param_idx}"));
            param_values.push(Box::new(pattern));
            param_idx += 1;
        }

        if let Some(min_balance) = filter.min_balance {
            where_clauses.push(format!("g.current_balance_minor >= ?{param_idx}"));
            param_values.push(Box::new(min_balance));
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let sql = format!(
            "SELECT g.id, g.card_number, g.pin, g.initial_balance_minor, g.current_balance_minor,
             g.currency, g.status, g.issued_to, g.issue_date, g.expiry_date, g.created_by, g.updated_at
             FROM gift_cards g {where_sql} ORDER BY g.updated_at DESC"
        );

        let mut stmt = self.conn.prepare(&sql)?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let cards: Vec<GiftCard> = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(GiftCard {
                    id: row.get("id")?,
                    card_number: row.get("card_number")?,
                    pin: row.get("pin")?,
                    initial_balance_minor: row.get("initial_balance_minor")?,
                    current_balance_minor: row.get("current_balance_minor")?,
                    currency: row.get("currency")?,
                    status: row.get("status")?,
                    issued_to: row.get("issued_to")?,
                    issue_date: row.get("issue_date")?,
                    expiry_date: row.get("expiry_date")?,
                    created_by: row.get("created_by")?,
                    updated_at: row.get("updated_at")?,
                })
            })?
            .map(|r| Ok(r?))
            .collect::<Result<Vec<_>, CoreError>>()?;

        let mut results = Vec::new();
        for card in cards {
            let transactions = self.get_transactions_for_card(&card.id, 5)?;
            results.push(GiftCardWithTransactions { transactions, card });
        }

        Ok(results)
    }

    fn get_transactions_for_card(
        &self,
        gift_card_id: &str,
        limit: i64,
    ) -> Result<Vec<GiftCardTransaction>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, gift_card_id, sale_id, txn_type, amount_minor, balance_after_minor, notes, created_at
             FROM gift_card_transactions WHERE gift_card_id = ?1
             ORDER BY created_at DESC LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![gift_card_id, limit], |row| {
            Ok(GiftCardTransaction {
                id: row.get("id")?,
                gift_card_id: row.get("gift_card_id")?,
                sale_id: row.get("sale_id")?,
                txn_type: row.get("txn_type")?,
                amount_minor: row.get("amount_minor")?,
                balance_after_minor: row.get("balance_after_minor")?,
                notes: row.get("notes")?,
                created_at: row.get("created_at")?,
            })
        })?;

        rows.map(|r| Ok(r?)).collect()
    }

    /// Get full gift card detail with transaction history.
    pub fn get_gift_card_detail(
        &self,
        card_number_or_id: &str,
    ) -> Result<Option<GiftCardWithTransactions>, CoreError> {
        let card = match self.get_gift_card(card_number_or_id)? {
            Some(c) => c,
            None => return Ok(None),
        };

        let transactions = self.get_transactions_for_card(&card.id, 50)?;

        Ok(Some(GiftCardWithTransactions { card, transactions }))
    }

    /// Get the current balance of a gift card.
    pub fn get_gift_card_balance(
        &self,
        card_number_or_id: &str,
    ) -> Result<Option<(i64, String, String)>, CoreError> {
        let card = match self.get_gift_card(card_number_or_id)? {
            Some(c) => c,
            None => return Ok(None),
        };
        Ok(Some((
            card.current_balance_minor,
            card.currency,
            card.status,
        )))
    }

    /// Redeem a gift card for a sale. Idempotent for retry — if the same
    /// `(card_id, sale_id)` pair already has a redeem transaction, returns
    /// the existing result instead of double-deducting.
    pub fn redeem_gift_card(
        &self,
        card_number_or_id: &str,
        amount_minor: i64,
        sale_id: &str,
    ) -> Result<RedeemGiftCardResult, CoreError> {
        if amount_minor <= 0 {
            return Err(CoreError::Validation {
                field: "amount_minor",
                message: "redemption amount must be positive".into(),
            });
        }

        let card = match self.get_gift_card(card_number_or_id)? {
            Some(c) => c,
            None => {
                return Err(CoreError::NotFound {
                    entity: "gift_card",
                    id: card_number_or_id.to_owned(),
                });
            }
        };

        if card.status != "active" {
            return Err(CoreError::Validation {
                field: "status",
                message: format!("gift card is not active (status: {})", card.status),
            });
        }

        // Check expiry.
        if let Some(ref expiry) = card.expiry_date
            && chrono::Utc::now()
                > chrono::DateTime::parse_from_rfc3339(expiry)
                    .unwrap_or(chrono::DateTime::UNIX_EPOCH.into())
        {
            return Err(CoreError::Validation {
                field: "expiry_date",
                message: "gift card has expired".into(),
            });
        }

        if card.current_balance_minor < amount_minor {
            return Err(CoreError::Validation {
                field: "current_balance_minor",
                message: format!(
                    "insufficient balance: have {}, requested {}",
                    card.current_balance_minor, amount_minor
                ),
            });
        }

        // Idempotency check: if we already have a redeem txn for this sale + card, return it.
        let existing = self.conn.query_row(
            "SELECT id, amount_minor, balance_after_minor, notes, created_at
             FROM gift_card_transactions
             WHERE gift_card_id = ?1 AND sale_id = ?2 AND txn_type = 'redeem'",
            params![card.id, sale_id],
            |row| {
                Ok(GiftCardTransaction {
                    id: row.get(0)?,
                    gift_card_id: card.id.clone(),
                    sale_id: Some(sale_id.to_owned()),
                    txn_type: "redeem".into(),
                    amount_minor: row.get(1)?,
                    balance_after_minor: row.get(2)?,
                    notes: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        );

        if let Ok(txn) = existing {
            let updated = self.get_gift_card_by_raw_id(&card.id)?;
            return Ok(RedeemGiftCardResult {
                card: updated.unwrap_or(card),
                transaction: txn,
            });
        }

        let txn_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let new_balance = card.current_balance_minor - amount_minor;

        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO gift_card_transactions (id, gift_card_id, sale_id, txn_type, amount_minor,
             balance_after_minor, notes, created_at)
             VALUES (?1, ?2, ?3, 'redeem', ?4, ?5, ?6, ?7)",
            params![
                txn_id,
                card.id,
                sale_id,
                -amount_minor,
                new_balance,
                format!("Redeemed {} on sale {}", amount_minor, sale_id),
                now,
            ],
        )?;

        tx.execute(
            "UPDATE gift_cards SET current_balance_minor = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_balance, now, card.id],
        )?;

        // If balance is zero, auto-set status to redeemed.
        if new_balance == 0 {
            tx.execute(
                "UPDATE gift_cards SET status = 'redeemed' WHERE id = ?1",
                params![card.id],
            )?;
        }

        tx.commit()?;

        let updated_card = self.get_gift_card_by_raw_id(&card.id)?.unwrap();

        Ok(RedeemGiftCardResult {
            card: updated_card,
            transaction: GiftCardTransaction {
                id: txn_id,
                gift_card_id: card.id,
                sale_id: Some(sale_id.to_owned()),
                txn_type: "redeem".into(),
                amount_minor: -amount_minor,
                balance_after_minor: new_balance,
                notes: format!("Redeemed {} on sale {}", amount_minor, sale_id),
                created_at: now,
            },
        })
    }

    /// Top up a gift card with additional funds.
    pub fn top_up_gift_card(
        &self,
        card_number_or_id: &str,
        amount_minor: i64,
    ) -> Result<GiftCardWithTransactions, CoreError> {
        if amount_minor <= 0 {
            return Err(CoreError::Validation {
                field: "amount_minor",
                message: "top-up amount must be positive".into(),
            });
        }

        let card = match self.get_gift_card(card_number_or_id)? {
            Some(c) => c,
            None => {
                return Err(CoreError::NotFound {
                    entity: "gift_card",
                    id: card_number_or_id.to_owned(),
                });
            }
        };

        if card.status != "active" && card.status != "frozen" {
            return Err(CoreError::Validation {
                field: "status",
                message: format!("cannot top-up card with status: {}", card.status),
            });
        }

        let txn_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let new_balance = card.current_balance_minor + amount_minor;

        let tx = self.conn.unchecked_transaction()?;

        // If frozen, reactivate.
        if card.status == "frozen" {
            tx.execute(
                "UPDATE gift_cards SET status = 'active' WHERE id = ?1",
                params![card.id],
            )?;
        }

        tx.execute(
            "INSERT INTO gift_card_transactions (id, gift_card_id, sale_id, txn_type, amount_minor,
             balance_after_minor, notes, created_at)
             VALUES (?1, ?2, NULL, 'topup', ?3, ?4, ?5, ?6)",
            params![
                txn_id,
                card.id,
                amount_minor,
                new_balance,
                format!("Top-up of {} on card {}", amount_minor, card.card_number),
                now,
            ],
        )?;

        tx.execute(
            "UPDATE gift_cards SET current_balance_minor = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_balance, now, card.id],
        )?;

        tx.commit()?;

        let updated_card = self.get_gift_card_by_raw_id(&card.id)?.unwrap();
        let transactions = self.get_transactions_for_card(&card.id, 5)?;

        Ok(GiftCardWithTransactions {
            card: updated_card,
            transactions,
        })
    }

    /// Freeze a gift card (prevent further redemptions).
    pub fn freeze_gift_card(&self, card_number_or_id: &str) -> Result<GiftCard, CoreError> {
        let card = match self.get_gift_card(card_number_or_id)? {
            Some(c) => c,
            None => {
                return Err(CoreError::NotFound {
                    entity: "gift_card",
                    id: card_number_or_id.to_owned(),
                });
            }
        };

        if card.status != "active" {
            return Err(CoreError::Validation {
                field: "status",
                message: format!("cannot freeze card with status: {}", card.status),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.conn.execute(
            "UPDATE gift_cards SET status = 'frozen', updated_at = ?1 WHERE id = ?2",
            params![now, card.id],
        )?;

        self.get_gift_card_by_raw_id(&card.id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "gift_card",
                id: card.id,
            })
    }

    /// Unfreeze a gift card (re-enable redemptions).
    pub fn unfreeze_gift_card(&self, card_number_or_id: &str) -> Result<GiftCard, CoreError> {
        let card = match self.get_gift_card(card_number_or_id)? {
            Some(c) => c,
            None => {
                return Err(CoreError::NotFound {
                    entity: "gift_card",
                    id: card_number_or_id.to_owned(),
                });
            }
        };

        if card.status != "frozen" {
            return Err(CoreError::Validation {
                field: "status",
                message: format!("card is not frozen (status: {})", card.status),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.conn.execute(
            "UPDATE gift_cards SET status = 'active', updated_at = ?1 WHERE id = ?2",
            params![now, card.id],
        )?;

        self.get_gift_card_by_raw_id(&card.id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "gift_card",
                id: card.id,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        conn
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn seed_user(conn: &Connection, id: &str) {
        // The actual users schema (from 021_shifts.sql et al) uses
        // `username, pin_hash, display_name, role_id` rather than the
        // `name, pin, role` columns a casual reader might guess from
        // the crate's domain types. Seed the FK target role first.
        conn.execute(
            "INSERT OR IGNORE INTO roles (id, name, description, permissions, created_at, updated_at)
             VALUES ('role-owner', 'Owner', 'Owner role', '[\"*\"]',
                     '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id,
                                created_at, updated_at)
             VALUES (?1, ?2, 'hash', ?3, 'role-owner',
                     '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            params![id, id, id],
        )
        .unwrap();
    }

    #[test]
    fn issue_gift_card_creates_card_and_transaction() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        let result = store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-1001".into(),
                pin: None,
                initial_amount_minor: 50000,
                currency: "IDR".into(),
                issued_to: Some("Alice".into()),
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();

        assert_eq!(result.card.card_number, "GC-1001");
        assert_eq!(result.card.current_balance_minor, 50000);
        assert_eq!(result.card.status, "active");
        assert_eq!(result.transactions.len(), 1);
        assert_eq!(result.transactions[0].txn_type, "issue");
    }

    #[test]
    fn issue_gift_card_with_zero_amount_fails() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        let err = store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-1002".into(),
                pin: None,
                initial_amount_minor: 0,
                currency: "IDR".into(),
                issued_to: None,
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::Validation {
                field: "initial_amount_minor",
                ..
            }
        ));
    }

    #[test]
    fn get_gift_card_by_card_number() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-2001".into(),
                pin: None,
                initial_amount_minor: 100000,
                currency: "IDR".into(),
                issued_to: None,
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();

        let card = store(&conn).get_gift_card("GC-2001").unwrap().unwrap();
        assert_eq!(card.current_balance_minor, 100000);
    }

    #[test]
    fn get_gift_card_returns_none_for_unknown() {
        let conn = fresh();
        let card = store(&conn).get_gift_card("NONEXISTENT").unwrap();
        assert!(card.is_none());
    }

    #[test]
    fn get_gift_card_balance_returns_tuple() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-3001".into(),
                pin: None,
                initial_amount_minor: 75000,
                currency: "IDR".into(),
                issued_to: Some("Bob".into()),
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();

        let (balance, currency, status) = store(&conn)
            .get_gift_card_balance("GC-3001")
            .unwrap()
            .unwrap();
        assert_eq!(balance, 75000);
        assert_eq!(currency, "IDR");
        assert_eq!(status, "active");
    }

    #[test]
    fn redeem_gift_card_deducts_balance() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-4001".into(),
                pin: None,
                initial_amount_minor: 50000,
                currency: "IDR".into(),
                issued_to: None,
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();

        // Seed a sale for FK reference.
        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at, subtotal_minor, tax_total_minor)
             VALUES ('sale-1', 25000, 'IDR', 0, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', 25000, 0)",
            [],
        ).unwrap();

        let result = store(&conn)
            .redeem_gift_card("GC-4001", 25000, "sale-1")
            .unwrap();
        assert_eq!(result.card.current_balance_minor, 25000);
        assert_eq!(result.transaction.amount_minor, -25000);
        assert_eq!(result.transaction.txn_type, "redeem");
    }

    #[test]
    fn redeem_gift_card_is_idempotent() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-4002".into(),
                pin: None,
                initial_amount_minor: 50000,
                currency: "IDR".into(),
                issued_to: None,
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();

        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at, subtotal_minor, tax_total_minor)
             VALUES ('sale-2', 10000, 'IDR', 0, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', 10000, 0)",
            [],
        ).unwrap();

        let r1 = store(&conn)
            .redeem_gift_card("GC-4002", 10000, "sale-2")
            .unwrap();
        let r2 = store(&conn)
            .redeem_gift_card("GC-4002", 10000, "sale-2")
            .unwrap();
        assert_eq!(r1.card.current_balance_minor, r2.card.current_balance_minor);
        assert_eq!(r1.transaction.id, r2.transaction.id);
    }

    #[test]
    fn redeem_insufficient_balance_fails() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-5001".into(),
                pin: None,
                initial_amount_minor: 5000,
                currency: "IDR".into(),
                issued_to: None,
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();

        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at, subtotal_minor, tax_total_minor)
             VALUES ('sale-3', 50000, 'IDR', 0, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', 50000, 0)",
            [],
        ).unwrap();

        let err = store(&conn)
            .redeem_gift_card("GC-5001", 10000, "sale-3")
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::Validation {
                field: "current_balance_minor",
                ..
            }
        ));
    }

    #[test]
    fn top_up_increases_balance() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-6001".into(),
                pin: None,
                initial_amount_minor: 10000,
                currency: "IDR".into(),
                issued_to: None,
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();

        // SQLite's `gift_card_transactions.created_at` is stored at
        // millisecond precision (RFC-3339 ms via `chrono::SecondsFormat::Millis`).
        // When `issue` and `topup` land in the same millisecond, the
        // `ORDER BY created_at DESC` in `get_transactions_for_card` has
        // no deterministic tie-breaker — SQLite picks an arbitrary order
        // for tied rows, which makes the `transactions[0].txn_type ==
        // "topup"` assertion below flake. Sleeping 5ms guarantees a
        // distinct timestamp; the duration matches the existing pattern
        // in `crate::tests::shift_integration`.
        std::thread::sleep(std::time::Duration::from_millis(5));

        let result = store(&conn).top_up_gift_card("GC-6001", 20000).unwrap();
        assert_eq!(result.card.current_balance_minor, 30000);
        assert_eq!(result.transactions[0].txn_type, "topup");
    }

    #[test]
    fn freeze_and_unfreeze() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-7001".into(),
                pin: None,
                initial_amount_minor: 50000,
                currency: "IDR".into(),
                issued_to: None,
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();

        let frozen = store(&conn).freeze_gift_card("GC-7001").unwrap();
        assert_eq!(frozen.status, "frozen");

        let unfrozen = store(&conn).unfreeze_gift_card("GC-7001").unwrap();
        assert_eq!(unfrozen.status, "active");
    }

    #[test]
    fn list_gift_cards_with_filters() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-L1".into(),
                pin: None,
                initial_amount_minor: 10000,
                currency: "IDR".into(),
                issued_to: Some("Alice".into()),
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();
        store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-L2".into(),
                pin: None,
                initial_amount_minor: 20000,
                currency: "IDR".into(),
                issued_to: Some("Bob".into()),
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();

        let results = store(&conn)
            .list_gift_cards(GiftCardFilter {
                search: Some("Alice".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].card.card_number, "GC-L1");

        let all = store(&conn)
            .list_gift_cards(GiftCardFilter::default())
            .unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn redeem_on_frozen_card_fails() {
        let conn = fresh();
        seed_user(&conn, "staff-1");
        store(&conn)
            .issue_gift_card(IssueGiftCardInput {
                card_number: "GC-8001".into(),
                pin: None,
                initial_amount_minor: 50000,
                currency: "IDR".into(),
                issued_to: None,
                created_by: "staff-1".into(),
                expiry_date: None,
            })
            .unwrap();
        store(&conn).freeze_gift_card("GC-8001").unwrap();

        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at, subtotal_minor, tax_total_minor)
             VALUES ('sale-8', 10000, 'IDR', 0, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', 10000, 0)",
            [],
        ).unwrap();

        let err = store(&conn)
            .redeem_gift_card("GC-8001", 10000, "sale-8")
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::Validation {
                field: "status",
                ..
            }
        ));
    }
}
