use super::*;
use crate::migrations;
use rusqlite::Connection;

fn fresh() -> Connection {
    migrations::fresh_db()
}

fn store(conn: &Connection) -> Store<'_> {
    Store::new(conn)
}

fn seed_customer(conn: &Connection, id: &str, name: &str) {
    conn.execute(
        "INSERT INTO customers (id, name, notes, created_at, updated_at)
         VALUES (?1, ?2, '', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
        params![id, name],
    )
    .unwrap();
}

fn seed_sale(conn: &Connection, id: &str) {
    seed_sale_for_customer(conn, id, None, 0);
}

fn seed_sale_for_customer(
    conn: &Connection,
    id: &str,
    customer_id: Option<&str>,
    total_minor: i64,
) {
    conn.execute(
        "INSERT INTO sales (id, total_minor, currency, line_count, status, customer_id, created_at, updated_at, subtotal_minor, tax_total_minor)
         VALUES (?1, ?2, 'USD', 0, 'completed', ?3, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', ?2, 0)",
        params![id, total_minor, customer_id],
    )
    .unwrap();
}

#[test]
fn get_or_create_creates_account() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    let account = store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();
    assert_eq!(account.customer_id, "cust-1");
    assert_eq!(account.points, 0);
    assert_eq!(account.lifetime_points, 0);
    assert_eq!(account.tier_id.as_deref(), Some("tier-bronze"));
}

#[test]
fn get_or_create_returns_existing() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    let a1 = store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();
    let a2 = store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();
    assert_eq!(a1.id, a2.id);
}

#[test]
fn earn_points_creates_transaction() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    let txn = store(&conn).earn_points("cust-1", "sale-1", 1000).unwrap();
    assert_eq!(txn.txn_type, "earn");
    assert_eq!(txn.points, 100); // 1000 * 10 / 100 * 1.0

    let details = store(&conn).get_loyalty_account("cust-1").unwrap().unwrap();
    assert_eq!(details.account.points, 100);
    assert_eq!(details.account.lifetime_points, 100);
}

#[test]
fn earn_points_is_idempotent_for_replayed_sale() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    let first = store(&conn).earn_points("cust-1", "sale-1", 1000).unwrap();
    let second = store(&conn).earn_points("cust-1", "sale-1", 1000).unwrap();

    assert_eq!(first.id, second.id);
    let details = store(&conn).get_loyalty_account("cust-1").unwrap().unwrap();
    assert_eq!(details.account.points, first.points);
    assert_eq!(details.account.lifetime_points, first.points);
    assert_eq!(details.recent_transactions.len(), 1);
}

#[test]
fn earn_points_is_idempotent_even_when_replay_total_differs() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    let first = store(&conn).earn_points("cust-1", "sale-1", 1000).unwrap();
    let second = store(&conn).earn_points("cust-1", "sale-1", 9999).unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.points, second.points);
}

#[test]
fn update_tier_rejects_invalid_values() {
    let conn = fresh();
    let s = store(&conn);

    for (field, args) in [
        ("name", ("", 0, 10, 1.0, "#ffffff")),
        ("min_points", ("Bronze", -1, 10, 1.0, "#ffffff")),
        ("points_per_unit", ("Bronze", 0, 0, 1.0, "#ffffff")),
        ("earn_multiplier", ("Bronze", 0, 10, 0.0, "#ffffff")),
        ("colour", ("Bronze", 0, 10, 1.0, "not-a-colour")),
    ] {
        let err = s
            .update_tier("tier-bronze", args.0, args.1, args.2, args.3, args.4)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field: actual, .. } if actual == field));
    }
}

#[test]
fn earn_points_with_silver_multiplier() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    // Create account first, then bump to Silver (1.25x multiplier).
    store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();
    conn.execute(
        "UPDATE loyalty_accounts SET tier_id = 'tier-silver' WHERE customer_id = 'cust-1'",
        [],
    )
    .unwrap();

    let txn = store(&conn).earn_points("cust-1", "sale-1", 1000).unwrap();
    // 1000 * 10 / 100 * 1.25 = 125
    assert_eq!(txn.points, 125);
}

