use super::*;

#[test]
fn roundtrip_api_key() {
    let machine_id = "abc123def456789";
    let original = "oz_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    let encrypted = encrypt_api_key(original, machine_id).unwrap();
    let decrypted = decrypt_api_key(&encrypted, machine_id).unwrap();

    assert_eq!(decrypted, original);
}

#[test]
fn roundtrip_empty_string() {
    let encrypted = encrypt_api_key("", "machine-1").unwrap();
    let decrypted = decrypt_api_key(&encrypted, "machine-1").unwrap();
    assert_eq!(decrypted, "");
}

#[test]
fn different_machine_id_fails() {
    let encrypted = encrypt_api_key("secret", "machine-a").unwrap();
    let result = decrypt_api_key(&encrypted, "machine-b");
    assert!(
        result.is_err(),
        "decryption with wrong machine ID should fail"
    );
}

#[test]
fn same_plaintext_produces_different_ciphertext() {
    // Each encryption uses a fresh random nonce, so ciphertext
    // should differ across calls.
    let c1 = encrypt_api_key("secret", "machine-1").unwrap();
    let c2 = encrypt_api_key("secret", "machine-1").unwrap();
    assert_ne!(c1, c2, "nonce should produce distinct ciphertexts");
}

#[test]
fn corrupted_ciphertext_fails() {
    let encrypted = encrypt_api_key("secret", "machine-1").unwrap();
    // Flip a byte in the base64 string
    let mut chars: Vec<char> = encrypted.chars().collect();
    chars[5] = if chars[5] == 'A' { 'B' } else { 'A' };
    let corrupted: String = chars.into_iter().collect();

    let result = decrypt_api_key(&corrupted, "machine-1");
    assert!(
        result.is_err(),
        "corrupted ciphertext should fail decryption"
    );
}

#[test]
fn empty_ciphertext_fails() {
    let result = decrypt_api_key("", "machine-1");
    assert!(result.is_err());
}

#[test]
fn too_short_ciphertext_fails() {
    // 11 bytes of base64 (too short for 12-byte nonce + 16-byte tag)
    let result = decrypt_api_key("YWJj", "machine-1");
    assert!(result.is_err());
}

#[test]
fn key_is_deterministic() {
    let k1 = derive_key(API_KEY_DOMAIN, "machine-1");
    let k2 = derive_key(API_KEY_DOMAIN, "machine-1");
    assert_eq!(k1, k2);
}

#[test]
fn different_machine_ids_produce_different_keys() {
    let k1 = derive_key(API_KEY_DOMAIN, "machine-a");
    let k2 = derive_key(API_KEY_DOMAIN, "machine-b");
    assert_ne!(k1, k2);
}

#[test]
fn smtp_password_roundtrip() {
    let machine_id = "test-machine-123";
    let original = "my-smtp-password";

    let encrypted = encrypt_smtp_password(original, machine_id).unwrap();
    let decrypted = decrypt_smtp_password(&encrypted, machine_id).unwrap();

    assert_eq!(decrypted, original);
}

#[test]
fn smtp_legacy_plaintext_passthrough() {
    // Legacy plaintext passwords are returned unchanged.
    let result = decrypt_smtp_password("plaintext-password", "machine-1").unwrap();
    assert_eq!(result, "plaintext-password");
}

#[test]
fn profile_field_roundtrip() {
    let original = "3201010101010001";
    let encrypted = encrypt_profile_field(original).unwrap();
    let decrypted = decrypt_profile_field(&encrypted).unwrap();
    assert_eq!(decrypted, original);
}

#[test]
fn profile_field_uses_fresh_nonce() {
    let c1 = encrypt_profile_field("123456789").unwrap();
    let c2 = encrypt_profile_field("123456789").unwrap();
    assert_ne!(c1, c2, "each encryption must use a fresh nonce");
}

#[test]
fn profile_field_decrypt_garbage_fails_closed() {
    assert!(decrypt_profile_field("garbage").is_err());
    assert!(decrypt_profile_field("").is_err());
    // A value from another domain must not decrypt.
    let smtp = encrypt_smtp_at_rest("not-a-national-id");
    assert!(decrypt_profile_field(&smtp).is_err());
}

#[test]
fn api_and_smtp_domains_are_isolated() {
    let machine_id = "machine-1";
    let plaintext = "shared-secret";

    let api_encrypted = encrypt_api_key(plaintext, machine_id).unwrap();
    let _smtp_encrypted = encrypt_smtp_password(plaintext, machine_id).unwrap();

    // Same plaintext, same machine → different ciphertext (due to nonce),
    // but more importantly: API-key ciphertext should NOT decrypt as SMTP.
    let smtp_decrypt_of_api = decrypt_smtp_password(&api_encrypted, machine_id).unwrap();
    // Since smtp_decrypt falls back to plaintext on failure, a corrupted
    // decrypt returns the ciphertext itself — NOT the original plaintext.
    assert_ne!(
        smtp_decrypt_of_api, plaintext,
        "API-key ciphertext should not decrypt with SMTP domain key"
    );
}
