use super::*;

#[test]
fn create_promotion_args_deserialize_minimal() {
    let json = r#"{"name":"Summer Sale","promo_type":"percentage","value_minor":10}"#;
    let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.name, "Summer Sale");
    assert_eq!(args.promo_type, "percentage");
    assert_eq!(args.value_minor, 10);
    assert!(args.active);
    assert_eq!(args.min_order_minor, 0);
    assert_eq!(args.description, "");
}

#[test]
fn create_promotion_args_deserialize_all_fields() {
    let json = r#"{"name":"Flash Deal","description":"Limited time","promo_type":"fixed_amount","value_minor":500,"min_qty":2,"trigger_sku":"SKU-A","reward_sku":"SKU-B","reward_qty":1,"starts_at":"2026-01-01","ends_at":"2026-12-31","min_order_minor":5000,"category_id":"cat-1","active":true}"#;
    let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.name, "Flash Deal");
    assert_eq!(args.min_order_minor, 5000);
    assert_eq!(args.category_id.unwrap(), "cat-1");
}

#[test]
fn create_promotion_args_explicit_inactive() {
    let json = r#"{"name":"Draft","promo_type":"percentage","value_minor":5,"active":false}"#;
    let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
    assert!(!args.active);
}

#[test]
fn create_promotion_args_debug() {
    let args = CreatePromotionArgs {
        name: "Test".into(),
        description: "Desc".into(),
        promo_type: "percentage".into(),
        value_minor: 10,
        min_qty: None,
        trigger_sku: None,
        reward_sku: None,
        reward_qty: None,
        starts_at: None,
        ends_at: None,
        min_order_minor: 0,
        category_id: None,
        active: true,
    };
    let debug = format!("{:?}", args);
    assert!(debug.contains("Test"));
}