#[test]
fn redeem_points_deducts_and_returns_value() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale_for_customer(&conn, "sale-1", Some("cust-1"), 5000);
    seed_sale_for_customer(&conn, "sale-2", Some("cust-1"), 1000);
    store(&conn).earn_points("cust-1", "sale-1", 5000).unwrap();

    let (txn, discount) = store(&conn).redeem_points("cust-1", 200, "sale-2").unwrap();
    assert_eq!(txn.points, -200);
    assert_eq!(discount, 200); // 200 points = 200 minor units
    // The seeded sale is USD (exp 2): 200 minor units render as a decimal.
    assert_eq!(txn.description, "Redeemed 200 points for 2.00 discount");

    let details = store(&conn).get_loyalty_account("cust-1").unwrap().unwrap();
    // 5000 * 10 / 100 * 1.0 = 500 earned - 200 redeemed = 300
    assert_eq!(details.account.points, 300);
}

#[test]
fn redeem_points_note_uses_sale_currency_exponent() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale_for_customer(&conn, "sale-earn", Some("cust-1"), 5000);
    // IDR sale (exp 0): the minor unit IS the Rupiah, so the discount
    // stays raw in the description.
    conn.execute(
        "INSERT INTO sales (id, total_minor, currency, line_count, status, customer_id, created_at, updated_at, subtotal_minor, tax_total_minor)
         VALUES ('sale-idr', 10000, 'IDR', 0, 'completed', 'cust-1', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', 10000, 0)",
        [],
    )
    .unwrap();
    store(&conn)
        .earn_points("cust-1", "sale-earn", 5000)
        .unwrap();

    let (txn, discount) = store(&conn)
        .redeem_points("cust-1", 500, "sale-idr")
        .unwrap();
    assert_eq!(discount, 500);
    assert_eq!(txn.description, "Redeemed 500 points for 500 discount");

    // The DB-stored row (written inside the transaction) formats the same way.
    let details = store(&conn).get_loyalty_account("cust-1").unwrap().unwrap();
    let stored = details
        .recent_transactions
        .iter()
        .find(|t| t.txn_type == "redeem")
        .unwrap();
    assert_eq!(stored.description, "Redeemed 500 points for 500 discount");
}

#[test]
fn redeem_points_retry_returns_original_transaction_after_balance_changes() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale_for_customer(&conn, "sale-earn", Some("cust-1"), 5000);
    seed_sale_for_customer(&conn, "sale-redeem", Some("cust-1"), 1000);
    store(&conn)
        .earn_points("cust-1", "sale-earn", 5000)
        .unwrap();

    let first = store(&conn)
        .redeem_points("cust-1", 200, "sale-redeem")
        .unwrap();
    let retry = store(&conn)
        .redeem_points("cust-1", 200, "sale-redeem")
        .unwrap();

    assert_eq!(first.0.id, retry.0.id);
    assert_eq!(first.1, retry.1);
    assert_eq!(
        store(&conn)
            .get_loyalty_account("cust-1")
            .unwrap()
            .unwrap()
            .account
            .points,
        300
    );
}

#[test]
fn redeem_points_rejects_customer_mismatch_and_over_discount() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_customer(&conn, "cust-2", "Bob");
    seed_sale_for_customer(&conn, "sale-earn", Some("cust-1"), 5000);
    seed_sale_for_customer(&conn, "sale-other", Some("cust-2"), 1000);
    seed_sale_for_customer(&conn, "sale-small", Some("cust-1"), 50);
    store(&conn)
        .earn_points("cust-1", "sale-earn", 5000)
        .unwrap();

    let mismatch = store(&conn)
        .redeem_points("cust-1", 100, "sale-other")
        .unwrap_err();
    assert!(matches!(
        mismatch,
        CoreError::Validation {
            field: "sale_id",
            ..
        }
    ));

    let over_discount = store(&conn)
        .redeem_points("cust-1", 100, "sale-small")
        .unwrap_err();
    assert!(matches!(
        over_discount,
        CoreError::Validation {
            field: "points",
            ..
        }
    ));
}

#[test]
fn redeem_points_insufficient() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale_for_customer(&conn, "sale-1", Some("cust-1"), 1000);
    store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();
    let err = store(&conn)
        .redeem_points("cust-1", 100, "sale-1")
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "points"));
}

#[test]
fn list_tiers_returns_seeded() {
    let conn = fresh();
    let tiers = store(&conn).list_tiers().unwrap();
    assert_eq!(tiers.len(), 4);
    assert_eq!(tiers[0].name, "Bronze");
    assert_eq!(tiers[1].name, "Silver");
    assert_eq!(tiers[2].name, "Gold");
    assert_eq!(tiers[3].name, "Platinum");
}

