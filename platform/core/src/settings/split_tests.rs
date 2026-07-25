//! Split-sanity tests for the R5 mechanical refactor of `settings.rs`.
//!
//! These tests do not re-prove business logic; they verify that the
//! module split preserved the public API surface and re-exports.

use super::test_helpers::fresh;
use super::{Settings, keys};

/// Verifies that the public re-export paths for `Settings` and `keys`
/// are still reachable from outside the crate after the split.
#[test]
fn public_api_paths_exist_after_split() {
    let conn = fresh();
    let _ = Settings::get(&conn, keys::STORE_NAME);
    assert!(!keys::STORE_NAME.is_empty());
    assert!(!keys::RECEIPT_FOOTER.is_empty());
    // The same constants are reachable through the public `settings::keys` path.
    assert_eq!(crate::settings::keys::STORE_NAME, "store.name");
    assert_eq!(crate::settings::keys::DEFAULT_CURRENCY, "currency.default");
}

/// A typed helper and the raw `Settings::get` with the matching key
/// constant should read and write the same row.
#[test]
fn typed_and_raw_helpers_share_key_constants() {
    let conn = fresh();
    Settings::set_store_name(&conn, "Split Test Store").unwrap();
    let via_typed = Settings::get_store_name(&conn).unwrap();
    let via_raw = Settings::get(&conn, keys::STORE_NAME).unwrap();
    assert_eq!(via_typed, via_raw);
}

/// Ensure the well-known key constants have the expected string values.
/// This catches accidental renames during the mechanical split.
#[test]
fn well_known_keys_have_expected_values() {
    assert_eq!(keys::STORE_NAME, "store.name");
    assert_eq!(keys::DEFAULT_CURRENCY, "currency.default");
    assert_eq!(keys::RECEIPT_FOOTER, "receipt.footer");
    assert_eq!(keys::SYNC_ENABLED, "sync_enabled");
}

/// Reading via a typed helper after writing via a raw helper should work,
/// confirming the split did not create duplicate key namespaces.
#[test]
fn raw_write_round_trips_through_typed_helper() {
    let conn = fresh();
    Settings::set(&conn, keys::STORE_ADDRESS, "123 Split Ave").unwrap();
    assert_eq!(
        Settings::get_store_address(&conn).unwrap(),
        Some("123 Split Ave".into())
    );
}
