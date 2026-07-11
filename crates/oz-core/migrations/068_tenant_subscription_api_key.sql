-- ADR #9 Phase 2: License Server Client Integration
--
-- Adds signed_payload and api_key columns to tenant_subscription.
-- signed_payload: The JSON payload from the license server (stored for re-verification).
-- api_key: Used for subsequent renew and status API calls.

ALTER TABLE tenant_subscription ADD COLUMN signed_payload TEXT NOT NULL DEFAULT '';
ALTER TABLE tenant_subscription ADD COLUMN api_key TEXT NOT NULL DEFAULT '';

-- Update the bootstrap row.
UPDATE tenant_subscription SET signed_payload = '', api_key = '' WHERE tenant_id = 'default';
