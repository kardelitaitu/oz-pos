use super::*;

// ── CreateTaxRateRequest deserialization ────────────────────

#[test]
fn create_tax_rate_request_minimal() {
    let json = r#"{"name":"VAT 10%","rate_bps":1000,"is_default":true,"is_inclusive":false}"#;
    let req: CreateTaxRateRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.name, "VAT 10%");
    assert_eq!(req.rate_bps, 1000);
    assert!(req.is_default);
    assert!(!req.is_inclusive);
}

#[test]
fn create_tax_rate_request_inclusive() {
    let json = r#"{"name":"GST 5%","rate_bps":500,"is_default":false,"is_inclusive":true}"#;
    let req: CreateTaxRateRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.name, "GST 5%");
    assert_eq!(req.rate_bps, 500);
    assert!(!req.is_default);
    assert!(req.is_inclusive);
}
