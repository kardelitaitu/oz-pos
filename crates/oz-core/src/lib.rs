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
pub mod cart;
pub mod cash_payout;
pub mod category;
pub mod customer;
pub mod db;
pub mod error;
pub mod events;
pub mod exchange_rate;
pub mod features;
pub mod inventory;
pub mod migrations;
pub mod money;
pub mod offline;
pub mod ozpkg;
pub mod payment;
pub mod product;
pub mod product_variant;
pub mod refund;
pub mod sale;
pub mod settings;
pub mod shift;
pub mod sku;
pub mod store_profile;
pub mod sync_client;
pub mod tax_rate;
pub mod terminal;
pub mod user;

pub use audit::AuditEntry;
pub use cart::{Cart, CartError, CartId, CartLine};
pub use cash_payout::CashPayout;
pub use category::Category;
pub use customer::Customer;
pub use db::{ProductWithDetails, Store};
pub use error::CoreError;
pub use features::{Feature, FeatureRegistry};
pub use foundation;
pub use foundation::{InvalidTransition, SaleStatus};
pub use inventory::Inventory;
pub use money::{Currency, Money};
pub use offline::{OfflineQueueItem, OfflineQueueStatus};
pub use payment::{Payment, PaymentSplitArg};
pub use product::Product;
pub use product_variant::ProductVariant;
pub use refund::{Refund, RefundLine};
pub use sale::{Sale, SaleLine};
pub use settings::Settings;
pub use shift::Shift;
pub use sku::{LineId, Sku};
pub use store_profile::StoreProfile;
pub use sync_client::{SyncAttemptResult, SyncConfig, sync_pending, sync_pending_async};
pub use terminal::Terminal;
pub use user::{Role, User, builtin_roles, seed_users};
