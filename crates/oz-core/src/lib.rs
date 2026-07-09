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

pub mod audit;
pub mod auth;
pub mod cache;
pub mod cart;
pub mod cash_payout;
pub mod category;
pub mod customer;
pub mod db;
pub mod error;
pub mod events;
pub mod exchange_rate;
pub mod features;
/// Gift cards — issue, redeem, top-up, freeze, balance checks.
pub mod gift_card;
pub mod inventory;
pub mod kds;
/// Loyalty program — points, tiers, and redemption.
pub mod loyalty;
pub mod migrations;
pub mod money;
pub mod offline;
pub mod ozpkg;
pub mod payment;
pub mod product;
/// Product bundles — sell multiple SKUs as one item.
pub mod product_bundle;
pub mod product_variant;
pub mod promotion;
pub mod purchase_order;
pub mod recipe;
pub mod refund;
pub mod sale;
pub mod settings;
pub mod shift;
pub mod sku;
pub mod stock_count;
pub mod stock_transfer;
pub mod store_profile;
pub mod supplier;
pub mod sync_client;
/// Restaurant table management — floor plan positions and statuses.
pub mod table;
pub mod tax_rate;
pub mod terminal;
pub mod terminal_override;
pub mod terminal_profile;
pub mod user;
/// Per-user display preferences (card size, font size, etc.).
pub mod user_preferences;

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
