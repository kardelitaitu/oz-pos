-- 107_loyalty_integrity.sql
-- Loyalty integrity hardening:
--   * one earn projection per account/sale
--   * one redemption projection per account/sale
--   * reject invalid tier configuration at the database boundary
--
-- Existing databases may contain duplicate projections from pre-107 event
-- replays. Collapse those duplicates before creating the unique indexes, then
-- rebuild balances from the surviving ledger so this migration is safe to
-- apply to upgraded stores as well as fresh installs.

DELETE FROM loyalty_transactions
WHERE id IN (
    SELECT id
    FROM loyalty_transactions
    WHERE sale_id IS NOT NULL
      AND txn_type IN ('earn', 'redeem')
      AND id NOT IN (
          SELECT MIN(id)
          FROM loyalty_transactions
          WHERE sale_id IS NOT NULL
            AND txn_type IN ('earn', 'redeem')
          GROUP BY account_id, sale_id, txn_type
      )
);

UPDATE loyalty_accounts
SET points = COALESCE((
        SELECT SUM(points)
        FROM loyalty_transactions
        WHERE account_id = loyalty_accounts.id
    ), 0),
    lifetime_points = COALESCE((
        SELECT SUM(points)
        FROM loyalty_transactions
        WHERE account_id = loyalty_accounts.id
          AND txn_type = 'earn'
    ), 0),
    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now');

CREATE UNIQUE INDEX IF NOT EXISTS uq_loyalty_earn_sale
    ON loyalty_transactions(account_id, sale_id)
    WHERE sale_id IS NOT NULL AND txn_type = 'earn';

CREATE UNIQUE INDEX IF NOT EXISTS uq_loyalty_redeem_sale
    ON loyalty_transactions(account_id, sale_id)
    WHERE sale_id IS NOT NULL AND txn_type = 'redeem';

CREATE TRIGGER IF NOT EXISTS loyalty_tiers_validate_insert
BEFORE INSERT ON loyalty_tiers
WHEN trim(NEW.name) = ''
  OR NEW.min_points < 0
  OR NEW.points_per_unit <= 0
  OR NEW.earn_multiplier <= 0
  OR length(NEW.colour) <> 7
  OR substr(NEW.colour, 1, 1) <> '#'
  OR substr(NEW.colour, 2) GLOB '*[^0-9A-Fa-f]*'
BEGIN
    SELECT RAISE(ABORT, 'invalid loyalty tier configuration');
END;

CREATE TRIGGER IF NOT EXISTS loyalty_tiers_validate_update
BEFORE UPDATE OF name, min_points, points_per_unit, earn_multiplier, colour
ON loyalty_tiers
WHEN trim(NEW.name) = ''
  OR NEW.min_points < 0
  OR NEW.points_per_unit <= 0
  OR NEW.earn_multiplier <= 0
  OR length(NEW.colour) <> 7
  OR substr(NEW.colour, 1, 1) <> '#'
  OR substr(NEW.colour, 2) GLOB '*[^0-9A-Fa-f]*'
BEGIN
    SELECT RAISE(ABORT, 'invalid loyalty tier configuration');
END;
