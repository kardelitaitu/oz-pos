//! EDC command DTO tests.
//!
//! The DTO conversions and serialisation are unit-tested here; the
//! commands themselves are exercised through the mock terminal in the
//! desktop-client integration path.

use super::{EdcResultDto, EdcStatusDto};
use oz_payment::drivers::edc::PaymentResult;

#[test]
fn status_dto_serializes() {
    let dto = EdcStatusDto {
        status: "ready".into(),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["status"], "ready");
}

#[test]
fn result_dto_from_payment_result() {
    let result = PaymentResult {
        success: true,
        transaction_id: Some("mock-txn-001".into()),
        auth_code: Some("MOCKAUTH".into()),
        card_scheme: Some("Visa".into()),
        card_last4: Some("1111".into()),
        message: "approved".into(),
    };
    let dto: EdcResultDto = result.into();
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["transactionId"], "mock-txn-001");
    assert_eq!(json["authCode"], "MOCKAUTH");
    assert_eq!(json["cardScheme"], "Visa");
    assert_eq!(json["cardLast4"], "1111");
    assert_eq!(json["message"], "approved");
}

#[test]
fn result_dto_failure_shape() {
    let result = PaymentResult {
        success: false,
        transaction_id: None,
        auth_code: None,
        card_scheme: None,
        card_last4: None,
        message: "declined".into(),
    };
    let dto: EdcResultDto = result.into();
    assert!(!dto.success);
    assert!(dto.transaction_id.is_none());
    assert_eq!(dto.message, "declined");
}
