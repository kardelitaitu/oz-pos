//! Loyalty & Gift Card Service — business logic workflows.

use crate::models::{GiftCard, LoyaltyAccount};
use crate::repository::LoyaltyRepository;
use rusqlite::Connection;

/// Service encapsulating loyalty program and gift card business logic.
pub struct LoyaltyService;

impl LoyaltyService {
    /// Retrieve loyalty account for customer.
    pub fn get_account_by_customer(
        conn: &Connection,
        customer_id: &str,
    ) -> Result<Option<LoyaltyAccount>, anyhow::Error> {
        let repo = LoyaltyRepository::new(conn);
        repo.get_account_by_customer(customer_id)
    }

    /// Retrieve gift card by card number.
    pub fn get_gift_card(
        conn: &Connection,
        card_number: &str,
    ) -> Result<Option<GiftCard>, anyhow::Error> {
        let repo = LoyaltyRepository::new(conn);
        repo.get_gift_card_by_number(card_number)
    }
}
