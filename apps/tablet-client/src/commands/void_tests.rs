use super::*;

#[test]
fn void_sale_args_deserialize() {
    let json = r#"{"sale_id":"s1","user_id":"u1","reason":"customer cancelled"}"#;
    let args: VoidSaleArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.sale_id, "s1");
    assert_eq!(args.user_id, "u1");
    assert_eq!(args.reason, "customer cancelled");
}

#[test]
fn void_sale_args_debug() {
    let args = VoidSaleArgs {
        sale_id: "s2".into(),
        user_id: "u2".into(),
        reason: "wrong item".into(),
    };
    let debug = format!("{:?}", args);
    assert!(debug.contains("s2"));
    assert!(debug.contains("wrong item"));
}

#[test]
fn void_sale_args_deserialize_empty_reason() {
    let json = r#"{"sale_id":"s3","user_id":"u3","reason":""}"#;
    let args: VoidSaleArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.sale_id, "s3");
    assert_eq!(args.reason, "");
}
