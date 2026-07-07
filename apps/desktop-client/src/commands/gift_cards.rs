use serde::Serialize;
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::gift_card::{
    GiftCard, GiftCardFilter, GiftCardWithTransactions, IssueGiftCardInput, RedeemGiftCardResult,
};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct BalanceResult {
    pub balance_minor: i64,
    pub currency: String,
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