#[test]
fn update_tier_modifies_fields() {
    let conn = fresh();
    let updated = store(&conn)
        .update_tier("tier-bronze", "Bronze Updated", 0, 15, 1.5, "#ff0000")
        .unwrap();
    assert_eq!(updated.name, "Bronze Updated");
    assert_eq!(updated.points_per_unit, 15);
    assert_eq!(updated.earn_multiplier, 1.5);
}

#[test]
fn get_points_value_converts_correctly() {
    let conn = fresh();
    assert_eq!(store(&conn).get_points_value(100).unwrap(), 100);
    assert_eq!(store(&conn).get_points_value(50).unwrap(), 50);
    assert_eq!(store(&conn).get_points_value(0).unwrap(), 0);
    assert!(matches!(
        store(&conn).get_points_value(-1),
        Err(CoreError::Validation {
            field: "points",
            ..
        })
    ));
}

#[test]
fn customer_not_found_returns_error() {
    let conn = fresh();
    let err = store(&conn)
        .get_or_create_loyalty_account("nonexistent")
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "customer"));
}

// ── Additional edge-case tests ─────────────────────────────────

#[test]
fn list_loyalty_accounts_ordered_by_points() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_customer(&conn, "cust-2", "Bob");
    seed_customer(&conn, "cust-3", "Charlie");
    seed_sale(&conn, "sale-1");
    seed_sale(&conn, "sale-2");

    let s = store(&conn);
    // Create accounts
    s.get_or_create_loyalty_account("cust-1").unwrap();
    s.get_or_create_loyalty_account("cust-2").unwrap();
    s.get_or_create_loyalty_account("cust-3").unwrap();

    // Earn different point amounts
    s.earn_points("cust-1", "sale-1", 5000).unwrap(); // 500 points
    s.earn_points("cust-2", "sale-2", 10000).unwrap(); // 1000 points

    let accounts = s.list_loyalty_accounts().unwrap();
    assert_eq!(accounts.len(), 3);
    // ORDER BY lifetime_points DESC: cust-2 (1000) first, cust-1 (500) second, cust-3 (0) third
    assert_eq!(accounts[0].account.customer_id, "cust-2");
    assert_eq!(accounts[1].account.customer_id, "cust-1");
    assert_eq!(accounts[2].account.customer_id, "cust-3");
}

#[test]
fn list_loyalty_accounts_empty() {
    let conn = fresh();
    // Seed a customer but don't create any loyalty accounts
    seed_customer(&conn, "cust-1", "Alice");
    let accounts = store(&conn).list_loyalty_accounts().unwrap();
    assert!(accounts.is_empty());
}

#[test]
fn earn_points_validation_zero_total() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();

    // total_minor = 0 → 0 points → Validation error
    let err = store(&conn).earn_points("cust-1", "sale-1", 0).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "total_minor"));
}

#[test]
fn earn_points_small_total_rounds_to_one() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();

    // total_minor = 9 → base = 90 → 90.0 / 100.0 * 1.0 = 0.9 → rounds to 1
    // With the fix for integer truncation, fractional cents are preserved
    let txn = store(&conn).earn_points("cust-1", "sale-1", 9).unwrap();
    assert_eq!(
        txn.points, 1,
        "9 cents with 10 pts/unit should round 0.9 → 1"
    );
}

#[test]
fn earn_points_tiny_total_rounds_to_zero() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();

    // total_minor = 4 → base = 40 → 40.0 / 100.0 * 1.0 = 0.4 → rounds to 0 → err
    let err = store(&conn).earn_points("cust-1", "sale-1", 4).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "total_minor"));
}

#[test]
fn earn_points_no_integer_truncation_for_sub_dollar_amounts() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();

    // total_minor = 155 ($1.55), points_per_unit = 10, earn_multiplier = 1.0
    // Correct math: 155 * 10 / 100 = 15.5 → round → 16
    // Integer-division bug: 155 * 10 / 100 = 15 (truncated) → 15
    let txn = store(&conn).earn_points("cust-1", "sale-1", 155).unwrap();
    assert_eq!(
        txn.points, 16,
        "$1.55 with 10 pts/unit should earn 16 pts (rounded from 15.5), \
         not {} from integer truncation",
        txn.points
    );

    // Verify account balance matches
    let details = store(&conn).get_loyalty_account("cust-1").unwrap().unwrap();
    assert_eq!(details.account.points, 16);
    assert_eq!(details.account.lifetime_points, 16);
}

