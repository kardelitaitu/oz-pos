-- ADR sync-plan-gating (follow-up): Stripe customer → tenant mapping.
--
-- When a subscription is created we learn the Stripe customer id
-- (`cus_xxx`) and which OZ-POS tenant it belongs to (from the
-- `tenant_id` metadata set on the Checkout Session). Later Stripe events
-- (renewal `invoice.paid`, `customer.subscription.updated`, …) carry the
-- customer id but NOT the metadata, so this table lets the webhook
-- resolve the tenant and keep its sync plan in sync with billing.
CREATE TABLE IF NOT EXISTS stripe_customers (
    stripe_customer_id TEXT PRIMARY KEY,
    tenant_id          TEXT NOT NULL,
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
