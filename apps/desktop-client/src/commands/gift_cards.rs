//! Gift card management Tauri commands.
//!
//! Provides CRUD operations for gift cards including:
//! - Issue new gift cards with an initial balance
//! - Look up cards by number or ID
//! - List cards with optional filtering
//! - Get current balance
//! - Redeem (spend) card balance at POS
//! - Top up (add value) to existing cards
//! - Freeze/unfreeze cards (e.g., for fraud prevention)

use serde::Serialize;
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::gift_card::{
    GiftCard, GiftCardFilter, GiftCardWithTransactions, IssueGiftCardInput, RedeemGiftCardResult,
};

use crate::error::AppError;
use crate::state::AppState;

/// Result of a balance inquiry for a gift card.
///
/// Returned by `get_gift_card_balance` to show the card's
/// current balance, currency, and active status.
#[derive(Debug, Serialize)]
pub struct BalanceResult {
    /// Current balance in minor units (cents).
    pub balance_minor: i64,
    /// ISO-4217 currency code (e.g., "USD", "IDR").
    pub currency: String,
    /// Card status: "active", "frozen", or "redeemed".
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_result_debug() {
        let r = BalanceResult {
            balance_minor: 50000,
            currency: "IDR".into(),
            status: "active".into(),
        };
        let debug = format!("{r:?}");
        assert!(debug.contains("50000"));
        assert!(debug.contains("IDR"));
        assert!(debug.contains("active"));
    }

    #[test]
    fn balance_result_serialize() {
        let r = BalanceResult {
            balance_minor: 0,
            currency: "USD".into(),
            status: "frozen".into(),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["balance_minor"], 0);
        assert_eq!(json["currency"], "USD");
        assert_eq!(json["status"], "frozen");
    }

    #[test]
    fn balance_result_zero_and_empty() {
        let r = BalanceResult {
            balance_minor: 0,
            currency: String::new(),
            status: "active".into(),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["balance_minor"], 0);
        assert_eq!(json["currency"], "");
        assert_eq!(json["status"], "active");
    }
}

/// Issue a new gift card with an initial balance.
///
/// Creates a new gift card with a unique card number and stores
/// the initial loaded value. Returns the created gift card with
/// its transaction history.
#[command]
pub async fn issue_gift_card(
    input: IssueGiftCardInput,
    state: State<'_, AppState>,
) -> Result<GiftCardWithTransactions, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.issue_gift_card(input)?;
    drop(db);
    Ok(result)
}

/// Get a gift card by its card number or internal ID.
///
/// Looks up a gift card and returns it with all associated
/// transactions (issue, top-ups, redemptions).
#[command]
pub async fn get_gift_card(
    card_number_or_id: String,
    state: State<'_, AppState>,
) -> Result<Option<GiftCardWithTransactions>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.get_gift_card_detail(&card_number_or_id)?;
    drop(db);
    Ok(result)
}

/// List all gift cards with optional filtering by status.
///
/// Returns a list of gift cards with their transaction history.
/// Use the `filter` parameter to filter by card status (active, frozen, redeemed).
#[command]
pub async fn list_gift_cards(
    filter: GiftCardFilter,
    state: State<'_, AppState>,
) -> Result<Vec<GiftCardWithTransactions>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.list_gift_cards(filter)?;
    drop(db);
    Ok(result)
}

/// Get the current balance of a gift card.
///
/// Returns the balance in minor units, currency code, and card status.
/// Returns `None` if the card does not exist.
#[command]
pub async fn get_gift_card_balance(
    card_number_or_id: String,
    state: State<'_, AppState>,
) -> Result<Option<BalanceResult>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.get_gift_card_balance(&card_number_or_id)?;
    drop(db);
    Ok(
        result.map(|(balance_minor, currency, status)| BalanceResult {
            balance_minor,
            currency,
            status,
        }),
    )
}

/// Redeem (spend) a gift card balance against a sale.
///
/// Deducts the specified amount from the card balance and records
/// a redemption transaction. Returns the updated gift card with
/// transaction history.
#[command]
pub async fn redeem_gift_card(
    card_number_or_id: String,
    amount_minor: i64,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<RedeemGiftCardResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.redeem_gift_card(&card_number_or_id, amount_minor, &sale_id)?;
    drop(db);
    Ok(result)
}

/// Add value (top up) to an existing gift card.
///
/// Increases the card's balance by the specified amount and records
/// a top-up transaction. Returns the updated gift card with history.
#[command]
pub async fn top_up_gift_card(
    card_number_or_id: String,
    amount_minor: i64,
    state: State<'_, AppState>,
) -> Result<GiftCardWithTransactions, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.top_up_gift_card(&card_number_or_id, amount_minor)?;
    drop(db);
    Ok(result)
}

/// Freeze a gift card.
///
/// Prevents further transactions on the card (e.g., for fraud prevention
/// or customer request). The card balance is preserved.
#[command]
pub async fn freeze_gift_card(
    card_number_or_id: String,
    state: State<'_, AppState>,
) -> Result<GiftCard, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.freeze_gift_card(&card_number_or_id)?;
    drop(db);
    Ok(result)
}

/// Unfreeze a previously frozen gift card.
///
/// Restores normal transaction capabilities to the card.
#[command]
pub async fn unfreeze_gift_card(
    card_number_or_id: String,
    state: State<'_, AppState>,
) -> Result<GiftCard, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.unfreeze_gift_card(&card_number_or_id)?;
    drop(db);
    Ok(result)
}
