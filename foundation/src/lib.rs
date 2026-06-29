//! Foundation crate for OZ-POS.
//!
//! Contains the value objects, contracts, enums, and error types that
//! are shared across all other crates. This crate has minimal
//! dependencies so it can be used everywhere without pulling in heavy
//! transitive deps.
//!
//! # Contents
//!
//! - [`money`] — [`Money`] and [`Currency`] primitives
//! - [`sku`] — [`Sku`] and [`LineId`] identifiers
//! - [`cart`] — [`Cart`], [`CartLine`], [`CartId`], [`CartError`]
//! - [`enums`] — shared enums ([`SaleStatus`], [`PaymentMethod`])
//! - [`contracts`] — [`Module`], [`Service`], [`EventHandler`] traits
//! - [`errors`] — shared error types

pub mod cart;
pub mod contracts;
pub mod enums;
pub mod errors;
pub mod money;
pub mod sku;

pub use cart::{Cart, CartError, CartId, CartLine};
pub use enums::{InvalidTransition, PaymentMethod, SaleStatus};
pub use errors::{ConflictError, NotFoundError, ValidationError};
pub use money::{Currency, InvalidCurrencyCode, Money};
pub use sku::{LineId, Sku};
