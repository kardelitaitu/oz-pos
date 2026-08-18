
use super::*;

#[test]
fn create_args_reject_legacy_actor_field() {
    let args: CreateStockCountArgs =
        serde_json::from_str(r#"{"countType":"full","notes":"cycle","countedBy":"forged"}"#)
            .unwrap();
    assert_eq!(args.count_type, "full");
    assert_eq!(args.notes, "cycle");
}

#[test]
fn complete_args_use_camel_case() {
    let args: CompleteStockCountArgs =
        serde_json::from_str(r#"{"countId":"count-1"}"#).unwrap();
    assert_eq!(args.count_id, "count-1");
}

#[test]
fn quantity_validation_rejects_negative_values() {
    assert!(validate_quantity("counted_qty", -1).is_err());
    assert!(validate_quantity("counted_qty", 0).is_ok());
}
