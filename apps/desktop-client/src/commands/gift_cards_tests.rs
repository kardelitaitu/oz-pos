
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
