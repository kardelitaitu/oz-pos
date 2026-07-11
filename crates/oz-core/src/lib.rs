//! Domain types for OZ-POS.
//!
//! `oz-core` is the foundation crate of the framework. It contains the
//! types every other crate builds on: [`Money`] and [`Currency`] for
//! pricing, [`Cart`] and [`CartLine`] for the sale pipeline, [`Sku`] and
//! [`LineId`] identifiers, the SQL [`migrations`] runner, and the
//! domain-level error type [`CoreError`].
//!
//! Rules enforced here (see `AGENTS.md` for the full policy):
//!
//! - **Money is always `i64` minor units.** Never `f32`/`f64`.
//! - **All public items have `///` docs.** Library consumers depend on them.
//! - **`#![deny(unsafe_code)]`** is on; open a discussion before adding `unsafe`.
//! - **`#![warn(missing_docs)]`** is on; new public items must be documented.

#![deny(unsafe_code)]
#![warn(missing_docs)]

/// Immutable audit log — cash management and data-modification events.
pub mod audit;
/// Authentication and session management.
pub mod auth;
/// In-memory and Redis-backed caching.
pub mod cache;
/// Open cart and checkout session.
pub mod cart;
/// Cash-in / cash-out drawer transactions.
pub mod cash_payout;
/// Product category tree.
pub mod category;
/// Customer profiles and contact data.
pub mod customer;
/// SQLite data access layer — one module per domain aggregate.
pub mod db;
/// Domain error types.
pub mod error;
/// Domain event types for cross-crate communication.
pub mod events;
/// Currency exchange-rate store and conversion.
pub mod exchange_rate;
/// Feature-gate registry and runtime guards.
pub mod features;
/// Gift cards — issue, redeem, top-up, freeze, balance checks.
pub mod gift_card;
/// Stock-on-hand queries and reservation.
pub mod inventory;
/// Kitchen Display System order pipeline.
pub mod kds;
/// License server client — verify, activate, renew subscriptions (ADR #9).
pub mod license_verification;
/// Loyalty program — points, tiers, and redemption.
pub mod loyalty;
/// SQL migration definitions embedded at compile time.
pub mod migrations;
/// Money and currency primitives (re-exported from `foundation`).
pub mod money;
/// Offline queue — queued mutations for sync when connectivity returns.
pub mod offline;
/// OZ-POS package metadata reader (`.ozpkg` bundles).
pub mod ozpkg;
/// Payment processing and split-tender allocation.
pub mod payment;
/// Product catalog — SKU, price, type, metadata.
pub mod product;
/// Product bundles — sell multiple SKUs as one item.
pub mod product_bundle;
/// Product variants — sizes, colours, options.
pub mod product_variant;
/// Discount and promotion rules.
pub mod promotion;
/// Purchase orders — order stock from suppliers.
pub mod purchase_order;
/// Product recipes — bill-of-materials for make-from-scratch items.
pub mod recipe;
/// Refund and return processing.
pub mod refund;
/// Completed sale records and sale-line items.
pub mod sale;
/// Active user session state.
pub mod session;
/// Persistent key-value settings store.
pub mod settings;
/// Cashier shift open/close and float management.
pub mod shift;
/// Stock-keeping unit identifier.
pub mod sku;
/// Stock-count and inventory adjustment.
pub mod stock_count;
/// Inter-store stock transfers.
pub mod stock_transfer;
/// Store/branch profile settings.
pub mod store_profile;
/// Tenant subscription and license state.
pub mod subscription;
/// Supplier directory.
pub mod supplier;
/// LAN peer discovery for offline sync.
pub mod sync;
/// Pending-sync push/pull client.
pub mod sync_client;
/// Restaurant table management — floor plan positions and statuses.
pub mod table;
/// Tax-rate catalog.
pub mod tax_rate;
/// Registered terminal device.
pub mod terminal;
/// Per-terminal feature overrides per store.
pub mod terminal_override;
/// Terminal profile configuration.
pub mod terminal_profile;
/// Staff user accounts and role-based access control.
pub mod user;
/// Per-user display preferences (card size, font size, etc.).
pub mod user_preferences;

