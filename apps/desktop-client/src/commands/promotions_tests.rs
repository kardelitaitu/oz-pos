
use super::*;

#[test]
fn create_promotion_args_deserialize_minimal() {
    let json = r#"{"name":"10% Off","promo_type":"percentage","value_minor":10}"#;
    let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.name, "10% Off");
    assert_eq!(args.description, "");
    assert_eq!(args.promo_type, "percentage");
    assert_eq!(args.value_minor, 10);
    assert!(args.min_qty.is_none());
    assert!(args.trigger_sku.is_none());
    assert!(args.reward_sku.is_none());
    assert!(args.reward_qty.is_none());
    assert!(args.starts_at.is_none());
    assert!(args.ends_at.is_none());
    assert_eq!(args.min_order_minor, 0);
    assert!(args.category_id.is_none());
    assert!(args.active);
}

#[test]
fn create_promotion_args_deserialize_all_fields() {
    let json = r#"{
        "name": "Buy 2 Get 1",
        "description": "Buy two coffees, get one free",
        "promo_type": "buy_x_get_y",
        "value_minor": 100,
        "min_qty": 2,
        "trigger_sku": "COFFEE",
        "reward_sku": "COFFEE",
        "reward_qty": 1,
        "starts_at": "2026-01-01T00:00:00.000Z",
        "ends_at": "2026-12-31T23:59:59.000Z",
        "min_order_minor": 1000,
        "category_id": "cat-drinks",
        "active": true
    }"#;
    let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.name, "Buy 2 Get 1");
    assert_eq!(args.description, "Buy two coffees, get one free");
    assert_eq!(args.promo_type, "buy_x_get_y");
    assert_eq!(args.value_minor, 100);
    assert_eq!(args.min_qty, Some(2));
    assert_eq!(args.trigger_sku.as_deref(), Some("COFFEE"));
    assert_eq!(args.reward_sku.as_deref(), Some("COFFEE"));
    assert_eq!(args.reward_qty, Some(1));
    assert_eq!(args.min_order_minor, 1000);
    assert_eq!(args.category_id.as_deref(), Some("cat-drinks"));
    assert!(args.active);
}

#[test]
fn create_promotion_args_active_defaults_true() {
    let json = r#"{"name":"test","promo_type":"fixed_amount","value_minor":500}"#;
    let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
    assert!(args.active, "active should default to true");
}

#[test]
fn create_promotion_args_explicit_inactive() {
    let json =
        r#"{"name":"Disabled Promo","promo_type":"percentage","value_minor":5,"active":false}"#;
    let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
    assert!(!args.active);
}

#[test]
fn create_promotion_args_debug() {
    let args = CreatePromotionArgs {
        name: "Flash Sale".into(),
        description: "Limited time".into(),
        promo_type: "percentage".into(),
        value_minor: 20,
        min_qty: None,
        trigger_sku: None,
        reward_sku: None,
        reward_qty: None,
        starts_at: Some("2026-07-01T00:00:00.000Z".into()),
        ends_at: Some("2026-07-07T23:59:59.000Z".into()),
        min_order_minor: 0,
        category_id: None,
        active: true,
    };
    let debug = format!("{args:?}");
    assert!(debug.contains("Flash Sale"));
    assert!(debug.contains("percentage"));
    assert!(debug.contains("2026-07-01"));
}

#[test]
fn promotions_scoped_rejects_invalid_token() {
    let state = AppState::for_test();
    let result = state.resolve_session("nonexistent-token");
    assert!(matches!(result, Err(AppError::InvalidSession)));
}
