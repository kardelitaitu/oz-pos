//! Short-lived HMAC-signed picker tickets for the pre-session
//! workspace picker (audit/06 residual).
//!
//! Parity with the desktop client: `staff_login` mints a ticket after
//! successful authentication. The pre-session `list_workspaces` /
//! `list_workspace_screens` commands verify it and resolve the REAL
//! user + role from the global identity database — caller-supplied
//! `role_id` / `user_id` are never trusted for listing.
//!
//! A ticket is `{user_id}.{expiry_ts}.{hex_hmac}` where the HMAC
//! covers `picker:{user_id}:{expiry_ts}` with a per-process secret
//! held in `AppState`. The short TTL (5 minutes) keeps the bootstrap
//! credential narrow: once an opaque session token exists, all
//! authenticated commands use it instead.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// How long a picker ticket stays valid, in seconds.
///
/// 5 minutes is enough for login → workspace selection; the opaque
/// session token created by `create_session` takes over afterwards.
pub const PICKER_TICKET_TTL_SECS: i64 = 300;

/// Sign a picker ticket for `user_id` valid until `expiry_ts`.
///
/// Format: `{user_id}.{expiry_ts}.{hex_hmac}` — the HMAC covers
/// `picker:{user_id}:{expiry_ts}` so neither the user nor the expiry
/// can be altered without the secret.
pub fn sign_picker_ticket(secret: &[u8], user_id: &str, expiry_ts: i64) -> String {
    let mut mac = new_mac(secret);
    mac.update(b"picker:");
    mac.update(user_id.as_bytes());
    mac.update(b":");
    mac.update(expiry_ts.to_string().as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());
    format!("{user_id}.{expiry_ts}.{sig}")
}

/// Build the HMAC-SHA256 instance for the picker-ticket domain.
///
/// HMAC-SHA256 accepts any key length, so `new_from_slice` cannot
/// fail for the byte slice we hand it (SHA-256 block size is 64 bytes,
/// far below the 255-byte HMAC limit). The `expect` is provably total.
fn new_mac(secret: &[u8]) -> HmacSha256 {
    // SAFETY: HMAC-SHA256 accepts any key length (SHA-256 block is 64 bytes,
    // far below the 255-byte HMAC limit), so new_from_slice cannot fail.
    HmacSha256::new_from_slice(secret).expect("HMAC-SHA256 accepts any key length")
}

/// Verify a picker ticket at time `now_ts`.
///
/// Returns the bound `user_id` when the signature is valid and the
/// ticket has not expired. Returns `None` for every failure mode
/// (forged, expired, malformed) so the caller surfaces one uniform
/// denial — the ticket cannot be used as an enumeration oracle.
///
/// Signature comparison uses `Mac::verify_slice` (constant-time),
/// never a byte-string equality that could leak the match position.
pub fn verify_picker_ticket(secret: &[u8], ticket: &str, now_ts: i64) -> Option<String> {
    let mut parts = ticket.splitn(3, '.');
    let user_id = parts.next()?;
    let expiry_ts: i64 = parts.next()?.parse().ok()?;
    let sig = parts.next()?;

    if user_id.is_empty() || expiry_ts < now_ts {
        return None;
    }

    let mut mac = new_mac(secret);
    mac.update(b"picker:");
    mac.update(user_id.as_bytes());
    mac.update(b":");
    mac.update(expiry_ts.to_string().as_bytes());

    let expected = hex::decode(sig).ok()?;
    mac.verify_slice(&expected).ok()?;

    Some(user_id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret() -> Vec<u8> {
        b"test-picker-ticket-secret".to_vec()
    }

    #[test]
    fn roundtrip_returns_user_id() {
        let sig = sign_picker_ticket(&secret(), "user-owner", 1_800_000_000);
        let user = verify_picker_ticket(&secret(), &sig, 1_799_900_000);
        assert_eq!(user.as_deref(), Some("user-owner"));
    }

    #[test]
    fn expired_ticket_is_rejected() {
        let sig = sign_picker_ticket(&secret(), "user-owner", 1_800_000_000);
        assert_eq!(verify_picker_ticket(&secret(), &sig, 1_800_000_301), None);
    }

    #[test]
    fn forged_ticket_is_rejected() {
        let sig = sign_picker_ticket(b"attacker-secret".as_slice(), "user-owner", 1_800_000_000);
        assert_eq!(verify_picker_ticket(&secret(), &sig, 1_799_900_000), None);
    }

    #[test]
    fn tampered_signature_is_rejected() {
        let sig = sign_picker_ticket(&secret(), "user-owner", 1_800_000_000);
        let tampered = format!("{}X", &sig[..sig.len() - 1]);
        assert_eq!(
            verify_picker_ticket(&secret(), &tampered, 1_799_900_000),
            None
        );
    }

    #[test]
    fn tampered_user_id_is_rejected() {
        let sig = sign_picker_ticket(&secret(), "user-owner", 1_800_000_000);
        let rest = sig.trim_start_matches("user-owner");
        let rebound = format!("user-cashier{rest}");
        assert_eq!(
            verify_picker_ticket(&secret(), &rebound, 1_799_900_000),
            None
        );
    }

    #[test]
    fn malformed_ticket_is_rejected() {
        assert_eq!(
            verify_picker_ticket(&secret(), "garbage", 1_799_900_000),
            None
        );
        assert_eq!(verify_picker_ticket(&secret(), "", 1_799_900_000), None);
        assert_eq!(
            verify_picker_ticket(&secret(), "user-owner.1800000000", 1_799_900_000),
            None
        );
    }

    #[test]
    fn ttl_constant_matches_window() {
        let sig = sign_picker_ticket(&secret(), "user-owner", 1_800_000_000);
        assert_eq!(
            verify_picker_ticket(&secret(), &sig, 1_800_000_000),
            Some("user-owner".to_owned())
        );
        assert_eq!(
            verify_picker_ticket(&secret(), &sig, 1_800_000_000 + 1),
            None
        );
        assert_eq!(PICKER_TICKET_TTL_SECS, 300);
    }
}
