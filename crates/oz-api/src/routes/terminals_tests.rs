
use super::*;

#[test]
fn hash_secret_is_stable_hex() {
    let a = hash_secret("secret-1");
    let b = hash_secret("secret-1");
    assert_eq!(a, b);
    assert_eq!(a.len(), 64);
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(a, hash_secret("secret-2"));
    assert!(!a.contains("secret"));
}

#[test]
fn verify_terminal_credentials_matches_only_correct_secret() {
    let conn = oz_core::migrations::fresh_db();
    let secret = generate_device_secret();
    conn.execute(
        "INSERT INTO sync_terminals (terminal_id, secret_hash, label)
         VALUES ('term-1', ?1, 'front')",
        rusqlite::params![hash_secret(&secret)],
    )
    .unwrap();

    assert!(
        verify_terminal_credentials(&conn, "term-1", &secret)
            .unwrap()
            .is_some()
    );
    assert!(
        verify_terminal_credentials(&conn, "term-1", "wrong-secret")
            .unwrap()
            .is_none()
    );
    assert!(
        verify_terminal_credentials(&conn, "unknown", &secret)
            .unwrap()
            .is_none()
    );
}