/// Generate a new time-ordered UUIDv7 primary key.
///
/// UUIDv7 embeds a millisecond-precision timestamp in the high bits,
/// providing better B-tree index locality in SQLite and preventing
/// ID collisions when multiple offline registers generate IDs
/// independently (ADR #6).
///
/// Use this helper for all new entity IDs. Avoid `Uuid::new_v4()`
/// in production code.
#[must_use]
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Default optimistic concurrency version (ADR #6).
///
/// Used as the `#[serde(default)]` value for [`Product::version`]
/// and [`Sale::version`] so that deserialization from pre-migration
/// payloads succeeds.
#[doc(hidden)]
pub fn default_version() -> i64 {
    1
}

pub use audit::AuditEntry;
#[cfg(feature = "cache-redis")]
pub use cache::redis_cache::RedisCache;
pub use cache::{Cache, NoopCache, create_cache};
pub use cart::{Cart, CartError, CartId, CartLine};
pub use cash_payout::CashPayout;
pub use category::Category;
pub use customer::Customer;
pub use db::reports::{
    CategoryBreakdownRow, DailyRevenueRow, HourlyHeatmapRow, LowStockAlert, MonthlyRevenueRow,
    TopProductRow, WeeklyRevenueRow,
};
pub use db::{ProductWithDetails, Store};
pub use error::{CoreError, CoreErrorKind};
pub use features::{
    Feature, FeatureGuard, FeatureGuardRegistry, FeatureRegistry, KdsFeatureGuard,
    ShiftFeatureGuard,
};
pub use foundation;
pub use foundation::{InvalidTransition, SaleStatus};
pub use gift_card::{
    GiftCard, GiftCardFilter, GiftCardTransaction, GiftCardWithTransactions, IssueGiftCardInput,
    RedeemGiftCardResult,
};
pub use inventory::Inventory;
pub use kds::{CreateKdsOrderInput, KdsOrder, KdsStatus};
pub use loyalty::{LoyaltyAccount, LoyaltyAccountWithDetails, LoyaltyTier, LoyaltyTransaction};
pub use money::{Currency, Money};
pub use offline::{OfflineQueueItem, OfflineQueueStatus};
pub use payment::{Payment, PaymentSplitArg};
pub use platform_core::rbac::{AuthorizationError, has_permission, permissions};
pub use product::{Product, ProductType};
pub use product_bundle::{BundleItem, BundleWithItems, ProductBundle};
pub use product_variant::ProductVariant;
pub use promotion::{Promotion, PromotionApplication, PromotionType};
pub use purchase_order::{PurchaseOrder, PurchaseOrderLine, PurchaseOrderWithLines};
pub use recipe::RecipeItem;
pub use refund::{Refund, RefundLine};
pub use sale::{Sale, SaleLine};
pub use settings::Settings;
pub use shift::Shift;
pub use sku::{LineId, Sku};
pub use stock_count::{CountType, StockAdjustment, StockCount, StockCountLine, StockCountStatus};
pub use stock_transfer::{StockTransfer, StockTransferLine};
pub use store_profile::StoreProfile;
pub use subscription::{InstanceStatus, SubscriptionTier, TenantSubscription};
pub use supplier::Supplier;
pub use sync_client::{
    PullResult, SyncAttemptResult, SyncConfig, pull_snapshot, sync_pending, sync_pending_async,
};
pub use table::{Table, TableStatus};
pub use terminal::Terminal;
pub use terminal_override::TerminalFeatureOverride;
pub use terminal_profile::TerminalProfile;
pub use user::{Role, User, builtin_roles, seed_users};
pub use user_preferences::UserPreferences;
