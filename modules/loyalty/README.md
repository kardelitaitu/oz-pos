<!-- Audit stamp: 2026-07-24 · Hermes-Agent · status: ACCURATE · Loyalty Module documentation -->

# Loyalty Module

**Status:** Active (ADR #30 Modularized)

## Overview

The Loyalty module manages customer loyalty accounts, points accrual, tier progression, gift card issuing/redemption, and reward redemptions across the POS application.

## Module Info

| Field        | Value           |
|--------------|-----------------|
| ID           | `loyalty`       |
| Version      | `0.0.19`        |
| Dependencies | `[crm]`         |
| Permissions  | `loyalty:read`, `loyalty:write`, `giftcards:issue` |

## Code Structure (ADR #30)

- **Domain Models** — `LoyaltyTier`, `LoyaltyAccount`, `LoyaltyTransaction`, `GiftCard`, `GiftCardTransaction` (`modules/loyalty/src/models.rs`)
- **Database Persistence** — `LoyaltyRepository` operating on `&Connection`/`&Transaction` (`modules/loyalty/src/repository.rs`)
- **Business Logic** — `LoyaltyService` (`modules/loyalty/src/service.rs`)
- **Frontend** — Loyalty screens (`ui/src/features/loyalty/`, `ui/src/features/gift-cards/`)
- **UI Registration** — Self-registering via `registerLoyaltyFeature()` and `registerGiftCardsFeature()` (`ui/src/features/loyalty/register.tsx`, `ui/src/features/gift-cards/register.tsx`)

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration and verifies database connection
2. **`on_start`** — Prepares loyalty service and event listeners
3. **`on_stop`** — Flushes state and releases resources