#[test]
fn earn_points_multiple_sub_dollar_amounts_stack_correctly() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    seed_sale(&conn, "sale-2");
    seed_sale(&conn, "sale-3");
    let s = store(&conn);
    s.get_or_create_loyalty_account("cust-1").unwrap();

    // Three sub-dollar purchases that should round correctly
    // $1.55 → 16 pts, $2.49 → 25 pts, $0.99 → 10 pts
    s.earn_points("cust-1", "sale-1", 155).unwrap();
    s.earn_points("cust-1", "sale-2", 249).unwrap();
    s.earn_points("cust-1", "sale-3", 99).unwrap();

    // Expected: 16 + 25 + 10 = 51 (with correct float division)
    // Bug: 15 + 24 + 9 = 48 (with integer truncation before f64 cast)
    let details = s.get_loyalty_account("cust-1").unwrap().unwrap();
    assert_eq!(
        details.account.points, 51,
        "accumulated points from 155+249+99 should be 51, got {}",
        details.account.points
    );
    assert_eq!(
        details.account.lifetime_points, 51,
        "lifetime_points should also be 51, got {}",
        details.account.lifetime_points
    );
}

#[test]
fn redeem_points_zero_returns_validation_error() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale_for_customer(&conn, "sale-1", Some("cust-1"), 5000);
    seed_sale_for_customer(&conn, "sale-2", Some("cust-1"), 1000);
    store(&conn).earn_points("cust-1", "sale-1", 5000).unwrap();

    let err = store(&conn)
        .redeem_points("cust-1", 0, "sale-2")
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "points"));
}

#[test]
fn redeem_points_negative_returns_validation_error() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale_for_customer(&conn, "sale-1", Some("cust-1"), 5000);
    seed_sale_for_customer(&conn, "sale-2", Some("cust-1"), 1000);
    store(&conn).earn_points("cust-1", "sale-1", 5000).unwrap();

    let err = store(&conn)
        .redeem_points("cust-1", -50, "sale-2")
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "points"));
}

#[test]
fn redeem_points_no_account_returns_not_found() {
    let conn = fresh();
    seed_sale(&conn, "sale-1");
    let err = store(&conn)
        .redeem_points("nonexistent", 100, "sale-1")
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "loyalty_account"));
}

#[test]
fn update_tier_not_found() {
    let conn = fresh();
    let err = store(&conn)
        .update_tier("nonexistent", "No Tier", 0, 10, 1.0, "#000")
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "loyalty_tier"));
}

#[test]
fn earn_points_updates_tier_automatically() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    seed_sale(&conn, "sale-2");

    let s = store(&conn);
    s.get_or_create_loyalty_account("cust-1").unwrap();

    // Earn enough points to reach Silver tier (min_points = 200 for Silver)
    // 2000 * 10 / 100 * 1.0 = 200 points → should auto-upgrade to Silver
    s.earn_points("cust-1", "sale-1", 2000).unwrap();

    let details = s.get_loyalty_account("cust-1").unwrap().unwrap();
    assert_eq!(details.account.tier_id.as_deref(), Some("tier-silver"));
    assert!(details.tier.is_some());
    assert_eq!(details.tier.as_ref().unwrap().name, "Silver");
}

#[test]
fn earn_points_multiple_stacked() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    seed_sale(&conn, "sale-2");
    seed_sale(&conn, "sale-3");

    let s = store(&conn);
    s.get_or_create_loyalty_account("cust-1").unwrap();

    // sale-1: Bronze 1.0x → 1000*10/100=100 points. Upgrades to Silver.
    // sale-2: Silver 1.25x → 2000*10/100*1.25=250 points. Total: 350.
    // sale-3: Silver 1.25x → 3000*10/100*1.25=375 points. Total: 725.
    s.earn_points("cust-1", "sale-1", 1000).unwrap();
    s.earn_points("cust-1", "sale-2", 2000).unwrap();
    s.earn_points("cust-1", "sale-3", 3000).unwrap();

    let details = s.get_loyalty_account("cust-1").unwrap().unwrap();
    assert_eq!(details.account.points, 725);
    assert_eq!(details.account.lifetime_points, 725);
    // Should have 3 recent transactions
    assert_eq!(details.recent_transactions.len(), 3);
}

// ── LOY-02 (migration 128): unique earn/redeem projection index ──

