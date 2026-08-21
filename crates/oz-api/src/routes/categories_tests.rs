use super::*;
use axum::http::StatusCode;

// ── store_error_response ────────────────────────────────────

/// categories.rs uses a simplified store_error_response that
/// maps ALL CoreError variants to 500 (unlike the more granular
/// versions in products.rs or sales.rs).
#[test]
fn all_errors_return_500() {
    let errors = vec![
        CoreError::Internal("db fail".into()),
        CoreError::NotFound {
            entity: "category",
            id: "nope".into(),
        },
        CoreError::Validation {
            field: "name",
            message: "required".into(),
        },
        CoreError::Conflict {
            entity: "category",
            field: "name",
        },
        CoreError::Db(rusqlite::Error::InvalidParameterName("x".into())),
        CoreError::MoneyOverflow {
            left: 1,
            right: 1,
            currency: "USD".into(),
        },
        CoreError::CurrencyMismatch("USD".into(), "EUR".into()),
    ];
    for err in errors {
        let resp = store_error_response(err);
        assert_eq!(
            resp.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "all CoreError variants should return 500"
        );
    }
}
