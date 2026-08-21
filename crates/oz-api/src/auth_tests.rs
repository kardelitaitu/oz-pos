use super::*;

#[test]
fn create_and_validate() {
    let resp = create_token("test-script", Some(1), None, None).unwrap();
    let claims = validate_token(&resp.token).unwrap();
    assert_eq!(claims.sub, "test-script");
    assert_eq!(claims.jti, resp.token_id);
}

#[test]
fn bad_token_is_rejected() {
    assert!(validate_token("not.a.jwt").is_err());
}

#[test]
fn tampered_token_is_rejected() {
    let resp = create_token("tamper", Some(24), None, None).unwrap();
    // Append junk to invalidate the signature.
    let bad = format!("{}x", resp.token);
    assert!(validate_token(&bad).is_err());
}

#[test]
fn expired_token_is_rejected() {
    // Create a token that was already expired 1 hour ago.
    let resp = create_token("expired", Some(-1), None, None).unwrap();
    let result = validate_token(&resp.token);
    assert!(result.is_err(), "expired token should be rejected");
}

#[test]
fn empty_token_is_rejected() {
    assert!(validate_token("").is_err());
}

#[test]
fn whitespace_only_token_is_rejected() {
    assert!(validate_token("   ").is_err());
}

#[test]
fn create_token_default_expiry_works() {
    // None expiry should default to 24 hours and produce a valid token.
    let resp = create_token("default-exp", None, None, None).unwrap();
    assert!(!resp.token.is_empty());
    assert!(!resp.expires_at.is_empty());
    assert!(!resp.token_id.is_empty());
    let claims = validate_token(&resp.token).unwrap();
    assert_eq!(claims.sub, "default-exp");
}

#[test]
fn token_id_is_uuid_v4_format() {
    let resp = create_token("uuid-test", Some(1), None, None).unwrap();
    assert_eq!(resp.token_id.len(), 36, "UUID v4 should be 36 chars");
    assert_eq!(
        resp.token_id.chars().filter(|c| *c == '-').count(),
        4,
        "UUID should have 4 hyphens"
    );
}

#[test]
fn expires_at_is_valid_rfc3339() {
    let resp = create_token("rfc3339", Some(1), None, None).unwrap();
    // RFC 3339: "2025-01-15T10:30:00+00:00" or "2025-01-15T10:30:00Z"
    assert!(
        resp.expires_at.contains('T'),
        "should contain 'T' separator"
    );
    assert!(
        resp.expires_at.ends_with('Z') || resp.expires_at.contains('+'),
        "should end with Z or contain timezone offset"
    );
    // Should be parseable by chrono.
    let parsed = chrono::DateTime::parse_from_rfc3339(&resp.expires_at);
    assert!(
        parsed.is_ok(),
        "expires_at should parse as RFC 3339: {}",
        resp.expires_at
    );
}

#[test]
fn claims_have_non_empty_fields() {
    let resp = create_token("fields", Some(1), None, None).unwrap();
    let claims = validate_token(&resp.token).unwrap();
    assert!(!claims.sub.is_empty());
    assert!(!claims.jti.is_empty());
    assert!(claims.exp > 0);
    assert!(claims.iat > 0);
}

#[test]
fn claims_exp_is_after_iat() {
    let resp = create_token("time-order", Some(1), None, None).unwrap();
    let claims = validate_token(&resp.token).unwrap();
    assert!(claims.exp > claims.iat, "exp should be after iat");
}

#[test]
fn two_tokens_have_different_ids() {
    let a = create_token("a", Some(1), None, None).unwrap();
    let b = create_token("b", Some(1), None, None).unwrap();
    assert_ne!(a.token_id, b.token_id, "each token should have a unique ID");
    assert_ne!(a.token, b.token, "each token should have a unique JWT");
}

#[test]
fn token_response_serialization() {
    let resp = TokenResponse {
        token: "fake.jwt.here".into(),
        expires_at: "2025-06-15T12:00:00Z".into(),
        token_id: "550e8400-e29b-41d4-a716-446655440000".into(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"token\":\"fake.jwt.here\""));
    assert!(json.contains("\"expires_at\":\"2025-06-15T12:00:00Z\""));
    assert!(json.contains("\"token_id\":\"550e8400-e29b-41d4-a716-446655440000\""));
}

#[test]
fn token_with_zero_hour_expiry_is_well_formed() {
    // 0-hour expiry: token may or may not be valid depending on
    // clock precision, but it should always be structurally correct.
    let resp = create_token("zero", Some(0), None, None).unwrap();
    assert!(!resp.token.is_empty());
    assert!(!resp.token_id.is_empty());
    assert!(!resp.expires_at.is_empty());
}