/// The audit's LOY-02 recommendation: enforce idempotency at the
/// database layer with a unique projection key (account_id, sale_id,
/// txn_type) so a CONCURRENT replay — both callers passing the
/// application-layer lookup, then both inserting — can never
/// double-award. `earn_points`/`redeem_points` recover from the
/// resulting ConstraintViolation by returning the winning row, but that
/// recovery depends on the index existing. Pin: a second row for the
/// same projection must be rejected by the database.
#[test]
fn duplicate_earn_projection_row_is_rejected_by_database() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale(&conn, "sale-1");
    let account = store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();

    // The winning concurrent replay row is already in the ledger.
    conn.execute(
        "INSERT INTO loyalty_transactions (id, account_id, sale_id, points, txn_type, description, created_at)
         VALUES ('txn-win', ?1, 'sale-1', 100, 'earn', 'winning row', '2025-01-01T00:00:00.000Z')",
        params![account.id],
    )
    .unwrap();

    // A losing concurrent replay must hit the unique projection index
    // instead of double-awarding the same sale.
    let duplicate = conn.execute(
        "INSERT INTO loyalty_transactions (id, account_id, sale_id, points, txn_type, description, created_at)
         VALUES ('txn-lose', ?1, 'sale-1', 100, 'earn', 'losing row', '2025-01-01T00:00:00.000Z')",
        params![account.id],
    );
    assert!(
        matches!(
            &duplicate,
            Err(rusqlite::Error::SqliteFailure(code, _))
                if code.code == rusqlite::ErrorCode::ConstraintViolation
        ),
        "duplicate (account_id, sale_id, 'earn') row must be rejected by the unique \
         projection index, got: {duplicate:?}"
    );
}

/// The same projection must also hold for redemptions: one 'redeem'
/// row per (account, sale). A retried checkout must not double-deduct.
#[test]
fn duplicate_redeem_projection_row_is_rejected_by_database() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale_for_customer(&conn, "sale-1", Some("cust-1"), 5000);
    let account = store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();
    conn.execute(
        "INSERT INTO loyalty_transactions (id, account_id, sale_id, points, txn_type, description, created_at)
         VALUES ('txn-win', ?1, 'sale-1', -100, 'redeem', 'winning row', '2025-01-01T00:00:00.000Z')",
        params![account.id],
    )
    .unwrap();

    let duplicate = conn.execute(
        "INSERT INTO loyalty_transactions (id, account_id, sale_id, points, txn_type, description, created_at)
         VALUES ('txn-lose', ?1, 'sale-1', -100, 'redeem', 'losing row', '2025-01-01T00:00:00.000Z')",
        params![account.id],
    );
    assert!(
        matches!(
            &duplicate,
            Err(rusqlite::Error::SqliteFailure(code, _))
                if code.code == rusqlite::ErrorCode::ConstraintViolation
        ),
        "duplicate (account_id, sale_id, 'redeem') row must be rejected, got: {duplicate:?}"
    );
}

/// A sale may legitimately carry BOTH an earn and a redeem row — the
/// projection key is (account, sale, TYPE), so the index must not
/// collapse different transaction types for the same sale.
#[test]
fn earn_and_redeem_for_same_sale_are_distinct_projections() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    seed_sale_for_customer(&conn, "sale-1", Some("cust-1"), 5000);
    let account = store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();

    conn.execute(
        "INSERT INTO loyalty_transactions (id, account_id, sale_id, points, txn_type, description, created_at)
         VALUES ('txn-earn', ?1, 'sale-1', 500, 'earn', 'earned', '2025-01-01T00:00:00.000Z')",
        params![account.id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO loyalty_transactions (id, account_id, sale_id, points, txn_type, description, created_at)
         VALUES ('txn-redeem', ?1, 'sale-1', -100, 'redeem', 'redeemed', '2025-01-02T00:00:00.000Z')",
        params![account.id],
    )
    .unwrap();
}

/// adjust/expire rows have no sale binding (NULL sale_id). SQLite unique
/// indexes treat NULLs as distinct, so multiple sale-less rows for the
/// same account must stay insertable — the projection index must not
/// over-constrain them.
#[test]
fn sale_less_transaction_rows_are_not_projection_constrained() {
    let conn = fresh();
    seed_customer(&conn, "cust-1", "Alice");
    let account = store(&conn)
        .get_or_create_loyalty_account("cust-1")
        .unwrap();

    for i in 0..2 {
        let id = format!("txn-adjust-{i}");
        conn.execute(
            "INSERT INTO loyalty_transactions (id, account_id, sale_id, points, txn_type, description, created_at)
             VALUES (?1, ?2, NULL, -10, 'adjust', 'manual adjustment', '2025-01-01T00:00:00.000Z')",
            params![id, account.id],
        )
        .unwrap_or_else(|e| panic!("sale-less adjust row {i} must insert: {e}"));
    }
}
