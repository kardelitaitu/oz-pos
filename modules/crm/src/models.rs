//! CRM domain models — Customer profile and contact tracking.

use foundation::{Email, Phone};
use serde::{Deserialize, Serialize};

/// A repeat buyer tracked by the POS.
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
    /// Create a new customer.
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
            currency: "USD".to_string(),
            notes: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    /// Builder method for setting email.
    #[must_use]
    pub fn with_email(mut self, email: Email) -> Self {
        self.email = Some(email);
        self
    }

    /// Builder method for setting phone.
    #[must_use]
    pub fn with_phone(mut self, phone: Phone) -> Self {
        self.phone = Some(phone);
        self
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use foundation::contact::{Email, Phone};

    // ── Customer ────────────────────────────────────────────────────

    #[test]
    fn customer_new_sets_defaults() {
        let c = Customer::new("Alice");
        assert_eq!(c.name, "Alice");
        assert!(c.email.is_none());
        assert!(c.phone.is_none());
        assert_eq!(c.loyalty_points, 0);
        assert_eq!(c.total_spent_minor, 0);
        assert_eq!(c.currency, "USD");
        assert!(c.notes.is_empty());
    }

    #[test]
    fn customer_new_trims_name() {
        let c = Customer::new("  Bob  ");
        assert_eq!(c.name, "Bob");
    }

    #[test]
    #[should_panic(expected = "customer name must not be empty")]
    fn customer_new_rejects_empty_name() {
        Customer::new("  ");
    }

    #[test]
    fn customer_new_generates_unique_id() {
        let a = Customer::new("A");
        let b = Customer::new("B");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn customer_with_email() {
        let c = Customer::new("Alice")
            .with_email(Email::new("alice@example.com").unwrap());
        assert_eq!(c.email.as_ref().unwrap().as_str(), "alice@example.com");
    }

    #[test]
    fn customer_with_phone() {
        let c = Customer::new("Bob")
            .with_phone(Phone::new("+1-555-0102").unwrap());
        assert_eq!(c.phone.as_ref().unwrap().as_str(), "+1-555-0102");
    }

    #[test]
    fn customer_builder_chain() {
        let c = Customer::new("Carol")
            .with_email(Email::new("carol@example.com").unwrap())
            .with_phone(Phone::new("+6281234567890").unwrap());
        assert!(c.email.is_some());
        assert!(c.phone.is_some());
    }

    #[test]
    fn customer_serde_roundtrip() {
        let c = Customer::new("Dave")
            .with_email(Email::new("dave@example.com").unwrap())
            .with_phone(Phone::new("+1-555-0199").unwrap());
        let json = serde_json::to_string(&c).unwrap();
        let back: Customer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Dave");
        assert_eq!(back.email.as_ref().unwrap().as_str(), "dave@example.com");
        assert_eq!(back.phone.as_ref().unwrap().as_str(), "+1-555-0199");
    }

    #[test]
    fn customer_serde_none_fields() {
        let c = Customer::new("Eve");
        let json = serde_json::to_string(&c).unwrap();
        let back: Customer = serde_json::from_str(&json).unwrap();
        assert!(back.email.is_none());
        assert!(back.phone.is_none());
    }
}
