use super::*;
use oz_security::Keyring;

#[test]
fn key_name_is_constant() {
    assert_eq!(ENCRYPTION_KEY_NAME, "oz-pos/encryption-key");
}

#[test]
fn rotation_status_defaults() {
    // When there's no key, status should reflect that.
    let keyring = oz_security::InMemoryKeyring::new();
    assert_eq!(keyring.key_created_at("test").unwrap(), None);
}

#[tokio::test]
async fn get_key_rotation_info_returns_status() {
    // Exercise the async thread-isolation bridge without platform
    // dependencies or a Secret Service/D-Bus session.
    let status = key_rotation_info_with(|| {
        Ok(Box::new(oz_security::InMemoryKeyring::new()) as Box<dyn Keyring>)
    })
    .await
    .unwrap();
    assert!(!status.has_key);
    assert!(status.created_at.is_none());
    assert!(status.age_days.is_none());
}

#[test]
fn key_rotation_info_reports_created_key() {
    let keyring = oz_security::InMemoryKeyring::new();
    keyring.rotate_key(ENCRYPTION_KEY_NAME).unwrap();

    let status = key_rotation_status(&keyring).unwrap();
    assert!(status.has_key);
    assert!(status.created_at.is_some());
    assert_eq!(status.age_days, Some(0));
}
