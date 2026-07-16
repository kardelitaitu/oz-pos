-- Migrate store-specific default_currency to global currency.default.
--
-- The old `store.default_currency` key was store-scoped; the new
-- `currency.default` key is a global application-level setting.
-- This migration copies the value if it exists and removes the old key.

INSERT OR IGNORE INTO settings (key, value, updated_at)
    SELECT 'currency.default', value, updated_at
    FROM settings
    WHERE key = 'store.default_currency';

DELETE FROM settings WHERE key = 'store.default_currency';
