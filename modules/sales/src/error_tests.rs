//! Sibling unit tests for `error.rs` (AGENTS.md: no tests in production files).

use super::*;

#[test]
fn sales_error_validation_message() {
    let err = SalesError::validation("currency", "invalid currency code");
    assert!(matches!(
        err,
        SalesError::Validation { field, .. } if field == "currency"
    ));
    assert_eq!(
        format!("{err}"),
        "validation error on currency: invalid currency code"
    );
}

#[test]
fn sales_error_not_found_message() {
    let err = SalesError::NotFound {
        entity: "sale",
        id: "bad-id".into(),
    };
    assert_eq!(format!("{err}"), "not found: sale bad-id");
}

#[test]
fn sales_error_from_rusqlite() {
    let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
    let err = SalesError::from(rusqlite_err);
    assert!(matches!(err, SalesError::Db(_)));
}
