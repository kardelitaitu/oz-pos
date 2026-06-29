//! Payment domain type — individual payment tenders within a sale.
//!
//! A [`Payment`] represents a single tender against a sale. Most sales
//! have one payment (e.g. "cash" for the full amount), but split
//! payments produce multiple payment records (e.g. $10 cash + $5 card).
//!
//! # Schema mapping
//!
//! Maps to the `payments` table (migration `022_payments_table.sql`).

use serde::{Deserialize, Serialize};

use crate::money::Money;

/// A single payment tender against a sale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payment {
    /// Internal row id (UUID v4).
    pub id: String,

    /// FK to `sales.id`.
    pub sale_id: String,

    /// Payment method ("cash", "card", "other", etc.).
    pub method: String,

    /// Amount tendered in minor units.
    pub amount: Money,

    /// ISO-8601 timestamp of when this payment was recorded.
    pub created_at: String,
}

/// Argument used to describe a payment split when completing a sale.
///
/// This is the serialisation boundary type used in IPC commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentSplitArg {
    /// Payment method ("cash", "card", "other", etc.).
    pub method: String,

    /// Amount in minor units.
    pub amount_minor: i64,
}
