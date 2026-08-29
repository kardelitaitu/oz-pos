# Promotions Module

**Status:** Stub (lifecycle only — no domain logic yet)

## Overview

The Promotions module will own discount rules and their evaluation: percentage
and fixed-amount discounts, buy-X-get-Y, bundle pricing, time-windowed
campaigns, and coupon codes. It answers one question for the cart: given these
lines, which discounts apply and what do they subtract?

## Module Info

| Field        | Value |
|--------------|-------|
| ID           | `promotions` |
| Crate        | `modules-promotions` |
| Version      | `0.1.0` |
| Dependencies | `["sales"]` — discounts are evaluated against a cart |
| Permissions  | `promotions:view`, `promotions:apply`, `promotions:manage` |
| Feature flag | `promotions-engine` (`crates/oz-core/src/features.rs`) |

## Currently Owns

Nothing. `PromotionsModule` registers with the kernel, declares its dependency
on `sales`, and logs its lifecycle transitions.

## Design notes for the promotion

- Discount amounts are `Money` (`i64` minor units). Percentage rules must
  round once, at the line level, and the rounding direction must be explicit —
  never accumulate fractional minor units.
- Rule evaluation should be pure: `(cart, active_rules) -> Vec<DiscountLine>`.
  Keeping it free of database access makes it cheap to property-test against
  the "total never goes negative" invariant.
- Stacking order matters and must be deterministic. Decide it once, encode it
  in the rule type, and test it.

## Promotion Checklist

- [ ] `models.rs` — `PromotionRule`, `RuleKind`, `DiscountLine`, `Coupon`
- [ ] `repository.rs` — rule storage (namespace: `promotions_*`)
- [ ] `service.rs` — pure evaluation + coupon redemption in a transaction
- [ ] Property tests: total never negative, stacking is order-stable
- [ ] Gate the UI on the `promotions-engine` feature flag

See `modules/README.md` for the full promotion path.
