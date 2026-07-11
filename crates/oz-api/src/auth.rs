//! JSON Web Token generation and validation for the OZ-POS OpenAPI.
//!
//! Tokens are signed with HS256 and carry an `exp` (expiration) claim.
//! The signing secret is loaded from the `OZ_API_SECRET` env var at
//! startup. Every request to a protected route must include an
//! `Authorization: Bearer <token>` header.
//!
//! Token generation via `POST /api/v1/tokens` returns the JWT string
//! and the expiry timestamp. There is no revocation list in this pass;
//! tokens are valid until their `exp` claim expires.

use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

const DEFAULT_EXPIRY_HOURS: i64 = 24;

/// The payload embedded in every API token.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiTokenClaims {
    /// Standard JWT subject — a human-readable label for this token.
    pub sub: String,
    /// Token identifier (UUID v4).
    pub jti: String,
    /// Standard JWT expiry (epoch seconds).
    pub exp: usize,
    /// When the token was issued (epoch seconds).
    pub iat: usize,
    /// Tenant / store ID for multi-tenant cloud isolation.
    /// `None` for single-store deployments (backward compatible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Response body returned when a new token is created.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// The signed JWT string. Pass this as `Authorization: Bearer <token>`.
    pub token: String,
    /// ISO-8601 expiry timestamp for display.
    pub expires_at: String,
    /// Token identifier (same as `jti` in claims).
    pub token_id: String,
}

/// Load the signing secret from the environment.
///
/// Falls back to a hard-coded dev secret if `OZ_API_SECRET` is unset,
/// so the server starts in development without extra config. Production
/// deployments MUST set `OZ_API_SECRET`.
fn signing_secret() -> String {
    std::env::var("OZ_API_SECRET")
        .unwrap_or_else(|_| "oz-pos-dev-secret-change-in-production".into())
}

/// Generate a new signed JWT with the given subject label, optionally
/// scoped to a tenant.
///
/// The token expires after `expiry_hours` (default 24). Returns the
/// signed token string and its expiry timestamp.
pub fn create_token(
    subject: &str,
    expiry_hours: Option<i64>,
    tenant_id: Option<&str>,
) -> TokenResponse {
    let hours = expiry_hours.unwrap_or(DEFAULT_EXPIRY_HOURS);
    let now = Utc::now();
    let exp_time = now + Duration::hours(hours);
    let token_id = uuid::Uuid::now_v7().to_string();

    let claims = ApiTokenClaims {
        sub: subject.to_owned(),
        jti: token_id.clone(),
        exp: exp_time.timestamp() as usize,
        iat: now.timestamp() as usize,
        tenant_id: tenant_id.map(|s| s.to_owned()),
    };

    let secret = signing_secret();
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());
    let token =
        encode(&Header::default(), &claims, &encoding_key).expect("token encoding is infallible");

    TokenResponse {
        token,
        expires_at: exp_time.to_rfc3339(),
        token_id,
    }
}

/// Validate a JWT and return its claims.
///
/// Returns `Ok(claims)` if the token is valid and not expired.
pub fn validate_token(token_str: &str) -> Result<ApiTokenClaims, jsonwebtoken::errors::Error> {
    let secret = signing_secret();
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::default();
    validation.validate_exp = true;
    decode::<ApiTokenClaims>(token_str, &decoding_key, &validation).map(|data| data.claims)
}

/// Axum middleware that rejects requests without a valid JWT.
///
/// Attach to protected routes via `Router::layer(from_fn(auth_middleware))`.
pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match validate_token(token) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_validate() {
        let resp = create_token("test-script", Some(1), None);
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
        let resp = create_token("tamper", Some(24), None);
        // Append junk to invalidate the signature.
        let bad = format!("{}x", resp.token);
        assert!(validate_token(&bad).is_err());
    }

    #[test]
    fn expired_token_is_rejected() {
        // Create a token that was already expired 1 hour ago.
        let resp = create_token("expired", Some(-1), None);
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
        let resp = create_token("default-exp", None, None);
        assert!(!resp.token.is_empty());
        assert!(!resp.expires_at.is_empty());
        assert!(!resp.token_id.is_empty());
        let claims = validate_token(&resp.token).unwrap();
        assert_eq!(claims.sub, "default-exp");
    }

    #[test]
    fn token_id_is_uuid_v4_format() {
        let resp = create_token("uuid-test", Some(1), None);
        assert_eq!(resp.token_id.len(), 36, "UUID v4 should be 36 chars");
        assert_eq!(
            resp.token_id.chars().filter(|c| *c == '-').count(),
            4,
            "UUID should have 4 hyphens"
        );
    }

    #[test]
    fn expires_at_is_valid_rfc3339() {
        let resp = create_token("rfc3339", Some(1), None);
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
        let resp = create_token("fields", Some(1), None);
        let claims = validate_token(&resp.token).unwrap();
        assert!(!claims.sub.is_empty());
        assert!(!claims.jti.is_empty());
        assert!(claims.exp > 0);
        assert!(claims.iat > 0);
    }

    #[test]
    fn claims_exp_is_after_iat() {
        let resp = create_token("time-order", Some(1), None);
        let claims = validate_token(&resp.token).unwrap();
        assert!(claims.exp > claims.iat, "exp should be after iat");
    }

    #[test]
    fn two_tokens_have_different_ids() {
        let a = create_token("a", Some(1), None);
        let b = create_token("b", Some(1), None);
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
        let resp = create_token("zero", Some(0), None);
        assert!(!resp.token.is_empty());
        assert!(!resp.token_id.is_empty());
        assert!(!resp.expires_at.is_empty());
    }
}
