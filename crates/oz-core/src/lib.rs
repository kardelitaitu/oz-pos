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

pub mod cart;
pub mod category;
pub mod error;
pub mod migrations;
pub mod money;
pub mod product;
pub mod sku;

pub use cart::{Cart, CartError, CartId, CartLine};
pub use category::Category;
pub use error::CoreError;
pub use money::{Currency, Money};
pub use product::Product;
pub use sku::{LineId, Sku};
