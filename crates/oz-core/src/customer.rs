//! Customer domain type — customer relationship management.
//!
//! A [`Customer`] stores contact information and loyalty data for
//! repeat buyers. Customers are stored in the `customers` table
//! (migration `007_customers.sql`).

use serde::{Deserialize, Serialize};

use foundation::{Email, Phone};

/// A repeat buyer tracked by the POS.
///
/// # Schema mapping
///
/// Maps 1:1 to the `customers` table. Monetary fields use
/// integer minor units for consistency with [`crate::Money`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Customer {
    /// Internal row id (UUID v4).
    pub id: String,

    /// Display name.
    pub name: String,

    /// Optional email address.
    pub email: Option<Email>,

    /// Optional phone number.
    pub phone: Option<Phone>,

    /// Accumulated loyalty points.
    pub loyalty_points: i64,

    /// Total lifetime spend in minor units.
    pub total_spent_minor: i64,

    /// Currency code for `total_spent_minor`.
    pub currency: String,

    /// Free-form notes about this customer.
    pub notes: String,

    /// ISO-8601 creation timestamp.
    pub created_at: String,

    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl Customer {
    /// Create a new customer with the given name.
    ///
    /// Generates a fresh UUID for `id`. Optional fields default to
    /// `None` or empty/zero values.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty after trimming.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into().trim().to_owned();
        assert!(!name.is_empty(), "customer name must not be empty");

        Self {
            id: uuid::Uuid::now_v7().to_string(),
            name,
            email: None,
            phone: None,
            loyalty_points: 0,
            total_spent_minor: 0,
            currency: "USD".into(),
            notes: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    /// Set the email address (builder-style).
    #[must_use]
    pub fn with_email(mut self, email: Email) -> Self {
        self.email = Some(email);
        self
    }

    /// Set the phone number (builder-style).
    #[must_use]
    pub fn with_phone(mut self, phone: Phone) -> Self {
        self.phone = Some(phone);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::{Email, Phone};

    #[test]
    fn new_customer() {
        let c = Customer::new("Alice");
        assert_eq!(c.name, "Alice");
        assert!(c.email.is_none());
        assert!(c.phone.is_none());
        assert_eq!(c.loyalty_points, 0);
        assert_eq!(c.total_spent_minor, 0);
        assert_eq!(c.currency, "USD");
        assert!(c.created_at.is_empty());
    }

    #[test]
    #[should_panic(expected = "customer name must not be empty")]
    fn new_panics_on_empty_name() {
        Customer::new("   ");
    }

    #[test]
    fn builder_sets_email() {
        let c = Customer::new("Bob").with_email(Email::new("bob@example.com").unwrap());
        assert_eq!(c.email, Some(Email::new("bob@example.com").unwrap()));
    }

    #[test]
    fn builder_sets_phone() {
        let c = Customer::new("Bob").with_phone(Phone::new("+1-555-0100").unwrap());
        assert_eq!(c.phone, Some(Phone::new("+1-555-0100").unwrap()));
    }

    #[test]
    fn builder_chains() {
        let c = Customer::new("Bob")
            .with_email(Email::new("bob@example.com").unwrap())
            .with_phone(Phone::new("+1-555-0100").unwrap());
        assert_eq!(c.email, Some(Email::new("bob@example.com").unwrap()));
        assert_eq!(c.phone, Some(Phone::new("+1-555-0100").unwrap()));
    }

    #[test]
    fn serde_roundtrip() {
        let c = Customer::new("Alice").with_email(Email::new("alice@example.com").unwrap());
        let json = serde_json::to_string(&c).unwrap();
        let back: Customer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn serde_with_phone() {
        let c = Customer::new("Carol").with_phone(Phone::new("+6281234567890").unwrap());
        let json = serde_json::to_string(&c).unwrap();
        let back: Customer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
        assert_eq!(back.phone, Some(Phone::new("+6281234567890").unwrap()));
    }

    #[test]
    fn debug_output() {
        let c = Customer::new("Dave");
        let debug = format!("{c:?}");
        assert!(debug.contains("Dave"));
        assert!(debug.contains("USD"));
    }

    #[test]
    fn serde_deserialize_minimal() {
        let json = r#"{"id":"c1","name":"Minimal","email":null,"phone":null,"loyalty_points":0,"total_spent_minor":0,"currency":"IDR","notes":"","created_at":"","updated_at":""}"#;
        let c: Customer = serde_json::from_str(json).unwrap();
        assert_eq!(c.name, "Minimal");
        assert_eq!(c.currency, "IDR");
        assert!(c.email.is_none());
        assert!(c.phone.is_none());
    }
}
